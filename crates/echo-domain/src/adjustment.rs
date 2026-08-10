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
/// Echo's authored parametric equalizer has a fixed, bounded band count.
pub const PARAMETRIC_EQ_BAND_COUNT: usize = 6;
pub const MIN_EQ_FREQUENCY_HERTZ: u16 = 20;
pub const MAX_EQ_FREQUENCY_HERTZ: u16 = 20_000;
/// Parametric equalizer Q is stored in hundredths.
pub const MIN_EQ_Q_HUNDREDTHS: u16 = 10;
pub const MAX_EQ_Q_HUNDREDTHS: u16 = 2_000;
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

/// Stable filter shape for one authored parametric equalizer band.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EqualizerFilterKind {
    #[default]
    Bell,
    LowShelf,
    HighShelf,
    Notch,
}

impl EqualizerFilterKind {
    #[must_use]
    pub const fn catalog_value(self) -> i64 {
        match self {
            Self::Bell => 0,
            Self::LowShelf => 1,
            Self::HighShelf => 2,
            Self::Notch => 3,
        }
    }

    /// Restores the stable Catalog representation.
    ///
    /// # Errors
    ///
    /// Returns [`EqualizerFilterKindValueError`] for unknown values.
    pub const fn from_catalog_value(value: i64) -> Result<Self, EqualizerFilterKindValueError> {
        match value {
            0 => Ok(Self::Bell),
            1 => Ok(Self::LowShelf),
            2 => Ok(Self::HighShelf),
            3 => Ok(Self::Notch),
            _ => Err(EqualizerFilterKindValueError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EqualizerFilterKindValueError;

impl std::fmt::Display for EqualizerFilterKindValueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("equalizer filter must be bell, low shelf, high shelf, or notch")
    }
}

impl std::error::Error for EqualizerFilterKindValueError {}

/// One stable authored band. Coefficients remain an audio-engine concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParametricEqualizerBand {
    pub enabled: bool,
    pub filter_kind: EqualizerFilterKind,
    pub frequency_hertz: u16,
    pub q_hundredths: u16,
    pub gain_centibels: i16,
}

impl ParametricEqualizerBand {
    #[must_use]
    pub const fn new(
        enabled: bool,
        filter_kind: EqualizerFilterKind,
        frequency_hertz: u16,
        q_hundredths: u16,
        gain_centibels: i16,
    ) -> Self {
        Self {
            enabled,
            filter_kind,
            frequency_hertz,
            q_hundredths,
            gain_centibels,
        }
    }
}

/// Six-band authored equalizer intent shared by persistence and execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParametricEqualizer {
    bands: [ParametricEqualizerBand; PARAMETRIC_EQ_BAND_COUNT],
}

impl Default for ParametricEqualizer {
    fn default() -> Self {
        Self::flat()
    }
}

impl ParametricEqualizer {
    #[must_use]
    pub const fn new(bands: [ParametricEqualizerBand; PARAMETRIC_EQ_BAND_COUNT]) -> Self {
        Self { bands }
    }

    #[must_use]
    pub const fn flat() -> Self {
        use EqualizerFilterKind::{Bell, HighShelf, LowShelf};
        Self::new([
            ParametricEqualizerBand::new(true, LowShelf, 120, 71, 0),
            ParametricEqualizerBand::new(false, Bell, 250, 100, 0),
            ParametricEqualizerBand::new(true, Bell, 1_000, 100, 0),
            ParametricEqualizerBand::new(false, Bell, 3_000, 100, 0),
            ParametricEqualizerBand::new(false, Bell, 5_000, 100, 0),
            ParametricEqualizerBand::new(true, HighShelf, 8_000, 71, 0),
        ])
    }

    /// Losslessly lifts Echo's former fixed-band gains into the new topology.
    #[must_use]
    pub const fn from_legacy_gains(low: i16, mid: i16, high: i16) -> Self {
        let mut equalizer = Self::flat();
        equalizer.bands[0].gain_centibels = low;
        equalizer.bands[2].gain_centibels = mid;
        equalizer.bands[5].gain_centibels = high;
        equalizer
    }

    #[must_use]
    pub const fn bands(self) -> [ParametricEqualizerBand; PARAMETRIC_EQ_BAND_COUNT] {
        self.bands
    }

    #[must_use]
    pub fn is_flat(self) -> bool {
        self.bands.iter().all(|band| {
            !band.enabled
                || (band.filter_kind != EqualizerFilterKind::Notch && band.gain_centibels == 0)
        })
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
    pub equalizer: ParametricEqualizer,
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
            equalizer: ParametricEqualizer::flat(),
            compressor: CompressorSettings::standard(),
            limiter: LimiterSettings::standard(),
        }
    }

    #[must_use]
    pub const fn with_equalizer(mut self, equalizer: ParametricEqualizer) -> Self {
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
    equalizer: ParametricEqualizer,
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
        for band in effects.equalizer.bands {
            if !(MIN_EQ_GAIN_CENTIBELS..=MAX_EQ_GAIN_CENTIBELS).contains(&band.gain_centibels)
                || !(MIN_EQ_FREQUENCY_HERTZ..=MAX_EQ_FREQUENCY_HERTZ)
                    .contains(&band.frequency_hertz)
                || !(MIN_EQ_Q_HUNDREDTHS..=MAX_EQ_Q_HUNDREDTHS).contains(&band.q_hundredths)
            {
                return Err(AdjustmentGraphError::EqualizerBandOutOfRange);
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
    pub const fn equalizer(self) -> ParametricEqualizer {
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
    EqualizerBandOutOfRange,
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
            Self::EqualizerBandOutOfRange => {
                "equalizer band frequency, Q, or gain is outside the supported range"
            }
            Self::CompressorOutOfRange => "compressor parameters are outside the supported range",
            Self::LimiterOutOfRange => "limiter parameters are outside the supported range",
        })
    }
}

impl std::error::Error for AdjustmentGraphError {}

#[cfg(test)]
mod tests;
