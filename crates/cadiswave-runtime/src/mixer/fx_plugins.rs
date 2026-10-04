use serde_json::Value;
use std::{ffi::OsStr, fs, os::unix::fs::MetadataExt, path::Path};

fn trusted_plugin(path: &Path) -> bool {
    let uid = rustix::process::geteuid().as_raw();
    for ancestor in path.ancestors() {
        let Ok(info) = fs::symlink_metadata(ancestor) else {
            return false;
        };
        if info.file_type().is_symlink() || !matches!(info.uid(), 0) && info.uid() != uid {
            return false;
        }
        if ancestor == path {
            if !info.is_file() || info.nlink() != 1 || info.mode() & 0o022 != 0 {
                return false;
            }
        } else {
            // A root-owned sticky directory protects each owner's child entry.
            let sticky_root = info.uid() == 0 && info.mode() & 0o1000 != 0;
            if !info.is_dir() || info.mode() & 0o022 != 0 && !sticky_root {
                return false;
            }
        }
    }
    true
}

/// Resolve known SWH plugins without replacing an operator's search path.
pub(super) fn resolve(config: &mut Value, prefix: Option<&Path>, operator_path: Option<&OsStr>) {
    if operator_path.is_some() {
        return;
    }
    let Some(prefix) = prefix.filter(|path| path.is_absolute()) else {
        return;
    };
    let Some(modules) = config
        .get_mut("context.modules")
        .and_then(Value::as_array_mut)
    else {
        return;
    };
    for module in modules {
        if module["name"] != "libpipewire-module-filter-chain" {
            continue;
        }
        let Some(nodes) = module
            .pointer_mut("/args/filter.graph/nodes")
            .and_then(Value::as_array_mut)
        else {
            continue;
        };
        for node in nodes {
            if node["type"] != "ladspa" {
                continue;
            }
            let Some(plugin @ ("gate_1410" | "sc4m_1916")) = node["plugin"].as_str() else {
                continue;
            };
            let path = prefix.join("lib/ladspa").join(format!("{plugin}.so"));
            if trusted_plugin(&path)
                && let Some(path) = path.to_str()
            {
                node["plugin"] = Value::String(path.to_owned());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        fs,
        os::unix::fs::{PermissionsExt, symlink},
    };

    fn config() -> Value {
        json!({"context.modules":[{"name":"libpipewire-module-filter-chain","args":{"filter.graph":{"nodes":[
            {"type":"ladspa","plugin":"gate_1410","label":"gate"},
            {"type":"ladspa","plugin":"sc4m_1916","label":"sc4m"},
            {"type":"ladspa","plugin":"custom_plugin","label":"custom"},
            {"type":"builtin","plugin":"gate_1410","label":"copy"}
        ]}}}]})
    }
    fn plugins(prefix: &Path) -> std::path::PathBuf {
        let directory = prefix.join("lib/ladspa");
        fs::create_dir_all(&directory).unwrap();
        for name in ["gate_1410", "sc4m_1916"] {
            let path = directory.join(format!("{name}.so"));
            fs::write(&path, b"fixture; never loaded").unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o644)).unwrap();
        }
        directory
    }
    fn nodes(config: &Value) -> &Value {
        &config["context.modules"][0]["args"]["filter.graph"]["nodes"]
    }

    #[test]
    fn known_plugins_resolve_under_the_installed_prefix_only() {
        let root = tempfile::tempdir().unwrap();
        let directory = plugins(root.path());
        let mut config = config();
        resolve(&mut config, Some(root.path()), None);
        assert_eq!(
            nodes(&config)[0]["plugin"],
            directory.join("gate_1410.so").to_str().unwrap()
        );
        assert_eq!(
            nodes(&config)[1]["plugin"],
            directory.join("sc4m_1916.so").to_str().unwrap()
        );
        assert_eq!(nodes(&config)[2]["plugin"], "custom_plugin");
        assert_eq!(nodes(&config)[3]["plugin"], "gate_1410");
    }
    #[test]
    fn absent_plugins_and_explicit_operator_paths_keep_pipewire_resolution() {
        let root = tempfile::tempdir().unwrap();
        let original = config();
        for prefix in [None, Some(root.path())] {
            let mut config = original.clone();
            resolve(&mut config, prefix, None);
            assert_eq!(config, original);
        }
        plugins(root.path());
        for path in [OsStr::new("/operator/plugins"), OsStr::new("")] {
            let mut config = original.clone();
            resolve(&mut config, Some(root.path()), Some(path));
            assert_eq!(config, original);
        }
    }
    #[test]
    fn linked_and_writable_plugins_are_never_selected() {
        let root = tempfile::tempdir().unwrap();
        let directory = plugins(root.path());
        let target = directory.join("gate_1410.so");
        let original = directory.join("original.so");
        fs::rename(&target, &original).unwrap();
        symlink(&original, &target).unwrap();
        fs::set_permissions(
            directory.join("sc4m_1916.so"),
            fs::Permissions::from_mode(0o666),
        )
        .unwrap();
        let original = config();
        let mut config = original.clone();
        resolve(&mut config, Some(root.path()), None);
        assert_eq!(config, original);
        fs::set_permissions(
            directory.join("sc4m_1916.so"),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        fs::hard_link(directory.join("sc4m_1916.so"), directory.join("linked.so")).unwrap();
        resolve(&mut config, Some(root.path()), None);
        assert_eq!(config, original);
    }
    #[test]
    fn writable_or_linked_plugin_directories_are_never_selected() {
        let root = tempfile::tempdir().unwrap();
        let directory = plugins(root.path());
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o777)).unwrap();
        let original = config();
        let mut config = original.clone();
        resolve(&mut config, Some(root.path()), None);
        assert_eq!(config, original);
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o755)).unwrap();
        let moved = root.path().join("moved");
        fs::rename(&directory, &moved).unwrap();
        symlink(&moved, &directory).unwrap();
        resolve(&mut config, Some(root.path()), None);
        assert_eq!(config, original);
    }
}
