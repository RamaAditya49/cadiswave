use super::*;
#[test]
fn disabled_animations_show_confirmed_position() {
    assert_eq!(
        knob::interpolate(0.0, 0.5, std::time::Duration::from_millis(16), false),
        0.5
    );
}
#[test]
#[ignore = "Requires the isolated GTK runner"]
fn rendering_device_page_does_not_submit_commands() {
    gtk::init().unwrap();
    let rig = crate::ui::test_support::Rig::new(
        serde_json::json!({}),
        vec![crate::ui::test_support::unit("A", 2, -10.0)],
    );
    let locale = std::rc::Rc::new(std::cell::RefCell::new(
        crate::i18n::I18n::new(cadiswave_core::locale::LanguageChoice::English, "en").unwrap(),
    ));
    let routes_before = rig.routing_command_count();
    let page = DevicePage::new(controls::DeviceControls::new(rig.handle()), locale);
    for _ in 0..20 {
        page.render(&rig.snapshot());
    }
    assert_eq!(rig.device_command_count(), 0);
    assert_eq!(rig.routing_command_count(), routes_before);
}
#[test]
#[ignore = "Requires the isolated GTK runner"]
fn native_controls_use_the_shared_runtime() {
    gtk::init().unwrap();
    let rig = crate::ui::test_support::Rig::new(
        serde_json::json!({}),
        vec![crate::ui::test_support::unit("A", 2, -10.0)],
    );
    let locale = std::rc::Rc::new(std::cell::RefCell::new(
        crate::i18n::I18n::new(cadiswave_core::locale::LanguageChoice::English, "en").unwrap(),
    ));
    let page = DevicePage::new(controls::DeviceControls::new(rig.handle()), locale);
    page.render(&rig.snapshot());
    use adw::prelude::*;
    page.mute.emit_clicked();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while rig.device_command_count() < 1 && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(rig.device_command_count(), 1);
    let routes_after_mute = rig.routing_command_count();
    page.knob.scale.set_value(0.5);
    while rig.device_command_count() < 2 && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(rig.device_command_count(), 2);
    assert_eq!(rig.routing_command_count(), routes_after_mute);
}
#[test]
fn save_feedback_requires_matching_persistence_completion() {
    use cadiswave_core::model::*;
    use settings::PersistenceState;
    let mut state = PersistenceState::Pending(CommandId(3));
    assert!(!state.complete(CommandId(4), &CommandOutcome::Applied { revision: 1 }));
    assert_eq!(state, PersistenceState::Pending(CommandId(3)));
    assert!(state.complete(
        CommandId(3),
        &CommandOutcome::Rejected(OperationError::unavailable("disk full"))
    ));
    assert!(matches!(state,PersistenceState::Failed(ref message) if message.contains("disk full")));
}
#[test]
#[ignore = "Requires the isolated GTK runner"]
fn unavailable_settings_do_not_submit_hardware_commands() {
    use adw::prelude::*;
    adw::init().unwrap();
    let rig = crate::ui::test_support::Rig::new(
        serde_json::json!({}),
        vec![crate::ui::test_support::unit("A", 2, -10.0)],
    );
    let locale = std::rc::Rc::new(std::cell::RefCell::new(
        crate::i18n::I18n::new(cadiswave_core::locale::LanguageChoice::English, "en").unwrap(),
    ));
    let routes_before = rig.routing_command_count();
    let settings =
        settings::DeviceSettings::new(controls::DeviceControls::new(rig.handle()), locale);
    for widget in crate::ui::test_support::descendants::<gtk::Widget>(&settings.dialog) {
        if let Ok(row) = widget.downcast::<adw::ActionRow>()
            && !row.is_sensitive()
        {
            row.emit_by_name::<()>("activated", &[]);
        }
    }
    assert_eq!(rig.device_command_count(), 0);
    assert_eq!(rig.routing_command_count(), routes_before);
}

#[test]
#[ignore = "Requires the isolated GTK runner"]
fn verified_clipguard_is_available_in_device_settings() {
    use adw::prelude::*;
    adw::init().unwrap();
    let mut wave = crate::ui::test_support::unit("A", 2, -10.0);
    wave.id.profile = cadiswave_core::profiles::ProfileId::Wave3;
    wave.info.api = "5.3".into();
    wave.state = cadiswave_core::model::Observation::Known(
        cadiswave_core::protocol::ConfigBuffer::decode(wave.id.profile, &[0; 16])
            .unwrap()
            .state(),
    );
    let rig = crate::ui::test_support::Rig::new(serde_json::json!({}), vec![wave]);
    let locale = std::rc::Rc::new(std::cell::RefCell::new(
        crate::i18n::I18n::new(cadiswave_core::locale::LanguageChoice::English, "en").unwrap(),
    ));
    crate::i18n::activate(locale.clone());
    let settings =
        settings::DeviceSettings::new(controls::DeviceControls::new(rig.handle()), locale);
    let window = adw::Window::builder()
        .default_width(640)
        .default_height(900)
        .build();
    window.present();
    settings.dialog.present(Some(&window));
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(400);
    while std::time::Instant::now() < deadline {
        while glib::MainContext::default().pending() {
            glib::MainContext::default().iteration(false);
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let clipguard = crate::ui::test_support::descendants::<adw::ActionRow>(&settings.dialog)
        .into_iter()
        .find(|row| row.title() == "Clipguard")
        .expect("Clipguard setting");
    assert!(
        clipguard.is_sensitive(),
        "The exact Wave:3 profile maps Clipguard"
    );
    assert_eq!(
        rig.device_command_count(),
        0,
        "Rendering cannot write hardware"
    );
    settings.dialog.close();
    window.close();
}
#[test]
fn repeated_connections_do_not_repeat_notifications() {
    let rig = crate::ui::test_support::Rig::new(
        serde_json::json!({}),
        vec![crate::ui::test_support::unit("A", 2, -10.0)],
    );
    let p = projection::DeviceProjection::from_snapshot(&rig.snapshot());
    let mut notifier = compact::ConnectionNotifier::default();
    assert!(notifier.observe(&p).is_some());
    assert!(notifier.observe(&p).is_none());
    let missing =
        projection::DeviceProjection::from_snapshot(&cadiswave_core::model::AppSnapshot::default());
    assert!(notifier.observe(&missing).is_some());
    assert!(notifier.observe(&missing).is_none());
}
#[test]
#[ignore = "Requires the isolated GTK runner"]
fn opening_compact_controls_and_switching_language_keeps_runtime_state() {
    use adw::prelude::*;
    adw::init().unwrap();
    let rig = crate::ui::test_support::Rig::new(
        serde_json::json!({}),
        vec![crate::ui::test_support::unit("A", 2, -10.0)],
    );
    let locale = std::rc::Rc::new(std::cell::RefCell::new(
        crate::i18n::I18n::new(cadiswave_core::locale::LanguageChoice::English, "en").unwrap(),
    ));
    let before = rig.device_states();
    let routes = rig.routing_command_count();
    let compact =
        compact::CompactControls::new(controls::DeviceControls::new(rig.handle()), locale.clone());
    compact.render(&rig.snapshot());
    let window = adw::Window::new();
    window.set_content(Some(&compact.widget));
    window.present();
    locale
        .borrow_mut()
        .set_choice(cadiswave_core::locale::LanguageChoice::Indonesian, "en")
        .unwrap();
    compact.retranslate();
    compact.render(&rig.snapshot());
    assert_eq!(before, rig.device_states());
    assert_eq!(routes, rig.routing_command_count());
    assert_eq!(rig.device_command_count(), 0);
    window.close();
}
#[test]
#[ignore = "Requires the isolated GTK runner"]
fn text_editors_exclude_hardware_shortcuts() {
    use adw::prelude::*;
    gtk::init().unwrap();
    let text = gtk::Text::new();
    let entry = gtk::Entry::new();
    let view = gtk::TextView::new();
    for widget in [text.upcast::<gtk::Widget>(), entry.upcast(), view.upcast()] {
        assert!(!crate::app::shortcuts_allowed(Some(&widget)));
    }
    assert!(crate::app::shortcuts_allowed(Some(
        gtk::Button::new().upcast_ref()
    )));
}
#[test]
fn language_change_preserves_an_admitted_device_edit() {
    use cadiswave_core::model::*;
    use cadiswave_runtime::controller::{AppCommand, EditTiming};
    let rig = crate::ui::test_support::Rig::new(
        serde_json::json!({}),
        vec![crate::ui::test_support::unit("A", 2, -10.0)],
    );
    let controls = controls::DeviceControls::new(rig.handle());
    let before = rig.snapshot();
    let routes = rig.routing_command_count();
    let p = projection::DeviceProjection::from_snapshot(&before);
    let edit = controls.edit(&p, 0.5, EditTiming::Debounced).unwrap();
    rig.track(edit);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while rig.snapshot().unit_intents.is_empty() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let language = controls
        .submit(AppCommand::SetPreferences {
            changes: PreferencesEdit {
                language: Some(cadiswave_core::locale::LanguageChoice::Indonesian),
                ..Default::default()
            },
        })
        .unwrap();
    rig.track(language);
    assert!(matches!(
        rig.outcome(language),
        CommandOutcome::Applied { .. }
    ));
    assert!(matches!(rig.outcome(edit), CommandOutcome::Applied { .. }));
    assert_eq!(rig.snapshot().selected_unit, before.selected_unit);
    assert_eq!(
        rig.snapshot().preferences.language,
        cadiswave_core::locale::LanguageChoice::Indonesian
    );
    assert_eq!(rig.routing_command_count(), routes);
    assert_eq!(rig.device_command_count(), 1);
    assert_eq!(
        rig.device_states()[0].state.known().unwrap().gain_raw,
        75 * 256 / 2
    );
}

#[test]
#[ignore = "Requires the isolated GTK runner"]
fn native_device_controls_have_translated_accessible_labels() {
    use crate::ui::test_support::{Rig, descendants, unit};
    use adw::prelude::*;
    gtk::init().unwrap();
    let rig = Rig::new(serde_json::json!({}), vec![unit("A", 2, -12.0)]);
    let locale = std::rc::Rc::new(std::cell::RefCell::new(
        crate::i18n::I18n::new(cadiswave_core::locale::LanguageChoice::English, "en").unwrap(),
    ));
    crate::i18n::activate(locale.clone());
    let page = DevicePage::new(controls::DeviceControls::new(rig.handle()), locale.clone());
    let compact =
        compact::CompactControls::new(controls::DeviceControls::new(rig.handle()), locale.clone());
    for language in [
        cadiswave_core::locale::LanguageChoice::English,
        cadiswave_core::locale::LanguageChoice::Indonesian,
    ] {
        locale.borrow_mut().set_choice(language, "en").unwrap();
        crate::i18n::retranslate();
        for root in [
            page.widget.upcast_ref::<gtk::Widget>(),
            compact.widget.upcast_ref::<gtk::Widget>(),
        ] {
            let scales = descendants::<gtk::Scale>(root);
            assert_eq!(scales.len(), 2);
            for scale in scales {
                assert!(gtk::test_accessible_has_relation(
                    &scale,
                    gtk::AccessibleRelation::LabelledBy
                ));
            }
            let switches = descendants::<gtk::Switch>(root);
            assert_eq!(switches.len(), 1);
            assert!(gtk::test_accessible_has_relation(
                &switches[0],
                gtk::AccessibleRelation::LabelledBy
            ));
            let labels = descendants::<gtk::Label>(root);
            for key in ["dial-control", "mode-headphones", "low-impedance"] {
                assert!(
                    labels
                        .iter()
                        .any(|label| label.text() == crate::i18n::tr(key))
                );
            }
        }
    }
}
