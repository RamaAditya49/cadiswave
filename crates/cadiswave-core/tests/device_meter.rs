use cadiswave_core::{
    device_meter::selected_channels,
    model::*,
    pcm::ChannelPeaks,
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
    let unit = UnitSnapshot {
        id,
        info: DeviceInfo {
            serial: "A".into(),
            api: "1".into(),
            firmware: "1".into(),
        },
        state: Observation::Known(
            ConfigBuffer::decode(ProfileId::WaveXlr, &[0; 34])
                .unwrap()
                .state(),
        ),
        desired_mute: None,
        input_peak: 0.0,
        output_peak: 0.0,
        errors: vec![],
    };
    let mut source = Source::new("Mic".into(), SourceKind::Device);
    source.id = SourceId::new("mic").unwrap();
    source.node_name = "capture".into();
    let mut snapshot = AppSnapshot {
        units: Arc::new(vec![unit]),
        selected_unit: Some(id),
        ..Default::default()
    };
    Arc::make_mut(&mut snapshot.desired)
        .sources
        .insert(source.id.clone(), source);
    Arc::make_mut(&mut snapshot.captures).push(CaptureSnapshot {
        identity: NodeIdentity {
            server_cookie: 1,
            object_serial: "1".into(),
        },
        node_id: 1,
        node_name: "capture".into(),
        name: "Mic".into(),
        muted: Observation::Known(false),
        channels: Some(2),
        properties: serde_json::json!({"device.serial":"A"})
            .as_object()
            .unwrap()
            .clone(),
    });
    Arc::make_mut(&mut snapshot.channel_meters).insert(
        "src:mic".into(),
        ChannelPeaks::Stereo {
            left: 0.5,
            right: 0.25,
        },
    );
    snapshot
}
#[test]
fn display_requires_a_unique_selected_serial_and_capture() {
    let mut snapshot = fixture();
    assert_eq!(
        selected_channels(&snapshot),
        Some(ChannelPeaks::Stereo {
            left: 0.5,
            right: 0.25
        })
    );
    let capture = snapshot.captures[0].clone();
    Arc::make_mut(&mut snapshot.captures).push(capture);
    assert_eq!(selected_channels(&snapshot), None);
    snapshot = fixture();
    Arc::make_mut(&mut snapshot.captures)[0].properties.clear();
    assert_eq!(selected_channels(&snapshot), None);
    snapshot = fixture();
    let mut unit = snapshot.units[0].clone();
    unit.id.address = 3;
    Arc::make_mut(&mut snapshot.units).push(unit);
    assert_eq!(selected_channels(&snapshot), None);
    snapshot = fixture();
    snapshot.channel_meters = Arc::default();
    assert_eq!(selected_channels(&snapshot), None);
}
