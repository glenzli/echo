//! Authored Space routing and immutable impulse-response selection.
//!
//! Algorithmic and convolution processing are mutually exclusive views of the
//! single Space insert. Runtime paths and prepared sample buffers are never
//! persisted here; they are resolved from Catalog evidence at the consumer.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ContentHash;

pub const MAX_CONVOLUTION_WET_GAIN_CENTIBELS: i16 = 1_200;
pub const MIN_CONVOLUTION_WET_GAIN_CENTIBELS: i16 = -2_400;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum SpaceMode {
    #[default]
    Algorithmic = 0,
    Convolution = 1,
}

impl SpaceMode {
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        self as u8
    }

    /// Restores the stable desktop representation.
    ///
    /// # Errors
    ///
    /// Returns an error for unknown values.
    pub const fn from_wire_value(value: u8) -> Result<Self, SpaceModeValueError> {
        match value {
            0 => Ok(Self::Algorithmic),
            1 => Ok(Self::Convolution),
            _ => Err(SpaceModeValueError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpaceModeValueError;

impl std::fmt::Display for SpaceModeValueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("space mode must be algorithmic or convolution")
    }
}

impl std::error::Error for SpaceModeValueError {}

/// Stable identity of one prepared, rights-tracked local impulse response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImpulseResponseSelection {
    pub import_id: Uuid,
    pub source_hash: ContentHash,
    pub prepared_hash: ContentHash,
}

/// Authored mode and convolution controls for the single Space node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceSettings {
    #[serde(default)]
    pub mode: SpaceMode,
    #[serde(default)]
    pub impulse_response: Option<ImpulseResponseSelection>,
    #[serde(default = "default_mix_percent")]
    pub convolution_mix_percent: u8,
    #[serde(default)]
    pub convolution_wet_gain_centibels: i16,
}

const fn default_mix_percent() -> u8 {
    35
}

impl Default for SpaceSettings {
    fn default() -> Self {
        Self {
            mode: SpaceMode::Algorithmic,
            impulse_response: None,
            convolution_mix_percent: default_mix_percent(),
            convolution_wet_gain_centibels: 0,
        }
    }
}

impl SpaceSettings {
    #[must_use]
    pub const fn algorithmic() -> Self {
        Self {
            mode: SpaceMode::Algorithmic,
            impulse_response: None,
            convolution_mix_percent: 35,
            convolution_wet_gain_centibels: 0,
        }
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.convolution_mix_percent <= 100
            && self.convolution_wet_gain_centibels >= MIN_CONVOLUTION_WET_GAIN_CENTIBELS
            && self.convolution_wet_gain_centibels <= MAX_CONVOLUTION_WET_GAIN_CENTIBELS
            && (matches!(self.mode, SpaceMode::Algorithmic) || self.impulse_response.is_some())
    }
}

#[cfg(test)]
mod tests;
