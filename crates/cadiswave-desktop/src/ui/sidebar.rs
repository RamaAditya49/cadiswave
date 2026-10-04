use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

use adw::prelude::*;
use cadiswave_core::{
    model::{AppSnapshot, DeviceSetting, Lifecycle, PreferencesEdit, UnitId, UnitSnapshot},
    protocol::KnobMode,
};
use cadiswave_runtime::controller::{AppCommand, EditTiming};

use crate::{Submit, icons::Icons};

use super::components::{device_slider::DeviceSlider, value_row};

/// The sidebar renders controller state; it never owns device or preference state.
pub struct Sidebar {
    pub widget: gtk::ScrolledWindow,
    interaction: Rc<Interaction>,
    icons: Rc<Icons>,
    selector_group: adw::PreferencesGroup,
    selector: adw::ComboRow,
    selector_entries: RefCell<Vec<(UnitId, String)>>,
    microphone: adw::PreferencesGroup,
    headphones: adw::PreferencesGroup,
    mute: adw::SwitchRow,
    gain: DeviceSlider,
    gain_lock: gtk::ToggleButton,
    phantom: adw::SwitchRow,
    knob: gtk::Label,
    knob_row: adw::ActionRow,
    headphone: DeviceSlider,
    low_z: adw::SwitchRow,
    monitor: DeviceSlider,
    autostart: adw::SwitchRow,
    hidden_autostart: adw::SwitchRow,
    tray_color: adw::ComboRow,
    info: adw::ExpanderRow,
    firmware: gtk::Label,
    api: gtk::Label,
    serial: gtk::Label,
}

struct Interaction {
    updating: Cell<bool>,
    snapshot: RefCell<Arc<AppSnapshot>>,
    pending_selection: Cell<Option<UnitId>>,
}

impl Interaction {
    fn accepting(&self) -> bool {
        !self.updating.get()
            && matches!(
                self.snapshot.borrow().lifecycle,
                Lifecycle::Starting | Lifecycle::Running
            )
    }

    /// Copy the incarnation-scoped identity at the signal, never at a later timeout.
    fn selected(&self) -> Option<UnitId> {
        if !self.accepting() {
            return None;
        }
        if self.pending_selection.get().is_some() {
            return None;
        }
        let snapshot = self.snapshot.borrow();
        let id = snapshot.selected_unit?;
        snapshot
            .units
            .iter()
            .any(|unit| unit.id == id)
            .then_some(id)
    }
}

#[derive(Clone, Copy)]
enum Slider {
    Gain,
    Headphone,
    Monitor,
}

impl Sidebar {
    pub fn new(icons: Rc<Icons>, submit: Submit) -> Self {
        let interaction = Rc::new(Interaction {
            updating: Cell::new(false),
            snapshot: RefCell::new(Arc::new(AppSnapshot::default())),
            pending_selection: Cell::new(None),
        });
        let widget = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .build();
        let clamp = adw::Clamp::builder()
            .maximum_size(380)
            .margin_start(12)
            .margin_end(12)
            .margin_top(12)
            .margin_bottom(12)
            .build();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
        clamp.set_child(Some(&content));
        widget.set_child(Some(&clamp));

        let selector_group = adw::PreferencesGroup::new();
        selector_group.set_visible(false);
        let selector = adw::ComboRow::builder().title("Device").build();
        selector_group.add(&selector);
        content.append(&selector_group);

        let microphone = adw::PreferencesGroup::builder().title("Microphone").build();
        content.append(&microphone);
        let mute = adw::SwitchRow::builder()
            .title("Mute")
            .subtitle("Toggle microphone mute")
            .build();
        microphone.add(&mute);
        let gain = DeviceSlider::new(
            "Gain",
            8,
            "Microphone gain",
            &gtk::Adjustment::new(0.0, 0.0, 0x5000 as f64, 0x40 as f64, 0x200 as f64, 0.0),
            |unit, value| format_gain(value as u16, unit.profile.profile().gain_scale),
        );
        let gain_lock = gtk::ToggleButton::builder()
            .valign(gtk::Align::Center)
            .tooltip_text("Lock gain")
            .build();
        gain_lock.set_child(Some(&icons.image("changes-allow-symbolic", 16)));
        gain_lock.add_css_class("flat");
        gain.value_row.add_suffix(&gain_lock);
        gain.add_to(&microphone);
        let phantom = adw::SwitchRow::builder()
            .title("48V Phantom Power")
            .subtitle("For condenser microphones. Leave off for dynamic mics.")
            .build();
        microphone.add(&phantom);
        let (knob_row, knob) = value_row("Knob Controls", 0);
        knob_row.set_subtitle("What the physical knob adjusts");
        knob.remove_css_class("monospace");
        knob.add_css_class("dim-label");
        microphone.add(&knob_row);

        let headphones = adw::PreferencesGroup::builder().title("Headphones").build();
        content.append(&headphones);
        let headphone = DeviceSlider::new(
            "Volume",
            10,
            "Headphone volume",
            &gtk::Adjustment::new(-60.0, -60.0, 0.0, 0.5, 2.0, 0.0),
            |unit, value| {
                if unit.profile.profile().legacy.is_none() {
                    format!("{value:.2} dB")
                } else {
                    format!("{value:.1} dB")
                }
            },
        );
        headphone.add_to(&headphones);
        let low_z = adw::SwitchRow::builder()
            .title("Low Impedance")
            .subtitle("For low impedance headphones")
            .build();
        headphones.add(&low_z);
        let monitor = DeviceSlider::new(
            "Monitor Mix",
            8,
            "Microphone / PC monitor mix",
            &gtk::Adjustment::new(0.0, 0.0, 0x6400 as f64, 0x100 as f64, 0x800 as f64, 0.0),
            |unit, value| {
                format!(
                    "{:.0}%",
                    value * 100.0 / f64::from(unit.profile.profile().mix_max)
                )
            },
        );
        monitor
            .value_row
            .set_subtitle("Mic / PC monitoring balance");
        monitor.add_to(&headphones);
        {
            let interaction = interaction.clone();
            let submit = submit.clone();
            let microphone = microphone.downgrade();
            let headphones = headphones.downgrade();
            selector.connect_selected_notify(move |row| {
                if !interaction.accepting() {
                    return;
                }
                let unit = interaction
                    .snapshot
                    .borrow()
                    .units
                    .get(row.selected() as usize)
                    .map(|unit| unit.id);
                if let Some(unit) = unit {
                    // Do not expose A's rendered controls as gestures for B.
                    interaction.pending_selection.set(Some(unit));
                    if let Some(group) = microphone.upgrade() {
                        group.set_sensitive(false);
                    }
                    if let Some(group) = headphones.upgrade() {
                        group.set_sensitive(false);
                    }
                    submit(AppCommand::SelectUnit { unit: Some(unit) });
                }
            });
        }

        let settings = adw::PreferencesGroup::builder()
            .title("Application settings")
            .build();
        content.append(&settings);
        let autostart = adw::SwitchRow::builder()
            .title("Start at login")
            .subtitle("Keeps mixes routed before you open anything")
            .build();
        settings.add(&autostart);
        let hidden_autostart = adw::SwitchRow::builder()
            .title("Start in the tray")
            .subtitle("No window on login; open it from the tray icon")
            .build();
        settings.add(&hidden_autostart);
        let colors = gtk::StringList::new(&["White", "Black"]);
        let tray_color = adw::ComboRow::builder()
            .title("Tray icon color")
            .subtitle("White for dark panels, black for light panels. Red when muted.")
            .model(&colors)
            .build();
        settings.add(&tray_color);

        let info_group = adw::PreferencesGroup::new();
        content.append(&info_group);
        let info = adw::ExpanderRow::builder().title("Device Info").build();
        info_group.add(&info);
        let firmware = info_row(&info, "Firmware");
        let api = info_row(&info, "API");
        let serial = info_row(&info, "Serial");

        for (row, setting) in [
            (&mute, DeviceSetting::Mute as fn(bool) -> DeviceSetting),
            (&phantom, DeviceSetting::Phantom),
            (&low_z, DeviceSetting::LowImpedance),
        ] {
            let interaction = interaction.clone();
            let submit = submit.clone();
            row.connect_active_notify(move |row| {
                if !row.is_sensitive() {
                    return;
                }
                if let Some(unit) = interaction.selected() {
                    submit(AppCommand::SetDeviceSetting {
                        unit,
                        setting: setting(row.is_active()),
                        timing: EditTiming::Immediate,
                    });
                }
            });
        }
        for (slider, kind) in [
            (&gain, Slider::Gain),
            (&headphone, Slider::Headphone),
            (&monitor, Slider::Monitor),
        ] {
            let interaction = interaction.clone();
            let submit = submit.clone();
            slider.connect_changed(move |value| {
                let unit = interaction.selected()?;
                let profile = unit.profile.profile();
                let (setting, value) = match kind {
                    Slider::Gain => {
                        if interaction.snapshot.borrow().preferences.gain_locked {
                            return None;
                        }
                        let raw = value.clamp(0.0, f64::from(profile.gain_max)) as u16;
                        (DeviceSetting::GainRaw(raw), f64::from(raw))
                    }
                    Slider::Headphone => {
                        let db = value.clamp(profile.hp_min_db(), 0.0);
                        (DeviceSetting::HeadphoneDb(db), db)
                    }
                    Slider::Monitor => {
                        if !profile.has_monitor_mix() {
                            return None;
                        }
                        let raw = value.clamp(0.0, f64::from(profile.mix_max)) as u16;
                        (DeviceSetting::MonitorMix(raw), f64::from(raw))
                    }
                };
                submit(AppCommand::SetDeviceSetting {
                    unit,
                    setting,
                    timing: EditTiming::Debounced,
                });
                Some((unit, value))
            });
        }
        {
            let interaction = interaction.clone();
            let submit = submit.clone();
            gain_lock.connect_toggled(move |button| {
                if interaction.selected().is_some() {
                    submit(AppCommand::SetGainLock {
                        locked: button.is_active(),
                    });
                }
            });
        }
        // These switches describe the actual launcher, not an optimistic local file write.
        // Until the worker publishes success they remain at the observed state; errors
        // therefore cannot leave a checked switch claiming a nonexistent login entry.
        for (row, hidden_control) in [(&autostart, false), (&hidden_autostart, true)] {
            let interaction = interaction.clone();
            let submit = submit.clone();
            row.connect_active_notify(move |row| {
                if !interaction.accepting() || !row.is_sensitive() {
                    return;
                }
                let (enabled, hidden, actual) = {
                    let snapshot = interaction.snapshot.borrow();
                    if hidden_control {
                        (
                            snapshot.autostart,
                            row.is_active(),
                            snapshot.hidden_autostart,
                        )
                    } else {
                        (
                            row.is_active(),
                            snapshot.hidden_autostart,
                            snapshot.autostart,
                        )
                    }
                };
                // Snapping the row back below makes the switch notify once
                // more, after the guard is down, with the observed state. That
                // echo is no request: submitting it re-applied the old value
                // right behind the user's, so "off" deleted the login entry
                // and rewrote it ~100 ms later.
                if row.is_active() == actual {
                    return;
                }
                interaction.updating.set(true);
                row.set_active(actual);
                interaction.updating.set(false);
                submit(AppCommand::SetAutostart { enabled, hidden });
            });
        }
        {
            let interaction = interaction.clone();
            tray_color.connect_selected_notify(move |row| {
                if !interaction.accepting() {
                    return;
                }
                let color = match row.selected() {
                    0 => "white",
                    1 => "black",
                    _ => return,
                };
                submit(AppCommand::SetPreferences {
                    changes: PreferencesEdit {
                        tray_icon_color: Some(color.into()),
                        ..PreferencesEdit::default()
                    },
                });
            });
        }

        let sidebar = Self {
            widget,
            interaction,
            icons,
            selector_group,
            selector,
            selector_entries: RefCell::new(Vec::new()),
            microphone,
            headphones,
            mute,
            gain,
            gain_lock,
            phantom,
            knob,
            knob_row,
            headphone,
            low_z,
            monitor,
            autostart,
            hidden_autostart,
            tray_color,
            info,
            firmware,
            api,
            serial,
        };
        sidebar.render(Arc::new(AppSnapshot::default()));
        sidebar
    }

    pub fn render(&self, snapshot: Arc<AppSnapshot>) {
        self.interaction.updating.set(true);
        let accepting = matches!(snapshot.lifecycle, Lifecycle::Starting | Lifecycle::Running);
        if self.interaction.pending_selection.get().is_some_and(|id| {
            snapshot.selected_unit == Some(id) || !snapshot.units.iter().any(|unit| unit.id == id)
        }) {
            self.interaction.pending_selection.set(None);
        }
        let controls_ready = accepting && self.interaction.pending_selection.get().is_none();
        let selected = snapshot
            .selected_unit
            .and_then(|id| snapshot.units.iter().find(|unit| unit.id == id));
        {
            let mut entries = self.selector_entries.borrow_mut();
            let changed = entries.len() != snapshot.units.len()
                || entries
                    .iter()
                    .zip(snapshot.units.iter())
                    .any(|((id, serial), unit)| *id != unit.id || *serial != unit.info.serial);
            if changed {
                let labels: Vec<_> = snapshot
                    .units
                    .iter()
                    .map(|unit| {
                        let identity = if unit.info.serial.is_empty() {
                            format!("{:03}/{:03}", unit.id.bus, unit.id.address)
                        } else {
                            unit.info.serial.clone()
                        };
                        format!("{} — {identity}", unit.id.profile.profile().display_name)
                    })
                    .collect();
                let labels: Vec<_> = labels.iter().map(String::as_str).collect();
                self.selector
                    .set_model(Some(&gtk::StringList::new(&labels)));
                *entries = snapshot
                    .units
                    .iter()
                    .map(|unit| (unit.id, unit.info.serial.clone()))
                    .collect();
            }
        }
        self.selector.set_selected(
            snapshot
                .units
                .iter()
                .position(|unit| {
                    Some(unit.id)
                        == self
                            .interaction
                            .pending_selection
                            .get()
                            .or(snapshot.selected_unit)
                })
                .map_or(gtk::INVALID_LIST_POSITION, |index| index as u32),
        );
        self.selector_group.set_visible(snapshot.units.len() > 1);
        self.selector.set_sensitive(accepting);
        self.microphone
            .set_sensitive(controls_ready && selected.is_some());
        self.headphones
            .set_sensitive(controls_ready && selected.is_some());
        self.info.set_sensitive(selected.is_some());
        self.phantom
            .set_visible(selected.is_some_and(|unit| unit.id.profile.profile().has_phantom()));
        self.low_z
            .set_visible(selected.is_some_and(|unit| unit.id.profile.profile().has_low_z()));
        let has_monitor = selected.is_some_and(|unit| unit.id.profile.profile().has_monitor_mix());
        self.monitor.set_visible(has_monitor);

        self.gain_lock.set_active(snapshot.preferences.gain_locked);
        if self.interaction.snapshot.borrow().preferences.gain_locked
            != snapshot.preferences.gain_locked
        {
            let name = if snapshot.preferences.gain_locked {
                "changes-prevent-symbolic"
            } else {
                "changes-allow-symbolic"
            };
            self.gain_lock.set_child(Some(&self.icons.image(name, 16)));
        }
        self.gain_lock
            .set_tooltip_text(Some(if snapshot.preferences.gain_locked {
                "Gain locked — click to unlock"
            } else {
                "Lock gain"
            }));
        self.gain_lock
            .set_sensitive(accepting && selected.is_some());
        self.render_unit(selected, &snapshot);

        self.autostart.set_active(snapshot.autostart);
        self.autostart.set_sensitive(accepting);
        self.hidden_autostart.set_active(snapshot.hidden_autostart);
        self.hidden_autostart
            .set_sensitive(accepting && snapshot.autostart);
        self.tray_color.set_sensitive(accepting);
        // Artwork availability must never select or persist a different preference.
        self.tray_color
            .set_selected(match snapshot.preferences.tray_icon_color.as_str() {
                "white" => 0,
                "black" => 1,
                _ => gtk::INVALID_LIST_POSITION,
            });
        *self.interaction.snapshot.borrow_mut() = snapshot;
        self.interaction.updating.set(false);
    }

    fn render_unit(&self, unit: Option<&UnitSnapshot>, snapshot: &AppSnapshot) {
        let state = unit.and_then(|unit| unit.state.known());
        let profile = unit.map(|unit| unit.id.profile.profile());
        let mut gain = state.map(|state| state.gain_raw);
        let mut headphone = state.map(|state| state.hp_volume_db);
        let mut phantom = state.and_then(|state| state.phantom);
        let mut low_z = state.and_then(|state| state.low_impedance);
        let mut monitor = state.and_then(|state| state.monitor_mix);
        if let Some(intents) = unit.and_then(|unit| snapshot.unit_intents.get(&unit.id)) {
            for intent in intents {
                match *intent {
                    DeviceSetting::GainRaw(value) => gain = Some(value),
                    DeviceSetting::HeadphoneDb(value) => headphone = Some(value),
                    DeviceSetting::Phantom(value) => phantom = Some(value),
                    DeviceSetting::LowImpedance(value) => low_z = Some(value),
                    DeviceSetting::MonitorMix(value) => monitor = Some(value),
                    DeviceSetting::Mute(_) => {}
                }
            }
        }
        let muted = unit.and_then(UnitSnapshot::effective_mute);
        self.mute.set_sensitive(muted.is_some());
        self.mute.set_active(muted.unwrap_or(false));
        if let Some(profile) = profile {
            let gain = self.gain.widget().adjustment();
            gain.set_upper(f64::from(profile.gain_max));
            gain.set_step_increment((f64::from(profile.gain_scale) / 4.0).max(1.0));
            gain.set_page_increment(f64::from(profile.gain_scale) * 2.0);
            let headphone = self.headphone.widget().adjustment();
            headphone.set_lower(profile.hp_min_db());
            headphone.set_step_increment(if profile.legacy.is_none() { 0.25 } else { 0.5 });
            if profile.has_monitor_mix() {
                let monitor = self.monitor.widget().adjustment();
                monitor.set_upper(f64::from(profile.mix_max));
                monitor.set_step_increment((f64::from(profile.mix_max) / 100.0).max(1.0));
                monitor.set_page_increment(f64::from(profile.mix_max) * 0.08);
            }
        }
        self.gain.render(
            unit.and_then(|unit| gain.map(|raw| (unit.id, f64::from(raw)))),
            !snapshot.preferences.gain_locked,
        );
        self.headphone.render(
            unit.and_then(|unit| headphone.map(|db| (unit.id, db))),
            true,
        );
        self.phantom.set_sensitive(phantom.is_some());
        self.phantom.set_active(phantom.unwrap_or(false));
        self.low_z.set_sensitive(low_z.is_some());
        self.low_z.set_active(low_z.unwrap_or(false));
        self.monitor.render(
            unit.and_then(|unit| monitor.map(|raw| (unit.id, f64::from(raw)))),
            true,
        );
        self.knob_row
            .set_visible(profile.is_some_and(|profile| profile.legacy.is_some()));
        self.knob
            .set_label(match state.map(|state| state.knob_mode) {
                Some(KnobMode::Gain) => "Gain",
                Some(KnobMode::Headphones) => "Headphones",
                Some(KnobMode::MonitorMix) => "Monitor Mix",
                Some(KnobMode::None) | None => "—",
            });
        self.firmware
            .set_label(unit.map_or("—", |unit| nonempty(&unit.info.firmware)));
        self.api
            .set_label(unit.map_or("—", |unit| nonempty(&unit.info.api)));
        self.serial
            .set_label(unit.map_or("—", |unit| nonempty(&unit.info.serial)));
    }

    pub fn focus_settings(&self) {
        // The application reveals its existing split sidebar before this idle runs.
        let row = self.tray_color.downgrade();
        glib::idle_add_local_once(move || {
            if let Some(row) = row.upgrade() {
                row.grab_focus();
            }
        });
    }
}

fn nonempty(value: &str) -> &str {
    if value.is_empty() { "—" } else { value }
}

fn format_gain(raw: u16, scale: u16) -> String {
    if scale == 0 {
        return format!("0x{raw:04X}");
    }
    let value = format!("{:.2}", f64::from(raw) / f64::from(scale));
    format!("{} dB", value.trim_end_matches('0').trim_end_matches('.'))
}

fn info_row(expander: &adw::ExpanderRow, title: &str) -> gtk::Label {
    let (row, label) = value_row(title, 0);
    label.remove_css_class("monospace");
    label.add_css_class("dim-label");
    label.set_selectable(true);
    label.set_wrap(true);
    label.set_max_width_chars(22);
    expander.add_row(&row);
    label
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::test_support::{Rig, descendants, icons, unit};

    #[test]
    #[ignore = "requires the isolated installed GTK test runner"]
    fn new_gestures_never_target_previous_unit() {
        adw::init().expect("private GTK display");
        let a = unit("A", 1, -24.0);
        let b = unit("B", 2, -12.0);
        let rig = Rig::new(serde_json::json!({}), vec![a.clone(), b.clone()]);
        let sidebar = Sidebar::new(icons(), rig.submitter());
        let initial = rig.snapshot();
        assert_eq!(initial.selected_unit, Some(a.id));
        sidebar.render(initial.clone());
        assert!(sidebar.headphone.widget().is_sensitive() && sidebar.mute.is_sensitive());
        sidebar.selector.set_selected(1);
        // A stale render cannot re-enable controls with A's values while B is pending.
        sidebar.render(initial);
        assert_eq!(sidebar.selector.selected(), 1);
        assert!(!sidebar.headphone.widget().is_sensitive() && !sidebar.mute.is_sensitive());
        rig.finish_submissions();
        let selected = rig.snapshot();
        sidebar.render(selected.clone());
        assert!(sidebar.headphone.widget().is_sensitive() && sidebar.mute.is_sensitive());
        assert_eq!(sidebar.headphone.widget().value(), -12.0);
        sidebar.headphone.widget().set_value(-18.0);
        sidebar.render(selected);
        assert_eq!(sidebar.headphone.widget().value(), -18.0);
        assert!(
            descendants::<gtk::Label>(&sidebar.headphone.value_row)
                .iter()
                .any(|label| label.text() == "-18.0 dB")
        );
        sidebar.mute.set_active(true);
        rig.finish_submissions();
        let units = rig.device_states();
        let state = |id| {
            units
                .iter()
                .find(|unit| unit.id == id)
                .unwrap()
                .state
                .known()
                .unwrap()
        };
        assert_eq!(state(a.id).hp_volume_db, -24.0);
        assert!(!state(a.id).muted);
        assert_eq!(state(b.id).hp_volume_db, -18.0);
        assert!(state(b.id).muted);
        assert_eq!(rig.submitted_targets(), vec![b.id, b.id]);
    }

    #[test]
    #[ignore = "requires the isolated installed GTK test runner"]
    fn dock_controls_use_native_ranges_and_hide_the_absent_knob() {
        use cadiswave_core::{model::Observation, profiles::ProfileId, protocol::ConfigBuffer};
        adw::init().expect("private GTK display");
        let mut dock = unit("DOCK", 1, 0.0);
        dock.id.profile = ProfileId::XlrDockMk2;
        dock.state = Observation::Known(
            ConfigBuffer::decode_dock(&[0; 38], &[240, 0], &[100, 0, 0, 0, 0, 0])
                .unwrap()
                .state(),
        );
        let sidebar = Sidebar::new(icons(), Rc::new(|_| {}));
        sidebar.render(Arc::new(AppSnapshot {
            lifecycle: Lifecycle::Running,
            selected_unit: Some(dock.id),
            units: vec![dock].into(),
            ..Default::default()
        }));
        assert_eq!(sidebar.gain.widget().adjustment().upper(), 80.0);
        assert_eq!(sidebar.gain.widget().adjustment().step_increment(), 1.0);
        assert_eq!(sidebar.headphone.widget().adjustment().lower(), -60.0);
        assert_eq!(sidebar.headphone.widget().value(), -60.0);
        sidebar.headphone.widget().set_value(-12.25);
        assert!(
            descendants::<gtk::Label>(&sidebar.headphone.value_row)
                .iter()
                .any(|label| label.text() == "-12.25 dB")
        );
        assert_eq!(sidebar.monitor.widget().adjustment().upper(), 200.0);
        assert!(
            descendants::<gtk::Label>(&sidebar.monitor.value_row)
                .iter()
                .any(|label| label.text() == "50%")
        );
        assert!(!sidebar.knob_row.is_visible());
        let legacy = unit("LEGACY", 2, -90.0);
        sidebar.render(Arc::new(AppSnapshot {
            lifecycle: Lifecycle::Running,
            selected_unit: Some(legacy.id),
            units: vec![legacy].into(),
            ..Default::default()
        }));
        assert_eq!(sidebar.gain.widget().adjustment().step_increment(), 64.0);
        assert_eq!(sidebar.headphone.widget().adjustment().lower(), -128.0);
        assert_eq!(sidebar.headphone.widget().value(), -90.0);
        assert!(sidebar.knob_row.is_visible());
    }
}
