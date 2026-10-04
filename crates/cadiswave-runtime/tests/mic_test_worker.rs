use cadiswave_runtime::process;
#[allow(
    dead_code,
    reason = "Compile private worker seams for isolated fixtures."
)]
#[path = "../src/calibration.rs"]
mod calibration;
#[allow(
    dead_code,
    reason = "Compile private worker seams for isolated fixtures."
)]
#[path = "../src/mic_test.rs"]
mod mic_test;
#[allow(
    dead_code,
    reason = "Compile private worker seams for isolated fixtures."
)]
#[path = "../src/recovery.rs"]
mod recovery;

use cadiswave_core::{
    mic_test::{MicTestToken, PlaybackTarget},
    model::{ErrorCode, NodeIdentity, SourceId},
};
use mic_test::{MicTestResult, MicTestWorker, playback_with};
use std::{
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

fn token(session: u64) -> MicTestToken {
    MicTestToken {
        session,
        source: SourceId::new("mic").unwrap(),
        node_name: "raw_mic".into(),
        identity: NodeIdentity {
            server_cookie: 1,
            object_serial: "19".into(),
        },
        channels: 1,
    }
}
fn output() -> PlaybackTarget {
    PlaybackTarget {
        node_name: "headphones".into(),
        identity: NodeIdentity {
            server_cookie: 1,
            object_serial: "20".into(),
        },
    }
}

#[test]
fn recording_requires_completion_and_cancellation_discards_owned_pcm() {
    let (played, playback) = mpsc::channel();
    let (mut worker, events) = MicTestWorker::start_with(
        Arc::new(|_, _, _| Ok(vec![0; 96_000])),
        Arc::new(move |_, target, bytes, channels, _, _| {
            played
                .send((target.clone(), bytes.len(), channels))
                .unwrap();
            Ok(())
        }),
    )
    .unwrap();
    worker.record(1, token(1), 1).unwrap();
    assert!(matches!(
        events.recv_timeout(Duration::from_secs(2)).unwrap().result,
        Ok(MicTestResult::Recorded(_))
    ));
    worker.play(1, token(1), output()).unwrap();
    assert_eq!(
        playback.recv_timeout(Duration::from_secs(2)).unwrap(),
        (output(), 96_000, 1)
    );
    assert!(matches!(
        events.recv_timeout(Duration::from_secs(2)).unwrap().result,
        Ok(MicTestResult::Played)
    ));
    worker.cancel(1).unwrap();
    assert_eq!(
        worker.play(1, token(1), output()).unwrap_err().code,
        ErrorCode::Identity
    );
    worker.stop().unwrap();
}

#[test]
fn replaced_recordings_reject_stale_completion_and_retain_only_current_audio() {
    let (started, ready) = mpsc::channel();
    let (mut worker, events) = MicTestWorker::start_with(
        Arc::new(move |token, _, cancel| {
            if token.session == 1 {
                started.send(()).unwrap();
                let deadline = Instant::now() + Duration::from_secs(2);
                while !cancel.load(Ordering::Acquire) {
                    assert!(Instant::now() < deadline);
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
            Ok(vec![0; 96_000])
        }),
        Arc::new(|_, _, _, _, _, _| Ok(())),
    )
    .unwrap();
    worker.record(1, token(1), 1).unwrap();
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    worker.record(2, token(2), 1).unwrap();
    let old = events.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(old.token.session, 1);
    assert_eq!(old.result.unwrap_err().code, ErrorCode::Cancelled);
    assert_eq!(
        events
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .token
            .session,
        2
    );
    assert!(worker.play(1, token(1), output()).is_err());
    worker.stop().unwrap();
}

#[test]
fn cancelled_playback_never_spawns_and_deadline_reaps_a_stalled_child() {
    let error = playback_with(
        Arc::new(AtomicBool::new(true)),
        Duration::from_secs(1),
        || panic!("cancelled operation spawned"),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::Cancelled);
    let mut pid = 0;
    let error = playback_with(
        Arc::new(AtomicBool::new(false)),
        Duration::from_millis(50),
        || {
            let child = process::OwnedChild::spawn("sleep", &["30".into()], Stdio::null())?;
            pid = child.id();
            Ok(child)
        },
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::Unavailable);
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
}

#[test]
fn playback_identity_rejects_cookie_serial_class_and_missing_serial_changes() {
    struct Graph(serde_json::Value);
    impl recovery::Commands for Graph {
        fn cancelled(&self) -> bool {
            false
        }
        fn run(
            &self,
            program: &str,
            _: &[String],
            _: Duration,
            restoration: bool,
        ) -> cadiswave_core::model::Result<Vec<u8>> {
            assert_eq!(program, "pw-dump");
            assert!(!restoration);
            Ok(serde_json::to_vec(&self.0).unwrap())
        }
    }
    let original = serde_json::json!([
        {"type":"PipeWire:Interface:Core","info":{"cookie":1}},
        {"id":20,"type":"PipeWire:Interface:Node","info":{"state":"running","props":{"node.name":"headphones","media.class":"Audio/Sink","object.serial":"20"}}}
    ]);
    mic_test::validate_output(&output(), &Graph(original.clone())).unwrap();
    for change in ["cookie", "serial", "class", "missing_serial"] {
        let mut graph = original.clone();
        match change {
            "cookie" => graph[0]["info"]["cookie"] = serde_json::json!(2),
            "serial" => graph[1]["info"]["props"]["object.serial"] = serde_json::json!("21"),
            "class" => graph[1]["info"]["props"]["media.class"] = serde_json::json!("Audio/Source"),
            _ => {
                graph[1]["info"]["props"]
                    .as_object_mut()
                    .unwrap()
                    .remove("object.serial");
            }
        }
        assert_eq!(
            mic_test::validate_output(&output(), &Graph(graph))
                .unwrap_err()
                .code,
            ErrorCode::Identity,
            "accepted {change}"
        );
    }
}

#[test]
fn playback_input_uses_anonymous_memory_with_the_supervised_child() {
    use std::io::{Read, Seek, SeekFrom, Write};
    let fd = rustix::fs::memfd_create(
        "cadiswave-mic-test-fixture",
        rustix::fs::MemfdFlags::CLOEXEC,
    )
    .unwrap();
    let mut input = std::fs::File::from(fd);
    input.write_all(b"private pcm fixture").unwrap();
    input.seek(SeekFrom::Start(0)).unwrap();
    let mut child =
        process::OwnedChild::spawn_with_stdin("cat", &[], Stdio::from(input), Stdio::piped())
            .unwrap();
    let pid = child.id();
    let mut bytes = Vec::new();
    child
        .take_stdout()
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(bytes, b"private pcm fixture");
    child.terminate().unwrap();
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
}

#[test]
fn recording_bursts_have_bounded_admission_and_keep_the_last_accepted_session() {
    let release = Arc::new(AtomicBool::new(false));
    let gate = release.clone();
    let (started, ready) = mpsc::channel();
    let (mut worker, events) = MicTestWorker::start_with(
        Arc::new(move |token, _, _| {
            if token.session == 1 {
                started.send(()).unwrap();
                while !gate.load(Ordering::Acquire) {
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
            Ok(vec![0; 96_000])
        }),
        Arc::new(|_, _, _, _, _, _| Ok(())),
    )
    .unwrap();
    struct Release(Arc<AtomicBool>);
    impl Drop for Release {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }
    let _release = Release(release.clone());
    worker.record(1, token(1), 1).unwrap();
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    worker.record(2, token(2), 1).unwrap();
    worker.record(3, token(3), 1).unwrap();
    assert_eq!(
        worker.record(4, token(4), 1).unwrap_err().code,
        ErrorCode::Busy
    );
    release.store(true, Ordering::Release);
    assert_eq!(
        events
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .result
            .unwrap_err()
            .code,
        ErrorCode::Cancelled
    );
    assert_eq!(
        events
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .result
            .unwrap_err()
            .code,
        ErrorCode::Cancelled
    );
    let last = events.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(last.token.session, 3);
    assert!(last.result.is_ok());
    worker.stop().unwrap();
}

#[test]
fn worker_shutdown_cancels_and_reaps_its_active_playback_child() {
    let (started, ready) = mpsc::channel();
    let (mut worker, events) = MicTestWorker::start_with(
        Arc::new(|_, _, _| Ok(vec![0; 96_000])),
        Arc::new(move |_, _, _, _, _, cancel| {
            playback_with(cancel, Duration::from_secs(30), || {
                let child = process::OwnedChild::spawn("sleep", &["30".into()], Stdio::null())?;
                started.send(child.id()).unwrap();
                Ok(child)
            })
        }),
    )
    .unwrap();
    worker.record(1, token(1), 1).unwrap();
    assert!(
        events
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .result
            .is_ok()
    );
    worker.play(2, token(1), output()).unwrap();
    let pid = ready.recv_timeout(Duration::from_secs(2)).unwrap();
    let start = Instant::now();
    worker.stop().unwrap();
    assert!(start.elapsed() < Duration::from_secs(3));
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    assert_eq!(
        events
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .result
            .unwrap_err()
            .code,
        ErrorCode::Cancelled
    );
    assert_eq!(
        worker.record(3, token(2), 1).unwrap_err().code,
        ErrorCode::Cancelled
    );
}

#[test]
fn stereo_signed_pcm_reaches_playback_without_channel_or_polarity_changes() {
    let bytes = [0x00, 0x20, 0x00, 0xf0].repeat(48_000);
    let expected = bytes.clone();
    let (mut worker, events) = MicTestWorker::start_with(
        Arc::new(move |_, _, _| Ok(bytes.clone())),
        Arc::new(move |_, _, actual, channels, seconds, _| {
            assert_eq!(channels, 2);
            assert_eq!(seconds, 1);
            assert_eq!(actual, expected);
            Ok(())
        }),
    )
    .unwrap();
    let mut stereo = token(1);
    stereo.channels = 2;
    worker.record(1, stereo.clone(), 1).unwrap();
    let MicTestResult::Recorded(metrics) = events
        .recv_timeout(Duration::from_secs(2))
        .unwrap()
        .result
        .unwrap()
    else {
        panic!("recording result expected");
    };
    assert!((metrics.peak_db + 12.041199826559248).abs() < 1e-12);
    worker.play(2, stereo, output()).unwrap();
    assert!(
        events
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .result
            .is_ok()
    );
    worker.stop().unwrap();
}

#[test]
fn capture_identity_rejects_live_replacement_class_and_format_changes() {
    struct Graph(serde_json::Value);
    impl recovery::Commands for Graph {
        fn cancelled(&self) -> bool {
            false
        }
        fn run(
            &self,
            program: &str,
            _: &[String],
            _: Duration,
            restoration: bool,
        ) -> cadiswave_core::model::Result<Vec<u8>> {
            assert_eq!(program, "pw-dump");
            assert!(!restoration);
            Ok(serde_json::to_vec(&self.0).unwrap())
        }
    }
    let original = serde_json::json!([
        {"type":"PipeWire:Interface:Core","info":{"cookie":1}},
        {"id":19,"type":"PipeWire:Interface:Node","info":{"state":"running","props":{"node.name":"raw_mic","media.class":"Audio/Source","object.serial":"19","audio.channels":1}}}
    ]);
    mic_test::validate_capture_token(&token(1), &Graph(original.clone())).unwrap();
    for change in [
        "class",
        "cookie",
        "serial",
        "channels",
        "missing_class",
        "missing_serial",
    ] {
        let mut graph = original.clone();
        match change {
            "class" => graph[1]["info"]["props"]["media.class"] = serde_json::json!("Audio/Sink"),
            "cookie" => graph[0]["info"]["cookie"] = serde_json::json!(2),
            "serial" => graph[1]["info"]["props"]["object.serial"] = serde_json::json!("21"),
            "channels" => graph[1]["info"]["props"]["audio.channels"] = serde_json::json!(2),
            "missing_serial" => {
                graph[1]["info"]["props"]
                    .as_object_mut()
                    .unwrap()
                    .remove("object.serial");
            }
            _ => {
                graph[1]["info"]["props"]
                    .as_object_mut()
                    .unwrap()
                    .remove("media.class");
            }
        }
        assert!(
            mic_test::validate_capture_token(&token(1), &Graph(graph)).is_err(),
            "accepted {change}"
        );
    }
}
