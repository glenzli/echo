//! Stable domain contracts shared by Echo's catalog, cache, core, AI, and UI.
//!
//! This crate root is a public API and navigation facade:
//!
//! - `ids` owns strongly typed persistent identifiers;
//! - `original` owns the immutable original reference and content identity;
//! - `adjustment` owns validated non-destructive restoration intent;
//! - `source_edit` owns original-time edit segments and effect masks;
//! - `processing_recipe` owns reusable processing snapshots and their
//!   deterministic asset-local materialization;
//! - `analysis` owns progressive analysis levels and evidence contracts
//!   (`value + model + model_version + confidence + timestamp` — analysis is
//!   never treated as fact);
//! - `audio_asset` owns the aggregate root that composes them.
//!
//! Follow each entry module for its responsibility map; substantive behavior
//! belongs there rather than in this facade.

mod adjustment;
mod analysis;
mod audio_asset;
mod creative_vfx;
mod ids;
mod original;
mod processing_recipe;
mod source_edit;

pub use adjustment::{
    AdjustmentEffects, AdjustmentGraph, AdjustmentGraphError, ChannelRepairSettings,
    CompressorSettings, DeClickSettings, DeEsserSettings, DeHumSettings, DePlosiveSettings,
    EFFECT_NODE_COUNT, EffectChain, EffectChainError, EffectNodeKind, EffectNodeKindValueError,
    EqualizerFilterKind, FadeCurve, FadeCurveValueError, FadeCurves, LimiterSettings,
    MAX_CHANNEL_BALANCE_PERCENT, MAX_COMPRESSOR_ATTACK_MILLIS, MAX_COMPRESSOR_MAKEUP_CENTIBELS,
    MAX_COMPRESSOR_RATIO_TENTHS, MAX_COMPRESSOR_RELEASE_MILLIS, MAX_COMPRESSOR_THRESHOLD_CENTIBELS,
    MAX_DE_ESSER_FREQUENCY_HERTZ, MAX_DE_ESSER_REDUCTION_CENTIBELS,
    MAX_DE_ESSER_THRESHOLD_CENTIBELS, MAX_DE_PLOSIVE_FREQUENCY_HERTZ,
    MAX_DE_PLOSIVE_REDUCTION_CENTIBELS, MAX_DE_PLOSIVE_RELEASE_MILLIS,
    MAX_DE_PLOSIVE_SENSITIVITY_PERCENT, MAX_EQ_GAIN_CENTIBELS, MAX_GAIN_CENTIBELS,
    MAX_LIMITER_CEILING_CENTIBELS, MAX_LIMITER_RELEASE_MILLIS, MAX_LOW_CUT_HERTZ,
    MAX_NOISE_REDUCTION_CENTIBELS, MAX_NOISE_REDUCTION_SENSITIVITY_PERCENT,
    MAX_NOISE_REDUCTION_SMOOTHING_MILLIS, MAX_REVERB_DAMPING_PERCENT, MAX_REVERB_DECAY_MILLIS,
    MAX_REVERB_HIGH_CUT_HERTZ, MAX_REVERB_LOW_CUT_HERTZ, MAX_REVERB_MIX_PERCENT,
    MAX_REVERB_PRE_DELAY_MILLIS, MAX_REVERB_SIZE_PERCENT, MIN_CHANNEL_BALANCE_PERCENT,
    MIN_COMPRESSOR_ATTACK_MILLIS, MIN_COMPRESSOR_RATIO_TENTHS, MIN_COMPRESSOR_RELEASE_MILLIS,
    MIN_COMPRESSOR_THRESHOLD_CENTIBELS, MIN_DE_ESSER_FREQUENCY_HERTZ,
    MIN_DE_ESSER_THRESHOLD_CENTIBELS, MIN_DE_PLOSIVE_FREQUENCY_HERTZ,
    MIN_DE_PLOSIVE_RELEASE_MILLIS, MIN_EQ_GAIN_CENTIBELS, MIN_GAIN_CENTIBELS,
    MIN_LIMITER_CEILING_CENTIBELS, MIN_LIMITER_RELEASE_MILLIS, MIN_LOW_CUT_HERTZ,
    MIN_NOISE_REDUCTION_SMOOTHING_MILLIS, MIN_REVERB_DECAY_MILLIS, MIN_REVERB_HIGH_CUT_HERTZ,
    MIN_REVERB_LOW_CUT_HERTZ, MIN_REVERB_SIZE_PERCENT, NoiseReductionSettings, ParametricEqualizer,
    ParametricEqualizerBand, RestorationSettings, ReverbCharacter, ReverbCharacterValueError,
    ReverbSettings,
};
pub use analysis::{
    ALL_ANALYSIS_LEVELS, AnalysisKind, AnalysisLevel, AnalysisRecord, ModelIdentity,
};
pub use audio_asset::AudioAsset;
pub use creative_vfx::{
    BitcrusherSettings, ChorusSettings, CreativeVfxSettings, CreativeVfxSettingsError,
    CreativeVfxValueError, DelayVfxCharacter, DelayVfxSettings, DigitalDegradeVfxCharacter,
    DigitalDegradeVfxSettings, DriveVfxCharacter, DriveVfxSettings, EchoSettings, FlangerSettings,
    ModulationVfxCharacter, ModulationVfxSettings, PhaserSettings, RotaryVfxSettings,
    RotaryVfxSpeed, SampleRateReductionSettings, SceneVfxCharacter, SceneVfxSettings,
    SlapbackSettings, TransformVfxCharacter, TransformVfxSettings, TremoloSettings,
};
pub use ids::{AssetId, ProcessingRecipeId, ProcessingRecipeRevisionId};
pub use original::{AssetPathStatus, ContentHash, ContentHashParseError, OriginalRef};
pub use processing_recipe::{
    AdjustmentPatch, DEFAULT_PROCESSING_COMPONENTS, ProcessingComponent,
    ProcessingComponentValueError, ProcessingMergeMode, ProcessingRecipeError,
    ProcessingRecipeRevision,
};
pub use source_edit::{
    DEFAULT_EFFECT_MASK_FEATHER_MILLIS, EditSegment, EditSegmentState, EditSegmentStateValueError,
    EditTimeline, EditTimelineError, EffectMask, EffectMaskError, MAX_EDIT_GAP_MILLIS,
    MAX_EDIT_SEGMENTS, MAX_EFFECT_MASK_FEATHER_MILLIS, MAX_EFFECT_MASKS,
};

#[cfg(test)]
mod tests;
