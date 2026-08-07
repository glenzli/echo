//! The `AudioAsset` aggregate root: immutable original plus the highest
//! analysis level reached so far.
//!
//! The full projection (analysis records, adjustment graph, derived renders,
//! relations) lives in the catalog; this type is the stable composition
//! contract used across crate boundaries.

use serde::{Deserialize, Serialize};

use crate::{analysis::AnalysisLevel, ids::AssetId, original::OriginalRef};

/// One sound memory in the Library.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioAsset {
    pub id: AssetId,
    pub original: OriginalRef,
    /// Highest progressive analysis level currently satisfied. Levels may be
    /// re-run after a model upgrade, so this is a projection, not history.
    pub max_level: AnalysisLevel,
}

impl AudioAsset {
    /// Creates an asset aggregate.
    #[must_use]
    pub const fn new(id: AssetId, original: OriginalRef, max_level: AnalysisLevel) -> Self {
        Self {
            id,
            original,
            max_level,
        }
    }
}
