//! Keep mouse edits bound to the device captured at press time.
use super::{controls::DeviceControls, projection::DeviceProjection};
use adw::prelude::*;
use cadiswave_core::model::DeviceSetting;
use cadiswave_runtime::controller::EditTiming;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Clone, Copy)]
pub enum ScaleKind {
    Dial,
    Headphones,
}
pub struct ScaleInteraction {
    captured: Rc<RefCell<Option<DeviceProjection>>>,
    kind: ScaleKind,
}
fn submit(
    controls: &DeviceControls,
    p: &DeviceProjection,
    kind: ScaleKind,
    value: f64,
    timing: EditTiming,
) {
    match kind {
        ScaleKind::Dial => {
            let _ = controls.edit(p, value, timing);
        }
        ScaleKind::Headphones => {
            let _ = controls.set(p, DeviceSetting::HeadphoneDb(value), timing);
        }
    }
}
impl ScaleInteraction {
    pub fn bind(
        scale: &gtk::Scale,
        controls: DeviceControls,
        projection: Rc<RefCell<DeviceProjection>>,
        updating: Rc<Cell<bool>>,
        kind: ScaleKind,
    ) -> Self {
        let captured = Rc::new(RefCell::new(None));
        let (c, p, g, active) = (
            controls.clone(),
            projection.clone(),
            updating,
            captured.clone(),
        );
        scale.connect_value_changed(move |scale| {
            if !g.get() {
                let projection = active
                    .borrow()
                    .clone()
                    .unwrap_or_else(|| p.borrow().clone());
                submit(&c, &projection, kind, scale.value(), EditTiming::Debounced);
            }
        });
        let keyboard = gtk::EventControllerKey::new();
        keyboard.set_propagation_phase(gtk::PropagationPhase::Capture);
        let key_owned = Rc::new(Cell::new(false));
        let owned = key_owned.clone();
        let active = captured.clone();
        let p = projection.clone();
        keyboard.connect_key_pressed(move |_, key, _, _| {
            if matches!(
                key,
                gtk::gdk::Key::Left
                    | gtk::gdk::Key::Right
                    | gtk::gdk::Key::Up
                    | gtk::gdk::Key::Down
            ) && active.borrow().is_none()
            {
                *active.borrow_mut() = Some(p.borrow().clone());
                owned.set(true);
            }
            glib::Propagation::Proceed
        });
        let active = captured.clone();
        let c = controls.clone();
        let weak = scale.downgrade();
        let owned = key_owned;
        keyboard.connect_key_released(move |_, key, _, _| {
            if matches!(
                key,
                gtk::gdk::Key::Left
                    | gtk::gdk::Key::Right
                    | gtk::gdk::Key::Up
                    | gtk::gdk::Key::Down
            ) && owned.replace(false)
                && let (Some(p), Some(scale)) = (active.borrow_mut().take(), weak.upgrade())
            {
                submit(&c, &p, kind, scale.value(), EditTiming::Immediate);
            }
        });
        scale.add_controller(keyboard);
        let controller = gtk::EventControllerLegacy::new();
        controller.set_propagation_phase(gtk::PropagationPhase::Capture);
        let active = captured.clone();
        let weak = scale.downgrade();
        controller.connect_event(move |_, event| {
            use gtk::gdk::EventType;
            match event.event_type() {
                EventType::ButtonPress | EventType::TouchBegin => {
                    *active.borrow_mut() = Some(projection.borrow().clone())
                }
                EventType::ButtonRelease | EventType::TouchEnd => {
                    if let Some(p) = active.borrow_mut().take()
                        && let Some(scale) = weak.upgrade()
                    {
                        submit(&controls, &p, kind, scale.value(), EditTiming::Immediate);
                    }
                }
                EventType::TouchCancel => {
                    active.borrow_mut().take();
                }
                _ => {}
            }
            glib::Propagation::Proceed
        });
        scale.add_controller(controller);
        Self { captured, kind }
    }
    pub fn can_render_value(&self, p: &DeviceProjection) -> bool {
        self.captured.borrow().as_ref().is_none_or(|old| {
            old.unit != p.unit
                || !p.writable
                || matches!(self.kind, ScaleKind::Dial)
                    && old.state.as_ref().map(|s| s.knob_mode)
                        != p.state.as_ref().map(|s| s.knob_mode)
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::test_support::{Rig, unit};
    use cadiswave_runtime::controller::AppCommand;
    #[test]
    #[ignore = "Requires the isolated GTK runner"]
    fn active_mouse_edit_cannot_move_to_a_replacement_selection() {
        gtk::init().unwrap();
        let a = unit("A", 2, -10.0);
        let b = unit("B", 3, -12.0);
        let rig = Rig::new(serde_json::json!({}), vec![a, b.clone()]);
        let controls = DeviceControls::new(rig.handle());
        let projection = Rc::new(RefCell::new(DeviceProjection::from_snapshot(
            &rig.snapshot(),
        )));
        let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 0.005);
        let interaction = ScaleInteraction::bind(
            &scale,
            controls,
            projection.clone(),
            Rc::new(Cell::new(false)),
            ScaleKind::Dial,
        );
        *interaction.captured.borrow_mut() = Some(projection.borrow().clone());
        assert!(!interaction.can_render_value(&projection.borrow()));
        (rig.submitter())(AppCommand::SelectUnit { unit: Some(b.id) });
        rig.finish_submissions();
        *projection.borrow_mut() = DeviceProjection::from_snapshot(&rig.snapshot());
        scale.set_value(0.7);
        std::thread::sleep(std::time::Duration::from_millis(150));
        assert_eq!(rig.device_command_count(), 0);
    }
}
