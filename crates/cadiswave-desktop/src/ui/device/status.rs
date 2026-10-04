use cadiswave_core::model::ServiceState;
pub fn service_key(state: ServiceState) -> &'static str {
    match state {
        ServiceState::Running => "service-running",
        ServiceState::Stopped => "service-stopped",
        _ => "service-unavailable",
    }
}
