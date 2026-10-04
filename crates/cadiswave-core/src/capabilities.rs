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
        clipguard: false,
        hardware_low_cut: false,
        led: false,
        persistence: false,
        monitor_mix: profile.has_monitor_mix(),
        phantom: profile.has_phantom(),
        low_impedance: profile.has_low_z(),
    }
}
