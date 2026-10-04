use super::{
    controls::DeviceControls, knob, meters::Meters, page::label, projection::DeviceProjection,
};
use adw::prelude::*;
use cadiswave_core::model::*;
use cadiswave_runtime::controller::EditTiming;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Default)]
pub struct ConnectionNotifier {
    last: Option<UnitId>,
    seen: bool,
}
impl ConnectionNotifier {
    pub fn observe(&mut self, p: &DeviceProjection) -> Option<gio::Notification> {
        let next = p.unit;
        if (self.seen && self.last == next) || (!self.seen && next.is_none()) {
            self.seen = true;
            return None;
        }
        self.seen = true;
        self.last = next;
        let notification = gio::Notification::new(&crate::i18n::tr(if next.is_some() {
            "connection-notification"
        } else {
            "disconnection-notification"
        }));
        if next.is_some() {
            notification.set_body(Some(&crate::i18n::tr("connection-body")));
        }
        notification.set_default_action("app.present");
        Some(notification)
    }
}
pub struct CompactControls {
    pub widget: gtk::Box,
    pub mute: gtk::Button,
    connection: gtk::Label,
    value: gtk::Label,
    gain: gtk::Scale,
    hp: gtk::Scale,
    low_z: gtk::Switch,
    meters: Meters,
    service: gtk::Label,
    projection: Rc<RefCell<DeviceProjection>>,
    updating: Rc<Cell<bool>>,
    gain_interaction: super::interaction::ScaleInteraction,
    hp_interaction: super::interaction::ScaleInteraction,
}
impl CompactControls {
    pub fn new(controls: DeviceControls, _locale: Rc<RefCell<crate::i18n::I18n>>) -> Self {
        let widget = gtk::Box::new(gtk::Orientation::Vertical, 16);
        widget.add_css_class("cadiswave-panel");
        widget.set_margin_top(12);
        widget.set_margin_bottom(12);
        widget.set_margin_start(12);
        widget.set_margin_end(12);
        let connection = gtk::Label::new(None);
        connection.set_xalign(0.0);
        connection.add_css_class("cadiswave-accent");
        widget.append(&connection);
        let mute = gtk::Button::new();
        mute.add_css_class("cadiswave-mute-button");
        widget.append(&mute);
        let dial_label = label("dial-control", "cadiswave-caption");
        widget.append(&dial_label);
        let value = gtk::Label::new(None);
        value.add_css_class("cadiswave-reading");
        widget.append(&value);
        let gain = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 0.005);
        gain.set_draw_value(false);
        gain.update_relation(&[gtk::accessible::Relation::LabelledBy(&[
            dial_label.upcast_ref()
        ])]);
        widget.append(&gain);
        let headphone_label = label("mode-headphones", "cadiswave-caption");
        widget.append(&headphone_label);
        let hp = gtk::Scale::with_range(gtk::Orientation::Horizontal, -128.0, 0.0, 0.5);
        hp.update_relation(&[gtk::accessible::Relation::LabelledBy(&[
            headphone_label.upcast_ref()
        ])]);
        hp.set_draw_value(true);
        hp.set_digits(1);
        widget.append(&hp);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let title = label("low-impedance", "");
        title.set_hexpand(true);
        row.append(&title);
        let low_z = gtk::Switch::new();
        low_z.update_relation(&[gtk::accessible::Relation::LabelledBy(&[title.upcast_ref()])]);
        row.append(&low_z);
        widget.append(&row);
        let meters = Meters::new();
        meters.widget.set_content_height(100);
        widget.append(&meters.widget);
        let service = gtk::Label::new(None);
        service.set_wrap(true);
        widget.append(&service);
        let full = gtk::Button::new();
        crate::i18n::bind_english(&full, "label", "Open CadisWave");
        full.set_action_name(Some("app.present"));
        widget.append(&full);
        let updating = Rc::new(Cell::new(false));
        let projection = Rc::new(RefCell::new(DeviceProjection::from_snapshot(
            &controls.snapshot(),
        )));
        let (c, p) = (controls.clone(), projection.clone());
        mute.connect_clicked(move |_| {
            let _ = c.toggle_mute(&p.borrow());
        });
        let gain_interaction = super::interaction::ScaleInteraction::bind(
            &gain,
            controls.clone(),
            projection.clone(),
            updating.clone(),
            super::interaction::ScaleKind::Dial,
        );
        let hp_interaction = super::interaction::ScaleInteraction::bind(
            &hp,
            controls.clone(),
            projection.clone(),
            updating.clone(),
            super::interaction::ScaleKind::Headphones,
        );
        let (c, p, g) = (controls, projection.clone(), updating.clone());
        low_z.connect_state_set(move |_, active| {
            if !g.get() {
                let _ = c.set(
                    &p.borrow(),
                    DeviceSetting::LowImpedance(active),
                    EditTiming::Immediate,
                );
            }
            glib::Propagation::Proceed
        });
        Self {
            widget,
            mute,
            connection,
            value,
            gain,
            hp,
            low_z,
            meters,
            service,
            projection,
            updating,
            gain_interaction,
            hp_interaction,
        }
    }
    pub fn render(&self, snapshot: &AppSnapshot) {
        self.updating.set(true);
        let p = DeviceProjection::from_snapshot(snapshot);
        *self.projection.borrow_mut() = p.clone();
        self.connection.set_text(&crate::i18n::tr(if p.writable {
            "device-connected"
        } else {
            "device-disconnected"
        }));
        self.mute.set_label(&crate::i18n::tr(
            if p.state.as_ref().is_some_and(|s| s.muted) {
                "tap-to-unmute"
            } else {
                "tap-to-mute"
            },
        ));
        self.mute.set_sensitive(p.writable);
        self.gain
            .set_sensitive(p.writable && knob::position(&p).is_some());
        if self.gain_interaction.can_render_value(&p) {
            self.gain.set_value(knob::position(&p).unwrap_or(0.0));
        }
        self.hp.set_sensitive(p.writable);
        if let Some(state) = p.state.as_ref() {
            self.hp
                .set_range(p.unit.unwrap().profile.profile().hp_min_db(), 0.0);
            if self.hp_interaction.can_render_value(&p) {
                self.hp.set_value(state.hp_volume_db);
            }
            self.low_z.set_active(state.low_impedance.unwrap_or(false));
        }
        self.low_z.set_sensitive(
            p.writable && p.state.as_ref().is_some_and(|s| s.low_impedance.is_some()),
        );
        self.value.set_text(
            &p.state
                .as_ref()
                .zip(p.unit)
                .map(|(s, id)| {
                    use cadiswave_core::protocol::KnobMode;
                    let (key, reading) = match s.knob_mode {
                        KnobMode::Gain => (
                            "mode-gain",
                            format!(
                                "{:.1} dB",
                                f64::from(s.gain_raw) / f64::from(id.profile.profile().gain_scale)
                            ),
                        ),
                        KnobMode::Headphones => {
                            ("mode-headphones", format!("{:.1} dB", s.hp_volume_db))
                        }
                        KnobMode::MonitorMix => (
                            "mode-monitor",
                            s.monitor_mix
                                .map(|value| {
                                    format!(
                                        "{:.0}%",
                                        f64::from(value) * 100.0
                                            / f64::from(id.profile.profile().mix_max)
                                    )
                                })
                                .unwrap_or_else(|| crate::i18n::tr("unknown-reading")),
                        ),
                        KnobMode::None => return crate::i18n::tr("unknown-reading"),
                    };
                    format!("{}   {reading}", crate::i18n::tr(key))
                })
                .unwrap_or_else(|| crate::i18n::tr("unknown-reading")),
        );
        let peaks = cadiswave_core::device_meter::selected_channels(snapshot);
        self.meters.set_peaks(peaks);
        self.service.set_text(&format!(
            "{} · {}",
            crate::i18n::tr(super::status::service_key(snapshot.service_state)),
            crate::i18n::tr(if peaks.is_some() {
                "capture-ready"
            } else {
                "capture-waiting"
            })
        ));
        self.updating.set(false);
    }
    pub fn retranslate(&self) {
        crate::i18n::retranslate();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::test_support::{Rig, unit};
    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn compact_reading_follows_confirmed_dial_mode() {
        gtk::init().unwrap();
        for language in [
            cadiswave_core::locale::LanguageChoice::English,
            cadiswave_core::locale::LanguageChoice::Indonesian,
        ] {
            let locale = Rc::new(RefCell::new(
                crate::i18n::I18n::new(language, "en").unwrap(),
            ));
            crate::i18n::activate(locale.clone());
            for monitor in [false, true] {
                let mut device = unit("A", 2, -12.0);
                if let Observation::Known(state) = &mut device.state {
                    state.knob_mode = cadiswave_core::protocol::KnobMode::Headphones;
                }
                if monitor {
                    device.id.profile = cadiswave_core::profiles::ProfileId::Wave3;
                    if let Observation::Known(state) = &mut device.state {
                        state.knob_mode = cadiswave_core::protocol::KnobMode::MonitorMix;
                        state.monitor_mix = Some(25 * 256);
                    }
                }
                let rig = Rig::new(serde_json::json!({}), vec![device]);
                let compact =
                    CompactControls::new(DeviceControls::new(rig.handle()), locale.clone());
                compact.render(&rig.snapshot());
                let mode = crate::i18n::tr(if monitor {
                    "mode-monitor"
                } else {
                    "mode-headphones"
                });
                assert!(
                    compact.value.text().contains(&mode),
                    "{}",
                    compact.value.text()
                );
                assert!(
                    compact
                        .value
                        .text()
                        .contains(if monitor { "25%" } else { "-12.0 dB" })
                );
                assert_eq!(rig.device_command_count(), 0);
            }
        }
    }
}
