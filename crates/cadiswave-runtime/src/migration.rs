//! Import validated legacy state without replacing existing application files.
use crate::paths::open_directory_for_uid;
use cadiswave_core::{model::*, scenes};
use rustix::fs::{Mode, OFlags, RenameFlags};
use std::{
    fs::File,
    io::{Read, Write},
    os::fd::OwnedFd,
    path::{Path, PathBuf},
};

const FILES: [&str; 5] = [
    "sources.json",
    "mixdefs.json",
    "mixes.json",
    "ui-state.json",
    "scenes.json",
];
const LIMIT: u64 = 4 * 1024 * 1024;
#[derive(Debug)]
struct Entry {
    name: String,
    bytes: Vec<u8>,
    identity: (u64, u64),
}
#[derive(Debug)]
pub struct LegacyImport {
    source: PathBuf,
    destination: PathBuf,
    entries: Vec<Entry>,
    identity: (u64, u64),
}
fn io_error(error: rustix::io::Errno) -> OperationError {
    std::io::Error::from(error).into()
}
fn identity(fd: &OwnedFd) -> Result<(u64, u64)> {
    let stat = rustix::fs::fstat(fd).map_err(io_error)?;
    Ok((stat.st_dev, stat.st_ino))
}
type FileData = (Vec<u8>, (u64, u64));
fn read(dir: &OwnedFd, name: &str) -> Result<Option<FileData>> {
    let fd = match rustix::fs::openat(
        dir,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    ) {
        Ok(fd) => fd,
        Err(rustix::io::Errno::NOENT) => return Ok(None),
        Err(e) => return Err(io_error(e)),
    };
    let stat = rustix::fs::fstat(&fd).map_err(io_error)?;
    if rustix::fs::FileType::from_raw_mode(stat.st_mode) != rustix::fs::FileType::RegularFile
        || stat.st_size < 0
        || stat.st_size as u64 > LIMIT
    {
        return Err(OperationError::invalid(
            "Legacy state must be a bounded regular file",
        ));
    }
    let mut bytes = Vec::new();
    File::from(fd).take(LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LIMIT {
        return Err(OperationError::invalid(
            "Legacy state exceeds the import limit",
        ));
    }
    Ok(Some((bytes, (stat.st_dev, stat.st_ino))))
}
fn validate(name: &str, bytes: &[u8]) -> Result<()> {
    let value = serde_json::from_slice(bytes)?;
    match name {
        "sources.json" => {
            normalize_sources(value)?;
        }
        "mixdefs.json" => {
            normalize_mixes(value)?;
        }
        "mixes.json" => {
            MatrixState::from_value(value)?;
        }
        "ui-state.json" => {
            Preferences::from_value(value)?;
        }
        "scenes.json" => {
            scenes::decode_store(value)?;
        }
        _ => return Err(OperationError::invalid("Unknown legacy state file")),
    }
    Ok(())
}
pub fn inspect_legacy(source: &Path, destination: &Path) -> Result<LegacyImport> {
    let uid = rustix::process::geteuid().as_raw();
    let directory = open_directory_for_uid(source, false, uid)?;
    if crate::paths::has_symlink_ancestor(destination)? {
        return Err(OperationError::invalid("Linked import destination"));
    }
    let mut entries = Vec::new();
    for name in FILES {
        if let Some((bytes, identity)) = read(&directory, name)? {
            validate(name, &bytes)?;
            entries.push(Entry {
                name: name.into(),
                bytes,
                identity,
            });
        }
    }
    Ok(LegacyImport {
        source: source.into(),
        destination: destination.into(),
        entries,
        identity: identity(&directory)?,
    })
}
impl LegacyImport {
    pub fn apply(&self) -> Result<()> {
        let uid = rustix::process::geteuid().as_raw();
        let source = open_directory_for_uid(&self.source, false, uid)?;
        if identity(&source)? != self.identity {
            return Err(OperationError::invalid(
                "Legacy directory changed after inspection",
            ));
        }
        for entry in &self.entries {
            if read(&source, &entry.name)?.as_ref() != Some(&(entry.bytes.clone(), entry.identity))
            {
                return Err(OperationError::invalid(
                    "Legacy state changed after inspection",
                ));
            }
        }
        let destination = open_directory_for_uid(&self.destination, true, uid)?;
        for entry in &self.entries {
            let temporary = format!(".cadiswave-import-{}", uuid::Uuid::new_v4());
            let fd = rustix::fs::openat(
                &destination,
                &temporary,
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            )
            .map_err(io_error)?;
            let mut file = File::from(fd);
            let result = (|| -> Result<()> {
                file.write_all(&entry.bytes)?;
                file.sync_all()?;
                match rustix::fs::renameat_with(
                    &destination,
                    &temporary,
                    &destination,
                    &entry.name,
                    RenameFlags::NOREPLACE,
                ) {
                    Ok(()) | Err(rustix::io::Errno::EXIST) => Ok(()),
                    Err(e) => Err(io_error(e)),
                }
            })();
            let _ = rustix::fs::unlinkat(&destination, &temporary, rustix::fs::AtFlags::empty());
            result?;
        }
        Ok(())
    }
}
