use super::projection::DeviceProjection;
use cadiswave_core::{model::*, protocol::KnobMode};
use cadiswave_runtime::controller::{AppCommand, EditTiming, RuntimeHandle};
#[derive(Clone, Debug, PartialEq)]
pub enum ControlError {
    Unavailable,
    UnsupportedMode,
    InvalidValue,
    Runtime(String),
}
#[derive(Clone)]
pub struct DeviceControls {
    handle: RuntimeHandle,
}
pub fn setting_for(
    projection: &DeviceProjection,
    position: f64,
) -> std::result::Result<DeviceSetting, ControlError> {
    if !position.is_finite() || !(0.0..=1.0).contains(&position) {
        return Err(ControlError::InvalidValue);
    }
    let unit = projection.unit.ok_or(ControlError::Unavailable)?;
    let state = projection.state.as_ref().ok_or(ControlError::Unavailable)?;
    if !projection.writable {
        return Err(ControlError::Unavailable);
    }
    let profile = unit.profile.profile();
    Ok(match state.knob_mode {
        KnobMode::Gain => {
            let step = f64::from(profile.gain_scale) * 0.5;
            DeviceSetting::GainRaw(
                ((position * f64::from(profile.gain_max) / step).round() * step)
                    .min(f64::from(profile.gain_max)) as u16,
            )
        }
        KnobMode::Headphones => {
            DeviceSetting::HeadphoneDb((profile.hp_min_db() * (1.0 - position) * 2.0).round() / 2.0)
        }
        KnobMode::MonitorMix if profile.has_monitor_mix() => {
            DeviceSetting::MonitorMix((position * f64::from(profile.mix_max)).round() as u16)
        }
        _ => return Err(ControlError::UnsupportedMode),
    })
}
impl DeviceControls {
    pub fn new(handle: RuntimeHandle) -> Self {
        Self { handle }
    }
    pub fn snapshot(&self) -> std::sync::Arc<AppSnapshot> {
        self.handle.snapshot()
    }
    pub fn submit(&self, command: AppCommand) -> std::result::Result<CommandId, ControlError> {
        self.handle
            .submit(command)
            .map_err(|error| ControlError::Runtime(error.to_string()))
    }
    fn current(
        &self,
        projection: &DeviceProjection,
    ) -> std::result::Result<DeviceProjection, ControlError> {
        let current = DeviceProjection::from_snapshot(&self.handle.snapshot());
        if !projection.writable || !current.writable || current.unit != projection.unit {
            return Err(ControlError::Unavailable);
        }
        Ok(current)
    }
    pub fn edit(
        &self,
        projection: &DeviceProjection,
        value: f64,
        timing: EditTiming,
    ) -> std::result::Result<CommandId, ControlError> {
        let setting = setting_for(projection, value)?;
        let current = self.current(projection)?;
        if current.state.as_ref().map(|s| s.knob_mode)
            != projection.state.as_ref().map(|s| s.knob_mode)
        {
            return Err(ControlError::Unavailable);
        }
        self.submit(AppCommand::SetDeviceSetting {
            unit: current.unit.ok_or(ControlError::Unavailable)?,
            setting,
            timing,
        })
    }
    pub fn set(
        &self,
        projection: &DeviceProjection,
        setting: DeviceSetting,
        timing: EditTiming,
    ) -> std::result::Result<CommandId, ControlError> {
        let current = self.current(projection)?;
        self.submit(AppCommand::SetDeviceSetting {
            unit: current.unit.ok_or(ControlError::Unavailable)?,
            setting,
            timing,
        })
    }
    pub fn toggle_mute(
        &self,
        projection: &DeviceProjection,
    ) -> std::result::Result<CommandId, ControlError> {
        let current = self.current(projection)?;
        self.submit(AppCommand::ToggleDeviceMute {
            unit: current.unit.ok_or(ControlError::Unavailable)?,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::test_support::{Rig, unit};
    use cadiswave_core::protocol::KnobMode;
    use serde_json::json;
    #[test]
    fn nonfinite_and_unmapped_modes_never_produce_a_setting() {
        let rig = Rig::new(json!({}), vec![unit("A", 2, -10.0)]);
        let projection = DeviceProjection::from_snapshot(&rig.snapshot());
        let controls = DeviceControls::new(rig.handle());
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 2.0] {
            assert_eq!(
                controls.edit(&projection, value, EditTiming::Immediate),
                Err(ControlError::InvalidValue)
            );
        }
        let mut unknown = projection.clone();
        unknown.state.as_mut().unwrap().knob_mode = KnobMode::None;
        assert_eq!(
            setting_for(&unknown, 0.5),
            Err(ControlError::UnsupportedMode)
        );
        assert_eq!(rig.device_command_count(), 0);
    }
    #[test]
    fn disconnect_cancels_the_final_gesture_value() {
        let a = unit("A", 2, -10.0);
        let rig = Rig::new(json!({}), vec![a.clone()]);
        let controls = DeviceControls::new(rig.handle());
        let projection = DeviceProjection::from_snapshot(&rig.snapshot());
        rig.retire(a.id);
        assert_eq!(
            controls.edit(&projection, 0.5, EditTiming::Immediate),
            Err(ControlError::Unavailable)
        );
        assert_eq!(
            controls.toggle_mute(&projection),
            Err(ControlError::Unavailable)
        );
        assert_eq!(rig.device_command_count(), 0);
    }
    #[test]
    fn failed_completion_keeps_the_observed_value() {
        let rig = Rig::new(json!({}), vec![unit("A", 2, -10.0)]);
        let controls = DeviceControls::new(rig.handle());
        let projection = DeviceProjection::from_snapshot(&rig.snapshot());
        rig.fail_next_device_command();
        let id = controls
            .edit(&projection, 0.5, EditTiming::Immediate)
            .unwrap();
        assert!(!matches!(rig.outcome(id), CommandOutcome::Applied { .. }));
        assert_eq!(rig.device_states()[0].state.known().unwrap().gain_raw, 0);
    }
    #[test]
    fn selection_change_cancels_unsent_gesture_work() {
        let a = unit("A", 2, -10.0);
        let b = unit("B", 3, -20.0);
        let rig = Rig::new(json!({}), vec![a.clone(), b.clone()]);
        let controls = DeviceControls::new(rig.handle());
        let first = DeviceProjection::from_snapshot(&rig.snapshot());
        assert_eq!(first.unit, Some(a.id));
        let id = controls.edit(&first, 0.4, EditTiming::Immediate).unwrap();
        rig.track(id);
        rig.finish_submissions();
        (rig.submitter())(AppCommand::SelectUnit { unit: Some(b.id) });
        rig.finish_submissions();
        assert_eq!(
            controls.edit(&first, 0.8, EditTiming::Immediate),
            Err(ControlError::Unavailable)
        );
        assert_eq!(rig.device_states()[0].state.known().unwrap().gain_raw, 7680);
        assert_eq!(rig.device_states()[1].state.known().unwrap().gain_raw, 0);
        assert_eq!(rig.device_command_count(), 1);
    }
}
