//! Validate microphone tests and measure bounded signed-16 PCM.
use crate::{
    calibration::RATE,
    effects::{capture_channels, validate_raw_node},
    model::{NodeIdentity, OperationError, Result, SourceId},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MicTestToken {
    pub session: u64,
    pub source: SourceId,
    pub node_name: String,
    pub identity: NodeIdentity,
    pub channels: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackTarget {
    pub node_name: String,
    pub identity: NodeIdentity,
}
#[derive(Debug, Clone, PartialEq)]
pub struct MicTestMetrics {
    pub frames: u64,
    pub peak_db: f64,
    pub clipped_samples: u64,
    pub clipping: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MicTestPhase {
    Recording,
    Ready,
    Playing,
    Failed(String),
}
#[derive(Debug, Clone, PartialEq)]
pub struct MicTestSnapshot {
    pub token: MicTestToken,
    pub seconds: u32,
    pub phase: MicTestPhase,
    pub metrics: Option<MicTestMetrics>,
    pub output: Option<PlaybackTarget>,
}

pub fn validate_duration(seconds: u32) -> Result<()> {
    if !(1..=10).contains(&seconds) {
        return Err(OperationError::invalid(
            "Record between one and ten seconds",
        ));
    }
    Ok(())
}

pub fn validate_capture(node: &str, seconds: u32, channels: u32) -> Result<()> {
    validate_duration(seconds)?;
    validate_raw_node(node)?;
    if matches!(node, "auto" | "0" | "-1")
        || node.starts_with("cadiswave_")
        || node.ends_with(".monitor")
    {
        return Err(OperationError::invalid("Select an explicit raw microphone"));
    }
    capture_channels(Some(channels))?;
    Ok(())
}

/// Measure every channel. Report clipping without discarding the recording.
pub fn measure(raw: &[u8], channels: u32, seconds: u32) -> Result<MicTestMetrics> {
    validate_duration(seconds)?;
    let channels = capture_channels(Some(channels))? as usize;
    let frames = RATE as usize * seconds as usize;
    if raw.len() != frames * channels * 2 {
        return Err(OperationError::invalid(
            "Microphone test audio has an incorrect frame count",
        ));
    }
    let mut peak = 0_i32;
    let mut clipped_samples = 0_u64;
    for bytes in raw.as_chunks::<2>().0 {
        let sample = i32::from(i16::from_le_bytes([bytes[0], bytes[1]])).abs();
        peak = peak.max(sample);
        clipped_samples += u64::from(sample >= 32760);
    }
    Ok(MicTestMetrics {
        frames: frames as u64,
        peak_db: 20.0 * (f64::from(peak) / 32768.0).max(1e-7).log10(),
        clipped_samples,
        clipping: clipped_samples != 0,
    })
}
