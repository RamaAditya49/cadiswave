use cadiswave_core::mic_test::{measure, validate_capture, validate_duration};

#[test]
fn duration_and_target_bounds_reject_automatic_or_processed_capture() {
    for seconds in [0, 11, u32::MAX] {
        assert!(validate_duration(seconds).is_err());
    }
    for seconds in [1, 5, 10] {
        assert!(validate_duration(seconds).is_ok());
    }
    for node in [
        "",
        "auto",
        "0",
        "-1",
        "cadiswave_fx_mic",
        "cadiswave_capture",
        "alsa.monitor",
        "raw\0node",
    ] {
        assert!(validate_capture(node, 5, 1).is_err(), "accepted {node:?}");
    }
    assert!(validate_capture("alsa_input.usb-mic", 5, 2).is_ok());
    assert!(validate_capture("alsa_input.usb-mic", 5, 3).is_err());
}

#[test]
fn metrics_include_each_channel_and_report_clipping_without_rejecting_audio() {
    let mut raw = vec![0; 192_000];
    raw[2..4].copy_from_slice(&i16::MIN.to_le_bytes());
    let metrics = measure(&raw, 2, 1).unwrap();
    assert_eq!(metrics.frames, 48_000);
    assert_eq!(metrics.peak_db, 0.0);
    assert_eq!(metrics.clipped_samples, 1);
    assert!(metrics.clipping);
    let silence = measure(&vec![0; 96_000], 1, 1).unwrap();
    assert_eq!(silence.peak_db, -140.0);
    assert_eq!(silence.clipped_samples, 0);
    assert!(!silence.clipping);
}

#[test]
fn incomplete_or_excess_pcm_never_becomes_a_ready_recording() {
    for length in [0, 1, 95_998, 96_001, 192_000] {
        assert!(measure(&vec![0; length], 1, 1).is_err());
    }
}

#[test]
fn public_microphone_test_state_contains_metadata_without_private_identity() {
    use cadiswave_core::{mic_test::*, model::*};
    let mut snapshot = AppSnapshot::default();
    assert_eq!(snapshot.action_mic_test().unwrap(), "null");
    snapshot.mic_test = Some(MicTestSnapshot {
        token: MicTestToken {
            session: 12,
            source: SourceId::new("mic").unwrap(),
            node_name: "private microphone node".into(),
            identity: NodeIdentity {
                server_cookie: 1234567,
                object_serial: "private_serial".into(),
            },
            channels: 2,
        },
        seconds: 1,
        phase: MicTestPhase::Ready,
        metrics: Some(MicTestMetrics {
            frames: 48_000,
            peak_db: -12.0,
            clipped_samples: 0,
            clipping: false,
        }),
        output: None,
    });
    let encoded = snapshot.action_mic_test().unwrap();
    let value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(value["phase"], "ready");
    assert_eq!(value["session"], 12);
    assert_eq!(value["source"], "mic");
    assert_eq!(value["metrics"]["peak_db"], -12.0);
    assert!(!encoded.contains("private"));
    assert!(!encoded.contains("1234567"));
    assert!(!encoded.contains("channels"));
    assert!(!encoded.contains("pcm"));
}

#[test]
fn public_microphone_test_failure_reports_the_reason_without_audio() {
    use cadiswave_core::{mic_test::*, model::*};
    let snapshot = AppSnapshot {
        mic_test: Some(MicTestSnapshot {
            token: MicTestToken {
                session: 1,
                source: SourceId::new("mic").unwrap(),
                node_name: "raw".into(),
                identity: NodeIdentity {
                    server_cookie: 71,
                    object_serial: "private_serial".into(),
                },
                channels: 2,
            },
            seconds: 1,
            phase: MicTestPhase::Failed("Microphone test playback exceeded its deadline".into()),
            metrics: None,
            output: None,
        }),
        ..AppSnapshot::default()
    };
    let encoded = snapshot.action_mic_test().unwrap();
    let value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(
        value["error"],
        "Microphone test playback exceeded its deadline"
    );
    assert!(!encoded.contains("private_serial"));
    assert!(value["metrics"].is_null());
}
