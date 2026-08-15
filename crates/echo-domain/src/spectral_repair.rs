//! Original-first, non-destructive spectral attenuation intent.
//!
//! Regions describe repair in Original time and Hertz, never in display
//! pixels or cache-tile coordinates. The audio engine owns STFT execution;
//! this owner only preserves bounded user intent and its stable JSON shape.

use serde::{Deserialize, Serialize};

pub const MAX_SPECTRAL_REPAIR_REGIONS: usize = 64;
pub const MIN_SPECTRAL_FREQUENCY_HERTZ: u16 = 20;
pub const MAX_SPECTRAL_FREQUENCY_HERTZ: u16 = 24_000;
pub const MAX_SPECTRAL_ATTENUATION_CENTIBELS: i16 = 9_600;
pub const MAX_SPECTRAL_TIME_FEATHER_MILLIS: u16 = 250;
pub const MAX_SPECTRAL_FREQUENCY_FEATHER_HERTZ: u16 = 2_000;

/// One soft, rectangular source-time/frequency attenuation selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpectralAttenuationRegion {
    pub start_millis: u64,
    pub end_millis: u64,
    pub low_hertz: u16,
    pub high_hertz: u16,
    /// Positive attenuation depth: `0` is a no-op and `9600` is -96 dB.
    pub attenuation_centibels: i16,
    pub time_feather_millis: u16,
    pub frequency_feather_hertz: u16,
}

/// Asset-local spectral repair intent. It is deliberately separate from
/// effect masks and recipes because its regions name one Original's time.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpectralRepairSettings {
    /// Bypasses the complete Original-first spectral adjustment layer without
    /// discarding its authored regions.
    #[serde(default = "spectral_repair_enabled_by_default")]
    pub enabled: bool,
    #[serde(default)]
    pub regions: Vec<SpectralAttenuationRegion>,
}

const fn spectral_repair_enabled_by_default() -> bool {
    true
}

impl SpectralRepairSettings {
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            enabled: true,
            regions: Vec::new(),
        }
    }

    /// Returns whether this Original-first layer contributes to playback or
    /// export. Disabled layers retain their source-anchored intent for later
    /// re-enablement.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Validates a source-duration-bound spectral repair contract.
    pub fn validate(&self, source_duration_millis: u64) -> Result<(), SpectralRepairError> {
        if self.regions.len() > MAX_SPECTRAL_REPAIR_REGIONS {
            return Err(SpectralRepairError::TooManyRegions);
        }
        for region in &self.regions {
            if region.start_millis >= region.end_millis
                || region.end_millis > source_duration_millis
            {
                return Err(SpectralRepairError::InvalidTimeRange);
            }
            if region.low_hertz < MIN_SPECTRAL_FREQUENCY_HERTZ
                || region.low_hertz >= region.high_hertz
                || region.high_hertz > MAX_SPECTRAL_FREQUENCY_HERTZ
            {
                return Err(SpectralRepairError::InvalidFrequencyRange);
            }
            if !(0..=MAX_SPECTRAL_ATTENUATION_CENTIBELS).contains(&region.attenuation_centibels) {
                return Err(SpectralRepairError::InvalidAttenuation);
            }
            if region.time_feather_millis > MAX_SPECTRAL_TIME_FEATHER_MILLIS
                || region.frequency_feather_hertz > MAX_SPECTRAL_FREQUENCY_FEATHER_HERTZ
            {
                return Err(SpectralRepairError::InvalidFeather);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpectralRepairError {
    TooManyRegions,
    InvalidTimeRange,
    InvalidFrequencyRange,
    InvalidAttenuation,
    InvalidFeather,
}

impl std::fmt::Display for SpectralRepairError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyRegions => formatter.write_str("too many spectral repair regions"),
            Self::InvalidTimeRange => formatter.write_str("spectral repair time range is invalid"),
            Self::InvalidFrequencyRange => {
                formatter.write_str("spectral repair frequency range is invalid")
            }
            Self::InvalidAttenuation => {
                formatter.write_str("spectral repair attenuation is invalid")
            }
            Self::InvalidFeather => formatter.write_str("spectral repair feather is invalid"),
        }
    }
}

impl std::error::Error for SpectralRepairError {}

#[cfg(test)]
mod tests;
