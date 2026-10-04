//! Decode measured signed little-endian PCM without mixing display channels.
use crate::model::{OperationError, Result};
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ChannelPeaks {
    Mono(f64),
    Stereo { left: f64, right: f64 },
}
impl ChannelPeaks {
    pub fn maximum(self) -> f64 {
        match self {
            Self::Mono(value) => value,
            Self::Stereo { left, right } => left.max(right),
        }
    }
    pub fn is_valid(self) -> bool {
        match self {
            Self::Mono(value) => value.is_finite() && (0.0..=1.0).contains(&value),
            Self::Stereo { left, right } => [left, right]
                .into_iter()
                .all(|value| value.is_finite() && (0.0..=1.0).contains(&value)),
        }
    }
    pub fn zero(channels: u32) -> Self {
        if channels == 2 {
            Self::Stereo {
                left: 0.0,
                right: 0.0,
            }
        } else {
            Self::Mono(0.0)
        }
    }
}
pub struct PcmPeakDecoder {
    channels: usize,
    frame: [u8; 4],
    used: usize,
    scalar: f64,
}
impl PcmPeakDecoder {
    pub fn new(channels: u32) -> Result<Self> {
        if !matches!(channels, 1 | 2) {
            return Err(OperationError::invalid(
                "PCM channel count must be one or two",
            ));
        }
        Ok(Self {
            channels: channels as usize,
            frame: [0; 4],
            used: 0,
            scalar: 0.0,
        })
    }
    pub fn scalar_peak(&self) -> f64 {
        self.scalar
    }
    pub fn push(&mut self, bytes: &[u8]) -> Option<ChannelPeaks> {
        let mut peaks = [0.0_f64; 2];
        let mut frames = 0;
        self.scalar = 0.0;
        for byte in bytes {
            self.frame[self.used] = *byte;
            self.used += 1;
            if self.used != self.channels * 2 {
                continue;
            }
            self.used = 0;
            frames += 1;
            let mut sum = 0.0;
            for (channel, peak) in peaks.iter_mut().enumerate().take(self.channels) {
                let sample = f64::from(i16::from_le_bytes([
                    self.frame[channel * 2],
                    self.frame[channel * 2 + 1],
                ])) / 32768.0;
                *peak = peak.max(sample.abs());
                sum += sample;
            }
            self.scalar = self.scalar.max((sum / self.channels as f64).abs());
        }
        if frames == 0 {
            None
        } else if self.channels == 1 {
            Some(ChannelPeaks::Mono(peaks[0]))
        } else {
            Some(ChannelPeaks::Stereo {
                left: peaks[0],
                right: peaks[1],
            })
        }
    }
}
