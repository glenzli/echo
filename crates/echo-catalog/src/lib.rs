//! SQLite-backed catalog persistence.
//!
//! This crate owns the current schema and write transactions. It deliberately
//! knows nothing about Qt, `FFmpeg`, playback, or analysis models.
//!
//! Start with [`catalog`] for connection lifecycle and schema ownership,
//! [`asset_registration`] for idempotent source registration, and [`analysis`]
//! for append-only analysis evidence. The responsibility-named modules below
//! own feature reads and transactions.

mod analysis;
mod asset_registration;
mod catalog;
mod error;
mod schema;

pub use analysis::{AnalysisQueryError, AppendAnalysisRecord, query_analysis, record_analysis};
pub use asset_registration::{
    AssetLookup, AssetRegistrationInput, RegisterAsset, find_by_content_hash, find_by_id,
    list_assets, register_asset,
};
pub use catalog::{Catalog, CatalogStats, open_catalog};
pub use error::{CatalogError, CatalogErrorKind};

#[cfg(test)]
mod tests;
