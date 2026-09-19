//! Versioned, Original-bound noise evidence and explicit processing intent.

use serde::{Deserialize, Serialize};

use super::SpectralRepairError;

/// Version 1: 1025 bins of mean 2048-point Hann STFT power at 48 kHz,
/// normalized by (4 / 2048)^2. Integer units are hundredths of a decibel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoiseProfileSettings {
    pub algorithm_version: u16,
    pub enabled: bool,
    pub capture_start_millis: u64,
    pub capture_end_millis: u64,
    pub power_centibels: Vec<i16>,
    pub reduction_centibels: i16,
    pub sensitivity_centibels: i16,
    pub smoothing_bins: u16,
}

impl NoiseProfileSettings {
    /// Rejects malformed evidence even while bypassed, so re-enabling is safe.
    ///
    /// # Errors
    /// Returns `InvalidNoiseProfile` for unsupported versions or unbounded values.
    pub fn validate(&self, duration_millis: u64) -> Result<(), SpectralRepairError> {
        if self.algorithm_version != 1
            || self.capture_start_millis >= self.capture_end_millis
            || self.capture_end_millis > duration_millis
            || !(100..=30_000).contains(&(self.capture_end_millis - self.capture_start_millis))
            || self.power_centibels.len() != 1025
            || self
                .power_centibels
                .iter()
                .any(|value| !(-14_400..=1200).contains(value))
            || !(0..=3600).contains(&self.reduction_centibels)
            || !(0..=1200).contains(&self.sensitivity_centibels)
            || self.smoothing_bins > 8
        {
            return Err(SpectralRepairError::InvalidNoiseProfile);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
