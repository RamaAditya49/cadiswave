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
        widget.append(&label("dial-control", "cadiswave-caption"));
        let value = gtk::Label::new(None);
        value.add_css_class("cadiswave-reading");
        widget.append(&value);
        let gain = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 0.005);
        gain.set_draw_value(false);
        widget.append(&gain);
        widget.append(&label("mode-headphones", "cadiswave-caption"));
        let hp = gtk::Scale::with_range(gtk::Orientation::Horizontal, -128.0, 0.0, 0.5);
        hp.set_draw_value(true);
        hp.set_digits(1);
        widget.append(&hp);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let title = label("low-impedance", "");
        title.set_hexpand(true);
        row.append(&title);
        let low_z = gtk::Switch::new();
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
        let (c, p, g) = (controls.clone(), projection.clone(), updating.clone());
        gain.connect_value_changed(move |scale| {
            if !g.get() {
                let _ = c.edit(&p.borrow(), scale.value(), EditTiming::Debounced);
            }
        });
        let (c, p, g) = (controls.clone(), projection.clone(), updating.clone());
        hp.connect_value_changed(move |scale| {
            if !g.get() {
                let _ = c.set(
                    &p.borrow(),
                    DeviceSetting::HeadphoneDb(scale.value()),
                    EditTiming::Debounced,
                );
            }
        });
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
        self.gain.set_value(knob::position(&p).unwrap_or(0.0));
        self.hp.set_sensitive(p.writable);
        if let Some(state) = p.state.as_ref() {
            self.hp
                .set_range(p.unit.unwrap().profile.profile().hp_min_db(), 0.0);
            self.hp.set_value(state.hp_volume_db);
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
                    format!(
                        "{}   {:.1} dB",
                        crate::i18n::tr("mic-gain"),
                        f64::from(s.gain_raw) / f64::from(id.profile.profile().gain_scale)
                    )
                })
                .unwrap_or_else(|| crate::i18n::tr("unknown-reading")),
        );
        let peaks = cadiswave_core::device_meter::selected_channels(snapshot);
        self.meters.set_peaks(peaks);
        self.service.set_text(&format!(
            "{} · {}",
            crate::i18n::tr(super::status::service_key(&snapshot.service_status)),
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
