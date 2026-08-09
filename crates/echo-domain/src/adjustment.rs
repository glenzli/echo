//! Non-destructive restoration intent for one immutable original.
//!
//! The graph stores authored time-domain bounds and simple amplitude
//! envelopes in stable integer units. Execution-specific sample positions are
//! prepared by the audio engine and are never persisted as user intent.

use serde::{Deserialize, Serialize};

/// Lowest supported output gain in hundredths of one decibel.
pub const MIN_GAIN_CENTIBELS: i16 = -2_400;
/// Highest supported output gain in hundredths of one decibel.
pub const MAX_GAIN_CENTIBELS: i16 = 1_200;

/// One validated, non-destructive adjustment graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdjustmentGraph {
    trim_start_millis: u64,
    trim_end_millis: u64,
    fade_in_millis: u64,
    fade_out_millis: u64,
    gain_centibels: i16,
}

impl AdjustmentGraph {
    /// Creates a graph bounded by the immutable source duration.
    ///
    /// # Errors
    ///
    /// Returns [`AdjustmentGraphError`] when the range is empty, extends past
    /// the source, the fades overlap, or gain is outside the product range.
    pub fn new(
        source_duration_millis: u64,
        trim_start_millis: u64,
        trim_end_millis: u64,
        fade_in_millis: u64,
        fade_out_millis: u64,
        gain_centibels: i16,
    ) -> Result<Self, AdjustmentGraphError> {
        if source_duration_millis == 0
            || trim_start_millis >= trim_end_millis
            || trim_end_millis > source_duration_millis
        {
            return Err(AdjustmentGraphError::InvalidTrimRange);
        }
        if fade_in_millis.saturating_add(fade_out_millis) > trim_end_millis - trim_start_millis {
            return Err(AdjustmentGraphError::OverlappingFades);
        }
        if !(MIN_GAIN_CENTIBELS..=MAX_GAIN_CENTIBELS).contains(&gain_centibels) {
            return Err(AdjustmentGraphError::GainOutOfRange);
        }
        Ok(Self {
            trim_start_millis,
            trim_end_millis,
            fade_in_millis,
            fade_out_millis,
            gain_centibels,
        })
    }

    /// Creates the identity graph for a known-duration original.
    ///
    /// # Errors
    ///
    /// Returns [`AdjustmentGraphError::InvalidTrimRange`] for an unknown or
    /// zero duration.
    pub fn identity(source_duration_millis: u64) -> Result<Self, AdjustmentGraphError> {
        Self::new(source_duration_millis, 0, source_duration_millis, 0, 0, 0)
    }

    #[must_use]
    pub const fn trim_start_millis(self) -> u64 {
        self.trim_start_millis
    }

    #[must_use]
    pub const fn trim_end_millis(self) -> u64 {
        self.trim_end_millis
    }

    #[must_use]
    pub const fn fade_in_millis(self) -> u64 {
        self.fade_in_millis
    }

    #[must_use]
    pub const fn fade_out_millis(self) -> u64 {
        self.fade_out_millis
    }

    #[must_use]
    pub const fn gain_centibels(self) -> i16 {
        self.gain_centibels
    }
}

/// Stable validation failures for authored adjustment intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustmentGraphError {
    InvalidTrimRange,
    OverlappingFades,
    GainOutOfRange,
}

impl std::fmt::Display for AdjustmentGraphError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidTrimRange => "trim range must be non-empty and inside the source",
            Self::OverlappingFades => "fade durations must fit inside the trim range",
            Self::GainOutOfRange => "gain must be between -24 dB and +12 dB",
        })
    }
}

impl std::error::Error for AdjustmentGraphError {}

#[cfg(test)]
mod tests;
