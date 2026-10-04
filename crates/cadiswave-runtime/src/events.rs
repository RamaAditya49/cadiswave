//! Read service events without blocking the desktop thread.
use cadiswave_core::model::{OperationError, Result};
use std::{
    collections::VecDeque,
    io::{ErrorKind, Read},
    process::Stdio,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
const LINES: usize = 200;
const BYTES: usize = 4096;
#[derive(Default)]
pub struct EventBuffer(VecDeque<String>);
impl EventBuffer {
    pub fn push(&mut self, mut line: String) {
        let mut end = line.len().min(BYTES);
        while !line.is_char_boundary(end) {
            end -= 1;
        }
        line.truncate(end);
        if self.0.len() == LINES {
            self.0.pop_front();
        }
        self.0.push_back(line);
    }
    pub fn latest(&self) -> Vec<String> {
        self.0.iter().cloned().collect()
    }
}
pub struct EventTail {
    buffer: Arc<Mutex<EventBuffer>>,
    error: Arc<Mutex<Option<String>>>,
    cancel: Arc<AtomicBool>,
    done: mpsc::Receiver<()>,
    worker: Option<JoinHandle<()>>,
}
impl EventTail {
    pub fn start() -> Result<Self> {
        let buffer = Arc::new(Mutex::new(EventBuffer::default()));
        let error = Arc::new(Mutex::new(None));
        let cancel = Arc::new(AtomicBool::new(false));
        let (sender, done) = mpsc::channel();
        let (b, e, c) = (buffer.clone(), error.clone(), cancel.clone());
        let worker = thread::Builder::new()
            .name("cadiswave-events".into())
            .spawn(move || {
                let result = read_events(&b, &c);
                if let Err(failure) = result {
                    *e.lock().unwrap_or_else(|p| p.into_inner()) = Some(failure.to_string());
                }
                let _ = sender.send(());
            })?;
        Ok(Self {
            buffer,
            error,
            cancel,
            done,
            worker: Some(worker),
        })
    }
    pub fn latest(&self) -> Vec<String> {
        self.buffer
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .latest()
    }
    pub fn error(&self) -> Option<String> {
        self.error.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
    pub fn stop(&mut self) -> Result<()> {
        if self.worker.is_none() {
            return Ok(());
        }
        self.cancel.store(true, Ordering::Release);
        self.done
            .recv_timeout(Duration::from_secs(2))
            .map_err(|_| OperationError::unavailable("Event reader cleanup is pending"))?;
        self.worker
            .take()
            .unwrap()
            .join()
            .map_err(|_| OperationError::unavailable("Event reader failed"))?;
        Ok(())
    }
}
impl Drop for EventTail {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
fn read_events(buffer: &Mutex<EventBuffer>, cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        return Ok(());
    }
    let mut child = crate::process::OwnedChild::spawn(
        "journalctl",
        &[
            "--user",
            "-u",
            "cadiswave.service",
            "-n",
            "200",
            "-f",
            "-o",
            "cat",
            "--no-pager",
        ]
        .map(str::to_owned),
        Stdio::piped(),
    )?;
    let result = (|| {
        let mut stdout = child
            .take_stdout()
            .ok_or_else(|| OperationError::unavailable("Event output is unavailable"))?;
        crate::meter::nonblocking(&stdout)?;
        let mut bytes = [0; 1024];
        let mut line = Vec::with_capacity(BYTES);
        while !cancel.load(Ordering::Acquire) {
            match stdout.read(&mut bytes) {
                Ok(0) => break,
                Ok(count) => {
                    for byte in &bytes[..count] {
                        if *byte == b'\n' {
                            buffer
                                .lock()
                                .unwrap_or_else(|p| p.into_inner())
                                .push(String::from_utf8_lossy(&line).into_owned());
                            line.clear();
                        } else if line.len() < BYTES {
                            line.push(*byte);
                        }
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(25))
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e.into()),
            }
        }
        if !line.is_empty() {
            buffer
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .push(String::from_utf8_lossy(&line).into_owned());
        }
        Ok(())
    })();
    let cleanup = child.terminate();
    result.and(cleanup)
}
