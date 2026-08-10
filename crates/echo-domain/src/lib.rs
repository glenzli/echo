//! Stable domain contracts shared by Echo's catalog, cache, core, AI, and UI.
//!
//! This crate root is a public API and navigation facade:
//!
//! - `ids` owns strongly typed persistent identifiers;
//! - `original` owns the immutable original reference and content identity;
//! - `adjustment` owns validated non-destructive restoration intent;
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
mod ids;
mod original;

pub use adjustment::{
    AdjustmentEffects, AdjustmentGraph, AdjustmentGraphError, CompressorSettings, FadeCurve,
    FadeCurveValueError, FadeCurves, LimiterSettings, MAX_COMPRESSOR_ATTACK_MILLIS,
    MAX_COMPRESSOR_MAKEUP_CENTIBELS, MAX_COMPRESSOR_RATIO_TENTHS, MAX_COMPRESSOR_RELEASE_MILLIS,
    MAX_COMPRESSOR_THRESHOLD_CENTIBELS, MAX_EQ_GAIN_CENTIBELS, MAX_GAIN_CENTIBELS,
    MAX_LIMITER_CEILING_CENTIBELS, MAX_LIMITER_RELEASE_MILLIS, MAX_LOW_CUT_HERTZ,
    MIN_COMPRESSOR_ATTACK_MILLIS, MIN_COMPRESSOR_RATIO_TENTHS, MIN_COMPRESSOR_RELEASE_MILLIS,
    MIN_COMPRESSOR_THRESHOLD_CENTIBELS, MIN_EQ_GAIN_CENTIBELS, MIN_GAIN_CENTIBELS,
    MIN_LIMITER_CEILING_CENTIBELS, MIN_LIMITER_RELEASE_MILLIS, MIN_LOW_CUT_HERTZ,
    ThreeBandEqualizer,
};
pub use analysis::{
    ALL_ANALYSIS_LEVELS, AnalysisKind, AnalysisLevel, AnalysisRecord, ModelIdentity,
};
pub use audio_asset::AudioAsset;
pub use ids::AssetId;
pub use original::{AssetPathStatus, ContentHash, ContentHashParseError, OriginalRef};

#[cfg(test)]
mod tests;
