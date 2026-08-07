//! Core workflows: importing real recordings into the Library.
//!
//! This crate orchestrates the catalog, the cache, and (later) the C++ audio
//! engine and AI workers. It knows nothing about Qt.
//!
//! Start with [`import`] for the idempotent import path. Background job
//! scheduling arrives with the first real analysis consumer (see ROADMAP
//! M0); no scheduler exists before there are jobs to run.

mod error;
mod import;

pub use error::CoreErrorKind;
pub use import::{ImportOutcome, import_asset, import_asset_with_probe};
