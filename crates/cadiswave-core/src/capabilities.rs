//! Expose only controls with mapped profile operations.
use crate::profiles::ProfileId;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlCapabilities {
    pub clipguard: bool,
    pub hardware_low_cut: bool,
    pub led: bool,
    pub persistence: bool,
    pub monitor_mix: bool,
    pub phantom: bool,
    pub low_impedance: bool,
}
pub fn for_profile(profile: ProfileId) -> ControlCapabilities {
    let profile = profile.profile();
    ControlCapabilities {
        clipguard: profile.has_clipguard(),
        hardware_low_cut: profile.has_hardware_low_cut(),
        led: false,
        persistence: false,
        monitor_mix: profile.has_monitor_mix(),
        phantom: profile.has_phantom(),
        low_impedance: profile.has_low_z(),
    }
}

/// Wave:3 Clipguard requires its reviewed API 5.3 or 5.4 layout.
pub fn clipguard_available(profile: ProfileId, api: &str) -> bool {
    match profile {
        ProfileId::Wave3 => matches!(api, "5.3" | "5.4"),
        ProfileId::XlrDockMk2 => true,
        ProfileId::WaveXlr | ProfileId::WaveXlrMk2 => false,
    }
}
