use super::*;
use crate::mic_test::{MicTestEvent, MicTestResult};
use cadiswave_core::{
    effects::capture_channels,
    mic_test::{self, MicTestPhase, MicTestSnapshot, MicTestToken, PlaybackTarget},
};

pub(super) struct MicTestSession {
    record_command: CommandId,
    token: MicTestToken,
    binding: CaptureBinding,
    pending: Option<CommandId>,
}
impl Controller {
    fn mic_capture(&self, source: &SourceId, seconds: u32) -> Result<&CaptureSnapshot> {
        // Use the last exact identity; the worker checks the live capture before audio.
        self.store.sources.require_writable()?;
        let row = self.store.sources.value().get(source).ok_or_else(|| {
            OperationError::new(ErrorCode::Identity, "Microphone test source was removed")
        })?;
        if row.kind != SourceKind::Device {
            return Err(OperationError::invalid("Select a device microphone source"));
        }
        let mut captures = self
            .view
            .captures
            .iter()
            .filter(|capture| capture.node_name == row.node_name);
        let capture = captures.next().ok_or_else(|| {
            OperationError::new(
                ErrorCode::Identity,
                "Microphone test source is disconnected",
            )
        })?;
        if captures.next().is_some() {
            return Err(OperationError::new(
                ErrorCode::Identity,
                "Microphone test source is ambiguous",
            ));
        }
        let channels = capture_channels(capture.channels)?;
        mic_test::validate_capture(&row.node_name, seconds, channels)?;
        let serial = capture.properties.get("object.serial").and_then(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .or_else(|| value.as_u64().map(|n| n.to_string()))
        });
        if serial.as_deref() != Some(capture.identity.object_serial.as_str())
            || serial.as_deref().is_none_or(str::is_empty)
            || capture
                .properties
                .get("media.class")
                .and_then(serde_json::Value::as_str)
                != Some("Audio/Source")
        {
            return Err(OperationError::new(
                ErrorCode::Identity,
                "Microphone test needs the current raw capture identity",
            ));
        }
        Ok(capture)
    }
    fn validate_mic_test(&self) -> Result<()> {
        let session = self.mic_test.as_ref().ok_or_else(|| {
            OperationError::new(ErrorCode::Identity, "Microphone test session expired")
        })?;
        let snapshot = self.view.mic_test.as_ref().ok_or_else(|| {
            OperationError::new(ErrorCode::Identity, "Microphone test session expired")
        })?;
        // The worker checks live identities before and after each audio operation.
        // A global discovery race does not prove that this capture changed.
        let capture = self.mic_capture(&session.token.source, snapshot.seconds)?;
        if capture.identity != session.token.identity
            || capture.node_name != session.token.node_name
            || capture_channels(capture.channels)? != session.token.channels
            || self.bindings.get(&session.token.node_name) != Some(&session.binding)
        {
            return Err(OperationError::new(
                ErrorCode::Identity,
                "Microphone test source generation or channels changed",
            ));
        }
        if let Some(output) = &snapshot.output {
            self.mic_output(&output.node_name).and_then(|actual| {
                if actual == *output {
                    Ok(())
                } else {
                    Err(OperationError::new(
                        ErrorCode::Identity,
                        "Microphone test output generation changed",
                    ))
                }
            })?;
        }
        Ok(())
    }
    fn mic_output(&self, name: &str) -> Result<PlaybackTarget> {
        if name.is_empty() || name.contains('\0') || matches!(name, "auto" | "0" | "-1") {
            return Err(OperationError::invalid(
                "Select an explicit connected playback output",
            ));
        }
        let mut outputs = self
            .view
            .outputs
            .iter()
            .filter(|output| output.node_name == name);
        let output = outputs.next().ok_or_else(|| {
            OperationError::new(ErrorCode::Identity, "Microphone test output disconnected")
        })?;
        if outputs.next().is_some() || output.identity.object_serial.is_empty() {
            return Err(OperationError::new(
                ErrorCode::Identity,
                "Microphone test output identity is ambiguous",
            ));
        }
        Ok(PlaybackTarget {
            node_name: output.node_name.clone(),
            identity: output.identity.clone(),
        })
    }
    pub(super) fn record_mic_test(
        &mut self,
        command: CommandId,
        source: &SourceId,
        seconds: u32,
    ) -> Result<()> {
        mic_test::validate_duration(seconds)?;
        let capture = self.mic_capture(source, seconds)?;
        let token = MicTestToken {
            session: self.next_session,
            source: source.clone(),
            node_name: capture.node_name.clone(),
            identity: capture.identity.clone(),
            channels: capture_channels(capture.channels)?,
        };
        let binding = self
            .bindings
            .get(&token.node_name)
            .cloned()
            .ok_or_else(|| {
                OperationError::new(
                    ErrorCode::Identity,
                    "Microphone test source has no current binding",
                )
            })?;
        let next = self.next_session.checked_add(1).ok_or_else(|| {
            OperationError::unavailable("Microphone test session space exhausted")
        })?;
        self.cancel_mic_test();
        self.backend.dispatch(BackendCommand::RecordMicTest {
            job: command.0,
            token: token.clone(),
            seconds,
        })?;
        self.next_session = next;
        self.view.mic_test = Some(MicTestSnapshot {
            token: token.clone(),
            seconds,
            phase: MicTestPhase::Recording,
            metrics: None,
            output: None,
        });
        self.mic_test = Some(MicTestSession {
            record_command: command,
            token,
            binding,
            pending: Some(command),
        });
        if let Some(request) = self.requests.get_mut(&command) {
            request.waiting = true;
        }
        self.clear_issue("microphone test");
        self.dirty = true;
        Ok(())
    }
    pub(super) fn play_mic_test(
        &mut self,
        command: CommandId,
        session: u64,
        expected: &PlaybackTarget,
    ) -> Result<()> {
        self.refresh_mic_test_validity();
        self.validate_mic_test()?;
        let current = self.mic_test.as_ref().expect("validated session");
        if current.token.session != session
            || current.pending.is_some()
            || self
                .view
                .mic_test
                .as_ref()
                .is_none_or(|snapshot| snapshot.phase != MicTestPhase::Ready)
        {
            return Err(OperationError::new(
                ErrorCode::Identity,
                "Microphone test recording is not ready",
            ));
        }
        let output = self.mic_output(&expected.node_name)?;
        if output != *expected {
            return Err(OperationError::new(
                ErrorCode::Identity,
                "Selected microphone test output generation changed",
            ));
        }
        self.backend.dispatch(BackendCommand::PlayMicTest {
            job: command.0,
            token: current.token.clone(),
            output: output.clone(),
        })?;
        self.mic_test.as_mut().expect("validated session").pending = Some(command);
        let snapshot = self.view.mic_test.as_mut().expect("validated snapshot");
        snapshot.phase = MicTestPhase::Playing;
        snapshot.output = Some(output);
        if let Some(request) = self.requests.get_mut(&command) {
            request.waiting = true;
        }
        self.dirty = true;
        Ok(())
    }
    pub(super) fn close_mic_test(&mut self, record: CommandId) {
        if self
            .mic_test
            .as_ref()
            .is_some_and(|session| session.record_command == record)
        {
            self.cancel_mic_test();
        }
    }
    pub(super) fn cancel_mic_test(&mut self) {
        if let Some(session) = self.mic_test.take() {
            if let Err(error) = self
                .backend
                .dispatch(BackendCommand::CancelMicTest(session.token.session))
            {
                self.issue("microphone test cancellation", error);
            }
            if let Some(command) = session.pending {
                self.finish(command, CommandOutcome::Cancelled);
            }
        }
        self.view.mic_test = None;
        self.dirty = true;
    }
    pub(super) fn cancel_mic_test_token(&mut self, session: u64) -> Result<()> {
        if self
            .view
            .mic_test
            .as_ref()
            .is_none_or(|snapshot| snapshot.token.session != session)
        {
            return Err(OperationError::new(
                ErrorCode::Identity,
                "Microphone test session is no longer current",
            ));
        }
        self.cancel_mic_test();
        Ok(())
    }
    fn expire_mic_test(&mut self, error: OperationError) {
        if let Some(session) = self.mic_test.take() {
            if let Err(cancel) = self
                .backend
                .dispatch(BackendCommand::CancelMicTest(session.token.session))
            {
                self.issue("microphone test cancellation", cancel);
            }
            if let Some(command) = session.pending {
                self.finish(command, CommandOutcome::Rejected(error.clone()));
            }
        }
        if let Some(snapshot) = &mut self.view.mic_test {
            snapshot.phase = MicTestPhase::Failed(error.to_string());
            snapshot.metrics = None;
            snapshot.output = None;
        }
        self.issue("microphone test", error);
        self.dirty = true;
    }
    pub(super) fn refresh_mic_test_validity(&mut self) {
        if self.mic_test.is_some()
            && let Err(error) = self.validate_mic_test()
        {
            self.expire_mic_test(error);
        }
    }
    pub(super) fn mic_test_complete(&mut self, event: MicTestEvent) {
        if self.frozen {
            return;
        }
        let Some(session) = self
            .mic_test
            .as_ref()
            .filter(|session| session.token == event.token)
        else {
            return;
        };
        let Some(command) = session.pending.filter(|command| command.0 == event.job) else {
            return;
        };
        let snapshot = self
            .view
            .mic_test
            .as_ref()
            .expect("active session snapshot");
        if event.output != snapshot.output
            || !matches!(
                (&snapshot.phase, &event.result),
                (MicTestPhase::Recording, Ok(MicTestResult::Recorded(_)))
                    | (MicTestPhase::Playing, Ok(MicTestResult::Played))
                    | (_, Err(_))
            )
        {
            return;
        }
        if let Err(error) = self.validate_mic_test() {
            self.expire_mic_test(error);
            return;
        }
        match event.result {
            Err(error) => {
                self.expire_mic_test(error);
            }
            Ok(result) => {
                self.mic_test.as_mut().expect("current session").pending = None;
                let snapshot = self.view.mic_test.as_mut().expect("current snapshot");
                if let MicTestResult::Recorded(metrics) = result {
                    snapshot.metrics = Some(metrics);
                }
                snapshot.phase = MicTestPhase::Ready;
                if let Some(request) = self.requests.get_mut(&command) {
                    request.waiting = false;
                }
                self.finish_if_ready(command);
                self.dirty = true;
            }
        }
    }
}
