use super::{controls::DeviceControls, projection::DeviceProjection};
use adw::prelude::*;
use cadiswave_core::protocol::KnobMode;
use cadiswave_runtime::controller::EditTiming;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};
pub fn interpolate(current: f64, target: f64, elapsed: Duration, animations: bool) -> f64 {
    if !animations || (target - current).abs() < 0.0001 {
        return target;
    }
    current + (target - current) * (1.0 - (-elapsed.as_secs_f64() / 0.07).exp())
}
pub fn position(projection: &DeviceProjection) -> Option<f64> {
    let state = projection.state.as_ref()?;
    let profile = projection.unit?.profile.profile();
    Some(
        match state.knob_mode {
            KnobMode::Gain => f64::from(state.gain_raw) / f64::from(profile.gain_max),
            KnobMode::Headphones => 1.0 - state.hp_volume_db / profile.hp_min_db(),
            KnobMode::MonitorMix => f64::from(state.monitor_mix?) / f64::from(profile.mix_max),
            KnobMode::None => return None,
        }
        .clamp(0.0, 1.0),
    )
}
pub struct Knob {
    pub widget: gtk::Box,
    area: gtk::DrawingArea,
    pub scale: gtk::Scale,
    projection: Rc<RefCell<DeviceProjection>>,
    value: Rc<Cell<f64>>,
    target: Rc<Cell<f64>>,
    ticking: Rc<Cell<bool>>,
    updating: Rc<Cell<bool>>,
    interaction: super::interaction::ScaleInteraction,
    drag: Rc<RefCell<Option<DeviceProjection>>>,
}
impl Knob {
    pub fn add_compact_setters(&self, breakpoint: &adw::Breakpoint) {
        for property in ["content-width", "content-height"] {
            breakpoint.add_setter(&self.area, property, Some(&180_i32.to_value()));
        }
    }
    pub fn new(controls: DeviceControls) -> Self {
        let widget = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let area = gtk::DrawingArea::builder()
            .content_width(240)
            .content_height(240)
            .hexpand(true)
            .build();
        let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 0.005);
        scale.set_draw_value(false);
        scale.set_tooltip_text(Some(&crate::i18n::tr("dial-control")));
        let value = Rc::new(Cell::new(0.0));
        let target = Rc::new(Cell::new(0.0));
        let draw = value.clone();
        area.set_draw_func(move |_, cr, w, h| {
            let size = f64::from(w.min(h));
            let radius = size * 0.375;
            let cx = f64::from(w) / 2.0;
            let cy = f64::from(h) / 2.0;
            cr.set_line_width(3.0);
            cr.set_line_cap(gtk::cairo::LineCap::Round);
            for segment in 0..25 {
                let angle = (-225.0 + f64::from(segment) * 270.0 / 24.0).to_radians();
                if f64::from(segment) / 24.0 <= draw.get() {
                    cr.set_source_rgb(0.9, 0.96, 0.92);
                } else {
                    cr.set_source_rgb(0.21, 0.23, 0.22);
                }
                cr.move_to(
                    cx + angle.cos() * (radius + 14.0),
                    cy + angle.sin() * (radius + 14.0),
                );
                cr.line_to(
                    cx + angle.cos() * (radius + 23.0),
                    cy + angle.sin() * (radius + 23.0),
                );
                let _ = cr.stroke();
            }
            let gradient = gtk::cairo::RadialGradient::new(
                cx - radius * 0.3,
                cy - radius * 0.4,
                0.0,
                cx,
                cy,
                radius,
            );
            gradient.add_color_stop_rgb(0.0, 0.22, 0.24, 0.23);
            gradient.add_color_stop_rgb(1.0, 0.055, 0.06, 0.055);
            let _ = cr.set_source(&gradient);
            cr.arc(cx, cy, radius, 0.0, std::f64::consts::TAU);
            let _ = cr.fill();
            cr.set_source_rgb(0.3, 0.32, 0.3);
            cr.set_line_width(1.0);
            cr.arc(cx, cy, radius, 0.0, std::f64::consts::TAU);
            let _ = cr.stroke();
            let angle = (-225.0 + draw.get() * 270.0).to_radians();
            cr.set_source_rgb(0.49, 1.0, 0.61);
            cr.set_line_width(4.0);
            cr.move_to(
                cx + angle.cos() * (radius - 16.0),
                cy + angle.sin() * (radius - 16.0),
            );
            cr.line_to(
                cx + angle.cos() * (radius - 6.0),
                cy + angle.sin() * (radius - 6.0),
            );
            let _ = cr.stroke();
        });
        let projection = Rc::new(RefCell::new(DeviceProjection::from_snapshot(
            &controls.snapshot(),
        )));
        let updating = Rc::new(Cell::new(false));
        let interaction = super::interaction::ScaleInteraction::bind(
            &scale,
            controls.clone(),
            projection.clone(),
            updating.clone(),
            super::interaction::ScaleKind::Dial,
        );
        let preview = value.clone();
        let guard = updating.clone();
        let weak = area.downgrade();
        scale.connect_value_changed(move |scale| {
            if !guard.get() {
                preview.set(scale.value());
                if let Some(area) = weak.upgrade() {
                    area.queue_draw();
                }
            }
        });
        let gesture = gtk::GestureDrag::new();
        let captured = Rc::new(RefCell::new(None));
        let capture = captured.clone();
        let p = projection.clone();
        let start = Rc::new(Cell::new(0.0));
        let s = start.clone();
        gesture.connect_drag_begin(move |_, _, _| {
            s.set(position(&p.borrow()).unwrap_or(0.0));
            *capture.borrow_mut() = Some(p.borrow().clone());
        });
        let capture = captured.clone();
        let s = start.clone();
        let c = controls.clone();
        let weak = scale.downgrade();
        let guard = updating.clone();
        let preview = value.clone();
        let weak_area = area.downgrade();
        gesture.connect_drag_update(move |_, x, y| {
            if let Some(p) = capture.borrow().as_ref() {
                let pos = (s.get() + (x - y) / 240.0).clamp(0.0, 1.0);
                if c.edit(p, pos, EditTiming::Debounced).is_ok()
                    && let Some(scale) = weak.upgrade()
                {
                    preview.set(pos);
                    if let Some(area) = weak_area.upgrade() {
                        area.queue_draw();
                    }
                    guard.set(true);
                    scale.set_value(pos);
                    guard.set(false);
                }
            }
        });
        let capture = captured.clone();
        let s = start;
        let controls_for_cancel = controls.clone();
        let c = controls;
        gesture.connect_drag_end(move |_, x, y| {
            if let Some(p) = capture.borrow_mut().take() {
                let _ = c.edit(
                    &p,
                    (s.get() + (x - y) / 240.0).clamp(0.0, 1.0),
                    EditTiming::Immediate,
                );
            }
        });
        let capture = captured.clone();
        let c = controls_for_cancel;
        let preview = value.clone();
        let weak = area.downgrade();
        gesture.connect_cancel(move |_, _| {
            capture.borrow_mut().take();
            let p = DeviceProjection::from_snapshot(&c.snapshot());
            preview.set(position(&p).unwrap_or(0.0));
            if let Some(area) = weak.upgrade() {
                area.queue_draw();
            }
        });
        area.add_controller(gesture);
        widget.append(&area);
        widget.append(&scale);
        Self {
            widget,
            area,
            scale,
            projection,
            value,
            target,
            ticking: Rc::new(Cell::new(false)),
            updating,
            interaction,
            drag: captured,
        }
    }
    pub fn set_projection(&self, projection: &DeviceProjection) {
        *self.projection.borrow_mut() = projection.clone();
        let position = position(projection);
        self.updating.set(true);
        if let (Some(state), Some(unit)) = (projection.state.as_ref(), projection.unit) {
            let profile = unit.profile.profile();
            let step = match state.knob_mode {
                KnobMode::Gain => f64::from(profile.gain_scale) * 0.5 / f64::from(profile.gain_max),
                KnobMode::Headphones => 0.5 / -profile.hp_min_db(),
                KnobMode::MonitorMix => 1.0 / f64::from(profile.mix_max),
                _ => 0.005,
            };
            self.scale.set_increments(step, step * 10.0);
        }
        self.scale
            .set_sensitive(projection.writable && position.is_some());
        if self.interaction.can_render_value(projection) && self.drag.borrow().is_none() {
            self.scale.set_value(position.unwrap_or(0.0));
        }
        self.updating.set(false);
        if !self.interaction.can_render_value(projection) {
            return;
        }
        let next = position.unwrap_or(0.0);
        if self.drag.borrow().as_ref().is_some_and(|old| {
            old.unit == projection.unit
                && old.state.as_ref().map(|s| s.knob_mode)
                    == projection.state.as_ref().map(|s| s.knob_mode)
                && projection.writable
        }) {
            return;
        }
        if self.target.get() == next && (self.value.get() - next).abs() < 0.0001 {
            return;
        }
        self.target.set(next);
        if !self.area.is_mapped() {
            self.value.set(next);
            self.area.queue_draw();
            return;
        }
        if self.ticking.replace(true) {
            return;
        }
        let (value, target, ticking) = (
            self.value.clone(),
            self.target.clone(),
            self.ticking.clone(),
        );
        let previous = Cell::new(0_i64);
        self.area.add_tick_callback(move |area, clock| {
            let animations =
                gtk::Settings::default().is_none_or(|settings| settings.is_gtk_enable_animations());
            let now = clock.frame_time();
            let old = previous.replace(now);
            let elapsed = Duration::from_micros(if old == 0 {
                16_000
            } else {
                (now - old).max(0) as u64
            });
            value.set(interpolate(
                value.get(),
                target.get(),
                elapsed,
                animations && area.is_mapped(),
            ));
            area.queue_draw();
            if !area.is_mapped() || (value.get() - target.get()).abs() < 0.0001 {
                value.set(target.get());
                ticking.set(false);
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }
}
