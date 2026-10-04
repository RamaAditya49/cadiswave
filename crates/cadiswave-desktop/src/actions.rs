use cadiswave_core::{
    model::{AppSnapshot, MixId, OperationError, Result, SourceId},
    routing::source_groups,
    scenes::SceneId,
};
use cadiswave_runtime::controller::{AppCommand, RuntimeHandle};
use gio::prelude::*;
use glib::{
    Variant,
    variant::{StaticVariantType, ToVariant},
};
use std::{cell::Cell, rc::Rc};

pub struct ActionCallbacks {
    pub uninstall: Rc<dyn Fn()>,
    pub prepare_uninstall: Rc<dyn Fn(String)>,
}

pub struct ActionRegistry {
    snapshot: gio::SimpleAction,
    groups: gio::SimpleAction,
    mic_test: gio::SimpleAction,
    prepare: gio::SimpleAction,
    status: gio::SimpleAction,
    status_icon: Cell<&'static str>,
    revision: Cell<Option<u64>>,
}

impl ActionRegistry {
    pub fn register(
        app: &adw::Application,
        handle: RuntimeHandle,
        callbacks: ActionCallbacks,
    ) -> Self {
        for (name, signature) in [
            (
                "record-microphone-test",
                <(String, u32)>::static_variant_type(),
            ),
            (
                "play-microphone-test",
                <(u64, String)>::static_variant_type(),
            ),
            ("discard-microphone-test", u64::static_variant_type()),
            ("switch-group", String::static_variant_type()),
            ("set-source-level", <(String, f64)>::static_variant_type()),
            ("toggle-source-mute", String::static_variant_type()),
            (
                "set-cell-level",
                <(String, String, f64)>::static_variant_type(),
            ),
            (
                "toggle-cell-mute",
                <(String, String)>::static_variant_type(),
            ),
            ("apply-scene", String::static_variant_type()),
            ("save-scene", String::static_variant_type()),
            ("delete-scene", String::static_variant_type()),
            ("toggle-fx", <(String, String)>::static_variant_type()),
        ] {
            let action = gio::SimpleAction::new(name, Some(&signature));
            let runtime = handle.clone();
            action.connect_activate(move |_, parameter| {
                let result = parameter
                    .ok_or_else(|| OperationError::invalid("Missing action parameter"))
                    .and_then(|parameter| command(name, parameter, &runtime.snapshot()));
                match result {
                    Ok(command) => {
                        if let Err(error) = runtime.submit(command) {
                            log::warn!("Remote {name}: {error}");
                        }
                    }
                    Err(error) => log::warn!("Remote {name}: {error}"),
                }
            });
            app.add_action(&action);
        }
        let snapshot = read_action(app, "snapshot", "{}".to_variant(), &handle);
        let mic_test = read_action(app, "microphone-test", "null".to_variant(), &handle);
        let groups = read_action(
            app,
            "source-groups",
            Vec::<String>::new().to_variant(),
            &handle,
        );
        read_action(app, "scenes", "{}".to_variant(), &handle);
        read_action(app, "levels", "{}".to_variant(), &handle);
        let uninstall = gio::SimpleAction::new("uninstall", None);
        uninstall.connect_activate(move |_, _| (callbacks.uninstall)());
        app.add_action(&uninstall);
        let prepare = gio::SimpleAction::new_stateful(
            "prepare-uninstall",
            Some(&String::static_variant_type()),
            &"idle".to_variant(),
        );
        prepare.connect_change_state(|_, _| {});
        prepare.connect_activate(move |_, parameter| {
            if let Some(identity) = parameter.and_then(|value| value.get::<String>()) {
                (callbacks.prepare_uninstall)(identity);
            }
        });
        app.add_action(&prepare);
        let status =
            gio::SimpleAction::new_stateful("status-icon", None, &"cadiswave-white".to_variant());
        status.connect_change_state(|_, _| {});
        status.connect_activate(|_, _| {});
        app.add_action(&status);
        let registry = Self {
            snapshot,
            groups,
            mic_test,
            prepare,
            status,
            status_icon: Cell::new("cadiswave-white"),
            revision: Cell::new(None),
        };
        registry.refresh(&handle.snapshot());
        registry
    }

    pub fn refresh(&self, snapshot: &AppSnapshot) {
        let icon = crate::icons::status_icon(snapshot);
        if self.status_icon.replace(icon) != icon {
            self.status.set_state(&icon.to_variant());
        }
        // Audio completion does not change the desired routing revision.
        match snapshot.action_mic_test() {
            Ok(json) => self.mic_test.set_state(&json.to_variant()),
            Err(error) => log::warn!("Publishing microphone test state: {error}"),
        }
        if self.revision.get() == Some(snapshot.revision) {
            return;
        }
        match snapshot.action_snapshot() {
            Ok(json) => {
                self.snapshot.set_state(&json.to_variant());
                self.groups
                    .set_state(&source_groups(&snapshot.desired.sources).to_variant());
                self.revision.set(Some(snapshot.revision));
            }
            Err(error) => log::warn!("Publishing remote snapshot: {error}"),
        }
    }

    pub fn set_uninstall_state(&self, state: &str) {
        self.prepare.set_state(&state.to_variant());
    }
}

fn read_action(
    app: &adw::Application,
    name: &'static str,
    initial: Variant,
    handle: &RuntimeHandle,
) -> gio::SimpleAction {
    let action = gio::SimpleAction::new_stateful(name, None, &initial);
    action.connect_change_state(|_, _| {});
    let handle = handle.clone();
    action.connect_activate(move |action, _| {
        let snapshot = handle.snapshot();
        if name == "source-groups" {
            action.set_state(&source_groups(&snapshot.desired.sources).to_variant());
            return;
        }
        let result = match name {
            "snapshot" => snapshot.action_snapshot(),
            "microphone-test" => snapshot.action_mic_test(),
            "scenes" => snapshot.action_scenes(),
            "levels" => snapshot.action_levels(),
            _ => return,
        };
        match result {
            Ok(json) => action.set_state(&json.to_variant()),
            Err(error) => log::warn!("Reading remote {name}: {error}"),
        }
    });
    app.add_action(&action);
    action
}

fn parameter<T: glib::variant::FromVariant>(value: &Variant) -> Result<T> {
    value
        .get()
        .ok_or_else(|| OperationError::invalid("Wrong action parameter type"))
}

fn command(name: &str, value: &Variant, snapshot: &AppSnapshot) -> Result<AppCommand> {
    Ok(match name {
        "record-microphone-test" => {
            let (source, seconds): (String, u32) = parameter(value)?;
            AppCommand::RecordMicTest {
                source: SourceId::new(source)?,
                seconds,
            }
        }
        "play-microphone-test" => {
            let (session, name): (u64, String) = parameter(value)?;
            let mut outputs = snapshot
                .outputs
                .iter()
                .filter(|output| output.node_name == name);
            let output = outputs.next().ok_or_else(|| {
                OperationError::invalid("Select a connected microphone test output")
            })?;
            if outputs.next().is_some() || output.identity.object_serial.is_empty() {
                return Err(OperationError::invalid(
                    "Microphone test output identity is ambiguous",
                ));
            }
            AppCommand::PlayMicTest {
                session,
                output: cadiswave_core::mic_test::PlaybackTarget {
                    node_name: output.node_name.clone(),
                    identity: output.identity.clone(),
                },
            }
        }
        "discard-microphone-test" => AppCommand::CancelMicTest {
            session: parameter(value)?,
        },
        "switch-group" => AppCommand::SwitchGroup {
            group: parameter(value)?,
        },
        "toggle-source-mute" => AppCommand::ToggleSourceMute {
            source: SourceId::new(parameter::<String>(value)?)?,
        },
        "set-source-level" => {
            let (source, level): (String, f64) = parameter(value)?;
            AppCommand::SetSourceLevel {
                source: SourceId::new(source)?,
                level,
            }
        }
        "set-cell-level" => {
            let (source, mix, level): (String, String, f64) = parameter(value)?;
            let source = SourceId::new(source)?;
            let mix = MixId::new(mix)?;
            // Resolve the current mute inside the controller, after any queued toggle.
            AppCommand::SetCellLevel { source, mix, level }
        }
        "toggle-cell-mute" => {
            let (source, mix): (String, String) = parameter(value)?;
            AppCommand::ToggleCellMute {
                source: SourceId::new(source)?,
                mix: MixId::new(mix)?,
            }
        }
        "toggle-fx" => {
            let (source, effect): (String, String) = parameter(value)?;
            AppCommand::ToggleFx {
                source: SourceId::new(source)?,
                effect,
            }
        }
        "save-scene" => AppCommand::SaveScene {
            name: parameter(value)?,
        },
        "apply-scene" => AppCommand::ApplyScene {
            scene: SceneId::new(parameter::<String>(value)?)?,
        },
        "delete-scene" => AppCommand::DeleteScene {
            scene: SceneId::new(parameter::<String>(value)?)?,
        },
        _ => return Err(OperationError::invalid("Unknown remote action")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(name: &str, value: &Variant) -> Result<AppCommand> {
        let snapshot = AppSnapshot {
            outputs: std::sync::Arc::new(vec![cadiswave_core::model::OutputSnapshot {
                identity: cadiswave_core::model::NodeIdentity {
                    server_cookie: 1,
                    object_serial: "20".into(),
                },
                node_id: 20,
                node_name: "fixture_output".into(),
                name: "Headphones".into(),
                priority: 0,
                is_wave: false,
                properties: Default::default(),
            }]),
            ..AppSnapshot::default()
        };
        command(name, value, &snapshot)
    }
    #[test]
    fn microphone_test_actions_parse_only_their_declared_types() {
        let source = SourceId::new("mic_a").unwrap();
        assert!(
            matches!(parse("record-microphone-test", &("mic_a", 1_u32).to_variant()).unwrap(), AppCommand::RecordMicTest { source: actual, seconds: 1 } if actual == source)
        );
        assert!(
            matches!(parse("play-microphone-test", &(12_u64, "fixture_output").to_variant()).unwrap(), AppCommand::PlayMicTest { session: 12, output } if output.node_name == "fixture_output" && output.identity.object_serial == "20")
        );
        assert!(matches!(
            parse("discard-microphone-test", &12_u64.to_variant()).unwrap(),
            AppCommand::CancelMicTest { session: 12 }
        ));
        for (action, parameter) in [
            ("record-microphone-test", ("mic_a", -1_i32).to_variant()),
            ("record-microphone-test", ("", 1_u32).to_variant()),
            (
                "play-microphone-test",
                (12_u32, "fixture_output").to_variant(),
            ),
            ("discard-microphone-test", 12_u32.to_variant()),
        ] {
            assert!(parse(action, &parameter).is_err());
        }
    }
}
