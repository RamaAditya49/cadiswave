use crate::ui::device::controls::{ControlError, DeviceControls};
use adw::prelude::*;
use cadiswave_core::{
    device_settings::microphone_sources, effects::FxSettings, model::*,
    voice_presets::BuiltinPreset,
};
use cadiswave_runtime::controller::{AppCommand, EditTiming};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn source_settings(snapshot: &AppSnapshot, source: &SourceId) -> Result<FxSettings> {
    if !microphone_sources(snapshot).contains(source) {
        return Err(OperationError::unavailable(crate::i18n::tr(
            "software-source-unavailable",
        )));
    }
    Ok(snapshot
        .pending_fx
        .get(source)
        .cloned()
        .or_else(|| {
            snapshot
                .desired
                .sources
                .get(source)
                .and_then(|source| source.fx.clone())
        })
        .unwrap_or_default())
}

#[derive(Debug, PartialEq)]
enum ActionState {
    Idle,
    Pending(CommandId),
    Applied,
    Failed(String),
}
impl ActionState {
    fn complete(&mut self, id: CommandId, outcome: &CommandOutcome) -> bool {
        if *self != Self::Pending(id) {
            return false;
        }
        *self = match outcome {
            CommandOutcome::Applied { .. } => Self::Applied,
            CommandOutcome::Rejected(error) => Self::Failed(error.to_string()),
            CommandOutcome::Cancelled => Self::Failed(crate::i18n::tr("software-action-cancelled")),
            other => Self::Failed(format!("{other:?}")),
        };
        true
    }
}

#[derive(Default)]
struct SourceChoice {
    selected: Option<SourceId>,
}
impl SourceChoice {
    fn refresh(&mut self, candidates: &[SourceId]) {
        if !self
            .selected
            .as_ref()
            .is_some_and(|id| candidates.contains(id))
        {
            self.selected = (candidates.len() == 1).then(|| candidates[0].clone());
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum PresetChoice {
    Builtin(BuiltinPreset),
    Saved(String),
}
#[derive(Clone, Copy)]
enum ActionKind {
    Effects,
    Save,
    Delete,
}
struct Model {
    controls: DeviceControls,
    source: RefCell<SourceChoice>,
    source_ids: RefCell<Vec<SourceId>>,
    source_labels: RefCell<Vec<String>>,
    saved_names: RefCell<Vec<String>>,
    saved_labels: RefCell<Vec<String>>,
    preset: RefCell<Option<PresetChoice>>,
    updating: Cell<bool>,
    state: RefCell<ActionState>,
    action: Cell<ActionKind>,
    source_changed: RefCell<Option<Rc<dyn Fn()>>>,
    source_index: Cell<u32>,
    lowcut_index: Cell<u32>,
    builtin_index: Cell<u32>,
    saved_index: Cell<u32>,
}
struct Widgets {
    source: adw::ComboRow,
    lowcut: adw::ComboRow,
    builtin: adw::ComboRow,
    saved: adw::ComboRow,
    apply: gtk::Button,
    name: adw::EntryRow,
    save: gtk::Button,
    delete: gtk::Button,
    feedback: gtk::Label,
    details: gtk::Label,
}
pub struct SoftwareSettings {
    pub group: adw::PreferencesGroup,
    widgets: Rc<Widgets>,
    model: Rc<Model>,
}
fn combo(group: &adw::PreferencesGroup, title: &str, hint: &str) -> adw::ComboRow {
    let row = adw::ComboRow::new();
    crate::i18n::bind(&row, "title", title);
    crate::i18n::bind(&row, "subtitle", hint);
    row.set_subtitle_lines(0);
    group.add(&row);
    row
}
fn button(key: &str) -> gtk::Button {
    let button = gtk::Button::new();
    crate::i18n::bind(&button, "label", key);
    button.set_valign(gtk::Align::Center);
    button
}
fn action_row(group: &adw::PreferencesGroup, title: &str, hint: &str, button: &gtk::Button) {
    let row = adw::ActionRow::new();
    crate::i18n::bind(&row, "title", title);
    crate::i18n::bind(&row, "subtitle", hint);
    row.set_subtitle_lines(0);
    row.add_suffix(button);
    group.add(&row);
}
impl SoftwareSettings {
    pub fn new(controls: DeviceControls) -> Self {
        let group = adw::PreferencesGroup::new();
        crate::i18n::bind(&group, "title", "software-processing");
        crate::i18n::bind(&group, "description", "software-processing-hint");
        let source = combo(&group, "software-source", "software-source-hint");
        crate::i18n::protect(&source);
        let lowcut = combo(&group, "low-cut-software", "software-lowcut-hint");
        let builtin = combo(&group, "software-builtin-presets", "software-builtin-hint");
        let saved = combo(&group, "software-saved-presets", "software-saved-hint");
        crate::i18n::protect(&saved);
        let apply = button("software-apply");
        apply.add_css_class("suggested-action");
        action_row(
            &group,
            "software-apply-preset",
            "software-apply-hint",
            &apply,
        );
        let name = adw::EntryRow::new();
        crate::i18n::bind(&name, "title", "software-preset-name");
        crate::i18n::protect(&name);
        group.add(&name);
        let save = button("software-save");
        action_row(&group, "software-save-preset", "software-save-hint", &save);
        let delete = button("software-delete");
        delete.add_css_class("destructive-action");
        action_row(
            &group,
            "software-delete-preset",
            "software-delete-hint",
            &delete,
        );
        let feedback = gtk::Label::new(None);
        feedback.set_wrap(true);
        feedback.set_xalign(0.0);
        let details = gtk::Label::new(None);
        details.set_wrap(true);
        details.set_selectable(true);
        details.set_xalign(0.0);
        crate::i18n::protect(&details);
        let footer = gtk::Box::new(gtk::Orientation::Vertical, 6);
        footer.set_margin_top(12);
        footer.append(&feedback);
        footer.append(&details);
        group.add(&footer);
        let widgets = Rc::new(Widgets {
            source,
            lowcut,
            builtin,
            saved,
            apply,
            name,
            save,
            delete,
            feedback,
            details,
        });
        let model = Rc::new(Model {
            controls,
            source: RefCell::new(SourceChoice::default()),
            source_ids: RefCell::default(),
            source_labels: RefCell::default(),
            saved_names: RefCell::default(),
            saved_labels: RefCell::default(),
            preset: RefCell::new(None),
            updating: Cell::new(false),
            state: RefCell::new(ActionState::Idle),
            action: Cell::new(ActionKind::Effects),
            source_changed: RefCell::new(None),
            source_index: Cell::new(gtk::INVALID_LIST_POSITION),
            lowcut_index: Cell::new(gtk::INVALID_LIST_POSITION),
            builtin_index: Cell::new(gtk::INVALID_LIST_POSITION),
            saved_index: Cell::new(gtk::INVALID_LIST_POSITION),
        });
        let target = model.clone();
        let weak = Rc::downgrade(&widgets);
        widgets.source.connect_selected_notify(move |row| {
            if !selection_changed(&target, &target.source_index, row.selected()) {
                return;
            }
            let selected = row
                .selected()
                .checked_sub(1)
                .and_then(|index| target.source_ids.borrow().get(index as usize).cloned());
            let changed = target.source.borrow().selected != selected;
            target.source.borrow_mut().selected = selected;
            if changed && let Some(callback) = target.source_changed.borrow().clone() {
                callback();
            }
            if let Some(widgets) = weak.upgrade() {
                render_widgets(&target, &widgets, &target.controls.snapshot());
            }
        });
        let target = model.clone();
        let weak = Rc::downgrade(&widgets);
        widgets.lowcut.connect_selected_notify(move |row| {
            if !selection_changed(&target, &target.lowcut_index, row.selected()) {
                return;
            }
            let Some(widgets) = weak.upgrade() else {
                return;
            };
            let Some(lowcut) = [0, 80, 120].get(row.selected() as usize).copied() else {
                return;
            };
            let command = selected_settings(&target).map(|(source, mut settings)| {
                settings.lowcut = lowcut;
                AppCommand::SetFx {
                    source,
                    settings,
                    timing: EditTiming::Immediate,
                }
            });
            submit(&target, &widgets, ActionKind::Effects, command);
        });
        let target = model.clone();
        let weak = Rc::downgrade(&widgets);
        widgets.builtin.connect_selected_notify(move |row| {
            if !selection_changed(&target, &target.builtin_index, row.selected()) {
                return;
            }
            *target.preset.borrow_mut() = match row.selected() {
                1 => Some(PresetChoice::Builtin(BuiltinPreset::Meeting)),
                2 => Some(PresetChoice::Builtin(BuiltinPreset::Podcast)),
                3 => Some(PresetChoice::Builtin(BuiltinPreset::Streaming)),
                _ => None,
            };
            if let Some(widgets) = weak.upgrade() {
                render_widgets(&target, &widgets, &target.controls.snapshot());
            }
        });
        let target = model.clone();
        let weak = Rc::downgrade(&widgets);
        widgets.saved.connect_selected_notify(move |row| {
            if !selection_changed(&target, &target.saved_index, row.selected()) {
                return;
            }
            *target.preset.borrow_mut() = row
                .selected()
                .checked_sub(1)
                .and_then(|index| target.saved_names.borrow().get(index as usize).cloned())
                .map(PresetChoice::Saved);
            if let Some(widgets) = weak.upgrade() {
                render_widgets(&target, &widgets, &target.controls.snapshot());
            }
        });
        let target = model.clone();
        let weak = Rc::downgrade(&widgets);
        widgets.apply.connect_clicked(move |_| {
            let Some(widgets) = weak.upgrade() else {
                return;
            };
            let snapshot = target.controls.snapshot();
            let command = selected_settings(&target).and_then(|(source, _)| {
                preset_settings(&target, &snapshot).map(|settings| AppCommand::SetFx {
                    source,
                    settings,
                    timing: EditTiming::Immediate,
                })
            });
            submit(&target, &widgets, ActionKind::Effects, command);
        });
        let target = model.clone();
        let weak = Rc::downgrade(&widgets);
        widgets.save.connect_clicked(move |_| {
            let Some(widgets) = weak.upgrade() else {
                return;
            };
            let name = widgets.name.text().to_string();
            let command = selected_settings(&target).and_then(|(_, settings)| {
                let mut presets = target.controls.snapshot().preferences.voice_presets.clone();
                presets.save(&name, settings.clone())?;
                Ok(AppCommand::SaveVoicePreset { name, settings })
            });
            submit(&target, &widgets, ActionKind::Save, command);
        });
        let target = model.clone();
        let weak = Rc::downgrade(&widgets);
        widgets.delete.connect_clicked(move |_| {
            let Some(widgets) = weak.upgrade() else {
                return;
            };
            let command = match &*target.preset.borrow() {
                Some(PresetChoice::Saved(name))
                    if target
                        .controls
                        .snapshot()
                        .preferences
                        .voice_presets
                        .get(name)
                        .is_some() =>
                {
                    Ok(AppCommand::DeleteVoicePreset { name: name.clone() })
                }
                _ => Err(OperationError::invalid(crate::i18n::tr(
                    "software-preset-unavailable",
                ))),
            };
            submit(&target, &widgets, ActionKind::Delete, command);
        });
        let result = Self {
            group,
            widgets,
            model,
        };
        result.retranslate();
        result
    }
    pub fn render(&self, snapshot: &AppSnapshot) {
        render_widgets(&self.model, &self.widgets, snapshot);
    }
    pub fn completed(&self, id: CommandId, outcome: &CommandOutcome) -> bool {
        let matched = self.model.state.borrow_mut().complete(id, outcome);
        if matched {
            self.render(&self.model.controls.snapshot());
        }
        matched
    }
    pub fn selected_source(&self) -> Option<SourceId> {
        self.model.source.borrow().selected.clone()
    }
    pub fn connect_source_changed(&self, callback: impl Fn() + 'static) {
        *self.model.source_changed.borrow_mut() = Some(Rc::new(callback));
    }
    pub fn retranslate(&self) {
        self.model.updating.set(true);
        self.widgets.lowcut.set_model(Some(&gtk::StringList::new(&[
            &crate::i18n::tr("software-lowcut-off"),
            "80 Hz",
            "120 Hz",
        ])));
        self.widgets.builtin.set_model(Some(&gtk::StringList::new(&[
            &crate::i18n::tr("software-preset-select"),
            &crate::i18n::tr("software-preset-meeting"),
            &crate::i18n::tr("software-preset-podcast"),
            &crate::i18n::tr("software-preset-streaming"),
        ])));
        for (widget, key) in [
            (
                self.widgets.source.upcast_ref::<gtk::Widget>(),
                "software-source",
            ),
            (self.widgets.lowcut.upcast_ref(), "low-cut-software"),
            (
                self.widgets.builtin.upcast_ref(),
                "software-builtin-presets",
            ),
            (self.widgets.saved.upcast_ref(), "software-saved-presets"),
            (self.widgets.name.upcast_ref(), "software-preset-name"),
            (self.widgets.apply.upcast_ref(), "software-apply"),
            (self.widgets.save.upcast_ref(), "software-save"),
            (self.widgets.delete.upcast_ref(), "software-delete"),
        ] {
            widget.update_property(&[gtk::accessible::Property::Label(&crate::i18n::tr(key))]);
        }
        self.model.source_labels.borrow_mut().clear();
        self.model.saved_labels.borrow_mut().clear();
        self.model.updating.set(false);
        self.render(&self.model.controls.snapshot());
    }
}
fn selected_settings(model: &Model) -> Result<(SourceId, FxSettings)> {
    let source = model.source.borrow().selected.clone().ok_or_else(|| {
        OperationError::unavailable(crate::i18n::tr("software-source-unavailable"))
    })?;
    let settings = source_settings(&model.controls.snapshot(), &source)?;
    Ok((source, settings))
}
fn preset_settings(model: &Model, snapshot: &AppSnapshot) -> Result<FxSettings> {
    match &*model.preset.borrow() {
        Some(PresetChoice::Builtin(preset)) => Ok(preset.settings()),
        Some(PresetChoice::Saved(name)) => snapshot
            .preferences
            .voice_presets
            .get(name)
            .cloned()
            .ok_or_else(|| OperationError::invalid(crate::i18n::tr("software-preset-unavailable"))),
        None => Err(OperationError::invalid(crate::i18n::tr(
            "software-preset-unavailable",
        ))),
    }
}
fn submit(model: &Model, widgets: &Widgets, action: ActionKind, command: Result<AppCommand>) {
    if matches!(*model.state.borrow(), ActionState::Pending(_)) {
        return;
    }
    model.action.set(action);
    let result = command.and_then(|command| {
        model.controls.submit(command).map_err(|error| {
            OperationError::unavailable(match error {
                ControlError::Runtime(message) => message,
                other => format!("{other:?}"),
            })
        })
    });
    *model.state.borrow_mut() = match result {
        Ok(id) => ActionState::Pending(id),
        Err(error) => ActionState::Failed(error.to_string()),
    };
    render_widgets(model, widgets, &model.controls.snapshot());
}
fn selection_changed(model: &Model, observed: &Cell<u32>, selected: u32) -> bool {
    if model.updating.get() || observed.get() == selected {
        return false;
    }
    observed.set(selected);
    true
}
fn select(row: &adw::ComboRow, observed: &Cell<u32>, selected: u32) {
    // GTK can defer a selection notification until the current callback returns.
    observed.set(selected);
    row.set_selected(selected);
}
fn set_model(row: &adw::ComboRow, cache: &RefCell<Vec<String>>, labels: Vec<String>) {
    if *cache.borrow() == labels {
        return;
    }
    let values: Vec<_> = labels.iter().map(String::as_str).collect();
    row.set_model(Some(&gtk::StringList::new(&values)));
    *cache.borrow_mut() = labels;
}
fn render_widgets(model: &Model, widgets: &Widgets, snapshot: &AppSnapshot) {
    model.updating.set(true);
    let ids = microphone_sources(snapshot);
    model.source.borrow_mut().refresh(&ids);
    let mut labels = vec![crate::i18n::tr(if ids.is_empty() {
        "software-source-none"
    } else {
        "software-source-select"
    })];
    labels.extend(ids.iter().map(|id| {
        let source = &snapshot.desired.sources[id];
        crate::i18n::format(
            "software-source-choice",
            &[("name", &source.name), ("node", &source.node_name)],
        )
    }));
    *model.source_ids.borrow_mut() = ids.clone();
    set_model(&widgets.source, &model.source_labels, labels);
    let source = model.source.borrow().selected.clone();
    select(
        &widgets.source,
        &model.source_index,
        source
            .as_ref()
            .and_then(|id| ids.iter().position(|candidate| candidate == id))
            .map_or(0, |index| index as u32 + 1),
    );
    let settings = source
        .as_ref()
        .and_then(|id| source_settings(snapshot, id).ok());
    select(
        &widgets.lowcut,
        &model.lowcut_index,
        settings
            .as_ref()
            .map_or(0, |settings| match settings.lowcut {
                80 => 1,
                120 => 2,
                _ => 0,
            }),
    );
    let mut names: Vec<String> = snapshot
        .preferences
        .voice_presets
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    let mut labels = vec![crate::i18n::tr("software-preset-select")];
    labels.extend(names.iter().cloned());
    if let Some(PresetChoice::Saved(name)) = &*model.preset.borrow()
        && !names.contains(name)
    {
        names.push(name.clone());
        labels.push(crate::i18n::format(
            "software-preset-missing",
            &[("name", name)],
        ));
    }
    *model.saved_names.borrow_mut() = names.clone();
    set_model(&widgets.saved, &model.saved_labels, labels);
    let preset = model.preset.borrow().clone();
    select(
        &widgets.builtin,
        &model.builtin_index,
        match preset {
            Some(PresetChoice::Builtin(BuiltinPreset::Meeting)) => 1,
            Some(PresetChoice::Builtin(BuiltinPreset::Podcast)) => 2,
            Some(PresetChoice::Builtin(BuiltinPreset::Streaming)) => 3,
            _ => 0,
        },
    );
    select(
        &widgets.saved,
        &model.saved_index,
        match &preset {
            Some(PresetChoice::Saved(name)) => names
                .iter()
                .position(|candidate| candidate == name)
                .map_or(0, |index| index as u32 + 1),
            _ => 0,
        },
    );
    model.updating.set(false);
    let pending = matches!(*model.state.borrow(), ActionState::Pending(_));
    widgets.source.set_sensitive(!pending && !ids.is_empty());
    widgets.lowcut.set_sensitive(!pending && settings.is_some());
    widgets.builtin.set_sensitive(!pending);
    widgets.saved.set_sensitive(!pending && !names.is_empty());
    widgets
        .apply
        .set_sensitive(!pending && settings.is_some() && preset_settings(model, snapshot).is_ok());
    widgets.name.set_sensitive(!pending);
    widgets.save.set_sensitive(!pending && settings.is_some());
    widgets.delete.set_sensitive(!pending && matches!(&preset, Some(PresetChoice::Saved(name)) if snapshot.preferences.voice_presets.get(name).is_some()));
    let state = model.state.borrow();
    let key = match (&*state, model.action.get()) {
        (ActionState::Idle, _) => None,
        (ActionState::Pending(_), ActionKind::Effects) => Some("software-fx-pending"),
        (ActionState::Pending(_), ActionKind::Save) => Some("software-save-pending"),
        (ActionState::Pending(_), ActionKind::Delete) => Some("software-delete-pending"),
        (ActionState::Applied, ActionKind::Effects) => Some("software-fx-applied"),
        (ActionState::Applied, ActionKind::Save) => Some("software-save-success"),
        (ActionState::Applied, ActionKind::Delete) => Some("software-delete-success"),
        (ActionState::Failed(_), _) => Some("software-action-failed"),
    };
    widgets
        .feedback
        .set_text(&key.map(crate::i18n::tr).unwrap_or_default());
    widgets.feedback.set_visible(key.is_some());
    let error = match &*state {
        ActionState::Failed(error) => error.as_str(),
        _ => "",
    };
    widgets.details.set_text(error);
    widgets.details.set_visible(!error.is_empty());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn fixture(nodes: &[&str]) -> (crate::ui::test_support::Rig, Vec<SourceId>) {
        let mut sources = Sources::new();
        let mut ids = Vec::new();
        for node in nodes {
            let mut source = Source::new("Mic".into(), SourceKind::Device);
            source.node_name = (*node).into();
            source.fx = Some(FxSettings {
                comp: true,
                eq_mid: 3.0,
                ..Default::default()
            });
            ids.push(source.id.clone());
            sources.insert(source.id.clone(), source);
        }
        let rig = crate::ui::test_support::Rig::new_with_sources(
            serde_json::json!({}),
            vec![crate::ui::test_support::unit("A", 2, -10.0)],
            serde_json::to_value(sources).unwrap(),
        );
        rig.set_captures(nodes.iter().map(|node| capture(node, "A")).collect());
        (rig, ids)
    }
    fn capture(node: &str, serial: &str) -> CaptureSnapshot {
        CaptureSnapshot {
            identity: NodeIdentity {
                server_cookie: 1,
                object_serial: node.into(),
            },
            node_id: if node.ends_with('2') { 11 } else { 10 },
            node_name: node.into(),
            name: "Mic".into(),
            muted: Observation::Known(false),
            channels: Some(1),
            properties: [("device.serial".into(), serial.into())]
                .into_iter()
                .collect(),
        }
    }
    fn finish(settings: &SoftwareSettings, rig: &crate::ui::test_support::Rig) -> CommandId {
        let id = match *settings.model.state.borrow() {
            ActionState::Pending(id) => id,
            ref state => panic!("Expected a pending command: {state:?}"),
        };
        let outcome = rig.outcome(id);
        assert!(
            matches!(outcome, CommandOutcome::Applied { .. }),
            "{outcome:?}"
        );
        assert!(settings.completed(id, &outcome));
        id
    }

    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn refresh_preserves_drafts_and_lowcut_keeps_other_effects() {
        adw::init().unwrap();
        let (rig, ids) = fixture(&["fixture_mic", "fixture_mic2"]);
        let a = ids[0].clone();
        let b = ids[1].clone();
        let settings = SoftwareSettings::new(DeviceControls::new(rig.handle()));
        assert_eq!(settings.selected_source(), None);
        let source_changes = Rc::new(Cell::new(0));
        let changes = source_changes.clone();
        settings.connect_source_changed(move || changes.set(changes.get() + 1));
        settings.widgets.source.set_selected(2);
        assert_eq!(settings.selected_source(), Some(b.clone()));
        settings.widgets.name.set_text("My voice");
        settings.widgets.builtin.set_selected(2);
        let routing = rig.routing_command_count();
        let source_model = settings.widgets.source.model().unwrap();
        let saved_model = settings.widgets.saved.model().unwrap();
        for _ in 0..10 {
            settings.render(&rig.snapshot());
        }
        assert_eq!(settings.widgets.source.model().unwrap(), source_model);
        assert_eq!(settings.widgets.saved.model().unwrap(), saved_model);
        assert_eq!(settings.widgets.name.text(), "My voice");
        assert_eq!(settings.widgets.builtin.selected(), 2);
        assert_eq!(settings.selected_source(), Some(b.clone()));
        assert_eq!(rig.routing_command_count(), routing);
        assert_eq!(source_changes.get(), 1);
        settings.widgets.lowcut.set_selected(1);
        let id = finish(&settings, &rig);
        assert!(!settings.completed(CommandId(id.0 + 100), &CommandOutcome::Cancelled));
        let snapshot = rig.snapshot();
        let actual = snapshot.desired.sources[&b].fx.as_ref().unwrap();
        assert_eq!(actual.lowcut, 80);
        assert!(actual.comp);
        assert_eq!(actual.eq_mid, 3.0);
        assert_eq!(snapshot.desired.sources[&a].fx.as_ref().unwrap().lowcut, 0);
        assert_eq!(settings.widgets.name.text(), "My voice");
    }

    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn presets_apply_only_on_click_and_named_save_and_delete_are_explicit() {
        adw::init().unwrap();
        let (rig, ids) = fixture(&["fixture_mic"]);
        let source = ids[0].clone();
        let settings = SoftwareSettings::new(DeviceControls::new(rig.handle()));
        let routing = rig.routing_command_count();
        settings.widgets.builtin.set_selected(1);
        settings.render(&rig.snapshot());
        assert_eq!(rig.routing_command_count(), routing);
        assert_eq!(
            rig.snapshot().desired.sources[&source]
                .fx
                .as_ref()
                .unwrap()
                .lowcut,
            0
        );
        settings.widgets.apply.emit_clicked();
        finish(&settings, &rig);
        let actual = rig.snapshot().desired.sources[&source].fx.clone().unwrap();
        assert_eq!(actual.lowcut, 120);
        assert!(actual.gate);
        assert_eq!(actual.comp_ratio, 2.0);
        settings.widgets.name.set_text("Meeting at home");
        settings.widgets.save.emit_clicked();
        finish(&settings, &rig);
        assert_eq!(
            rig.snapshot()
                .preferences
                .voice_presets
                .get("Meeting at home"),
            Some(&actual)
        );
        assert_eq!(settings.widgets.name.text(), "Meeting at home");
        settings.widgets.saved.set_selected(1);
        settings.widgets.delete.emit_clicked();
        finish(&settings, &rig);
        assert!(rig.snapshot().preferences.voice_presets.is_empty());
        assert_eq!(
            rig.snapshot().desired.sources[&source].fx.as_ref(),
            Some(&actual)
        );
        assert_eq!(settings.widgets.name.text(), "Meeting at home");
        settings.widgets.save.emit_clicked();
        finish(&settings, &rig);
        settings.widgets.name.set_text(" invalid ");
        settings.widgets.save.emit_clicked();
        assert_eq!(
            settings.widgets.details.text(),
            "Preset names need 1–48 characters without outer spaces or control characters"
        );
        assert_eq!(settings.widgets.name.text(), " invalid ");
        assert_eq!(rig.snapshot().preferences.voice_presets.len(), 1);
    }

    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn stale_microphone_callback_does_not_retarget_another_source() {
        adw::init().unwrap();
        let (rig, ids) = fixture(&["fixture_mic", "fixture_mic2"]);
        let a = ids[0].clone();
        let b = ids[1].clone();
        let settings = SoftwareSettings::new(DeviceControls::new(rig.handle()));
        settings.widgets.source.set_selected(1);
        assert_eq!(settings.selected_source(), Some(a.clone()));
        rig.set_captures(vec![capture("fixture_mic2", "A")]);
        assert_eq!(rig.snapshot().captures.len(), 1);
        assert_eq!(microphone_sources(&rig.snapshot()), vec![b.clone()]);
        let routing = rig.routing_command_count();
        assert_eq!(settings.widgets.lowcut.selected(), 0);
        settings.widgets.lowcut.set_selected(1);
        assert!(
            matches!(*settings.model.state.borrow(), ActionState::Failed(_)),
            "{:?}",
            settings.model.state.borrow()
        );
        assert!(!settings.widgets.details.text().is_empty());
        assert_eq!(rig.routing_command_count(), routing);
        let snapshot = rig.snapshot();
        assert_eq!(snapshot.desired.sources[&a].fx.as_ref().unwrap().lowcut, 0);
        assert_eq!(snapshot.desired.sources[&b].fx.as_ref().unwrap().lowcut, 0);
    }

    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn narrow_localized_layout_keeps_user_names_and_actions_readable() {
        use cadiswave_core::locale::LanguageChoice;
        adw::init().unwrap();
        let (rig, ids) = fixture(&["fixture_mic"]);
        let locale = Rc::new(RefCell::new(
            crate::i18n::I18n::new(LanguageChoice::English, "en").unwrap(),
        ));
        crate::i18n::activate(locale.clone());
        let settings = SoftwareSettings::new(DeviceControls::new(rig.handle()));
        settings.widgets.name.set_text("Meeting");
        settings.widgets.builtin.set_selected(2);
        let dialog = adw::PreferencesDialog::builder()
            .content_width(340)
            .content_height(950)
            .build();
        let page = adw::PreferencesPage::new();
        page.add(&settings.group);
        dialog.add(&page);
        let window = adw::Window::builder()
            .default_width(390)
            .default_height(1000)
            .build();
        window.present();
        dialog.present(Some(&window));
        for (width, choice, suffix) in [
            (390, LanguageChoice::English, "en"),
            (640, LanguageChoice::English, "en"),
            (390, LanguageChoice::Indonesian, "id"),
            (640, LanguageChoice::Indonesian, "id"),
        ] {
            window.set_default_size(width, 1000);
            dialog.set_content_width(width - 20);
            locale.borrow_mut().set_choice(choice, "en").unwrap();
            crate::i18n::bind_tree(&settings.group);
            crate::i18n::retranslate();
            settings.retranslate();
            settings.render(&rig.snapshot());
            let until = std::time::Instant::now() + std::time::Duration::from_millis(300);
            while std::time::Instant::now() < until {
                while glib::MainContext::default().pending() {
                    glib::MainContext::default().iteration(false);
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            assert_eq!(settings.widgets.name.text(), "Meeting");
            assert_eq!(settings.widgets.builtin.selected(), 2);
            assert_eq!(settings.selected_source(), Some(ids[0].clone()));
            assert!(dialog.width() <= width);
            for button in [
                &settings.widgets.apply,
                &settings.widgets.save,
                &settings.widgets.delete,
            ] {
                let bounds = button.compute_bounds(&dialog).unwrap();
                assert!(bounds.width() >= 40.0);
                assert!(bounds.x() + bounds.width() <= dialog.width() as f32);
            }
            if let Some(output) = std::env::var_os("CADISWAVE_GALLERY_DIR") {
                let path = std::path::PathBuf::from(output)
                    .join(format!("software-settings-{width}-{suffix}.png"));
                cadiswave_runtime::process::CommandRunner::default()
                    .run(
                        "import",
                        &[
                            "-window".into(),
                            "root".into(),
                            path.to_string_lossy().into_owned(),
                        ],
                        std::time::Duration::from_secs(3),
                    )
                    .unwrap();
            }
        }
        dialog.close();
        window.close();
    }

    fn microphone_snapshot() -> (AppSnapshot, SourceId) {
        let mut snapshot = AppSnapshot::default();
        let unit = crate::ui::test_support::unit("A", 2, -10.0);
        snapshot.selected_unit = Some(unit.id);
        snapshot.units = Arc::new(vec![unit]);
        let mut source = Source::new("Microphone".into(), SourceKind::Device);
        source.node_name = "alsa_input.fixture".into();
        source.fx = Some(FxSettings {
            comp: true,
            eq_mid: 3.0,
            ..Default::default()
        });
        let id = source.id.clone();
        Arc::make_mut(&mut snapshot.desired)
            .sources
            .insert(id.clone(), source);
        snapshot.captures = Arc::new(vec![CaptureSnapshot {
            identity: NodeIdentity {
                server_cookie: 1,
                object_serial: "10".into(),
            },
            node_id: 10,
            node_name: "alsa_input.fixture".into(),
            name: "Microphone".into(),
            muted: Observation::Known(false),
            channels: Some(1),
            properties: [("device.serial".into(), "A".into())].into_iter().collect(),
        }]);
        (snapshot, id)
    }

    #[test]
    fn source_settings_reject_a_stale_capture_and_preserve_pending_effects() {
        let (mut snapshot, id) = microphone_snapshot();
        assert_eq!(source_settings(&snapshot, &id).unwrap().eq_mid, 3.0);
        Arc::make_mut(&mut snapshot.pending_fx).insert(
            id.clone(),
            FxSettings {
                gate: true,
                eq_high: 4.0,
                ..Default::default()
            },
        );
        let settings = source_settings(&snapshot, &id).unwrap();
        assert!(settings.gate);
        assert_eq!(settings.eq_high, 4.0);
        snapshot.captures = Arc::default();
        assert!(source_settings(&snapshot, &id).is_err());
    }

    #[test]
    fn completion_requires_the_current_command_and_preserves_backend_errors() {
        let mut state = ActionState::Pending(CommandId(8));
        let error = CommandOutcome::Rejected(OperationError::unavailable("fixture FX failure"));
        assert!(!state.complete(CommandId(7), &error));
        assert_eq!(state, ActionState::Pending(CommandId(8)));
        assert!(state.complete(CommandId(8), &error));
        assert_eq!(state, ActionState::Failed("fixture FX failure".into()));
    }

    #[test]
    fn multiple_sources_require_an_explicit_choice() {
        let mut choice = SourceChoice::default();
        choice.refresh(&[SourceId::generate(), SourceId::generate()]);
        assert_eq!(choice.selected, None);
    }

    #[test]
    fn refresh_preserves_the_exact_choice_until_it_disappears() {
        let a = SourceId::generate();
        let b = SourceId::generate();
        let mut choice = SourceChoice {
            selected: Some(b.clone()),
        };
        choice.refresh(&[a.clone(), b.clone()]);
        assert_eq!(choice.selected, Some(b));
        choice.refresh(&[]);
        assert_eq!(choice.selected, None);
        choice.refresh(std::slice::from_ref(&a));
        assert_eq!(choice.selected, Some(a));
    }
}
