//! Validate named effect presets before application persistence.
use crate::{
    effects::FxSettings,
    model::{OperationError, Result},
};
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};

const MAX_PRESETS: usize = 32;
const MAX_NAME_CHARS: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinPreset {
    Meeting,
    Podcast,
    Streaming,
}

impl BuiltinPreset {
    pub fn settings(self) -> FxSettings {
        let (lowcut, gate, gate_thresh, comp_thresh, comp_ratio, eq_low, eq_high) = match self {
            Self::Meeting => (120, true, -50.0, -20.0, 2.0, 0.0, 1.0),
            Self::Podcast => (80, false, -50.0, -18.0, 3.0, 1.0, 2.0),
            Self::Streaming => (80, true, -48.0, -16.0, 3.5, 0.0, 1.0),
        };
        FxSettings {
            lowcut,
            gate,
            gate_thresh,
            comp: true,
            comp_thresh,
            comp_ratio,
            eq_low,
            eq_high,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(transparent)]
pub struct VoicePresets(IndexMap<String, FxSettings>);

fn validate_name(name: &str) -> Result<()> {
    if name.trim() != name
        || name.is_empty()
        || name.chars().count() > MAX_NAME_CHARS
        || name.chars().any(char::is_control)
    {
        return Err(OperationError::invalid(
            "Preset names need 1–48 characters without outer spaces or control characters",
        ));
    }
    Ok(())
}

impl VoicePresets {
    pub fn save(&mut self, name: &str, settings: FxSettings) -> Result<()> {
        validate_name(name)?;
        let settings = settings.validated()?;
        if !self.0.contains_key(name) && self.0.len() >= MAX_PRESETS {
            return Err(OperationError::invalid("Keep at most 32 voice presets"));
        }
        self.0.insert(name.to_owned(), settings);
        Ok(())
    }
    pub fn delete(&mut self, name: &str) -> Result<()> {
        if self.0.shift_remove(name).is_none() {
            return Err(OperationError::invalid("Unknown voice preset"));
        }
        Ok(())
    }
    pub fn get(&self, name: &str) -> Option<&FxSettings> {
        self.0.get(name)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&String, &FxSettings)> {
        self.0.iter()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'de> Deserialize<'de> for VoicePresets {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let entries = IndexMap::<String, FxSettings>::deserialize(deserializer)?;
        let mut presets = Self::default();
        for (name, settings) in entries {
            presets
                .save(&name, settings)
                .map_err(serde::de::Error::custom)?;
        }
        Ok(presets)
    }
}
