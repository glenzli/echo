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

/// Stable fade interpolation authored independently for each edge.
///
/// The variants describe time-domain intent. The audio engine owns their
/// sample-domain evaluation so persistence never stores sampled envelopes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FadeCurve {
    #[default]
    Linear,
    Smooth,
    EqualPower,
}

impl FadeCurve {
    #[must_use]
    pub const fn catalog_value(self) -> i64 {
        match self {
            Self::Linear => 0,
            Self::Smooth => 1,
            Self::EqualPower => 2,
        }
    }

    /// Restores the stable Catalog representation.
    ///
    /// # Errors
    ///
    /// Returns [`FadeCurveValueError`] for unknown persisted values.
    pub const fn from_catalog_value(value: i64) -> Result<Self, FadeCurveValueError> {
        match value {
            0 => Ok(Self::Linear),
            1 => Ok(Self::Smooth),
            2 => Ok(Self::EqualPower),
            _ => Err(FadeCurveValueError),
        }
    }
}

/// A persisted fade curve value outside the stable enum contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FadeCurveValueError;

impl std::fmt::Display for FadeCurveValueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("fade curve must be linear, smooth, or equal power")
    }
}

impl std::error::Error for FadeCurveValueError {}

/// Fade interpolation selected independently for the two clip edges.
///
/// Keeping the pair as one value prevents bridge and persistence callers from
/// silently swapping two adjacent curve arguments.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FadeCurves {
    pub fade_in: FadeCurve,
    pub fade_out: FadeCurve,
}

impl FadeCurves {
    #[must_use]
    pub const fn new(fade_in: FadeCurve, fade_out: FadeCurve) -> Self {
        Self { fade_in, fade_out }
    }

    #[must_use]
    pub const fn linear() -> Self {
        Self::new(FadeCurve::Linear, FadeCurve::Linear)
    }
}

/// One validated, non-destructive adjustment graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdjustmentGraph {
    trim_start_millis: u64,
    trim_end_millis: u64,
    fade_in_millis: u64,
    fade_out_millis: u64,
    fade_in_curve: FadeCurve,
    fade_out_curve: FadeCurve,
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
        fade_curves: FadeCurves,
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
            fade_in_curve: fade_curves.fade_in,
            fade_out_curve: fade_curves.fade_out,
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
        Self::new(
            source_duration_millis,
            0,
            source_duration_millis,
            0,
            0,
            FadeCurves::linear(),
            0,
        )
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
    pub const fn fade_in_curve(self) -> FadeCurve {
        self.fade_in_curve
    }

    #[must_use]
    pub const fn fade_out_curve(self) -> FadeCurve {
        self.fade_out_curve
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
