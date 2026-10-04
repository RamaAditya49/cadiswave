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
        if let Ok(row) = widget.downcast::<adw::ActionRow>() {
            if !row.is_sensitive() {
                row.emit_by_name::<()>("activated", &[]);
            }
        }
    }
    assert_eq!(rig.device_command_count(), 0);
    assert_eq!(rig.routing_command_count(), routes_before);
}
