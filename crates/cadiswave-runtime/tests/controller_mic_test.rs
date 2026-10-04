use cadiswave_core::{effects::FxSettings, mic_test::*, model::*};
use cadiswave_runtime::{
    controller::{
        AppCommand, Backend, BackendCommand, BackendEvent, RuntimeEvent, RuntimeEvents,
        RuntimeHandle, ShutdownError,
    },
    mic_test::{MicTestEvent, MicTestResult},
    mixer::MixerObservation,
    uninstall::{UninstallPlan, UninstallResult},
};
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    path::{Path, PathBuf},
    pin::pin,
    sync::{Arc, mpsc},
    task::{Context, Poll, Wake, Waker},
    thread,
    time::{Duration, Instant},
};

fn playback_target(name: &str) -> PlaybackTarget {
    PlaybackTarget {
        node_name: name.into(),
        identity: NodeIdentity {
            server_cookie: 71,
            object_serial: "222".into(),
        },
    }
}

struct FixtureBackend {
    root: PathBuf,
    commands: mpsc::Sender<BackendCommand>,
    events: mpsc::Receiver<BackendEvent>,
}
impl Backend for FixtureBackend {
    fn identity(&self) -> &Path {
        &self.root
    }
    fn dispatch(&mut self, command: BackendCommand) -> Result<()> {
        self.commands
            .send(command)
            .map_err(|_| OperationError::unavailable("fixture receiver closed"))
    }
    fn next_event(&mut self) -> Option<BackendEvent> {
        self.events.try_recv().ok()
    }
    fn shutdown(&mut self) -> std::result::Result<(), ShutdownError> {
        Ok(())
    }
    fn prepare_removal(&mut self, _: &UninstallPlan, _: bool) -> Result<()> {
        panic!("calibration cannot prepare removal")
    }
    fn remove(&mut self, _: &UninstallPlan, _: bool) -> Result<UninstallResult> {
        panic!("calibration cannot remove an installation")
    }
}
struct ThreadWake(thread::Thread);
impl Wake for ThreadWake {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

struct Fixture {
    _root: tempfile::TempDir,
    handle: RuntimeHandle,
    events: RuntimeEvents,
    incoming: mpsc::Sender<BackendEvent>,
    commands: mpsc::Receiver<BackendCommand>,
    outcomes: HashMap<CommandId, CommandOutcome>,
    completed: HashSet<CommandId>,
    record_job: u64,
    source: SourceId,
    capture: CaptureSnapshot,
}
impl Fixture {
    fn new(channels: Option<u32>) -> Self {
        Self::with_observation(channels, true)
    }
    fn with_observation(channels: Option<u32>, observe: bool) -> Self {
        let root = tempfile::tempdir().unwrap();
        let mut row = Source::new("Fixture microphone".into(), SourceKind::Device);
        row.id = SourceId::new("mic").unwrap();
        row.node_name = "fixture_raw".into();
        row.extra.insert("channels".into(), serde_json::json!(8)); // Persisted metadata must not determine capture format.
        row.fx = Some(FxSettings {
            eq_low: 2.0,
            eq_mid: -1.0,
            delay_ms: 42.0,
            ..FxSettings::default()
        });
        let sources: Sources = [(row.id.clone(), row.clone())].into_iter().collect();
        std::fs::write(
            root.path().join("sources.json"),
            serde_json::to_vec(&sources).unwrap(),
        )
        .unwrap();
        let capture = CaptureSnapshot {
            identity: NodeIdentity {
                server_cookie: 71,
                object_serial: "1234".into(),
            },
            node_id: 9,
            node_name: row.node_name.clone(),
            name: row.name,
            muted: Observation::Known(false),
            channels,
            properties: serde_json::json!({"object.serial":1234, "media.class":"Audio/Source"})
                .as_object()
                .unwrap()
                .clone(),
        };
        let (send_commands, commands) = mpsc::channel();
        let (incoming, receive_events) = mpsc::channel();
        let identity = root.path().to_owned();
        let (handle, events) = RuntimeHandle::start_with(root.path().to_owned(), move || {
            Ok(Box::new(FixtureBackend {
                root: identity,
                commands: send_commands,
                events: receive_events,
            }))
        })
        .unwrap();
        let fixture = Self {
            _root: root,
            handle,
            events,
            incoming,
            commands,
            outcomes: HashMap::new(),
            completed: HashSet::new(),
            record_job: 0,
            source: row.id,
            capture,
        };
        if observe {
            fixture.graph(fixture.capture.clone());
            fixture.until(|snapshot| snapshot.captures.len() == 1);
        } else {
            fixture.until(|snapshot| snapshot.desired.sources.len() == 1);
        }
        fixture
    }
    fn graph(&self, capture: CaptureSnapshot) {
        self.incoming
            .send(BackendEvent::Graph(MixerObservation {
                observation: Observation::Known(()),
                captures: vec![capture],
                streams: Vec::new(),
                outputs: vec![OutputSnapshot {
                    identity: NodeIdentity {
                        server_cookie: 71,
                        object_serial: "222".into(),
                    },
                    node_id: 20,
                    node_name: "headphones".into(),
                    name: "Headphones".into(),
                    priority: 0,
                    is_wave: false,
                    properties: Default::default(),
                }],
                default_sink: None,
                meter_targets: Vec::new(),
                errors: Vec::new(),
                mix_identities: Default::default(),
                silent_sources: Default::default(),
                revision: self.handle.snapshot().revision,
            }))
            .unwrap();
    }
    fn until(&self, predicate: impl Fn(&AppSnapshot) -> bool) -> Arc<AppSnapshot> {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let snapshot = self.handle.snapshot();
            if predicate(&snapshot) {
                return snapshot;
            }
            assert!(
                Instant::now() < deadline,
                "controller snapshot did not reach expected state"
            );
            thread::sleep(Duration::from_millis(2));
        }
    }
    fn outcome(&mut self, id: CommandId) -> CommandOutcome {
        if let Some(outcome) = self.outcomes.remove(&id) {
            return outcome;
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let event = {
                let future = self.events.recv();
                let mut future = pin!(future);
                let waker = Waker::from(Arc::new(ThreadWake(thread::current())));
                let mut context = Context::from_waker(&waker);
                loop {
                    if let Poll::Ready(event) = future.as_mut().poll(&mut context) {
                        break event.unwrap();
                    }
                    assert!(Instant::now() < deadline, "command {id:?} did not complete");
                    thread::park_timeout(Duration::from_millis(5));
                }
            };
            if let RuntimeEvent::CommandFinished {
                id: completed,
                result,
            } = event
            {
                assert!(
                    self.completed.insert(completed),
                    "command completed more than once"
                );
                if completed == id {
                    return result;
                }
                self.outcomes.insert(completed, result);
            }
        }
    }
    fn submit(&mut self, command: AppCommand) -> CommandOutcome {
        let id = self.handle.submit(command).unwrap();
        self.outcome(id)
    }
    fn record(&mut self) -> (CommandId, MicTestToken) {
        let id = self
            .handle
            .submit(AppCommand::RecordMicTest {
                source: self.source.clone(),
                seconds: 1,
            })
            .unwrap();
        loop {
            if let BackendCommand::RecordMicTest {
                job,
                token,
                seconds,
            } = self.commands.recv_timeout(Duration::from_secs(3)).unwrap()
            {
                assert_eq!(seconds, 1);
                assert_eq!(job, id.0);
                self.record_job = job;
                return (id, token);
            }
        }
    }
    fn recorded(&self, token: MicTestToken) {
        self.incoming
            .send(BackendEvent::MicTest(MicTestEvent {
                job: self.record_job,
                token,
                output: None,
                result: Ok(MicTestResult::Recorded(MicTestMetrics {
                    frames: 48_000,
                    peak_db: -12.0,
                    clipped_samples: 0,
                    clipping: false,
                })),
            }))
            .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = self.handle.submit(AppCommand::Shutdown);
        self.handle.wait_stopped().unwrap();
    }
}

#[test]
fn generation_change_cancels_recording_and_rejects_stale_completion() {
    let mut fixture = Fixture::new(Some(1));
    let (id, token) = fixture.record();
    fixture.until(|s| {
        matches!(
            s.mic_test.as_ref().map(|s| &s.phase),
            Some(MicTestPhase::Recording)
        )
    });
    let mut replacement = fixture.capture.clone();
    replacement.identity.object_serial = "5678".into();
    replacement
        .properties
        .insert("object.serial".into(), serde_json::json!(5678));
    fixture.graph(replacement);
    assert!(matches!(fixture.outcome(id), CommandOutcome::Rejected(_)));
    fixture.until(|s| {
        matches!(
            s.mic_test.as_ref().map(|s| &s.phase),
            Some(MicTestPhase::Failed(_))
        )
    });
    fixture.recorded(token.clone());
    fixture.until(|s| s.captures[0].identity.object_serial == "5678");
    assert!(!matches!(
        fixture
            .handle
            .snapshot()
            .mic_test
            .as_ref()
            .map(|s| &s.phase),
        Some(MicTestPhase::Ready)
    ));
    let cancelled = std::iter::from_fn(|| fixture.commands.try_recv().ok()).any(
        |cmd| matches!(cmd, BackendCommand::CancelMicTest(session) if session == token.session),
    );
    assert!(cancelled);
}

#[test]
fn explicit_output_is_bound_to_its_generation_and_close_releases_session() {
    let mut fixture = Fixture::new(Some(2));
    let (record, token) = fixture.record();
    fixture.recorded(token.clone());
    assert!(matches!(
        fixture.outcome(record),
        CommandOutcome::Applied { .. }
    ));
    assert!(matches!(
        fixture.submit(AppCommand::PlayMicTest {
            session: token.session,
            output: playback_target("auto")
        }),
        CommandOutcome::Rejected(_)
    ));
    let play = fixture
        .handle
        .submit(AppCommand::PlayMicTest {
            session: token.session,
            output: playback_target("headphones"),
        })
        .unwrap();
    loop {
        if let BackendCommand::PlayMicTest {
            token: actual,
            output,
            ..
        } = fixture
            .commands
            .recv_timeout(Duration::from_secs(3))
            .unwrap()
        {
            assert_eq!(actual, token);
            assert_eq!(output.node_name, "headphones");
            assert_eq!(output.identity.object_serial, "222");
            break;
        }
    }
    assert!(matches!(
        fixture.submit(AppCommand::CancelMicTest {
            session: token.session
        }),
        CommandOutcome::Applied { .. }
    ));
    assert_eq!(fixture.outcome(play), CommandOutcome::Cancelled);
    fixture.until(|s| s.mic_test.is_none());
    fixture.recorded(token);
    assert!(fixture.handle.snapshot().mic_test.is_none());
}

#[test]
fn invalid_duration_and_missing_capture_never_start_recording() {
    let mut fixture = Fixture::new(Some(1));
    for seconds in [0, 11] {
        assert!(matches!(
            fixture.submit(AppCommand::RecordMicTest {
                source: fixture.source.clone(),
                seconds
            }),
            CommandOutcome::Rejected(_)
        ));
    }
    assert!(matches!(
        fixture.submit(AppCommand::RecordMicTest {
            source: SourceId::new("missing").unwrap(),
            seconds: 1
        }),
        CommandOutcome::Rejected(_)
    ));
    assert!(fixture.handle.snapshot().mic_test.is_none());
    assert!(
        !std::iter::from_fn(|| fixture.commands.try_recv().ok())
            .any(|cmd| matches!(cmd, BackendCommand::RecordMicTest { .. }))
    );
}

#[test]
fn output_disconnect_cancels_playback_without_using_the_default_output() {
    let mut fixture = Fixture::new(Some(1));
    let (record, token) = fixture.record();
    fixture.recorded(token.clone());
    assert!(matches!(
        fixture.outcome(record),
        CommandOutcome::Applied { .. }
    ));
    let play = fixture
        .handle
        .submit(AppCommand::PlayMicTest {
            session: token.session,
            output: playback_target("headphones"),
        })
        .unwrap();
    fixture.until(|s| {
        matches!(
            s.mic_test.as_ref().map(|s| &s.phase),
            Some(MicTestPhase::Playing)
        )
    });
    fixture
        .incoming
        .send(BackendEvent::Graph(MixerObservation {
            observation: Observation::Known(()),
            captures: vec![fixture.capture.clone()],
            streams: vec![],
            outputs: vec![],
            default_sink: Some("replacement_default".into()),
            meter_targets: vec![],
            errors: vec![],
            mix_identities: Default::default(),
            silent_sources: Default::default(),
            revision: fixture.handle.snapshot().revision,
        }))
        .unwrap();
    assert!(matches!(fixture.outcome(play), CommandOutcome::Rejected(_)));
    fixture.until(|s| {
        matches!(
            s.mic_test.as_ref().map(|s| &s.phase),
            Some(MicTestPhase::Failed(_))
        )
    });
    assert!(matches!(
        fixture.submit(AppCommand::PlayMicTest {
            session: token.session,
            output: playback_target("replacement_default")
        }),
        CommandOutcome::Rejected(_)
    ));
    assert!(std::iter::from_fn(|| fixture.commands.try_recv().ok()).any(
        |cmd| matches!(cmd, BackendCommand::CancelMicTest(session) if session == token.session)
    ));
}

#[test]
fn shutdown_discards_the_session_and_completes_recording_as_cancelled() {
    let mut fixture = Fixture::new(Some(1));
    let (record, _) = fixture.record();
    fixture.handle.submit(AppCommand::Shutdown).unwrap();
    fixture.handle.wait_stopped().unwrap();
    assert_eq!(fixture.outcome(record), CommandOutcome::Cancelled);
    assert!(fixture.handle.snapshot().mic_test.is_none());
}

#[test]
fn repeated_playback_ignores_a_previous_operation_completion() {
    let mut fixture = Fixture::new(Some(1));
    let (record, token) = fixture.record();
    fixture.recorded(token.clone());
    assert!(matches!(
        fixture.outcome(record),
        CommandOutcome::Applied { .. }
    ));
    let first = fixture
        .handle
        .submit(AppCommand::PlayMicTest {
            session: token.session,
            output: playback_target("headphones"),
        })
        .unwrap();
    fixture.until(|s| {
        matches!(
            s.mic_test.as_ref().map(|test| &test.phase),
            Some(MicTestPhase::Playing)
        )
    });
    let output = fixture
        .handle
        .snapshot()
        .mic_test
        .as_ref()
        .unwrap()
        .output
        .clone();
    fixture
        .incoming
        .send(BackendEvent::MicTest(MicTestEvent {
            job: first.0,
            token: token.clone(),
            output: output.clone(),
            result: Ok(MicTestResult::Played),
        }))
        .unwrap();
    assert!(matches!(
        fixture.outcome(first),
        CommandOutcome::Applied { .. }
    ));
    let second = fixture
        .handle
        .submit(AppCommand::PlayMicTest {
            session: token.session,
            output: playback_target("headphones"),
        })
        .unwrap();
    fixture.until(|s| {
        matches!(
            s.mic_test.as_ref().map(|test| &test.phase),
            Some(MicTestPhase::Playing)
        )
    });
    fixture
        .incoming
        .send(BackendEvent::MicTest(MicTestEvent {
            job: first.0,
            token: token.clone(),
            output: output.clone(),
            result: Ok(MicTestResult::Played),
        }))
        .unwrap();
    fixture
        .incoming
        .send(BackendEvent::Status {
            service: "duplicate-playback-consumed".into(),
            setup_required: false,
        })
        .unwrap();
    fixture.until(|s| s.service_status == "duplicate-playback-consumed");
    assert_eq!(
        fixture.handle.snapshot().mic_test.as_ref().unwrap().phase,
        MicTestPhase::Playing
    );
    fixture
        .incoming
        .send(BackendEvent::MicTest(MicTestEvent {
            job: second.0,
            token,
            output,
            result: Ok(MicTestResult::Played),
        }))
        .unwrap();
    assert!(matches!(
        fixture.outcome(second),
        CommandOutcome::Applied { .. }
    ));
}

#[test]
fn selecting_another_device_discards_an_active_microphone_test() {
    let mut fixture = Fixture::new(Some(1));
    let unit = |address| UnitSnapshot {
        id: UnitId {
            profile: cadiswave_core::profiles::ProfileId::WaveXlr,
            bus: 1,
            address,
            incarnation: 1,
        },
        info: cadiswave_core::protocol::DeviceInfo {
            api: "1.0".into(),
            firmware: "1.0".into(),
            serial: format!("fixture_{address}"),
        },
        state: Observation::Unknown(OperationError::unavailable("fixture state is unavailable")),
        desired_mute: None,
        input_peak: 0.0,
        output_peak: 0.0,
        errors: vec![],
    };
    let first = unit(1);
    let second = unit(2);
    fixture
        .incoming
        .send(BackendEvent::Unit(first.clone()))
        .unwrap();
    fixture
        .incoming
        .send(BackendEvent::Unit(second.clone()))
        .unwrap();
    fixture.until(|s| s.units.len() == 2);
    assert!(matches!(
        fixture.submit(AppCommand::SelectUnit {
            unit: Some(first.id)
        }),
        CommandOutcome::Applied { .. }
    ));
    let (record, _) = fixture.record();
    assert!(matches!(
        fixture.submit(AppCommand::SelectUnit {
            unit: Some(second.id)
        }),
        CommandOutcome::Applied { .. }
    ));
    assert!(fixture.handle.snapshot().mic_test.is_none());
    assert_eq!(fixture.outcome(record), CommandOutcome::Cancelled);
}

#[test]
fn closing_before_a_recording_snapshot_cancels_the_queued_session() {
    let mut fixture = Fixture::new(Some(1));
    assert!(fixture.handle.snapshot().mic_test.is_none());
    let record = fixture
        .handle
        .submit(AppCommand::RecordMicTest {
            source: fixture.source.clone(),
            seconds: 1,
        })
        .unwrap();
    let close = fixture
        .handle
        .submit(AppCommand::CloseMicTest { record })
        .unwrap();
    assert!(matches!(
        fixture.outcome(close),
        CommandOutcome::Applied { .. }
    ));
    assert_eq!(fixture.outcome(record), CommandOutcome::Cancelled);
    assert!(fixture.handle.snapshot().mic_test.is_none());
}

#[test]
fn same_name_output_replacement_rejects_the_cached_playback_identity() {
    let mut fixture = Fixture::new(Some(1));
    let (record, token) = fixture.record();
    fixture.recorded(token.clone());
    assert!(matches!(
        fixture.outcome(record),
        CommandOutcome::Applied { .. }
    ));
    let original = playback_target("headphones");
    let mut replaced = fixture.handle.snapshot().outputs[0].clone();
    replaced.identity.object_serial = "333".into();
    fixture
        .incoming
        .send(BackendEvent::Graph(MixerObservation {
            observation: Observation::Known(()),
            captures: vec![fixture.capture.clone()],
            streams: vec![],
            outputs: vec![replaced],
            default_sink: None,
            meter_targets: vec![],
            errors: vec![],
            mix_identities: Default::default(),
            silent_sources: Default::default(),
            revision: fixture.handle.snapshot().revision,
        }))
        .unwrap();
    fixture.until(|s| s.outputs[0].identity.object_serial == "333");
    assert!(matches!(
        fixture.submit(AppCommand::PlayMicTest {
            session: token.session,
            output: original
        }),
        CommandOutcome::Rejected(_)
    ));
    assert_eq!(
        fixture.handle.snapshot().mic_test.as_ref().unwrap().phase,
        MicTestPhase::Ready
    );
    assert!(
        !std::iter::from_fn(|| fixture.commands.try_recv().ok())
            .any(|cmd| matches!(cmd, BackendCommand::PlayMicTest { .. }))
    );
}

#[test]
fn closing_a_previous_record_request_preserves_another_clients_new_session() {
    let mut fixture = Fixture::new(Some(1));
    let first = fixture
        .handle
        .submit(AppCommand::RecordMicTest {
            source: fixture.source.clone(),
            seconds: 1,
        })
        .unwrap();
    let second = fixture
        .handle
        .submit(AppCommand::RecordMicTest {
            source: fixture.source.clone(),
            seconds: 1,
        })
        .unwrap();
    let close_first = fixture
        .handle
        .submit(AppCommand::CloseMicTest { record: first })
        .unwrap();
    assert!(matches!(
        fixture.outcome(close_first),
        CommandOutcome::Applied { .. }
    ));
    assert_eq!(fixture.outcome(first), CommandOutcome::Cancelled);
    assert_eq!(
        fixture.handle.snapshot().mic_test.as_ref().unwrap().phase,
        MicTestPhase::Recording
    );
    let close_second = fixture
        .handle
        .submit(AppCommand::CloseMicTest { record: second })
        .unwrap();
    assert!(matches!(
        fixture.outcome(close_second),
        CommandOutcome::Applied { .. }
    ));
    assert_eq!(fixture.outcome(second), CommandOutcome::Cancelled);
    assert!(fixture.handle.snapshot().mic_test.is_none());
}

#[test]
fn retiring_the_selected_device_discards_ready_pcm_before_capture_removal() {
    let mut fixture = Fixture::new(Some(1));
    let make_unit = |address| UnitSnapshot {
        id: UnitId {
            profile: cadiswave_core::profiles::ProfileId::WaveXlr,
            bus: 1,
            address,
            incarnation: 1,
        },
        info: cadiswave_core::protocol::DeviceInfo {
            api: "1.0".into(),
            firmware: "1.0".into(),
            serial: format!("fixture_{address}"),
        },
        state: Observation::Unknown(OperationError::unavailable("fixture state is unavailable")),
        desired_mute: None,
        input_peak: 0.0,
        output_peak: 0.0,
        errors: vec![],
    };
    let first = make_unit(1);
    let second = make_unit(2);
    fixture
        .incoming
        .send(BackendEvent::Unit(first.clone()))
        .unwrap();
    fixture
        .incoming
        .send(BackendEvent::Unit(second.clone()))
        .unwrap();
    fixture.until(|snapshot| snapshot.units.len() == 2);
    assert!(matches!(
        fixture.submit(AppCommand::SelectUnit {
            unit: Some(first.id)
        }),
        CommandOutcome::Applied { .. }
    ));
    let (record, token) = fixture.record();
    fixture.recorded(token);
    assert!(matches!(
        fixture.outcome(record),
        CommandOutcome::Applied { .. }
    ));
    fixture.until(|snapshot| {
        snapshot
            .mic_test
            .as_ref()
            .is_some_and(|test| test.phase == MicTestPhase::Ready)
    });
    fixture
        .incoming
        .send(BackendEvent::UnitRetired(first.id))
        .unwrap();
    fixture.until(|snapshot| snapshot.selected_unit == Some(second.id));
    assert_eq!(fixture.handle.snapshot().captures.len(), 1);
    assert!(fixture.handle.snapshot().mic_test.is_none());
}

#[test]
fn unrelated_graph_teardown_does_not_discard_a_valid_recording_or_playback() {
    let mut fixture = Fixture::new(Some(2));
    let (record, token) = fixture.record();
    let error = OperationError::unavailable("PipeWire nodes changed during discovery");
    fixture
        .incoming
        .send(BackendEvent::Graph(MixerObservation {
            observation: Observation::Unknown(error.clone()),
            errors: vec![OperationIssue {
                target: "graph".into(),
                message: error.to_string(),
            }],
            revision: fixture.handle.snapshot().revision,
            ..MixerObservation::default()
        }))
        .unwrap();
    fixture.until(|s| {
        s.errors
            .iter()
            .any(|issue| issue.message == error.to_string())
    });
    fixture.recorded(token.clone());
    assert!(matches!(
        fixture.outcome(record),
        CommandOutcome::Applied { .. }
    ));
    let play = fixture
        .handle
        .submit(AppCommand::PlayMicTest {
            session: token.session,
            output: playback_target("headphones"),
        })
        .unwrap();
    let (job, output) = loop {
        if let BackendCommand::PlayMicTest { job, output, .. } = fixture
            .commands
            .recv_timeout(Duration::from_secs(3))
            .unwrap()
        {
            break (job, output);
        }
    };
    fixture
        .incoming
        .send(BackendEvent::MicTest(MicTestEvent {
            job,
            token: token.clone(),
            output: Some(output),
            result: Ok(MicTestResult::Played),
        }))
        .unwrap();
    assert!(matches!(
        fixture.outcome(play),
        CommandOutcome::Applied { .. }
    ));
    let ready = fixture.handle.snapshot();
    assert_eq!(ready.mic_test.as_ref().unwrap().phase, MicTestPhase::Ready);
    assert_eq!(
        ready
            .mic_test
            .as_ref()
            .unwrap()
            .metrics
            .as_ref()
            .unwrap()
            .frames,
        48_000
    );
    fixture.graph(fixture.capture.clone());
    fixture.until(|s| {
        !s.errors
            .iter()
            .any(|issue| issue.message == error.to_string())
    });
    assert_eq!(
        fixture.handle.snapshot().mic_test.as_ref().unwrap().phase,
        MicTestPhase::Ready
    );
}

#[test]
fn new_recording_during_unknown_uses_only_the_previous_exact_capture() {
    let mut fixture = Fixture::new(Some(2));
    let error = OperationError::unavailable("PipeWire nodes changed during discovery");
    fixture
        .incoming
        .send(BackendEvent::Graph(MixerObservation {
            observation: Observation::Unknown(error.clone()),
            errors: vec![OperationIssue {
                target: "graph".into(),
                message: error.to_string(),
            }],
            revision: fixture.handle.snapshot().revision,
            ..MixerObservation::default()
        }))
        .unwrap();
    fixture.until(|s| {
        s.errors
            .iter()
            .any(|issue| issue.message == error.to_string())
    });
    let (record, token) = fixture.record();
    assert_eq!(token.identity, fixture.capture.identity);
    fixture.recorded(token);
    assert!(matches!(
        fixture.outcome(record),
        CommandOutcome::Applied { .. }
    ));
}

#[test]
fn known_source_replacement_after_unknown_discards_the_retained_recording() {
    let mut fixture = Fixture::new(Some(2));
    let (record, token) = fixture.record();
    fixture.recorded(token.clone());
    assert!(matches!(
        fixture.outcome(record),
        CommandOutcome::Applied { .. }
    ));
    let error = OperationError::unavailable("PipeWire nodes changed during discovery");
    fixture
        .incoming
        .send(BackendEvent::Graph(MixerObservation {
            observation: Observation::Unknown(error.clone()),
            errors: vec![OperationIssue {
                target: "graph".into(),
                message: error.to_string(),
            }],
            revision: fixture.handle.snapshot().revision,
            ..MixerObservation::default()
        }))
        .unwrap();
    fixture.until(|s| {
        s.errors
            .iter()
            .any(|issue| issue.message == error.to_string())
    });
    assert_eq!(
        fixture.handle.snapshot().mic_test.as_ref().unwrap().phase,
        MicTestPhase::Ready
    );
    let mut replaced = fixture.capture.clone();
    replaced.identity.object_serial = "5678".into();
    replaced
        .properties
        .insert("object.serial".into(), serde_json::json!(5678));
    fixture.graph(replaced);
    fixture.until(|s| {
        matches!(
            s.mic_test.as_ref().map(|test| &test.phase),
            Some(MicTestPhase::Failed(_))
        )
    });
    assert!(
        fixture
            .handle
            .snapshot()
            .mic_test
            .as_ref()
            .unwrap()
            .metrics
            .is_none()
    );
    assert!(std::iter::from_fn(|| fixture.commands.try_recv().ok()).any(|command| matches!(command, BackendCommand::CancelMicTest(session) if session == token.session)));
}

#[test]
fn recording_without_a_previous_known_capture_is_rejected() {
    let mut fixture = Fixture::with_observation(Some(2), false);
    assert!(matches!(
        fixture.submit(AppCommand::RecordMicTest {
            source: fixture.source.clone(),
            seconds: 1
        }),
        CommandOutcome::Rejected(_)
    ));
    assert!(fixture.handle.snapshot().mic_test.is_none());
    assert!(
        !std::iter::from_fn(|| fixture.commands.try_recv().ok())
            .any(|command| matches!(command, BackendCommand::RecordMicTest { .. }))
    );
}

#[test]
fn known_output_replacement_after_unknown_cancels_active_playback() {
    let mut fixture = Fixture::new(Some(2));
    let (record, token) = fixture.record();
    fixture.recorded(token.clone());
    assert!(matches!(
        fixture.outcome(record),
        CommandOutcome::Applied { .. }
    ));
    let play = fixture
        .handle
        .submit(AppCommand::PlayMicTest {
            session: token.session,
            output: playback_target("headphones"),
        })
        .unwrap();
    fixture.until(|s| {
        matches!(
            s.mic_test.as_ref().map(|test| &test.phase),
            Some(MicTestPhase::Playing)
        )
    });
    let error = OperationError::unavailable("PipeWire nodes changed during discovery");
    fixture
        .incoming
        .send(BackendEvent::Graph(MixerObservation {
            observation: Observation::Unknown(error.clone()),
            errors: vec![OperationIssue {
                target: "graph".into(),
                message: error.to_string(),
            }],
            revision: fixture.handle.snapshot().revision,
            ..MixerObservation::default()
        }))
        .unwrap();
    fixture.until(|s| {
        s.errors
            .iter()
            .any(|issue| issue.message == error.to_string())
    });
    assert_eq!(
        fixture.handle.snapshot().mic_test.as_ref().unwrap().phase,
        MicTestPhase::Playing
    );
    let mut output = fixture.handle.snapshot().outputs[0].clone();
    output.identity.object_serial = "333".into();
    fixture
        .incoming
        .send(BackendEvent::Graph(MixerObservation {
            observation: Observation::Known(()),
            captures: vec![fixture.capture.clone()],
            outputs: vec![output],
            revision: fixture.handle.snapshot().revision,
            ..MixerObservation::default()
        }))
        .unwrap();
    assert!(matches!(fixture.outcome(play), CommandOutcome::Rejected(_)));
    let failed = fixture.until(|s| {
        matches!(
            s.mic_test.as_ref().map(|test| &test.phase),
            Some(MicTestPhase::Failed(_))
        )
    });
    assert!(failed.mic_test.as_ref().unwrap().metrics.is_none());
    assert!(std::iter::from_fn(|| fixture.commands.try_recv().ok()).any(|command| matches!(command, BackendCommand::CancelMicTest(session) if session == token.session)));
}

#[test]
fn fresh_worker_identity_failure_expires_recording_during_global_unknown() {
    let mut fixture = Fixture::new(Some(2));
    let (record, token) = fixture.record();
    let discovery = OperationError::unavailable("PipeWire nodes changed during discovery");
    fixture
        .incoming
        .send(BackendEvent::Graph(MixerObservation {
            observation: Observation::Unknown(discovery.clone()),
            errors: vec![OperationIssue {
                target: "graph".into(),
                message: discovery.to_string(),
            }],
            revision: fixture.handle.snapshot().revision,
            ..MixerObservation::default()
        }))
        .unwrap();
    fixture.until(|s| {
        s.errors
            .iter()
            .any(|issue| issue.message == discovery.to_string())
    });
    let failure = OperationError::new(
        ErrorCode::Identity,
        "Microphone test target is not a raw audio source",
    );
    fixture
        .incoming
        .send(BackendEvent::MicTest(MicTestEvent {
            job: record.0,
            token,
            output: None,
            result: Err(failure.clone()),
        }))
        .unwrap();
    assert_eq!(
        fixture.outcome(record),
        CommandOutcome::Rejected(failure.clone())
    );
    let failed = fixture.until(|s| {
        matches!(
            s.mic_test.as_ref().map(|test| &test.phase),
            Some(MicTestPhase::Failed(_))
        )
    });
    assert_eq!(
        failed.mic_test.as_ref().unwrap().phase,
        MicTestPhase::Failed(failure.to_string())
    );
    assert!(failed.mic_test.as_ref().unwrap().metrics.is_none());
}
