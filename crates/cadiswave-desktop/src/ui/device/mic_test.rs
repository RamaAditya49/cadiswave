//! Present microphone test metadata and submit explicit user actions.
use super::controls::DeviceControls;
use adw::prelude::*;
use cadiswave_core::{
    mic_test::{MicTestPhase, PlaybackTarget},
    model::{AppSnapshot, CommandId, Lifecycle, SourceId},
};
use cadiswave_runtime::controller::AppCommand;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

struct Model {
    controls: DeviceControls,
    source: RefCell<Option<SourceId>>,
    outputs: RefCell<Vec<PlaybackTarget>>,
    selected: RefCell<Option<PlaybackTarget>>,
    labels: RefCell<Vec<String>>,
    updating: Cell<bool>,
    record_command: Cell<Option<CommandId>>,
    closed: Cell<bool>,
}
impl Model {
    fn close(&self) {
        if self.closed.replace(true) {
            return;
        }
        if let Some(record) = self.record_command.take() {
            let _ = self.controls.submit(AppCommand::CloseMicTest { record });
        }
    }
    fn cancel(&self) {
        if let Some(snapshot) = &self.controls.snapshot().mic_test {
            let _ = self.controls.submit(AppCommand::CancelMicTest {
                session: snapshot.token.session,
            });
        } else {
            self.close();
        }
    }
}
pub struct MicTestControls {
    pub container: gtk::Box,
    pub record: gtk::Button,
    pub play: gtk::Button,
    pub stop: gtk::Button,
    output: gtk::DropDown,
    duration: gtk::SpinButton,
    status: gtk::Label,
    model: Rc<Model>,
}
impl MicTestControls {
    pub fn new(controls: DeviceControls) -> Self {
        let container = gtk::Box::new(gtk::Orientation::Vertical, 12);
        let explanation = gtk::Label::new(None);
        explanation.set_xalign(0.0);
        explanation.set_wrap(true);
        crate::i18n::bind(&explanation, "label", "mic-test-help");
        container.append(&explanation);
        let duration = gtk::SpinButton::with_range(1.0, 10.0, 1.0);
        duration.set_value(5.0);
        duration.set_numeric(true);
        let duration_row = adw::ActionRow::new();
        crate::i18n::bind(&duration_row, "title", "mic-test-duration");
        duration_row.add_suffix(&duration);
        duration_row.set_activatable_widget(Some(&duration));
        container.append(&duration_row);
        let output = gtk::DropDown::new(None::<gtk::StringList>, None::<gtk::Expression>);
        output.set_hexpand(true);
        let output_label = gtk::Label::new(None);
        output_label.set_xalign(0.0);
        crate::i18n::bind(&output_label, "label", "mic-test-output");
        output.update_relation(&[gtk::accessible::Relation::LabelledBy(&[
            output_label.upcast_ref()
        ])]);
        container.append(&output_label);
        container.append(&output);
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let record = gtk::Button::new();
        let play = gtk::Button::new();
        let stop = gtk::Button::new();
        crate::i18n::bind(&record, "label", "mic-test-record");
        crate::i18n::bind(&play, "label", "mic-test-play");
        crate::i18n::bind(&stop, "label", "mic-test-stop");
        record.add_css_class("suggested-action");
        for button in [&record, &play, &stop] {
            buttons.append(button);
        }
        container.append(&buttons);
        let status = gtk::Label::new(None);
        status.set_xalign(0.0);
        status.set_wrap(true);
        container.append(&status);
        let model = Rc::new(Model {
            controls,
            source: RefCell::new(None),
            outputs: RefCell::new(vec![]),
            selected: RefCell::new(None),
            labels: RefCell::new(vec![]),
            updating: Cell::new(false),
            record_command: Cell::new(None),
            closed: Cell::new(false),
        });
        let target = model.clone();
        let feedback = status.clone();
        let duration_input = duration.clone();
        record.connect_clicked(move |_| {
            if target.updating.get() {
                return;
            }
            if let Some(source) = target.source.borrow().clone() {
                match target.controls.submit(AppCommand::RecordMicTest {
                    source,
                    seconds: duration_input.value_as_int() as u32,
                }) {
                    Ok(id) => {
                        target.record_command.set(Some(id));
                        target.closed.set(false);
                    }
                    Err(error) => feedback.set_text(&crate::i18n::format(
                        "mic-test-error",
                        &[("message", &format!("{error:?}"))],
                    )),
                }
            }
        });
        let target = model.clone();
        let feedback = status.clone();
        play.connect_clicked(move |_| {
            if target.updating.get() {
                return;
            }
            let snapshot = target.controls.snapshot();
            if let Some(test) = &snapshot.mic_test
                && let Some(output) = target.selected.borrow().as_ref()
            {
                // The controller captures the current output identity before dispatch.
                if let Err(error) = target.controls.submit(AppCommand::PlayMicTest {
                    session: test.token.session,
                    output: output.clone(),
                }) {
                    feedback.set_text(&crate::i18n::format(
                        "mic-test-error",
                        &[("message", &format!("{error:?}"))],
                    ));
                }
            }
        });
        let target = model.clone();
        stop.connect_clicked(move |_| {
            if !target.updating.get() {
                target.cancel();
            }
        });
        let target = model.clone();
        let playback_button = play.clone();
        output.connect_selected_notify(move |widget| {
            if target.updating.get() {
                return;
            }
            let selected = target
                .outputs
                .borrow()
                .get(
                    widget
                        .selected()
                        .checked_sub(1)
                        .map_or(usize::MAX, |index| index as usize),
                )
                .cloned();
            *target.selected.borrow_mut() = selected;
            let snapshot = target.controls.snapshot();
            if snapshot
                .mic_test
                .as_ref()
                .is_some_and(|test| test.phase == MicTestPhase::Playing)
            {
                target.cancel();
            }
            playback_button.set_sensitive(
                snapshot.lifecycle == Lifecycle::Running
                    && snapshot.mic_test.as_ref().is_some_and(|test| {
                        test.phase == MicTestPhase::Ready
                            && Some(&test.token.source) == target.source.borrow().as_ref()
                    })
                    && target.selected.borrow().is_some(),
            );
        });
        let target = model.clone();
        container.connect_map(move |_| target.closed.set(false));
        let target = model.clone();
        container.connect_unmap(move |_| {
            target.close();
        });
        Self {
            container,
            record,
            play,
            stop,
            output,
            duration,
            status,
            model,
        }
    }
    /// Cancel the recording when its dialog closes or its source selection changes.
    pub fn close(&self) {
        self.model.close();
    }
    /// Update widgets without submitting audio commands.
    pub fn render(&self, snapshot: &AppSnapshot, source: Option<SourceId>) {
        self.model.updating.set(true);
        *self.model.source.borrow_mut() = source.clone();
        let outputs: Vec<_> = snapshot
            .outputs
            .iter()
            .filter(|output| !output.identity.object_serial.is_empty())
            .map(|output| PlaybackTarget {
                node_name: output.node_name.clone(),
                identity: output.identity.clone(),
            })
            .collect();
        let mut labels = vec![crate::i18n::tr("mic-test-output-select")];
        labels.extend(
            snapshot
                .outputs
                .iter()
                .filter(|output| !output.identity.object_serial.is_empty())
                .map(|output| output.name.clone()),
        );
        if self.output.model().is_none()
            || *self.model.outputs.borrow() != outputs
            || *self.model.labels.borrow() != labels
        {
            let retained = self
                .model
                .selected
                .borrow()
                .as_ref()
                .and_then(|selected| outputs.iter().position(|output| output == selected));
            let items: Vec<_> = labels.iter().map(String::as_str).collect();
            self.output.set_model(Some(&gtk::StringList::new(&items)));
            self.output
                .set_selected(retained.map_or(0, |index| index as u32 + 1));
            *self.model.selected.borrow_mut() = retained.map(|index| outputs[index].clone());
            *self.model.outputs.borrow_mut() = outputs;
            *self.model.labels.borrow_mut() = labels;
        }
        let test = snapshot
            .mic_test
            .as_ref()
            .filter(|test| Some(&test.token.source) == source.as_ref());
        let busy = test.is_some_and(|test| {
            matches!(test.phase, MicTestPhase::Recording | MicTestPhase::Playing)
        });
        let running = snapshot.lifecycle == Lifecycle::Running;
        self.record
            .set_sensitive(running && source.is_some() && !busy);
        self.duration.set_sensitive(running && !busy);
        self.output.set_sensitive(running);
        self.play.set_sensitive(
            running
                && test.is_some_and(|test| test.phase == MicTestPhase::Ready)
                && self.model.selected.borrow().is_some(),
        );
        self.stop
            .set_sensitive(running && snapshot.mic_test.is_some());
        let mut text = match test.map(|test| &test.phase) {
            Some(MicTestPhase::Recording) => crate::i18n::tr("mic-test-recording"),
            Some(MicTestPhase::Playing) => crate::i18n::tr("mic-test-playing"),
            Some(MicTestPhase::Ready) => crate::i18n::tr("mic-test-ready"),
            Some(MicTestPhase::Failed(error)) => {
                crate::i18n::format("mic-test-error", &[("message", error)])
            }
            None => crate::i18n::tr(if source.is_some() {
                "mic-test-idle"
            } else {
                "mic-test-select-source"
            }),
        };
        if let Some(metrics) = test.and_then(|test| test.metrics.as_ref()) {
            text.push('\n');
            text.push_str(&crate::i18n::format(
                "mic-test-peak",
                &[("value", &format!("{:.1}", metrics.peak_db))],
            ));
            if metrics.clipping {
                text.push('\n');
                text.push_str(&crate::i18n::tr("mic-test-clipping"));
            }
        }
        self.status.set_text(&text);
        self.model.updating.set(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn microphone_rig() -> (crate::ui::test_support::Rig, SourceId) {
        use cadiswave_core::model::*;
        let rig = crate::ui::test_support::Rig::new(
            serde_json::json!({}),
            vec![crate::ui::test_support::unit("A", 2, -10.0)],
        );
        let mut row = Source::new("Microphone".into(), SourceKind::Device);
        row.node_name = "fixture_raw_mic".into();
        let source = row.id.clone();
        rig.set_test_audio(vec![CaptureSnapshot {
            identity: NodeIdentity { server_cookie: 1, object_serial: "19".into() }, node_id: 19,
            node_name: "fixture_raw_mic".into(), name: "Microphone".into(), channels: Some(1),
            muted: Observation::Known(false), properties: serde_json::json!({"object.serial": "19", "media.class": "Audio/Source", "device.serial": "A"}).as_object().unwrap().clone(),
        }], vec![OutputSnapshot {
            identity: NodeIdentity { server_cookie: 1, object_serial: "20".into() }, node_id: 20,
            node_name: "fixture_headphones".into(), name: "Headphones".into(), priority: 0, is_wave: false, properties: Default::default(),
        }]);
        let add = rig
            .handle()
            .submit(AppCommand::AddSource { source: row })
            .unwrap();
        let added = rig.outcome(add);
        assert!(matches!(added, CommandOutcome::Applied { .. }), "{added:?}");
        (rig, source)
    }
    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn repeated_render_does_not_start_audio_and_missing_output_disables_playback() {
        gtk::init().unwrap();
        use gtk::prelude::*;
        let rig = crate::ui::test_support::Rig::new(serde_json::json!({}), vec![]);
        let controls =
            MicTestControls::new(super::super::controls::DeviceControls::new(rig.handle()));
        let mut snapshot = (*rig.snapshot()).clone();
        snapshot.lifecycle = cadiswave_core::model::Lifecycle::Running;
        let source = cadiswave_core::model::SourceId::new("mic").unwrap();
        snapshot.mic_test = Some(cadiswave_core::mic_test::MicTestSnapshot {
            token: cadiswave_core::mic_test::MicTestToken {
                session: 1,
                source: source.clone(),
                node_name: "raw_mic".into(),
                identity: cadiswave_core::model::NodeIdentity {
                    server_cookie: 1,
                    object_serial: "1".into(),
                },
                channels: 1,
            },
            seconds: 1,
            phase: cadiswave_core::mic_test::MicTestPhase::Ready,
            metrics: None,
            output: None,
        });
        let before = rig.snapshot().revision;
        for _ in 0..20 {
            controls.render(&snapshot, Some(source.clone()));
        }
        assert!(!controls.play.is_sensitive());
        assert_eq!(controls.output.selected(), 0);
        assert!(rig.snapshot().mic_test.is_none());
        assert_eq!(rig.snapshot().revision, before);
        assert_eq!(rig.mic_command_count(), 0);
    }
    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn record_and_play_require_button_actions_and_close_discards_recording() {
        gtk::init().unwrap();
        let (rig, source) = microphone_rig();
        let controls =
            MicTestControls::new(super::super::controls::DeviceControls::new(rig.handle()));
        controls.render(&rig.snapshot(), Some(source.clone()));
        assert_eq!(rig.mic_command_count(), 0);
        controls.record.emit_clicked();
        let until = |predicate: &dyn Fn(&AppSnapshot) -> bool| {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            while !predicate(&rig.snapshot()) {
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        };
        until(&|s| {
            s.mic_test
                .as_ref()
                .is_some_and(|test| test.phase == MicTestPhase::Ready)
        });
        controls.render(&rig.snapshot(), Some(source.clone()));
        assert_eq!(rig.mic_command_count(), 1);
        assert!(!controls.play.is_sensitive());
        controls.output.set_selected(1);
        assert!(controls.play.is_sensitive());
        assert_eq!(rig.mic_command_count(), 1);
        controls.play.emit_clicked();
        until(&|s| {
            s.mic_test
                .as_ref()
                .is_some_and(|test| test.output.is_some() && test.phase == MicTestPhase::Ready)
        });
        for _ in 0..10 {
            controls.render(&rig.snapshot(), Some(source.clone()));
        }
        assert_eq!(rig.mic_command_count(), 2);
        controls.close();
        until(&|s| s.mic_test.is_none());
        assert_eq!(rig.mic_command_count(), 3);
        controls.render(&rig.snapshot(), Some(source));
        controls.record.emit_clicked();
        controls.close();
        controls.close();
        until(&|snapshot| snapshot.mic_test.is_none() && rig.mic_command_count() >= 5);
        assert_eq!(rig.mic_command_count(), 5);
    }

    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn wave_headphones_remain_an_explicit_playback_choice() {
        gtk::init().unwrap();
        let rig = crate::ui::test_support::Rig::new(serde_json::json!({}), vec![]);
        let controls =
            MicTestControls::new(super::super::controls::DeviceControls::new(rig.handle()));
        assert!(gtk::test_accessible_has_relation(
            &controls.output,
            gtk::AccessibleRelation::LabelledBy
        ));
        let mut snapshot = (*rig.snapshot()).clone();
        snapshot.outputs = std::sync::Arc::new(vec![cadiswave_core::model::OutputSnapshot {
            identity: cadiswave_core::model::NodeIdentity {
                server_cookie: 1,
                object_serial: "20".into(),
            },
            node_id: 20,
            node_name: "fixture_wave_headphones".into(),
            name: "Wave headphones".into(),
            priority: 1,
            is_wave: true,
            properties: Default::default(),
        }]);
        controls.render(&snapshot, None);
        assert_eq!(controls.output.model().unwrap().n_items(), 2);
        assert_eq!(controls.output.selected(), 0);
        assert_eq!(rig.mic_command_count(), 0);
    }
    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn repeated_output_observations_preserve_the_dropdown_model() {
        gtk::init().unwrap();
        let rig = crate::ui::test_support::Rig::new(serde_json::json!({}), vec![]);
        let controls =
            MicTestControls::new(super::super::controls::DeviceControls::new(rig.handle()));
        let snapshot = rig.snapshot();
        controls.render(&snapshot, None);
        let model = controls.output.model();
        for _ in 0..20 {
            controls.render(&snapshot, None);
        }
        assert_eq!(controls.output.model(), model);
    }
    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn playback_rejects_a_replaced_output_before_widget_refresh() {
        gtk::init().unwrap();
        let (rig, source) = microphone_rig();
        let controls =
            MicTestControls::new(super::super::controls::DeviceControls::new(rig.handle()));
        controls.render(&rig.snapshot(), Some(source.clone()));
        controls.record.emit_clicked();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while rig
            .snapshot()
            .mic_test
            .as_ref()
            .is_none_or(|test| test.phase != MicTestPhase::Ready)
        {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        controls.render(&rig.snapshot(), Some(source));
        controls.output.set_selected(1);
        let mut replacement = rig.snapshot().outputs[0].clone();
        replacement.identity.object_serial = "21".into();
        rig.set_test_audio((*rig.snapshot().captures).clone(), vec![replacement]);
        controls.play.emit_clicked();
        while !rig.snapshot().errors.iter().any(|issue| {
            issue
                .message
                .contains("Selected microphone test output generation changed")
        }) {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(rig.mic_command_count(), 1);
        assert_eq!(
            rig.snapshot().mic_test.as_ref().unwrap().phase,
            MicTestPhase::Ready
        );
        controls.close();
    }
    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn automatic_close_preserves_external_recordings_and_explicit_stop_cancels_them() {
        gtk::init().unwrap();
        let (rig, source) = microphone_rig();
        let controls =
            MicTestControls::new(super::super::controls::DeviceControls::new(rig.handle()));
        controls.render(&rig.snapshot(), Some(source.clone()));
        let record = rig
            .handle()
            .submit(AppCommand::RecordMicTest {
                source: source.clone(),
                seconds: 1,
            })
            .unwrap();
        assert!(matches!(
            rig.outcome(record),
            cadiswave_core::model::CommandOutcome::Applied { .. }
        ));
        controls.render(&rig.snapshot(), Some(source));
        controls.close();
        let barrier = rig
            .handle()
            .submit(AppCommand::CloseMicTest {
                record: CommandId(u64::MAX),
            })
            .unwrap();
        assert!(matches!(
            rig.outcome(barrier),
            cadiswave_core::model::CommandOutcome::Applied { .. }
        ));
        assert_eq!(rig.mic_command_count(), 1);
        assert_eq!(
            rig.snapshot().mic_test.as_ref().unwrap().phase,
            MicTestPhase::Ready
        );
        controls.stop.emit_clicked();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while rig.snapshot().mic_test.is_some() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(rig.mic_command_count(), 2);
    }
    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn output_placeholder_is_translated_and_never_selects_an_output() {
        gtk::init().unwrap();
        let (rig, source) = microphone_rig();
        let controls =
            MicTestControls::new(super::super::controls::DeviceControls::new(rig.handle()));
        let locale = Rc::new(RefCell::new(
            crate::i18n::I18n::new(cadiswave_core::locale::LanguageChoice::English, "en").unwrap(),
        ));
        crate::i18n::activate(locale.clone());
        for (language, expected) in [
            (
                cadiswave_core::locale::LanguageChoice::English,
                "Select an output",
            ),
            (
                cadiswave_core::locale::LanguageChoice::Indonesian,
                "Pilih keluaran",
            ),
        ] {
            locale.borrow_mut().set_choice(language, "en").unwrap();
            for _ in 0..10 {
                controls.render(&rig.snapshot(), Some(source.clone()));
            }
            assert_eq!(controls.output.selected(), 0);
            let model = controls
                .output
                .model()
                .unwrap()
                .downcast::<gtk::StringList>()
                .unwrap();
            assert_eq!(model.string(0).as_deref(), Some(expected));
            assert!(controls.model.selected.borrow().is_none());
            assert!(!controls.play.is_sensitive());
            assert_eq!(rig.mic_command_count(), 0);
        }
    }
}
