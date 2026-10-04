use super::{controls::DeviceControls, knob::Knob, meters::Meters, projection::DeviceProjection};
use adw::prelude::*;
use cadiswave_core::{model::*, protocol::KnobMode};
use cadiswave_runtime::controller::EditTiming;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
pub fn label(key: &str, class: &str) -> gtk::Label {
    let label = gtk::Label::new(None);
    label.set_xalign(0.0);
    if !class.is_empty() {
        label.add_css_class(class);
    }
    crate::i18n::bind(&label, "label", key);
    label
}
pub fn panel() -> gtk::Box {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 14);
    panel.add_css_class("cadiswave-panel");
    panel
}
pub struct DevicePage {
    pub widget: gtk::ScrolledWindow,
    columns: gtk::Box,
    face: gtk::Box,
    left: gtk::Box,
    center: gtk::Box,
    hp_interaction: super::interaction::ScaleInteraction,
    pub knob: Knob,
    meters: Meters,
    reading: gtk::Label,
    gain: gtk::Label,
    dial_value: gtk::Label,
    mode: gtk::Label,
    connection: gtk::Label,
    pub mute: gtk::Button,
    phantom: gtk::Label,
    headphone: gtk::Scale,
    headphone_value: gtk::Label,
    low_z: gtk::Switch,
    service: gtk::Label,
    readiness: gtk::Label,
    pending: gtk::Label,
    event: gtk::Label,
    event_text: gtk::TextView,
    info: gtk::Label,
    projection: Rc<RefCell<DeviceProjection>>,
    updating: Rc<Cell<bool>>,
}
impl DevicePage {
    pub fn new(controls: DeviceControls, _locale: Rc<RefCell<crate::i18n::I18n>>) -> Self {
        let widget = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .build();
        let body = gtk::Box::new(gtk::Orientation::Vertical, 24);
        body.set_margin_start(32);
        body.set_margin_end(32);
        body.set_margin_top(32);
        body.set_margin_bottom(20);
        let columns = gtk::Box::new(gtk::Orientation::Horizontal, 24);
        columns.set_vexpand(true);
        let left = gtk::Box::new(gtk::Orientation::Vertical, 20);
        left.set_width_request(260);
        left.set_hexpand(false);
        let input = panel();
        input.append(&label("input-level", "cadiswave-caption"));
        let meters = Meters::new();
        input.append(&meters.widget);
        let reading = label("unknown-reading", "cadiswave-reading");
        reading.set_wrap(true);
        reading.set_max_width_chars(16);
        input.append(&reading);
        let gain = gtk::Label::new(None);
        gain.set_xalign(0.0);
        gain.add_css_class("cadiswave-accent");
        input.append(&gain);
        left.append(&input);
        let dial = panel();
        dial.append(&label("dial-mode", "cadiswave-caption"));
        let mode = gtk::Label::new(None);
        mode.set_xalign(0.0);
        mode.add_css_class("title-3");
        dial.append(&mode);
        let hint = label("sync-active", "dim-label");
        hint.set_wrap(true);
        hint.set_max_width_chars(28);
        dial.append(&hint);
        left.append(&dial);
        let center = gtk::Box::new(gtk::Orientation::Vertical, 20);
        center.set_hexpand(true);
        center.set_width_request(300);
        let connection = gtk::Label::new(None);
        connection.add_css_class("cadiswave-connection");
        center.append(&connection);
        let device = panel();
        device.add_css_class("cadiswave-device");
        let mute = gtk::Button::new();
        mute.add_css_class("cadiswave-mute-button");
        device.append(&mute);
        let dial_value = gtk::Label::new(Some("—"));
        dial_value.add_css_class("cadiswave-dial-value");
        let face = gtk::Box::new(gtk::Orientation::Horizontal, 20);
        dial_value.set_hexpand(true);
        dial_value.set_valign(gtk::Align::Center);
        face.append(&dial_value);
        let knob = Knob::new(controls.clone());
        face.append(&knob.widget);
        device.append(&face);
        let dial_label = label("dial-control", "cadiswave-caption");
        knob.scale
            .update_relation(&[gtk::accessible::Relation::LabelledBy(&[
                dial_label.upcast_ref()
            ])]);
        device.append(&dial_label);
        let phantom = label("phantom-indicator", "dim-label");
        device.append(&phantom);
        let pending = label("pending-control", "cadiswave-pending");
        pending.set_visible(false);
        device.append(&pending);
        center.append(&device);
        let info = gtk::Label::new(None);
        info.set_wrap(true);
        info.set_selectable(true);
        info.add_css_class("monospace");
        info.add_css_class("dim-label");
        crate::i18n::protect(&info);
        center.append(&info);
        let right = gtk::Box::new(gtk::Orientation::Vertical, 20);
        right.set_width_request(280);
        right.set_hexpand(false);
        let hp = panel();
        let headphone_label = label("mode-headphones", "cadiswave-caption");
        hp.append(&headphone_label);
        let headphone_value = gtk::Label::new(None);
        headphone_value.set_xalign(0.0);
        headphone_value.add_css_class("cadiswave-reading");
        hp.append(&headphone_value);
        let headphone = gtk::Scale::with_range(gtk::Orientation::Horizontal, -128.0, 0.0, 0.5);
        headphone.set_draw_value(false);
        headphone.update_relation(&[gtk::accessible::Relation::LabelledBy(&[
            headphone_label.upcast_ref()
        ])]);
        hp.append(&headphone);
        let low = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let title = label("low-impedance", "");
        title.set_hexpand(true);
        title.set_wrap(true);
        low.append(&title);
        let low_z = gtk::Switch::new();
        low_z.update_relation(&[gtk::accessible::Relation::LabelledBy(&[title.upcast_ref()])]);
        low_z.set_valign(gtk::Align::Center);
        low.append(&low_z);
        hp.append(&low);
        right.append(&hp);
        let capture = panel();
        capture.append(&label("capture-fix", "cadiswave-caption"));
        let service = gtk::Label::new(None);
        service.set_xalign(0.0);
        capture.append(&service);
        let readiness = gtk::Label::new(None);
        readiness.set_xalign(0.0);
        readiness.set_wrap(true);
        capture.append(&readiness);
        let legacy = label("legacy-sync-unavailable", "dim-label");
        legacy.set_wrap(true);
        legacy.set_max_width_chars(26);
        capture.append(&legacy);
        right.append(&capture);
        columns.append(&left);
        columns.append(&center);
        columns.append(&right);
        body.append(&columns);
        let expander = gtk::Expander::new(Some(&crate::i18n::tr("latest-event")));
        crate::i18n::bind(&expander, "label", "latest-event");
        let event_text = gtk::TextView::builder()
            .editable(false)
            .cursor_visible(false)
            .monospace(true)
            .wrap_mode(gtk::WrapMode::WordChar)
            .build();
        crate::i18n::protect(&event_text);
        let events = gtk::ScrolledWindow::builder()
            .height_request(150)
            .child(&event_text)
            .build();
        expander.set_child(Some(&events));
        let event = label("no-events", "dim-label");
        event.set_ellipsize(gtk::pango::EllipsizeMode::End);
        body.append(&event);
        body.append(&expander);
        body.append(&label("keyboard-hints", "cadiswave-caption"));
        widget.set_child(Some(&body));
        let projection = Rc::new(RefCell::new(DeviceProjection::from_snapshot(
            &controls.snapshot(),
        )));
        let updating = Rc::new(Cell::new(false));
        let (p, c) = (projection.clone(), controls.clone());
        mute.connect_clicked(move |_| {
            let _ = c.toggle_mute(&p.borrow());
        });
        let hp_interaction = super::interaction::ScaleInteraction::bind(
            &headphone,
            controls.clone(),
            projection.clone(),
            updating.clone(),
            super::interaction::ScaleKind::Headphones,
        );
        let (p, c, guard) = (projection.clone(), controls, updating.clone());
        low_z.connect_state_set(move |_, active| {
            if !guard.get() {
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
            columns,
            face,
            left,
            center,
            hp_interaction,
            knob,
            meters,
            reading,
            gain,
            dial_value,
            mode,
            connection,
            mute,
            phantom,
            headphone,
            headphone_value,
            low_z,
            service,
            readiness,
            pending,
            event,
            event_text,
            info,
            projection,
            updating,
        }
    }
    pub fn install_breakpoints(
        &self,
        window: &adw::ApplicationWindow,
        split: &adw::OverlaySplitView,
    ) {
        let medium = adw::Breakpoint::new(
            adw::BreakpointCondition::parse("max-width: 1220sp").expect("constant breakpoint"),
        );
        medium.add_setter(
            &self.face,
            "orientation",
            Some(&gtk::Orientation::Vertical.to_value()),
        );
        window.add_breakpoint(medium);
        let small = adw::Breakpoint::new(
            adw::BreakpointCondition::parse("max-width: 1000sp").expect("constant breakpoint"),
        );
        small.add_setter(
            &self.face,
            "orientation",
            Some(&gtk::Orientation::Vertical.to_value()),
        );
        small.add_setter(
            &self.columns,
            "orientation",
            Some(&gtk::Orientation::Vertical.to_value()),
        );
        small.add_setter(split, "collapsed", Some(&true.to_value()));
        let (columns, center) = (self.columns.clone(), self.center.clone());
        small.connect_apply(move |_| columns.reorder_child_after(&center, None::<&gtk::Widget>));
        let (columns, left) = (self.columns.clone(), self.left.clone());
        small.connect_unapply(move |_| columns.reorder_child_after(&left, None::<&gtk::Widget>));
        window.add_breakpoint(small);
    }
    pub fn set_events(&self, events: &[String]) {
        if let Some(last) = events.last()
            && self.event.text() != *last
        {
            self.event.set_text(last);
            self.event_text.buffer().set_text(&events.join("\n"));
        }
    }
    pub fn render(&self, snapshot: &AppSnapshot) {
        self.updating.set(true);
        let p = DeviceProjection::from_snapshot(snapshot);
        *self.projection.borrow_mut() = p.clone();
        self.knob.set_projection(&p);
        let unit = p
            .unit
            .and_then(|id| snapshot.units.iter().find(|u| u.id == id));
        let state = p.state.as_ref();
        self.connection.set_text(&crate::i18n::tr(if p.writable {
            "device-connected"
        } else {
            "device-disconnected"
        }));
        self.mute
            .set_label(&crate::i18n::tr(if state.is_some_and(|s| s.muted) {
                "tap-to-unmute"
            } else {
                "tap-to-mute"
            }));
        self.mute.set_sensitive(p.writable);
        if state.is_some_and(|s| s.muted) {
            self.mute.add_css_class("cadiswave-muted");
        } else {
            self.mute.remove_css_class("cadiswave-muted");
        }
        self.pending.set_visible(!p.pending.is_empty());
        self.mode
            .set_text(&crate::i18n::tr(match state.map(|s| s.knob_mode) {
                Some(KnobMode::Gain) => "mode-gain",
                Some(KnobMode::Headphones) => "mode-headphones",
                Some(KnobMode::MonitorMix) => "mode-monitor",
                _ => "mode-unknown",
            }));
        self.gain.set_text(
            &state
                .zip(p.unit)
                .map(|(s, id)| {
                    format!(
                        "{}  {:.1} / {:.0} dB",
                        crate::i18n::tr("mic-gain"),
                        f64::from(s.gain_raw) / f64::from(id.profile.profile().gain_scale),
                        f64::from(id.profile.profile().gain_max)
                            / f64::from(id.profile.profile().gain_scale)
                    )
                })
                .unwrap_or_else(|| crate::i18n::tr("unknown-reading")),
        );
        self.dial_value.set_text(
            &state
                .zip(p.unit)
                .map(|(s, id)| match s.knob_mode {
                    KnobMode::Gain => format!(
                        "{:.1} dB",
                        f64::from(s.gain_raw) / f64::from(id.profile.profile().gain_scale)
                    ),
                    KnobMode::Headphones => format!("{:.1} dB", s.hp_volume_db),
                    KnobMode::MonitorMix => format!(
                        "{:.0}%",
                        f64::from(s.monitor_mix.unwrap_or(0)) * 100.0
                            / f64::from(id.profile.profile().mix_max)
                    ),
                    _ => "—".into(),
                })
                .unwrap_or_else(|| "—".into()),
        );
        self.phantom
            .set_opacity(if state.is_some_and(|s| s.phantom == Some(true)) {
                1.0
            } else {
                0.35
            });
        self.headphone.set_sensitive(p.writable);
        self.low_z
            .set_sensitive(p.writable && state.is_some_and(|s| s.low_impedance.is_some()));
        if let Some(state) = state {
            self.headphone
                .set_range(p.unit.unwrap().profile.profile().hp_min_db(), 0.0);
            if self.hp_interaction.can_render_value(&p) {
                self.headphone.set_value(state.hp_volume_db);
            }
            self.low_z.set_active(state.low_impedance.unwrap_or(false));
            self.headphone_value
                .set_text(&format!("{:.1} dB", state.hp_volume_db));
        } else {
            self.headphone_value.set_text("—");
        }
        let peaks = cadiswave_core::device_meter::selected_channels(snapshot);
        self.meters.set_peaks(peaks);
        if peaks.is_none() {
            self.reading.remove_css_class("cadiswave-reading");
        } else {
            self.reading.add_css_class("cadiswave-reading");
        }
        self.reading.set_text(
            &peaks
                .map(|p| {
                    if p.maximum() == 0.0 {
                        "−∞ dBFS".into()
                    } else {
                        format!("{:.1} dBFS", 20.0 * p.maximum().log10())
                    }
                })
                .unwrap_or_else(|| crate::i18n::tr("unknown-reading")),
        );
        self.service
            .set_text(&crate::i18n::tr(super::status::service_key(
                snapshot.service_state,
            )));
        self.readiness
            .set_text(&crate::i18n::tr(if peaks.is_some() {
                "capture-ready"
            } else {
                "capture-waiting"
            }));
        self.info.set_text(
            &unit
                .map(|u| {
                    format!(
                        "{}   ·   {}\n{}",
                        u.info.firmware, u.info.api, u.info.serial
                    )
                })
                .unwrap_or_else(|| crate::i18n::tr("no-device-hint")),
        );
        self.updating.set(false);
    }
    pub fn retranslate(&self) {
        crate::i18n::retranslate();
    }
}
