//! IR source-store error vocabulary.

use thiserror::Error;

/// Stable failure classification for import and source verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrStoreErrorKind {
    /// Import provenance or a caller-provided bound is invalid.
    InvalidInput,
    /// Source bytes exceed the supported import size.
    TooLarge,
    /// Stored bytes no longer match their content identity.
    Corrupt,
    /// Filesystem or serialization failure.
    Other,
}

/// One IR source-store failure.
#[derive(Debug, Error)]
#[error("{kind:?}: {message}")]
pub struct IrStoreError {
    pub kind: IrStoreErrorKind,
    pub message: String,
}

impl IrStoreError {
    #[must_use]
    pub fn new(kind: IrStoreErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl From<std::io::Error> for IrStoreError {
    fn from(error: std::io::Error) -> Self {
        Self::new(IrStoreErrorKind::Other, error.to_string())
    }
}

impl From<serde_json::Error> for IrStoreError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(IrStoreErrorKind::Other, error.to_string())
    }
}
