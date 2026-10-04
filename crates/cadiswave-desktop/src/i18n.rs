//! Application-owned Fluent bundles and explicit native text bindings.
mod messages;
use cadiswave_core::{
    locale::LanguageChoice,
    model::{OperationError, Result},
};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use gtk::prelude::*;
use std::{cell::RefCell, rc::Rc};

pub const EN: &str = include_str!("../../../data/locales/en.ftl");
pub const ID: &str = include_str!("../../../data/locales/id.ftl");
pub struct I18n {
    locale: &'static str,
    bundle: FluentBundle<FluentResource>,
    fallback: FluentBundle<FluentResource>,
}
fn bundle(locale: &str, text: &str) -> Result<FluentBundle<FluentResource>> {
    let resource = FluentResource::try_new(text.to_owned()).map_err(|(_, errors)| {
        OperationError::invalid(format!("Invalid Fluent catalog: {errors:?}"))
    })?;
    let language = locale
        .parse::<unic_langid::LanguageIdentifier>()
        .map_err(|error| OperationError::invalid(error.to_string()))?;
    let mut bundle = FluentBundle::new(vec![language]);
    bundle.set_use_isolating(false);
    bundle.add_resource(resource).map_err(|errors| {
        OperationError::invalid(format!("Invalid Fluent messages: {errors:?}"))
    })?;
    Ok(bundle)
}
impl I18n {
    pub fn new(choice: LanguageChoice, system_locale: &str) -> Result<Self> {
        let locale = choice.resolve(system_locale);
        Ok(Self {
            locale,
            bundle: bundle(locale, if locale == "id" { ID } else { EN })?,
            fallback: bundle("en", EN)?,
        })
    }
    pub fn locale(&self) -> &'static str {
        self.locale
    }
    pub fn set_choice(&mut self, choice: LanguageChoice, system_locale: &str) -> Result<()> {
        *self = Self::new(choice, system_locale)?;
        Ok(())
    }
    pub fn text(&self, key: &str, args: Option<&FluentArgs<'_>>) -> String {
        for bundle in [&self.bundle, &self.fallback] {
            if let Some(pattern) = bundle.get_message(key).and_then(|message| message.value()) {
                let mut errors = Vec::new();
                let text = bundle.format_pattern(pattern, args, &mut errors);
                if errors.is_empty() {
                    return text.into_owned();
                }
            }
        }
        key.into()
    }
}
pub fn system_locale() -> String {
    ["LANGUAGE", "LC_MESSAGES", "LANG"]
        .into_iter()
        .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
        .unwrap_or_else(|| "en".into())
}
struct Binding {
    object: glib::WeakRef<glib::Object>,
    property: String,
    key: String,
    expected: String,
}
thread_local! {
    static ACTIVE: RefCell<Option<Rc<RefCell<I18n>>>>=const{RefCell::new(None)};
    static BINDINGS: RefCell<Vec<Binding>>=const{RefCell::new(Vec::new())};
}
pub fn activate(locale: Rc<RefCell<I18n>>) {
    ACTIVE.with(|active| *active.borrow_mut() = Some(locale));
}
pub fn tr(key: &str) -> String {
    ACTIVE.with(|active| {
        active
            .borrow()
            .as_ref()
            .map(|locale| locale.borrow().text(key, None))
            .unwrap_or_else(|| {
                I18n::new(LanguageChoice::English, "en")
                    .map(|locale| locale.text(key, None))
                    .unwrap_or_else(|_| key.into())
            })
    })
}
pub fn translate(english: &str) -> String {
    messages::MESSAGES
        .iter()
        .find(|(_, text)| *text == english)
        .map_or_else(|| english.to_owned(), |(key, _)| tr(key))
}
pub fn format(key: &str, values: &[(&str, &str)]) -> String {
    let mut args = FluentArgs::new();
    for (name, value) in values {
        args.set(*name, *value);
    }
    ACTIVE.with(|active| {
        active
            .borrow()
            .as_ref()
            .map(|locale| locale.borrow().text(key, Some(&args)))
            .unwrap_or_else(|| {
                I18n::new(LanguageChoice::English, "en")
                    .map(|locale| locale.text(key, Some(&args)))
                    .unwrap_or_else(|_| key.into())
            })
    })
}
/// Exclude user text, hardware identifiers, and exact errors from inferred bindings.
pub fn protect(widget: &impl IsA<gtk::Widget>) {
    widget.add_css_class("cadiswave-user-content");
}
pub fn bind_english(object: &impl IsA<glib::Object>, property: &str, english: &str) {
    if let Some((key, _)) = messages::MESSAGES.iter().find(|(_, text)| *text == english) {
        bind(object, property, key);
    }
}
pub fn bind(object: &impl IsA<glib::Object>, property: &str, key: &str) {
    let object = object.as_ref();
    let text = tr(key);
    object.set_property(property, &text);
    BINDINGS.with(|bindings| {
        let mut bindings = bindings.borrow_mut();
        bindings.retain(|binding| binding.object.upgrade().is_some());
        if let Some(binding) = bindings.iter_mut().find(|binding| {
            binding.object.upgrade().as_ref() == Some(object) && binding.property == property
        }) {
            binding.key = key.into();
            binding.expected = text;
        } else {
            bindings.push(Binding {
                object: object.downgrade(),
                property: property.into(),
                key: key.into(),
                expected: text,
            });
        }
    });
}
pub fn bind_tree(widget: &impl IsA<gtk::Widget>) {
    let widget = widget.as_ref();
    if widget.has_css_class("cadiswave-user-content") {
        return;
    }
    for property in [
        "label",
        "title",
        "subtitle",
        "heading",
        "body",
        "description",
        "tooltip-text",
        "placeholder-text",
    ] {
        if widget.find_property(property).is_none() {
            continue;
        }
        let Ok(text) = widget.property_value(property).get::<String>() else {
            continue;
        };
        if let Some((key, _)) = messages::MESSAGES
            .iter()
            .find(|(_, english)| *english == text)
        {
            bind(widget, property, key);
        }
    }
    let mut child = widget.first_child();
    while let Some(next) = child {
        child = next.next_sibling();
        bind_tree(&next);
    }
}
pub fn retranslate() {
    BINDINGS.with(|bindings| {
        bindings.borrow_mut().retain_mut(|binding| {
            let Some(object) = binding.object.upgrade() else {
                return false;
            };
            if object
                .property_value(&binding.property)
                .get::<String>()
                .ok()
                .as_deref()
                != Some(binding.expected.as_str())
            {
                return false;
            }
            let text = tr(&binding.key);
            object.set_property(&binding.property, &text);
            binding.expected = text;
            true
        })
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    fn messages(text: &str) -> BTreeMap<&str, Vec<String>> {
        let resource = fluent_syntax::parser::parse(text).unwrap();
        resource
            .body
            .into_iter()
            .filter_map(|entry| {
                if let fluent_syntax::ast::Entry::Message(message) = entry {
                    let line = text
                        .lines()
                        .find(|line| line.starts_with(&format!("{} =", message.id.name)))
                        .unwrap();
                    let mut args: Vec<_> = line
                        .split('$')
                        .skip(1)
                        .map(|part| {
                            part.chars()
                                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                                .collect()
                        })
                        .collect();
                    args.sort();
                    Some((message.id.name, args))
                } else {
                    None
                }
            })
            .collect()
    }
    #[test]
    fn language_persistence_does_not_dispatch_audio_or_usb_commands() {
        use crate::ui::test_support::{Rig, unit};
        use cadiswave_core::model::*;
        use cadiswave_runtime::controller::AppCommand;
        let rig = Rig::new(serde_json::json!({}), vec![unit("A", 2, -10.0)]);
        let before = rig.routing_command_count();
        let id = rig
            .handle()
            .submit(AppCommand::SetPreferences {
                changes: PreferencesEdit {
                    language: Some(LanguageChoice::Indonesian),
                    ..Default::default()
                },
            })
            .unwrap();
        rig.track(id);
        rig.finish_submissions();
        assert_eq!(rig.routing_command_count(), before);
        assert_eq!(rig.device_command_count(), 0);
        assert_eq!(
            rig.snapshot().preferences.language,
            LanguageChoice::Indonesian
        );
        let value: serde_json::Value = serde_json::from_slice(
            &std::fs::read(rig.paths().identity.join("ui-state.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(value["language"], "id");
    }
    #[test]
    #[ignore = "requires the isolated installed GTK test runner"]
    fn native_bindings_preserve_user_labels_during_language_changes() {
        adw::init().unwrap();
        let locale = Rc::new(RefCell::new(
            I18n::new(LanguageChoice::English, "en").unwrap(),
        ));
        activate(locale.clone());
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let authored = gtk::Label::new(Some("Settings"));
        let user = gtk::Label::new(Some("Music"));
        protect(&user);
        root.append(&authored);
        root.append(&user);
        bind_tree(&root);
        locale
            .borrow_mut()
            .set_choice(LanguageChoice::Indonesian, "en")
            .unwrap();
        retranslate();
        assert_eq!(authored.text(), "Pengaturan");
        assert_eq!(user.text(), "Music");
        locale
            .borrow_mut()
            .set_choice(LanguageChoice::English, "en")
            .unwrap();
        retranslate();
        assert_eq!(authored.text(), "Settings");
    }
    #[test]
    fn catalogs_have_matching_keys_and_named_arguments() {
        assert_eq!(messages(EN), messages(ID));
    }
    #[test]
    fn language_change_keeps_bundle_owner_and_translates_values() {
        let mut locale = I18n::new(LanguageChoice::English, "id_ID").unwrap();
        assert_eq!(locale.text("device-connected", None), "Connected");
        locale
            .set_choice(LanguageChoice::Indonesian, "en_US")
            .unwrap();
        assert_eq!(locale.text("device-connected", None), "Terhubung");
        let mut args = FluentArgs::new();
        args.set("value", "20.0");
        assert_eq!(locale.text("gain-value", Some(&args)), "20.0 dB");
        assert_eq!(
            I18n::new(LanguageChoice::System, "fr_FR").unwrap().locale(),
            "en"
        );
    }
}
