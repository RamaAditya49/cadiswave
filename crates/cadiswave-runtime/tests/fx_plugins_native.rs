use cadiswave_core::{effects, model::*, voice_presets::BuiltinPreset};
use cadiswave_runtime::{
    mixer::{GraphBackend, SubprocessPipeWire},
    paths::RuntimePaths,
};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, atomic::AtomicBool},
    thread,
    time::{Duration, Instant},
};

fn graph() -> Value {
    let output = Command::new("pw-dump").output().unwrap();
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).unwrap()
}
fn node(graph: &Value, name: &str) -> Option<u64> {
    graph.as_array()?.iter().find(|node| {
        node.pointer("/info/props/node.name")
            .and_then(Value::as_str)
            == Some(name)
    })?["id"]
        .as_u64()
}
fn ports(graph: &Value, id: u64, direction: &str) -> Vec<(String, u64)> {
    let mut ports: Vec<_> = graph
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|port| {
            let props = port.pointer("/info/props")?;
            let owner = props["node.id"]
                .as_u64()
                .or_else(|| props["node.id"].as_str()?.parse().ok())?;
            if port["type"] != "PipeWire:Interface:Port"
                || owner != id
                || props["port.direction"] != direction
            {
                return None;
            }
            Some((
                props["audio.channel"].as_str()?.to_owned(),
                port["id"].as_u64()?,
            ))
        })
        .collect();
    ports.sort();
    ports
}
fn configure_stereo(id: u64, direction: &str) {
    let parameter = serde_json::json!({"direction":direction,"mode":"dsp","format":{
        "mediaType":"audio","mediaSubtype":"raw","format":"F32P","rate":48000,
        "channels":2,"position":["FL","FR"]
    }});
    let output = Command::new("pw-cli")
        .args([
            "set-param",
            &id.to_string(),
            "PortConfig",
            &parameter.to_string(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "Requires a private PipeWire namespace and trusted staged SWH plugins."]
fn meeting_podcast_and_streaming_load_real_plugins_and_keep_stereo_links() {
    assert_eq!(
        std::env::var("CADISWAVE_NATIVE_DSP_FIXTURE").as_deref(),
        Ok("1")
    );
    assert_eq!(std::env::var("XDG_RUNTIME_DIR").as_deref(), Ok("/work/run"));
    assert!(!Path::new("/dev/snd").exists());
    assert!(std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_none());
    let prefix = PathBuf::from("/plugin-prefix");
    let paths = RuntimePaths {
        executable: prefix.join("bin/cadiswave"),
        prefix: Some(prefix.clone()),
        data: prefix.join("share/cadiswave"),
        identity: prefix.clone(),
        maintenance: PathBuf::from("/maintenance"),
        source: None,
    };
    let mut backend = SubprocessPipeWire::new(paths, Arc::new(AtomicBool::new(false)));
    for preset in [
        BuiltinPreset::Meeting,
        BuiltinPreset::Podcast,
        BuiltinPreset::Streaming,
    ] {
        let mut source = Source::new("Private DSP fixture".into(), SourceKind::Device);
        source.id = SourceId::new("private_dsp").unwrap();
        source.node_name = "fixture_mic".into();
        source.fx = Some(preset.settings());
        let owner = "private_dsp_owner";
        let name = effects::fx_node_name(&source.id);
        let cap = format!("{name}_cap");
        let mut config: Value = serde_json::from_str(
            &effects::render_fx_config(&source, 2, owner)
                .unwrap()
                .unwrap(),
        )
        .unwrap();
        backend.resolve_filter_plugins(&mut config);
        let filter = config["context.modules"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|module| module["name"] == "libpipewire-module-filter-chain")
            .unwrap();
        filter["args"]["capture.props"]["node.autoconnect"] = Value::Bool(false);
        for plugin in filter["args"]["filter.graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|node| node["type"] == "ladspa")
        {
            assert!(
                plugin["plugin"]
                    .as_str()
                    .unwrap()
                    .starts_with("/plugin-prefix/lib/ladspa/")
            );
        }
        let file = tempfile::Builder::new()
            .prefix("cadiswave-fx-")
            .suffix(".conf")
            .tempfile()
            .unwrap();
        let mut rendered = serde_json::to_vec_pretty(&config).unwrap();
        rendered.push(b'\n');
        std::fs::write(file.path(), rendered).unwrap();
        let mut child = backend.spawn_filter(file.path()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let (input, output, raw) = loop {
            assert!(child.running().unwrap(), "{preset:?} filter exited");
            let graph = graph();
            if let (Some(input), Some(output), Some(raw)) = (
                node(&graph, &cap),
                node(&graph, &name),
                node(&graph, "fixture_mic"),
            ) {
                break (input, output, raw);
            }
            assert!(Instant::now() < deadline, "{preset:?} nodes did not appear");
            thread::sleep(Duration::from_millis(50));
        };
        // The private fixture has no session manager to configure adapter ports.
        configure_stereo(raw, "Output");
        configure_stereo(input, "Input");
        configure_stereo(output, "Output");
        let observed = graph();
        let raw_ports = ports(&observed, raw, "out");
        let inputs = ports(&observed, input, "in");
        assert_eq!(raw_ports.len(), 2);
        assert_eq!(inputs.len(), 2);
        assert_eq!(ports(&observed, output, "out").len(), 2);
        for ((a, source), (b, target)) in raw_ports.iter().zip(&inputs) {
            assert_eq!(a, b);
            assert!(
                Command::new("pw-link")
                    .args([source.to_string(), target.to_string()])
                    .status()
                    .unwrap()
                    .success()
            );
        }
        thread::sleep(Duration::from_secs(2));
        assert!(
            child.running().unwrap(),
            "{preset:?} filter exited after linking"
        );
        let observed = graph();
        for id in [input, output] {
            let owned = observed
                .as_array()
                .unwrap()
                .iter()
                .find(|node| node["id"] == id)
                .unwrap();
            assert_eq!(
                owned
                    .pointer("/info/props/cadiswave.owner")
                    .and_then(Value::as_str),
                Some(owner)
            );
        }
        for ((_, source), (_, target)) in raw_ports.iter().zip(&inputs) {
            assert!(observed.as_array().unwrap().iter().any(|link| link["type"]
                == "PipeWire:Interface:Link"
                && link.pointer("/info/output-port-id").and_then(Value::as_u64) == Some(*source)
                && link.pointer("/info/input-port-id").and_then(Value::as_u64) == Some(*target)));
        }
        child.terminate().unwrap();
        println!("{preset:?}: real plugins loaded, owned nodes and stereo links remained alive");
    }
}
