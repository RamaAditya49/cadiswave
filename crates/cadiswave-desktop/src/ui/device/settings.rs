use super::controls::DeviceControls;
use adw::prelude::*;
use cadiswave_core::{locale::LanguageChoice, model::*};
use cadiswave_runtime::controller::AppCommand;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Clone, Debug, PartialEq)]
pub enum PersistenceState {
    Idle,
    Pending(CommandId),
    Saved,
    Failed(String),
}
impl PersistenceState {
    pub fn complete(&mut self, id: CommandId, outcome: &CommandOutcome) -> bool {
        if *self != Self::Pending(id) {
            return false;
        }
        *self = match outcome {
            CommandOutcome::Applied { .. } => Self::Saved,
            CommandOutcome::Rejected(error) => Self::Failed(error.to_string()),
            other => Self::Failed(format!("{other:?}")),
        };
        true
    }
}
struct SaveModel {
    state: RefCell<PersistenceState>,
    draft: Cell<LanguageChoice>,
    dirty: Cell<bool>,
    updating: Cell<bool>,
    controls: DeviceControls,
}
pub struct DeviceSettings {
    pub dialog: adw::PreferencesDialog,
    language: adw::ComboRow,
    pub save: gtk::Button,
    revert: gtk::Button,
    feedback: gtk::Label,
    details: gtk::Label,
    model: Rc<SaveModel>,
}
fn unavailable(key: &str) -> adw::ActionRow {
    let row = adw::ActionRow::new();
    crate::i18n::bind(&row, "title", key);
    crate::i18n::bind(&row, "subtitle", "hardware-control-unavailable");
    let icon = gtk::Image::from_icon_name("changes-prevent-symbolic");
    icon.set_opacity(0.4);
    row.add_suffix(&icon);
    row.set_sensitive(false);
    row
}
fn group(page: &adw::PreferencesPage, key: &str) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    crate::i18n::bind(&group, "title", key);
    page.add(&group);
    group
}
impl DeviceSettings {
    pub fn new(controls: DeviceControls, _locale: Rc<RefCell<crate::i18n::I18n>>) -> Self {
        let dialog = adw::PreferencesDialog::builder()
            .content_width(480)
            .content_height(760)
            .build();
        crate::i18n::bind(&dialog, "title", "hardware-settings");
        let page = adw::PreferencesPage::new();
        dialog.add(&page);
        let hardware = group(&page, "hardware-processing");
        hardware.add(&unavailable("clipguard"));
        hardware.add(&unavailable("low-cut-hardware"));
        let software = adw::ActionRow::new();
        crate::i18n::bind(&software, "title", "low-cut-software");
        crate::i18n::bind(&software, "subtitle", "software-fx-hint");
        software.set_subtitle_lines(3);
        software.set_activatable(true);
        software.set_action_name(Some("win.mixer"));
        hardware.add(&software);
        let led = group(&page, "led-settings");
        led.add(&unavailable("led-color"));
        led.add(&unavailable("led-brightness"));
        let audio = group(&page, "sample-rate");
        let rate = adw::ActionRow::new();
        crate::i18n::bind(&rate, "title", "sample-rate");
        crate::i18n::bind(&rate, "subtitle", "sample-rate-readonly");
        rate.set_subtitle_lines(3);
        rate.set_sensitive(false);
        audio.add(&rate);
        let monitor = unavailable("monitor-mix");
        audio.add(&monitor);
        audio.add(&unavailable("save-to-device"));
        let app = group(&page, "settings");
        let language = adw::ComboRow::new();
        crate::i18n::bind(&language, "title", "language");
        language.set_model(Some(&gtk::StringList::new(&[
            &crate::i18n::tr("language-system"),
            "English",
            "Bahasa Indonesia",
        ])));
        app.add(&language);
        let model = Rc::new(SaveModel {
            state: RefCell::new(PersistenceState::Idle),
            draft: Cell::new(controls.snapshot().preferences.language),
            dirty: Cell::new(false),
            updating: Cell::new(false),
            controls,
        });
        let target = model.clone();
        language.connect_selected_notify(move |row| {
            if !target.updating.get() {
                target.draft.set(match row.selected() {
                    1 => LanguageChoice::English,
                    2 => LanguageChoice::Indonesian,
                    _ => LanguageChoice::System,
                });
                target.dirty.set(true);
            }
        });
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        buttons.set_halign(gtk::Align::End);
        let revert = gtk::Button::new();
        crate::i18n::bind(&revert, "label", "revert");
        let save = gtk::Button::new();
        crate::i18n::bind(&save, "label", "save-application");
        save.add_css_class("suggested-action");
        buttons.append(&revert);
        buttons.append(&save);
        app.add(&buttons);
        let feedback = gtk::Label::new(None);
        feedback.set_wrap(true);
        let details = gtk::Label::new(None);
        details.set_wrap(true);
        details.set_selectable(true);
        crate::i18n::protect(&details);
        app.add(&feedback);
        app.add(&details);
        let target = model.clone();
        let feedback_target = feedback.downgrade();
        save.connect_clicked(move |button| {
            if matches!(*target.state.borrow(), PersistenceState::Pending(_)) {
                return;
            }
            *target.state.borrow_mut() = match target.controls.submit(AppCommand::SetPreferences {
                changes: PreferencesEdit {
                    language: Some(target.draft.get()),
                    ..Default::default()
                },
            }) {
                Ok(id) => {
                    button.set_sensitive(false);
                    PersistenceState::Pending(id)
                }
                Err(error) => PersistenceState::Failed(format!("{error:?}")),
            };
            if let Some(feedback) = feedback_target.upgrade() {
                feedback.set_text(&crate::i18n::tr(
                    if matches!(*target.state.borrow(), PersistenceState::Pending(_)) {
                        "save-pending"
                    } else {
                        "save-failed"
                    },
                ));
            }
        });
        let target = model.clone();
        let row = language.downgrade();
        revert.connect_clicked(move |_| {
            if matches!(*target.state.borrow(), PersistenceState::Pending(_)) {
                return;
            }
            target
                .draft
                .set(target.controls.snapshot().preferences.language);
            target.dirty.set(false);
            *target.state.borrow_mut() = PersistenceState::Idle;
            if let Some(row) = row.upgrade() {
                target.updating.set(true);
                row.set_selected(language_index(target.draft.get()));
                target.updating.set(false);
            }
        });
        let result = Self {
            dialog,
            language,
            save,
            revert,
            feedback,
            details,
            model,
        };
        result.render(&result.model.controls.snapshot());
        result
    }
    pub fn completed(&self, id: CommandId, outcome: &CommandOutcome) -> bool {
        let matched = self.model.state.borrow_mut().complete(id, outcome);
        if matched && matches!(*self.model.state.borrow(), PersistenceState::Saved) {
            self.model.dirty.set(false);
        }
        self.render(&self.model.controls.snapshot());
        matched
    }
    pub fn render(&self, snapshot: &AppSnapshot) {
        let pending = matches!(*self.model.state.borrow(), PersistenceState::Pending(_));
        if !self.model.dirty.get() && !pending {
            self.model.draft.set(snapshot.preferences.language);
        }
        self.model.updating.set(true);
        self.language
            .set_selected(language_index(self.model.draft.get()));
        self.model.updating.set(false);
        self.language.set_sensitive(!pending);
        self.save.set_sensitive(!pending);
        self.revert.set_sensitive(!pending);
        self.feedback
            .set_text(&crate::i18n::tr(match &*self.model.state.borrow() {
                PersistenceState::Pending(_) => "save-pending",
                PersistenceState::Saved => "save-success",
                PersistenceState::Failed(_) => "save-failed",
                PersistenceState::Idle if self.model.dirty.get() => "unsaved-settings",
                _ => "save-application",
            }));
        self.details.set_text(&match &*self.model.state.borrow() {
            PersistenceState::Failed(message) => message.clone(),
            _ => String::new(),
        });
    }
    pub fn retranslate(&self) {
        crate::i18n::retranslate();
        self.model.updating.set(true);
        self.language.set_model(Some(&gtk::StringList::new(&[
            &crate::i18n::tr("language-system"),
            "English",
            "Bahasa Indonesia",
        ])));
        self.language
            .set_selected(language_index(self.model.draft.get()));
        self.model.updating.set(false);
    }
}
fn language_index(choice: LanguageChoice) -> u32 {
    match choice {
        LanguageChoice::System => 0,
        LanguageChoice::English => 1,
        LanguageChoice::Indonesian => 2,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn failed_save_retains_the_draft_and_reports_the_store_error() {
        adw::init().unwrap();
        let rig = crate::ui::test_support::Rig::new(serde_json::json!({}), vec![]);
        let locale = Rc::new(RefCell::new(
            crate::i18n::I18n::new(LanguageChoice::English, "en").unwrap(),
        ));
        let settings = DeviceSettings::new(DeviceControls::new(rig.handle()), locale);
        settings.language.set_selected(2);
        use std::os::unix::fs::PermissionsExt;
        let root = rig.paths().identity;
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o500)).unwrap();
        settings.save.emit_clicked();
        let id = match *settings.model.state.borrow() {
            PersistenceState::Pending(id) => id,
            _ => panic!("save was not submitted"),
        };
        let outcome = rig.outcome(id);
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(settings.completed(id, &outcome));
        assert!(matches!(
            *settings.model.state.borrow(),
            PersistenceState::Failed(_)
        ));
        assert_eq!(settings.model.draft.get(), LanguageChoice::Indonesian);
        assert!(settings.model.dirty.get());
        assert!(!settings.details.text().is_empty());
    }
}
