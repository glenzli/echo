//! Gain keyframes in the prepared source's time coordinate. Splitting, moving
//! or trimming a clip retains the same curve; interpolation is linear in dB.
use super::SoundAssemblyError;
use serde::{Deserialize, Serialize};

mod remap;

pub const MAX_GAIN_ENVELOPE_POINTS: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GainEnvelopePoint {
    pub source_millis: u64,
    pub gain_centibels: i16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GainEnvelope {
    pub enabled: bool,
    pub points: Vec<GainEnvelopePoint>,
}

impl GainEnvelope {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.enabled && self.points.is_empty()
    }

    /// Validates persisted points even when the curve is bypassed.
    /// # Errors
    /// Rejects oversized, unordered, duplicate or out-of-range keyframes.
    pub fn validate(&self) -> Result<(), SoundAssemblyError> {
        if self.points.len() > MAX_GAIN_ENVELOPE_POINTS
            || self.points.iter().any(|p| {
                !(-9600..=1200).contains(&p.gain_centibels) || p.source_millis > u64::MAX / 48_000
            })
            || self
                .points
                .windows(2)
                .any(|p| p[0].source_millis >= p[1].source_millis)
        {
            return Err(SoundAssemblyError::InvalidGainEnvelope);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
