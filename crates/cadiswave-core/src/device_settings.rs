//! Resolve software settings through the selected device's exact capture identity.
use crate::{
    model::*,
    protocol::{ConnectedIdentity, device_for_capture},
};

pub fn monitor_setting(profile: crate::profiles::ProfileId, percent: f64) -> Result<DeviceSetting> {
    if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
        return Err(OperationError::invalid(
            "Monitor balance must be between 0 and 100 percent",
        ));
    }
    if !profile.profile().has_monitor_mix() {
        return Err(OperationError::new(
            ErrorCode::Unsupported,
            "Hardware monitor mix is unavailable for this device",
        ));
    }
    Ok(DeviceSetting::MonitorMix(
        (percent * f64::from(profile.profile().mix_max) / 100.0).round() as u16,
    ))
}

pub fn microphone_sources(snapshot: &AppSnapshot) -> Vec<SourceId> {
    let Some(selected) = snapshot.selected_unit else {
        return Vec::new();
    };
    let units: Vec<_> = snapshot
        .units
        .iter()
        .map(|unit| ConnectedIdentity {
            unit: unit.id,
            serial: &unit.info.serial,
            connected: unit.state.known().is_some(),
        })
        .collect();
    snapshot
        .desired
        .sources
        .iter()
        .filter_map(|(id, source)| {
            if source.kind != SourceKind::Device
                || source.node_name.starts_with("cadiswave_")
                || source.node_name.ends_with(".monitor")
            {
                return None;
            }
            let mut matches = snapshot
                .captures
                .iter()
                .filter(|capture| capture.node_name == source.node_name);
            let capture = matches.next()?;
            if matches.next().is_some() {
                return None;
            }
            let serial = capture.properties.get("device.serial")?.as_str()?;
            (device_for_capture(serial, &units) == Some(selected)).then(|| id.clone())
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureRate {
    pub hz: u32,
    pub negotiated: bool,
}

pub fn capture_rate(capture: &CaptureSnapshot) -> Option<CaptureRate> {
    for (key, negotiated) in [("cadiswave.observed-rate", true), ("audio.rate", false)] {
        if let Some(value) = capture.properties.get(key) {
            let hz = value
                .as_u64()
                .and_then(|value| u32::try_from(value).ok())
                .or_else(|| value.as_str().and_then(|value| value.parse::<u32>().ok()))
                .filter(|hz| (8_000..=384_000).contains(hz));
            if let Some(hz) = hz {
                return Some(CaptureRate { hz, negotiated });
            }
        }
    }
    None
}
