use cadiswave_core::{model::*, protocol::DeviceState};
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceProjection {
    pub unit: Option<UnitId>,
    pub state: Option<DeviceState>,
    pub pending: Vec<DeviceSetting>,
    pub writable: bool,
}
impl DeviceProjection {
    pub fn from_snapshot(snapshot: &AppSnapshot) -> Self {
        let unit = snapshot
            .selected_unit
            .and_then(|id| snapshot.units.iter().find(|unit| unit.id == id));
        let state = unit.and_then(|unit| unit.state.known()).cloned();
        Self {
            unit: unit.map(|unit| unit.id),
            writable: state.is_some() && snapshot.lifecycle == Lifecycle::Running,
            pending: unit
                .and_then(|unit| snapshot.unit_intents.get(&unit.id))
                .cloned()
                .unwrap_or_default(),
            state,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::test_support::unit;
    use std::sync::Arc;
    #[test]
    fn missing_observation_never_enables_controls() {
        let projection = DeviceProjection::from_snapshot(&AppSnapshot::default());
        assert_eq!(projection.unit, None);
        assert_eq!(projection.state, None);
        assert!(!projection.writable);
    }
    #[test]
    fn selected_known_state_and_pending_intents_are_separate() {
        let observed = unit("A", 2, -10.0);
        let mut snapshot = AppSnapshot {
            selected_unit: Some(observed.id),
            units: Arc::new(vec![observed.clone()]),
            lifecycle: Lifecycle::Running,
            ..Default::default()
        };
        Arc::make_mut(&mut snapshot.unit_intents)
            .insert(observed.id, vec![DeviceSetting::Mute(true)]);
        let projection = DeviceProjection::from_snapshot(&snapshot);
        assert!(projection.writable);
        assert!(!projection.state.unwrap().muted);
        assert_eq!(projection.pending, vec![DeviceSetting::Mute(true)]);
        for lifecycle in [Lifecycle::Frozen, Lifecycle::Draining, Lifecycle::Stopped] {
            snapshot.lifecycle = lifecycle;
            assert!(!DeviceProjection::from_snapshot(&snapshot).writable);
        }
        snapshot.lifecycle = Lifecycle::Running;
        Arc::make_mut(&mut snapshot.units)[0].state =
            Observation::Unknown(OperationError::unavailable("fixture"));
        assert!(!DeviceProjection::from_snapshot(&snapshot).writable);
        snapshot.units = Arc::default();
        assert!(!DeviceProjection::from_snapshot(&snapshot).writable);
    }
}
