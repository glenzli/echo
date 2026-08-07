//! Cache-level error vocabulary.

use thiserror::Error;

/// Classification shared by cache failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheErrorKind {
    /// The bytes on disk no longer match their content identity.
    Corrupt,
    /// A payload exceeded the explicit size bound of the operation.
    TooLarge,
    /// Another failure, with the underlying message preserved.
    Other,
}

/// All cache crate failures.
#[derive(Debug, Error)]
#[error("{kind:?}: {message}")]
pub struct CacheError {
    pub kind: CacheErrorKind,
    pub message: String,
}

impl CacheError {
    /// Creates a cache error.
    #[must_use]
    pub fn new(kind: CacheErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl From<std::io::Error> for CacheError {
    fn from(error: std::io::Error) -> Self {
        Self::new(CacheErrorKind::Other, error.to_string())
    }
}
