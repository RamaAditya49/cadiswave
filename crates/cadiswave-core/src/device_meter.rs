//! Match measured capture channels to the selected device serial.
use crate::{
    model::*,
    pcm::ChannelPeaks,
    protocol::{ConnectedIdentity, device_for_capture},
};
pub fn selected_channels(snapshot: &AppSnapshot) -> Option<ChannelPeaks> {
    let selected = snapshot.selected_unit?;
    let units: Vec<_> = snapshot
        .units
        .iter()
        .map(|unit| ConnectedIdentity {
            unit: unit.id,
            serial: &unit.info.serial,
            connected: true,
        })
        .collect();
    let mut readings = Vec::new();
    for (id, source) in &snapshot.desired.sources {
        if source.kind != SourceKind::Device {
            continue;
        }
        let mut captures = snapshot
            .captures
            .iter()
            .filter(|capture| capture.node_name == source.node_name);
        let Some(capture) = captures.next() else {
            continue;
        };
        if captures.next().is_some() {
            return None;
        }
        let Some(serial) = capture
            .properties
            .get("device.serial")
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        if device_for_capture(serial, &units) == Some(selected) {
            if let Some(peaks) = snapshot.channel_meters.get(&format!("src:{id}")) {
                readings.push(*peaks);
            }
        }
    }
    if readings.len() == 1 {
        Some(readings[0])
    } else {
        None
    }
}
