//! Bridge-level error vocabulary.

use thiserror::Error;

/// Classification shared by bridge failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeErrorKind {
    /// The audio engine refused the request.
    EngineRejected,
    /// Another failure, with the underlying message preserved.
    Other,
}

/// All bridge failures.
#[derive(Debug, Error)]
#[error("{kind:?}: {message}")]
pub struct BridgeError {
    pub kind: BridgeErrorKind,
    pub message: String,
}

impl BridgeError {
    /// Creates a bridge error.
    #[must_use]
    pub fn new(kind: BridgeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl From<cxx::Exception> for BridgeError {
    fn from(error: cxx::Exception) -> Self {
        Self::new(BridgeErrorKind::EngineRejected, error.what())
    }
}
