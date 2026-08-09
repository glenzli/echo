//! Stable domain contracts shared by Echo's catalog, cache, core, AI, and UI.
//!
//! This crate root is a public API and navigation facade:
//!
//! - `ids` owns strongly typed persistent identifiers;
//! - `original` owns the immutable original reference and content identity;
//! - `analysis` owns progressive analysis levels and evidence contracts
//!   (`value + model + model_version + confidence + timestamp` — analysis is
//!   never treated as fact);
//! - `audio_asset` owns the aggregate root that composes them.
//!
//! Follow each entry module for its responsibility map; substantive behavior
//! belongs there rather than in this facade.

mod analysis;
mod audio_asset;
mod ids;
mod original;

pub use analysis::{
    ALL_ANALYSIS_LEVELS, AnalysisKind, AnalysisLevel, AnalysisRecord, ModelIdentity,
};
pub use audio_asset::AudioAsset;
pub use ids::AssetId;
pub use original::{AssetPathStatus, ContentHash, ContentHashParseError, OriginalRef};

#[cfg(test)]
mod tests;
