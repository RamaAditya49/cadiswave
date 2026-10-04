use cadiswave_core::effects::FxSettings;
use gtk::{glib, prelude::*};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub(crate) struct FxControls {
    pub(crate) popover: gtk::Popover,
    value: Rc<RefCell<FxSettings>>,
    updating: Rc<Cell<bool>>,
    lowcut: gtk::DropDown,
    gate: gtk::Switch,
    comp: gtk::Switch,
    mono: gtk::Switch,
    scales: Vec<gtk::Scale>,
    delay: gtk::SpinButton,
}

impl FxControls {
    pub(crate) fn new(
        changed: impl Fn(FxSettings) + 'static,
        calibrate: impl Fn() + 'static,
    ) -> Self {
        let popover = gtk::Popover::new();
        let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
        body.set_margin_start(12);
        body.set_margin_end(12);
        body.set_margin_top(12);
        body.set_margin_bottom(12);
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .max_content_height(600)
            .build();
        scroll.set_child(Some(&body));
        popover.set_child(Some(&scroll));
        let calibration = gtk::Button::with_label("Auto-calibrate microphone");
        calibration.add_css_class("flat");
        let popup = popover.downgrade();
        let calibrate: Rc<dyn Fn()> = Rc::new(calibrate);
        calibration.connect_clicked(move |_| {
            if let Some(popover) = popup.upgrade() {
                popover.popdown();
            }
            let calibrate = calibrate.clone();
            glib::idle_add_local_once(move || calibrate());
        });
        body.append(&calibration);
        let changed: Rc<dyn Fn(FxSettings)> = Rc::new(changed);
        let updating = Rc::new(Cell::new(false));
        let value = Rc::new(RefCell::new(FxSettings::default()));
        let lowcut = gtk::DropDown::from_strings(&["Off", "80 Hz", "120 Hz"]);
        fx_row(&body, "Low cut", &lowcut);
        let (guard, settings, callback) = (updating.clone(), value.clone(), changed.clone());
        lowcut.connect_selected_notify(move |dropdown| {
            if !guard.get() {
                settings.borrow_mut().lowcut = match dropdown.selected() {
                    1 => 80,
                    2 => 120,
                    _ => 0,
                };
                let value = settings.borrow().clone();
                callback(value);
            }
        });
        let mut switches = Vec::new();
        for (index, title) in [(0, "Gate"), (1, "Comp"), (2, "Mono")] {
            let switch = gtk::Switch::builder()
                .halign(gtk::Align::Start)
                .valign(gtk::Align::Center)
                .build();
            let (guard, settings, callback) = (updating.clone(), value.clone(), changed.clone());
            switch.connect_active_notify(move |switch| {
                if !guard.get() {
                    match index {
                        0 => settings.borrow_mut().gate = switch.is_active(),
                        1 => settings.borrow_mut().comp = switch.is_active(),
                        _ => settings.borrow_mut().mono = switch.is_active(),
                    }
                    let value = settings.borrow().clone();
                    callback(value);
                }
            });
            switches.push((title, switch));
        }
        let gate = switches[0].1.clone();
        let comp = switches[1].1.clone();
        let mono = switches[2].1.clone();
        let mut scales = Vec::new();
        for (index, title, low, high, step) in [
            (0, "Gate dB", -70.0, -20.0, 1.0),
            (1, "Comp dB", -30.0, 0.0, 1.0),
            (2, "Ratio", 1.0, 10.0, 0.5),
            (3, "Low dB", -12.0, 12.0, 1.0),
            (4, "Mid dB", -12.0, 12.0, 1.0),
            (5, "High dB", -12.0, 12.0, 1.0),
        ] {
            if index == 0 {
                fx_row(&body, "Gate", &gate);
            }
            if index == 1 {
                fx_row(&body, "Comp", &comp);
            }
            let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, low, high, step);
            scale.set_digits(if index == 2 { 1 } else { 0 });
            scale.set_hexpand(true);
            scale.set_size_request(160, -1);
            if index >= 3 {
                scale.add_mark(0.0, gtk::PositionType::Bottom, None);
            }
            fx_row(&body, title, &scale);
            let (guard, settings, callback) = (updating.clone(), value.clone(), changed.clone());
            scale.connect_value_changed(move |scale| {
                if !guard.get() {
                    let value = {
                        let mut settings = settings.borrow_mut();
                        match index {
                            0 => settings.gate_thresh = scale.value(),
                            1 => settings.comp_thresh = scale.value(),
                            2 => settings.comp_ratio = scale.value(),
                            3 => settings.eq_low = scale.value(),
                            4 => settings.eq_mid = scale.value(),
                            _ => settings.eq_high = scale.value(),
                        }
                        settings.clone()
                    };
                    callback(value);
                }
            });
            scales.push(scale);
        }
        let gate_scale = scales[0].downgrade();
        gate.connect_active_notify(move |switch| {
            if let Some(scale) = gate_scale.upgrade() {
                scale.set_sensitive(switch.is_active());
            }
        });
        let comp_scales = [scales[1].downgrade(), scales[2].downgrade()];
        comp.connect_active_notify(move |switch| {
            for scale in &comp_scales {
                if let Some(scale) = scale.upgrade() {
                    scale.set_sensitive(switch.is_active());
                }
            }
        });
        let delay = gtk::SpinButton::with_range(0.0, 500.0, 5.0);
        fx_row(&body, "Delay ms", &delay);
        fx_row(&body, "Mono", &mono);
        let (guard, settings) = (updating.clone(), value.clone());
        delay.connect_value_changed(move |delay| {
            if !guard.get() {
                settings.borrow_mut().delay_ms = delay.value();
                let value = settings.borrow().clone();
                changed(value);
            }
        });
        Self {
            popover,
            value,
            updating,
            lowcut,
            gate,
            comp,
            mono,
            scales,
            delay,
        }
    }

    pub(crate) fn render(&self, settings: FxSettings) {
        self.updating.set(true);
        self.lowcut.set_selected(match settings.lowcut {
            80 => 1,
            120 => 2,
            _ => 0,
        });
        self.gate.set_active(settings.gate);
        self.comp.set_active(settings.comp);
        self.mono.set_active(settings.mono);
        for (scale, value) in self.scales.iter().zip([
            settings.gate_thresh,
            settings.comp_thresh,
            settings.comp_ratio,
            settings.eq_low,
            settings.eq_mid,
            settings.eq_high,
        ]) {
            if scale.value() != value {
                scale.set_value(value);
            }
        }
        self.scales[0].set_sensitive(settings.gate);
        self.scales[1].set_sensitive(settings.comp);
        self.scales[2].set_sensitive(settings.comp);
        // set_value rewrites editable text even when the number is unchanged.
        if self.delay.value() != settings.delay_ms {
            self.delay.set_value(settings.delay_ms);
        }
        *self.value.borrow_mut() = settings;
        self.updating.set(false);
    }
}

fn fx_row(body: &gtk::Box, title: &str, control: &impl IsA<gtk::Widget>) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let label = gtk::Label::builder()
        .label(title)
        .xalign(0.0)
        .ellipsize(gtk::pango::EllipsizeMode::End)
        .width_chars(9)
        .max_width_chars(9)
        .build();
    row.append(&label);
    row.append(control);
    body.append(&row);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires the isolated installed GTK test runner"]
    fn delay_text_survives_unrelated_fx_refresh() {
        adw::init().expect("private GTK display");
        let changes = Rc::new(RefCell::new(Vec::new()));
        let captured = changes.clone();
        let fx = FxControls::new(move |settings| captured.borrow_mut().push(settings), || {});
        let settings = FxSettings::default();
        fx.render(settings.clone());
        fx.delay.set_text("125");
        fx.render(settings);
        assert_eq!(fx.delay.text(), "125");
        assert!(changes.borrow().is_empty(), "render must not echo changes");
        fx.delay.update();
        assert!(matches!(
            changes.borrow().as_slice(),
            [settings] if settings.delay_ms == 125.0
        ));
    }
}
