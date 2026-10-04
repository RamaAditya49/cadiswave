use cadiswave_core::{
    device_settings::{capture_rate, microphone_sources},
    model::*,
    profiles::ProfileId,
    protocol::{ConfigBuffer, DeviceInfo},
};
use std::sync::Arc;

fn fixture() -> AppSnapshot {
    let id = UnitId {
        profile: ProfileId::WaveXlr,
        bus: 1,
        address: 2,
        incarnation: 1,
    };
    let mut source = Source::new("Mic".into(), SourceKind::Device);
    source.id = SourceId::new("mic").unwrap();
    source.node_name = "raw_mic".into();
    let mut snapshot = AppSnapshot {
        selected_unit: Some(id),
        lifecycle: Lifecycle::Running,
        units: Arc::new(vec![UnitSnapshot {
            id,
            info: DeviceInfo {
                serial: "A".into(),
                firmware: "1".into(),
                api: "1".into(),
            },
            state: Observation::Known(ConfigBuffer::decode(id.profile, &[0; 34]).unwrap().state()),
            desired_mute: None,
            input_peak: 0.0,
            output_peak: 0.0,
            errors: vec![],
        }]),
        captures: Arc::new(vec![CaptureSnapshot {
            identity: NodeIdentity {
                server_cookie: 1,
                object_serial: "10".into(),
            },
            node_id: 10,
            node_name: "raw_mic".into(),
            name: "Raw Mic".into(),
            channels: Some(1),
            muted: Observation::Known(false),
            properties: serde_json::json!({"device.serial":"A", "audio.rate":48000})
                .as_object()
                .unwrap()
                .clone(),
        }]),
        ..Default::default()
    };
    Arc::make_mut(&mut snapshot.desired)
        .sources
        .insert(source.id.clone(), source);
    snapshot
}

#[test]
fn software_controls_resolve_only_exact_selected_capture_sources() {
    let mut snapshot = fixture();
    assert_eq!(
        microphone_sources(&snapshot),
        vec![SourceId::new("mic").unwrap()]
    );
    snapshot.selected_unit = None;
    assert!(microphone_sources(&snapshot).is_empty());
    snapshot = fixture();
    Arc::make_mut(&mut snapshot.captures)[0].properties["device.serial"] = "B".into();
    assert!(microphone_sources(&snapshot).is_empty());
    snapshot = fixture();
    let copy = snapshot.captures[0].clone();
    Arc::make_mut(&mut snapshot.captures).push(copy);
    assert!(microphone_sources(&snapshot).is_empty());
    snapshot = fixture();
    let mut unit = snapshot.units[0].clone();
    unit.id.address = 3;
    Arc::make_mut(&mut snapshot.units).push(unit);
    assert!(microphone_sources(&snapshot).is_empty());
}

#[test]
fn rate_observations_require_valid_numbers_and_identify_their_scope() {
    let mut capture = fixture().captures[0].clone();
    let rate = capture_rate(&capture).unwrap();
    assert_eq!(rate.hz, 48000);
    assert!(!rate.negotiated);
    capture
        .properties
        .insert("cadiswave.observed-rate".into(), 96000.into());
    let rate = capture_rate(&capture).unwrap();
    assert_eq!(rate.hz, 96000);
    assert!(rate.negotiated);
    capture.properties.remove("cadiswave.observed-rate");
    for value in [
        serde_json::json!(0),
        serde_json::json!("auto"),
        serde_json::json!(true),
        serde_json::json!(48000.5),
    ] {
        capture.properties.insert("audio.rate".into(), value);
        assert_eq!(capture_rate(&capture), None);
    }
}

#[test]
fn monitor_balance_uses_the_exact_profile_range() {
    use cadiswave_core::device_settings::monitor_setting;
    assert_eq!(
        monitor_setting(ProfileId::Wave3, 50.0).unwrap(),
        DeviceSetting::MonitorMix(12800)
    );
    assert_eq!(
        monitor_setting(ProfileId::XlrDockMk2, 50.0).unwrap(),
        DeviceSetting::MonitorMix(100)
    );
    for percent in [0.0, 50.0, 100.0] {
        assert_eq!(
            monitor_setting(ProfileId::WaveXlr, percent).unwrap(),
            DeviceSetting::MonitorMix((percent * 256.0) as u16)
        );
        assert!(monitor_setting(ProfileId::WaveXlrMk2, percent).is_err());
    }
    for percent in [f64::NAN, f64::INFINITY, -1.0, 101.0] {
        assert!(monitor_setting(ProfileId::Wave3, percent).is_err());
    }
}
