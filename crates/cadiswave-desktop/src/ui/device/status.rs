//! Translate only confirmed service states.
pub fn service_key(status: &str) -> &'static str {
    match status {
        "Capture service running" => "service-running",
        "Capture service stopped" => "service-stopped",
        _ => "service-unavailable",
    }
}
