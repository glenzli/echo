//! Core-level error vocabulary.

use thiserror::Error;

/// Classification shared by core workflow failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreErrorKind {
    /// The source file could not be read or hashed.
    SourceUnavailable,
    /// The audio engine refused the source.
    AudioEngineRejected,
    /// The catalog rejected the write.
    Catalog,
    /// Another failure, with the underlying message preserved.
    Other,
}

/// All core workflow failures.
#[derive(Debug, Error)]
#[error("{kind:?}: {message}")]
pub struct CoreError {
    pub kind: CoreErrorKind,
    pub message: String,
}

impl CoreError {
    /// Creates a core error.
    #[must_use]
    pub fn new(kind: CoreErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl From<echo_catalog::CatalogError> for CoreError {
    fn from(error: echo_catalog::CatalogError) -> Self {
        Self::new(CoreErrorKind::Catalog, error.to_string())
    }
}
