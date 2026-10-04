//! Own microphone test PCM and supervised audio children.
use crate::process::{CommandRunner, OwnedChild};
use cadiswave_core::{
    calibration::{GRACE_SECONDS, RATE},
    mic_test::{self, MicTestMetrics, MicTestToken, PlaybackTarget},
    model::{ErrorCode, OperationError, Result},
};
use std::{
    io::{Seek, SeekFrom, Write},
    process::Stdio,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

fn cancelled() -> OperationError {
    OperationError::new(ErrorCode::Cancelled, "Microphone test cancelled")
}
fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err(cancelled())
    } else {
        Ok(())
    }
}
pub(crate) fn validate_capture_token(
    token: &MicTestToken,
    runner: &impl crate::recovery::Commands,
) -> Result<()> {
    let graph = crate::recovery::validate_node(runner, &token.node_name, &token.identity, false)?;
    let node = graph
        .as_array()
        .and_then(|nodes| {
            nodes.iter().find(|node| {
                node.pointer("/info/props/node.name")
                    .and_then(serde_json::Value::as_str)
                    == Some(token.node_name.as_str())
            })
        })
        .ok_or_else(|| {
            OperationError::new(ErrorCode::Identity, "Microphone test capture disappeared")
        })?;
    let serial = node.pointer("/info/props/object.serial").and_then(|value| {
        value
            .as_str()
            .map(str::to_owned)
            .or_else(|| value.as_u64().map(|value| value.to_string()))
    });
    if serial.as_deref() != Some(token.identity.object_serial.as_str()) {
        return Err(OperationError::new(
            ErrorCode::Identity,
            "Microphone test needs the current capture object.serial",
        ));
    }
    if node
        .pointer("/info/props/media.class")
        .and_then(serde_json::Value::as_str)
        != Some("Audio/Source")
    {
        return Err(OperationError::new(
            ErrorCode::Identity,
            "Microphone test target is not a raw audio source",
        ));
    }
    let channels = node
        .pointer("/info/props/audio.channels")
        .map(|value| {
            value
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
                .ok_or_else(|| {
                    OperationError::new(
                        ErrorCode::Identity,
                        "Microphone test capture channels are unavailable",
                    )
                })
        })
        .transpose()?;
    if cadiswave_core::effects::capture_channels(channels)? != token.channels {
        return Err(OperationError::new(
            ErrorCode::Identity,
            "Microphone test capture channels changed",
        ));
    }
    Ok(())
}
fn capture(token: &MicTestToken, seconds: u32, cancel: Arc<AtomicBool>) -> Result<Vec<u8>> {
    mic_test::validate_capture(&token.node_name, seconds, token.channels)?;
    let runner = CommandRunner::new(cancel.clone());
    validate_capture_token(token, &runner)?;
    let bytes = crate::calibration::capture_raw(
        &token.identity.object_serial,
        seconds,
        token.channels,
        cancel,
    )?;
    validate_capture_token(token, &runner)?;
    Ok(bytes)
}
pub(crate) fn validate_output(
    output: &PlaybackTarget,
    runner: &impl crate::recovery::Commands,
) -> Result<()> {
    if output.node_name.is_empty()
        || output.node_name.contains('\0')
        || matches!(output.node_name.as_str(), "auto" | "0" | "-1")
        || output.identity.object_serial.is_empty()
    {
        return Err(OperationError::invalid(
            "Select an explicit playback output",
        ));
    }
    let graph = crate::recovery::validate_node(runner, &output.node_name, &output.identity, false)?;
    let sink = graph.as_array().and_then(|nodes| {
        nodes.iter().find(|node| {
            node.pointer("/info/props/node.name")
                .and_then(serde_json::Value::as_str)
                == Some(output.node_name.as_str())
        })
    });
    let serial = sink
        .and_then(|node| node.pointer("/info/props/object.serial"))
        .and_then(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .or_else(|| value.as_u64().map(|number| number.to_string()))
        });
    if serial.as_deref() != Some(output.identity.object_serial.as_str())
        || sink
            .and_then(|node| node.pointer("/info/props/media.class"))
            .and_then(serde_json::Value::as_str)
            != Some("Audio/Sink")
    {
        return Err(OperationError::new(
            ErrorCode::Identity,
            "Playback target is not a current audio output",
        ));
    }
    Ok(())
}
fn playback(
    token: &MicTestToken,
    output: &PlaybackTarget,
    bytes: &[u8],
    channels: u32,
    seconds: u32,
    cancel: Arc<AtomicBool>,
) -> Result<()> {
    mic_test::measure(bytes, channels, seconds)?;
    check_cancel(&cancel)?;
    let runner = CommandRunner::new(cancel.clone());
    validate_capture_token(token, &runner)?;
    validate_output(output, &runner)?;
    let fd = rustix::fs::memfd_create("cadiswave-mic-test", rustix::fs::MemfdFlags::CLOEXEC)
        .map_err(std::io::Error::from)?;
    let mut input = std::fs::File::from(fd);
    input.write_all(bytes)?;
    input.seek(SeekFrom::Start(0))?;
    let args = vec!["--playback".into(), "--target".into(), output.identity.object_serial.clone(), "--rate".into(), RATE.to_string(), "--channels".into(), channels.to_string(), "--format".into(), "s16".into(), "--properties".into(), r#"{ "media.name": "cadiswave_mic_test", "node.name": "cadiswave_mic_test", "application.name": "CadisWave", "node.dont-reconnect": true, "node.dont-fallback": true, "node.dont-move": true }"#.into(), "-".into()];
    playback_with(
        cancel,
        Duration::from_secs(u64::from(seconds + GRACE_SECONDS)),
        || OwnedChild::spawn_with_stdin("pw-cat", &args, Stdio::from(input), Stdio::null()),
    )?;
    validate_capture_token(token, &runner)?;
    validate_output(output, &runner)
}

pub(crate) fn playback_with(
    cancel: Arc<AtomicBool>,
    timeout: Duration,
    spawn: impl FnOnce() -> Result<OwnedChild>,
) -> Result<()> {
    check_cancel(&cancel)?;
    let deadline = Instant::now() + timeout;
    let mut child = spawn()?;
    let result = (|| {
        loop {
            check_cancel(&cancel)?;
            if let Some(status) = child.try_wait()? {
                return if status.success() {
                    Ok(())
                } else {
                    Err(OperationError::unavailable(
                        "Microphone test playback failed",
                    ))
                };
            }
            if Instant::now() >= deadline {
                return Err(OperationError::unavailable(
                    "Microphone test playback exceeded its deadline",
                ));
            }
            thread::sleep(Duration::from_millis(10));
        }
    })();
    let cleanup = child.terminate();
    result.and(cleanup)?;
    check_cancel(&cancel)
}

#[derive(Debug)]
pub enum MicTestResult {
    Recorded(MicTestMetrics),
    Played,
}
#[derive(Debug)]
pub struct MicTestEvent {
    pub job: u64,
    pub token: MicTestToken,
    pub output: Option<PlaybackTarget>,
    pub result: Result<MicTestResult>,
}
struct Active {
    job: u64,
    token: MicTestToken,
    cancel: Arc<AtomicBool>,
    ready: bool,
    busy: bool,
}
#[derive(Default)]
struct State {
    stopped: bool,
    active: Option<Active>,
}
enum Request {
    Record {
        job: u64,
        token: MicTestToken,
        seconds: u32,
        cancel: Arc<AtomicBool>,
    },
    Play {
        job: u64,
        token: MicTestToken,
        output: PlaybackTarget,
        cancel: Arc<AtomicBool>,
    },
    Cancel(u64),
}
struct Recording {
    token: MicTestToken,
    seconds: u32,
    bytes: Vec<u8>,
}
type CaptureReader =
    Arc<dyn Fn(&MicTestToken, u32, Arc<AtomicBool>) -> Result<Vec<u8>> + Send + Sync>;
type PlaybackReader = Arc<
    dyn Fn(&MicTestToken, &PlaybackTarget, &[u8], u32, u32, Arc<AtomicBool>) -> Result<()>
        + Send
        + Sync,
>;
pub struct MicTestWorker {
    state: Arc<Mutex<State>>,
    requests: Option<mpsc::SyncSender<Request>>,
    thread: Option<JoinHandle<()>>,
    stop_error: Option<OperationError>,
}
impl MicTestWorker {
    pub fn start() -> Result<(Self, mpsc::Receiver<MicTestEvent>)> {
        Self::start_with(Arc::new(capture), Arc::new(playback))
    }
    pub(crate) fn start_with(
        capture: CaptureReader,
        play: PlaybackReader,
    ) -> Result<(Self, mpsc::Receiver<MicTestEvent>)> {
        let (requests, incoming) = mpsc::sync_channel(2);
        let (events, receiver) = mpsc::channel();
        let state = Arc::new(Mutex::new(State::default()));
        let shared = state.clone();
        let thread = thread::Builder::new()
            .name("cadiswave-mic-test".into())
            .spawn(move || {
                let mut recording: Option<Recording> = None;
                for request in incoming {
                    let (job, token, output, cancel, result) = match request {
                        Request::Cancel(session) => {
                            if recording
                                .as_ref()
                                .is_some_and(|r| r.token.session == session)
                            {
                                recording = None;
                            }
                            continue;
                        }
                        Request::Record {
                            job,
                            token,
                            seconds,
                            cancel,
                        } => {
                            recording = None;
                            let result = check_cancel(&cancel)
                                .and_then(|()| capture(&token, seconds, cancel.clone()))
                                .and_then(|bytes| {
                                    check_cancel(&cancel)?;
                                    let metrics =
                                        mic_test::measure(&bytes, token.channels, seconds)?;
                                    recording = Some(Recording {
                                        token: token.clone(),
                                        seconds,
                                        bytes,
                                    });
                                    Ok(MicTestResult::Recorded(metrics))
                                });
                            (job, token, None, cancel, result)
                        }
                        Request::Play {
                            job,
                            token,
                            output,
                            cancel,
                        } => {
                            let result = check_cancel(&cancel).and_then(|()| {
                                let r = recording
                                    .as_ref()
                                    .filter(|r| r.token == token)
                                    .ok_or_else(|| {
                                        OperationError::new(
                                            ErrorCode::Identity,
                                            "Microphone test recording expired",
                                        )
                                    })?;
                                play(
                                    &token,
                                    &output,
                                    &r.bytes,
                                    token.channels,
                                    r.seconds,
                                    cancel.clone(),
                                )?;
                                Ok(MicTestResult::Played)
                            });
                            (job, token, Some(output), cancel, result)
                        }
                    };
                    let mut state = shared.lock().unwrap_or_else(|e| e.into_inner());
                    let current = !state.stopped
                        && state.active.as_ref().is_some_and(|active| {
                            active.job == job
                                && active.token == token
                                && Arc::ptr_eq(&active.cancel, &cancel)
                        })
                        && !cancel.load(Ordering::Acquire);
                    let result = if current { result } else { Err(cancelled()) };
                    if let Some(active) = state.active.as_mut().filter(|active| {
                        active.job == job
                            && active.token == token
                            && Arc::ptr_eq(&active.cancel, &cancel)
                    }) {
                        active.busy = false;
                        active.ready = result.is_ok();
                    }
                    if result.is_err() {
                        recording = None;
                    }
                    let _ = events.send(MicTestEvent {
                        job,
                        token,
                        output,
                        result,
                    });
                }
            })?;
        Ok((
            Self {
                state,
                requests: Some(requests),
                thread: Some(thread),
                stop_error: None,
            },
            receiver,
        ))
    }
    pub fn record(&self, job: u64, token: MicTestToken, seconds: u32) -> Result<()> {
        mic_test::validate_capture(&token.node_name, seconds, token.channels)?;
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.stopped {
            return Err(cancelled());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        self.send(Request::Record {
            job,
            token: token.clone(),
            seconds,
            cancel: cancel.clone(),
        })?;
        if let Some(active) = &state.active {
            active.cancel.store(true, Ordering::Release);
        }
        state.active = Some(Active {
            job,
            token,
            cancel,
            ready: false,
            busy: true,
        });
        Ok(())
    }
    pub fn play(&self, job: u64, token: MicTestToken, output: PlaybackTarget) -> Result<()> {
        if output.node_name.is_empty()
            || output.node_name.contains('\0')
            || matches!(output.node_name.as_str(), "auto" | "0" | "-1")
            || output.identity.object_serial.is_empty()
        {
            return Err(OperationError::invalid(
                "Select an explicit playback output",
            ));
        }
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.stopped {
            return Err(cancelled());
        }
        let active = state
            .active
            .as_mut()
            .filter(|active| {
                active.token == token && active.ready && !active.cancel.load(Ordering::Acquire)
            })
            .ok_or_else(|| {
                OperationError::new(
                    ErrorCode::Identity,
                    "Microphone test recording is not ready",
                )
            })?;
        if active.busy {
            return Err(OperationError::new(
                ErrorCode::Busy,
                "Microphone test playback is active",
            ));
        }
        self.send(Request::Play {
            job,
            token,
            output,
            cancel: active.cancel.clone(),
        })?;
        active.job = job;
        active.busy = true;
        Ok(())
    }
    pub fn cancel(&self, session: u64) -> Result<()> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state
            .active
            .as_ref()
            .is_some_and(|active| active.token.session == session)
        {
            if let Some(active) = state.active.take() {
                active.cancel.store(true, Ordering::Release);
            }
            // A full queue already contains work that observes the cancelled flag.
            if let Some(sender) = &self.requests {
                let _ = sender.try_send(Request::Cancel(session));
            }
        }
        Ok(())
    }
    fn send(&self, request: Request) -> Result<()> {
        self.requests
            .as_ref()
            .ok_or_else(cancelled)?
            .try_send(request)
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => {
                    OperationError::new(ErrorCode::Busy, "Microphone test queue is full")
                }
                mpsc::TrySendError::Disconnected(_) => {
                    OperationError::unavailable("Microphone test worker stopped")
                }
            })
    }
    pub fn stop(&mut self) -> Result<()> {
        {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            state.stopped = true;
            if let Some(active) = state.active.take() {
                active.cancel.store(true, Ordering::Release);
            }
        }
        self.requests.take();
        if let Some(thread) = self.thread.take() {
            self.stop_error = thread
                .join()
                .err()
                .map(|_| OperationError::unavailable("Microphone test worker panicked"));
        }
        self.stop_error.clone().map_or(Ok(()), Err)
    }
}
impl Drop for MicTestWorker {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
