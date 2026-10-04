//! Present exact-profile controls and confirmed device observations.
use super::super::{
    controls::DeviceControls,
    interaction::{ScaleInteraction, ScaleKind},
    projection::DeviceProjection,
};
use adw::prelude::*;
use cadiswave_core::{capabilities, device_settings::capture_rate, model::*};
use cadiswave_runtime::controller::EditTiming;
use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
};

pub struct HardwareSettings {
    pub processing: adw::PreferencesGroup,
    pub leds: adw::PreferencesGroup,
    pub audio: adw::PreferencesGroup,
    pub information: adw::PreferencesGroup,
    clipguard: adw::SwitchRow,
    lowcut: adw::SwitchRow,
    monitor: adw::ActionRow,
    balance: gtk::Scale,
    rate: adw::ActionRow,
    model: adw::ActionRow,
    firmware: adw::ActionRow,
    api: adw::ActionRow,
    connection: adw::ActionRow,
    feedback: gtk::Label,
    updating: Rc<Cell<bool>>,
    projection: Rc<RefCell<DeviceProjection>>,
    interaction: ScaleInteraction,
    commands: Rc<RefCell<HashSet<CommandId>>>,
    monitor_marks: RefCell<Vec<(f64, String)>>,
    observed_switches: [Rc<Cell<bool>>; 2],
}

fn group(key: &str) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::new();
    crate::i18n::bind(&group, "title", key);
    group
}
fn row(group: &adw::PreferencesGroup, key: &str) -> adw::ActionRow {
    let row = adw::ActionRow::new();
    crate::i18n::bind(&row, "title", key);
    row.set_subtitle_lines(3);
    group.add(&row);
    row
}
fn unavailable(group: &adw::PreferencesGroup, key: &str, explanation: &str) {
    let row = row(group, key);
    crate::i18n::bind(&row, "subtitle", explanation);
    let icon = gtk::Image::from_icon_name("changes-prevent-symbolic");
    icon.set_sensitive(false);
    row.add_suffix(&icon);
}

impl HardwareSettings {
    pub fn new(controls: DeviceControls) -> Self {
        let updating = Rc::new(Cell::new(false));
        let projection = Rc::new(RefCell::new(DeviceProjection::from_snapshot(
            &controls.snapshot(),
        )));
        let commands = Rc::new(RefCell::new(HashSet::new()));
        let processing = group("hardware-processing");
        let feedback = gtk::Label::builder()
            .wrap(true)
            .xalign(0.0)
            .selectable(true)
            .build();
        feedback.add_css_class("error");
        feedback.set_visible(false);
        crate::i18n::protect(&feedback);
        let switches = [
            (
                "clipguard",
                DeviceSetting::Clipguard as fn(bool) -> DeviceSetting,
            ),
            (
                "low-cut-hardware",
                DeviceSetting::HardwareLowCut as fn(bool) -> DeviceSetting,
            ),
        ]
        .map(|(key, setting)| {
            let row = adw::SwitchRow::new();
            crate::i18n::bind(&row, "title", key);
            row.set_subtitle_lines(3);
            let observed = Rc::new(Cell::new(false));
            let rendered = observed.clone();
            let (guard, current, controls, pending, feedback) = (
                updating.clone(),
                projection.clone(),
                controls.clone(),
                commands.clone(),
                feedback.clone(),
            );
            row.connect_active_notify(move |row| {
                if guard.get() || !row.is_sensitive() || row.is_active() == rendered.get() {
                    return;
                }
                match controls.set(
                    &current.borrow(),
                    setting(row.is_active()),
                    EditTiming::Immediate,
                ) {
                    Ok(id) => {
                        pending.borrow_mut().insert(id);
                        feedback.set_visible(false);
                        row.set_sensitive(false);
                    }
                    Err(error) => {
                        feedback.set_text(&format!("{error:?}"));
                        feedback.set_visible(true);
                    }
                }
            });
            processing.add(&row);
            (row, observed)
        });
        processing.add(&feedback);
        let [(clipguard, clipguard_observed), (lowcut, lowcut_observed)] = switches;
        let leds = group("led-settings");
        unavailable(&leds, "led-color", "hardware-feature-unmapped");
        unavailable(&leds, "led-brightness", "hardware-feature-unmapped");
        let audio = group("settings-audio");
        let monitor = row(&audio, "monitor-mix");
        let balance = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 0.5);
        balance.set_draw_value(false);
        balance.set_hexpand(true);
        balance.set_margin_start(12);
        balance.set_margin_end(12);
        balance.set_margin_bottom(12);
        balance.add_mark(50.0, gtk::PositionType::Bottom, None);
        let balance_row = adw::PreferencesRow::new();
        balance_row.set_activatable(false);
        balance_row.set_child(Some(&balance));
        audio.add(&balance_row);
        let interaction = ScaleInteraction::bind(
            &balance,
            controls.clone(),
            projection.clone(),
            updating.clone(),
            ScaleKind::Monitor,
        );
        let rate = row(&audio, "sample-rate");
        unavailable(&audio, "save-to-device", "hardware-persistence-unmapped");
        let information = group("settings-device-information");
        let model = row(&information, "settings-device-model");
        let firmware = row(&information, "ui-firmware");
        let api = row(&information, "settings-device-api");
        let connection = row(&information, "settings-device-connection");
        Self {
            processing,
            leds,
            audio,
            information,
            clipguard,
            lowcut,
            monitor,
            balance,
            rate,
            model,
            firmware,
            api,
            connection,
            feedback,
            updating,
            projection,
            interaction,
            commands,
            monitor_marks: RefCell::new(Vec::new()),
            observed_switches: [clipguard_observed, lowcut_observed],
        }
    }

    pub fn completed(&self, id: CommandId, outcome: &CommandOutcome) -> bool {
        if !self.commands.borrow_mut().remove(&id) {
            return false;
        }
        if let CommandOutcome::Rejected(error) = outcome {
            self.feedback.set_text(&error.to_string());
            self.feedback.set_visible(true);
        }
        true
    }

    pub fn render(&self, snapshot: &AppSnapshot, source: Option<&SourceId>) {
        self.updating.set(true);
        let p = DeviceProjection::from_snapshot(snapshot);
        let capability = p.unit.map(|unit| capabilities::for_profile(unit.profile));
        let selected = p
            .unit
            .and_then(|id| snapshot.units.iter().find(|unit| unit.id == id));
        let state = p.state.as_ref();
        for (index, (row, supported, value, pending)) in [
            (
                &self.clipguard,
                selected.is_some_and(|unit| {
                    capabilities::clipguard_available(unit.id.profile, &unit.info.api)
                }),
                state.and_then(|s| s.clipguard),
                p.pending
                    .iter()
                    .any(|setting| matches!(setting, DeviceSetting::Clipguard(_))),
            ),
            (
                &self.lowcut,
                capability.is_some_and(|c| c.hardware_low_cut),
                state.and_then(|s| s.hardware_low_cut),
                p.pending
                    .iter()
                    .any(|setting| matches!(setting, DeviceSetting::HardwareLowCut(_))),
            ),
        ]
        .into_iter()
        .enumerate()
        {
            self.observed_switches[index].set(value.unwrap_or(false));
            row.set_active(value.unwrap_or(false));
            row.set_sensitive(
                supported
                    && p.writable
                    && value.is_some()
                    && !pending
                    && self.commands.borrow().is_empty(),
            );
            row.set_subtitle(&crate::i18n::tr(if p.unit.is_none() {
                "settings-device-unavailable"
            } else if !supported {
                "hardware-feature-unmapped"
            } else if pending {
                "settings-hardware-pending"
            } else if !p.writable || value.is_none() {
                "settings-device-unavailable"
            } else {
                "settings-hardware-confirmed"
            }));
        }
        let mix = state
            .and_then(|s| s.monitor_mix)
            .zip(p.unit)
            .filter(|(raw, unit)| *raw <= unit.profile.profile().mix_max)
            .map(|(raw, unit)| 100.0 * f64::from(raw) / f64::from(unit.profile.profile().mix_max));
        let supported = capability.is_some_and(|c| c.monitor_mix);
        let desired_mix = p.pending.iter().rev().find_map(|setting| {
            if let DeviceSetting::MonitorMix(raw) = setting {
                p.unit
                    .filter(|unit| *raw <= unit.profile.profile().mix_max)
                    .map(|unit| 100.0 * f64::from(*raw) / f64::from(unit.profile.profile().mix_max))
            } else {
                None
            }
        });
        self.balance.set_visible(supported);
        self.balance
            .set_sensitive(supported && p.writable && mix.is_some());
        let endpoints = p.unit.and_then(|unit| match unit.profile {
            cadiswave_core::profiles::ProfileId::WaveXlr => {
                Some(("settings-monitor-pc", "settings-monitor-microphone"))
            }
            cadiswave_core::profiles::ProfileId::XlrDockMk2 => {
                Some(("settings-monitor-microphone", "settings-monitor-pc"))
            }
            _ => None,
        });
        let marks = endpoints
            .map(|(left, right)| {
                vec![
                    (0.0, crate::i18n::tr(left)),
                    (100.0, crate::i18n::tr(right)),
                ]
            })
            .unwrap_or_default();
        if *self.monitor_marks.borrow() != marks {
            self.balance.clear_marks();
            self.balance.add_mark(50.0, gtk::PositionType::Bottom, None);
            for (value, label) in &marks {
                self.balance
                    .add_mark(*value, gtk::PositionType::Bottom, Some(label));
            }
            *self.monitor_marks.borrow_mut() = marks;
        }
        self.balance
            .update_property(&[gtk::accessible::Property::Label(&crate::i18n::tr(
                "monitor-mix",
            ))]);
        if let Some(value) = mix {
            self.monitor.set_subtitle(&crate::i18n::format(
                if desired_mix.is_some() {
                    "settings-monitor-pending"
                } else {
                    "settings-monitor-observed"
                },
                &[("value", &format!("{value:.1}"))],
            ));
            let displayed = desired_mix.unwrap_or(value);
            if self.interaction.can_render_value(&p) && self.balance.value() != displayed {
                self.balance.set_value(displayed);
            }
        } else {
            self.monitor.set_subtitle(&crate::i18n::tr(if supported {
                "settings-device-unavailable"
            } else {
                "hardware-feature-unmapped"
            }));
        }
        let unit = p
            .unit
            .and_then(|id| snapshot.units.iter().find(|unit| unit.id == id));
        self.model.set_subtitle(
            &unit
                .map(|unit| {
                    format!(
                        "{} · {:04x}:{:04x}",
                        unit.id.profile.profile().display_name,
                        unit.id.profile.profile().vid,
                        unit.id.profile.profile().pid
                    )
                })
                .unwrap_or_else(|| crate::i18n::tr("ui-no-device-selected")),
        );
        for (row, value) in [
            (&self.firmware, unit.map(|u| u.info.firmware.as_str())),
            (&self.api, unit.map(|u| u.info.api.as_str())),
        ] {
            row.set_subtitle(
                &value
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
                    .unwrap_or_else(|| crate::i18n::tr("unknown-reading")),
            );
        }
        self.connection
            .set_subtitle(&crate::i18n::tr(if p.writable {
                "settings-device-connected"
            } else {
                "settings-device-unavailable"
            }));
        let capture = source
            .and_then(|id| snapshot.desired.sources.get(id))
            .and_then(|source| {
                let mut captures = snapshot
                    .captures
                    .iter()
                    .filter(|capture| capture.node_name == source.node_name);
                let capture = captures.next()?;
                captures.next().is_none().then_some(capture)
            });
        self.rate.set_subtitle(
            &capture
                .and_then(capture_rate)
                .map(|rate| {
                    crate::i18n::format(
                        if rate.negotiated {
                            "settings-rate-observed"
                        } else {
                            "settings-rate-configured"
                        },
                        &[("value", &rate.hz.to_string())],
                    )
                })
                .unwrap_or_else(|| crate::i18n::tr("settings-rate-unavailable")),
        );
        *self.projection.borrow_mut() = p;
        self.updating.set(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::test_support::{Rig, unit};
    use cadiswave_core::{profiles::ProfileId, protocol::ConfigBuffer};
    use std::time::{Duration, Instant};

    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn unverified_wave3_api_cannot_submit_clipguard() {
        adw::init().unwrap();
        let mut wave = unit("A", 2, -10.0);
        wave.id.profile = ProfileId::Wave3;
        wave.info.api = "5.2".into();
        wave.state = Observation::Known(
            ConfigBuffer::decode(ProfileId::Wave3, &[0; 16])
                .unwrap()
                .state(),
        );
        let rig = Rig::new(serde_json::json!({}), vec![wave]);
        let settings = HardwareSettings::new(DeviceControls::new(rig.handle()));
        settings.render(&rig.snapshot(), None);
        assert!(!settings.clipguard.is_sensitive());
        settings.clipguard.set_active(true);
        assert!(settings.commands.borrow().is_empty());
        assert_eq!(rig.device_command_count(), 0);
    }

    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn hardware_switch_reports_failure_without_changing_confirmed_state() {
        adw::init().unwrap();
        let mut wave = unit("A", 2, -10.0);
        wave.id.profile = ProfileId::Wave3;
        wave.info.api = "5.3".into();
        wave.state = Observation::Known(
            ConfigBuffer::decode(ProfileId::Wave3, &[0; 16])
                .unwrap()
                .state(),
        );
        let rig = Rig::new(serde_json::json!({}), vec![wave]);
        let settings = HardwareSettings::new(DeviceControls::new(rig.handle()));
        settings.render(&rig.snapshot(), None);
        rig.fail_next_device_command();
        settings.clipguard.set_active(true);
        let id = *settings.commands.borrow().iter().next().unwrap();
        let outcome = rig.outcome(id);
        assert!(matches!(outcome, CommandOutcome::Rejected(_)));
        assert!(settings.completed(id, &outcome));
        for _ in 0..20 {
            settings.render(&rig.snapshot(), None);
        }
        assert!(!settings.clipguard.is_active());
        assert!(settings.feedback.is_visible());
        assert!(!settings.feedback.text().is_empty());
        assert_eq!(rig.device_command_count(), 1);
        settings.clipguard.set_active(true);
        let id = *settings.commands.borrow().iter().next().unwrap();
        let outcome = rig.outcome(id);
        assert!(matches!(outcome, CommandOutcome::Applied { .. }));
        settings.completed(id, &outcome);
        settings.render(&rig.snapshot(), None);
        assert!(settings.clipguard.is_active());
        assert_eq!(rig.device_command_count(), 2);
    }

    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn original_monitor_slider_uses_confirmed_state_and_exact_scale() {
        adw::init().unwrap();
        let rig = Rig::new(serde_json::json!({}), vec![unit("A", 2, -10.0)]);
        let settings = HardwareSettings::new(DeviceControls::new(rig.handle()));
        for _ in 0..20 {
            settings.render(&rig.snapshot(), None);
        }
        assert!(settings.balance.is_sensitive());
        assert_eq!(settings.balance.value(), 0.0);
        assert_eq!(rig.device_command_count(), 0);
        settings.balance.set_value(50.0);
        let deadline = Instant::now() + Duration::from_secs(2);
        while rig.device_command_count() == 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(rig.device_command_count(), 1);
        rig.finish_submissions();
        settings.render(&rig.snapshot(), None);
        assert_eq!(
            rig.device_states()[0].state.known().unwrap().monitor_mix,
            Some(12800)
        );
        assert_eq!(settings.balance.value(), 50.0);
        for _ in 0..20 {
            settings.render(&rig.snapshot(), None);
        }
        assert_eq!(rig.device_command_count(), 1);
    }
}
