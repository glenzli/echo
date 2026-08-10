//! Non-destructive restoration intent for one immutable original.
//!
//! The graph stores authored time-domain bounds, amplitude envelopes, and a
//! bounded low-cut frequency in stable integer units. Execution-specific
//! sample positions and filter coefficients are prepared by the audio engine
//! and are never persisted as user intent.

use serde::{Deserialize, Serialize};

/// Lowest supported output gain in hundredths of one decibel.
pub const MIN_GAIN_CENTIBELS: i16 = -2_400;
/// Highest supported output gain in hundredths of one decibel.
pub const MAX_GAIN_CENTIBELS: i16 = 1_200;
/// Lowest supported enabled low-cut frequency in hertz.
pub const MIN_LOW_CUT_HERTZ: u16 = 20;
/// Highest supported low-cut frequency in hertz.
pub const MAX_LOW_CUT_HERTZ: u16 = 240;
/// Lowest supported gain for one equalizer band, in hundredths of a decibel.
pub const MIN_EQ_GAIN_CENTIBELS: i16 = -1_200;
/// Highest supported gain for one equalizer band, in hundredths of a decibel.
pub const MAX_EQ_GAIN_CENTIBELS: i16 = 1_200;
pub const MIN_COMPRESSOR_THRESHOLD_CENTIBELS: i16 = -6_000;
pub const MAX_COMPRESSOR_THRESHOLD_CENTIBELS: i16 = 0;
pub const MIN_COMPRESSOR_RATIO_TENTHS: u16 = 10;
pub const MAX_COMPRESSOR_RATIO_TENTHS: u16 = 200;
pub const MIN_COMPRESSOR_ATTACK_MILLIS: u16 = 1;
pub const MAX_COMPRESSOR_ATTACK_MILLIS: u16 = 200;
pub const MIN_COMPRESSOR_RELEASE_MILLIS: u16 = 20;
pub const MAX_COMPRESSOR_RELEASE_MILLIS: u16 = 2_000;
pub const MAX_COMPRESSOR_MAKEUP_CENTIBELS: i16 = 2_400;
pub const MIN_LIMITER_CEILING_CENTIBELS: i16 = -600;
pub const MAX_LIMITER_CEILING_CENTIBELS: i16 = 0;
pub const MIN_LIMITER_RELEASE_MILLIS: u16 = 20;
pub const MAX_LIMITER_RELEASE_MILLIS: u16 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressorSettings {
    pub enabled: bool,
    pub threshold_centibels: i16,
    pub ratio_tenths: u16,
    pub attack_millis: u16,
    pub release_millis: u16,
    pub makeup_centibels: i16,
}

impl Default for CompressorSettings {
    fn default() -> Self {
        Self::standard()
    }
}

impl CompressorSettings {
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            enabled: false,
            threshold_centibels: -1_800,
            ratio_tenths: 30,
            attack_millis: 10,
            release_millis: 120,
            makeup_centibels: 0,
        }
    }
}

/// Authored final-output peak limiter intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimiterSettings {
    pub enabled: bool,
    pub ceiling_centibels: i16,
    pub release_millis: u16,
}

impl Default for LimiterSettings {
    fn default() -> Self {
        Self::standard()
    }
}

impl LimiterSettings {
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            enabled: false,
            ceiling_centibels: -100,
            release_millis: 100,
        }
    }
}

/// Authored gain for Echo's fixed restoration equalizer bands.
///
/// Center frequencies and filter shapes belong to the execution contract;
/// persistence stores only stable per-band gain intent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreeBandEqualizer {
    low_gain_centibels: i16,
    mid_gain_centibels: i16,
    high_gain_centibels: i16,
}

impl ThreeBandEqualizer {
    #[must_use]
    pub const fn new(
        low_gain_centibels: i16,
        mid_gain_centibels: i16,
        high_gain_centibels: i16,
    ) -> Self {
        Self {
            low_gain_centibels,
            mid_gain_centibels,
            high_gain_centibels,
        }
    }

    #[must_use]
    pub const fn low_gain_centibels(self) -> i16 {
        self.low_gain_centibels
    }

    #[must_use]
    pub const fn mid_gain_centibels(self) -> i16 {
        self.mid_gain_centibels
    }

    #[must_use]
    pub const fn high_gain_centibels(self) -> i16 {
        self.high_gain_centibels
    }

    #[must_use]
    pub const fn is_flat(self) -> bool {
        self.low_gain_centibels == 0
            && self.mid_gain_centibels == 0
            && self.high_gain_centibels == 0
    }
}

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

/// Authored processing that applies inside the selected clip range.
///
/// This value keeps effect intent distinct from time-domain trim bounds and
/// avoids an order-sensitive sequence of scalar effect parameters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AdjustmentEffects {
    pub fade_curves: FadeCurves,
    pub gain_centibels: i16,
    pub low_cut_hertz: u16,
    pub equalizer: ThreeBandEqualizer,
    pub compressor: CompressorSettings,
    pub limiter: LimiterSettings,
}

impl AdjustmentEffects {
    #[must_use]
    pub const fn new(fade_curves: FadeCurves, gain_centibels: i16, low_cut_hertz: u16) -> Self {
        Self {
            fade_curves,
            gain_centibels,
            low_cut_hertz,
            equalizer: ThreeBandEqualizer::new(0, 0, 0),
            compressor: CompressorSettings::standard(),
            limiter: LimiterSettings::standard(),
        }
    }

    #[must_use]
    pub const fn with_equalizer(mut self, equalizer: ThreeBandEqualizer) -> Self {
        self.equalizer = equalizer;
        self
    }

    #[must_use]
    pub const fn with_compressor(mut self, compressor: CompressorSettings) -> Self {
        self.compressor = compressor;
        self
    }

    #[must_use]
    pub const fn with_limiter(mut self, limiter: LimiterSettings) -> Self {
        self.limiter = limiter;
        self
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
    low_cut_hertz: u16,
    equalizer: ThreeBandEqualizer,
    compressor: CompressorSettings,
    #[serde(default)]
    limiter: LimiterSettings,
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
        effects: AdjustmentEffects,
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
        if !(MIN_GAIN_CENTIBELS..=MAX_GAIN_CENTIBELS).contains(&effects.gain_centibels) {
            return Err(AdjustmentGraphError::GainOutOfRange);
        }
        if effects.low_cut_hertz != 0
            && !(MIN_LOW_CUT_HERTZ..=MAX_LOW_CUT_HERTZ).contains(&effects.low_cut_hertz)
        {
            return Err(AdjustmentGraphError::LowCutOutOfRange);
        }
        for gain in [
            effects.equalizer.low_gain_centibels,
            effects.equalizer.mid_gain_centibels,
            effects.equalizer.high_gain_centibels,
        ] {
            if !(MIN_EQ_GAIN_CENTIBELS..=MAX_EQ_GAIN_CENTIBELS).contains(&gain) {
                return Err(AdjustmentGraphError::EqualizerGainOutOfRange);
            }
        }
        let compressor = effects.compressor;
        if !(MIN_COMPRESSOR_THRESHOLD_CENTIBELS..=MAX_COMPRESSOR_THRESHOLD_CENTIBELS)
            .contains(&compressor.threshold_centibels)
            || !(MIN_COMPRESSOR_RATIO_TENTHS..=MAX_COMPRESSOR_RATIO_TENTHS)
                .contains(&compressor.ratio_tenths)
            || !(MIN_COMPRESSOR_ATTACK_MILLIS..=MAX_COMPRESSOR_ATTACK_MILLIS)
                .contains(&compressor.attack_millis)
            || !(MIN_COMPRESSOR_RELEASE_MILLIS..=MAX_COMPRESSOR_RELEASE_MILLIS)
                .contains(&compressor.release_millis)
            || !(0..=MAX_COMPRESSOR_MAKEUP_CENTIBELS).contains(&compressor.makeup_centibels)
        {
            return Err(AdjustmentGraphError::CompressorOutOfRange);
        }
        let limiter = effects.limiter;
        if !(MIN_LIMITER_CEILING_CENTIBELS..=MAX_LIMITER_CEILING_CENTIBELS)
            .contains(&limiter.ceiling_centibels)
            || !(MIN_LIMITER_RELEASE_MILLIS..=MAX_LIMITER_RELEASE_MILLIS)
                .contains(&limiter.release_millis)
        {
            return Err(AdjustmentGraphError::LimiterOutOfRange);
        }
        Ok(Self {
            trim_start_millis,
            trim_end_millis,
            fade_in_millis,
            fade_out_millis,
            fade_in_curve: effects.fade_curves.fade_in,
            fade_out_curve: effects.fade_curves.fade_out,
            gain_centibels: effects.gain_centibels,
            low_cut_hertz: effects.low_cut_hertz,
            equalizer: effects.equalizer,
            compressor,
            limiter,
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
            AdjustmentEffects::default(),
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

    /// High-pass cutoff in hertz, or zero when low-cut is disabled.
    #[must_use]
    pub const fn low_cut_hertz(self) -> u16 {
        self.low_cut_hertz
    }

    #[must_use]
    pub const fn equalizer(self) -> ThreeBandEqualizer {
        self.equalizer
    }

    #[must_use]
    pub const fn compressor(self) -> CompressorSettings {
        self.compressor
    }

    #[must_use]
    pub const fn limiter(self) -> LimiterSettings {
        self.limiter
    }
}

/// Stable validation failures for authored adjustment intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustmentGraphError {
    InvalidTrimRange,
    OverlappingFades,
    GainOutOfRange,
    LowCutOutOfRange,
    EqualizerGainOutOfRange,
    CompressorOutOfRange,
    LimiterOutOfRange,
}

impl std::fmt::Display for AdjustmentGraphError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidTrimRange => "trim range must be non-empty and inside the source",
            Self::OverlappingFades => "fade durations must fit inside the trim range",
            Self::GainOutOfRange => "gain must be between -24 dB and +12 dB",
            Self::LowCutOutOfRange => "low cut must be off or between 20 Hz and 240 Hz",
            Self::EqualizerGainOutOfRange => {
                "equalizer band gain must be between -12 dB and +12 dB"
            }
            Self::CompressorOutOfRange => "compressor parameters are outside the supported range",
            Self::LimiterOutOfRange => "limiter parameters are outside the supported range",
        })
    }
}

impl std::error::Error for AdjustmentGraphError {}

#[cfg(test)]
mod tests;
