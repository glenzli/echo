//! Durable, source-anchored impulse-response assets.
//!
//! IR source bytes are not cache entries: a sound may depend on them after the
//! original import path disappears. This crate therefore owns a durable,
//! content-addressed source store and append-only import provenance. Decoding,
//! 48 kHz preparation, runtime FFT banks, Catalog references, and UI projection
//! are separate lifecycles.

mod error;
mod source_store;

pub use error::{IrStoreError, IrStoreErrorKind};
pub use source_store::{
    ImportOutcome, IrImportProvenance, IrImportRecord, IrRightsDeclaration, IrSourceStore,
};
