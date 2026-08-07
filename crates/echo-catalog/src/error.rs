//! Catalog-level error vocabulary.

use thiserror::Error;

/// Classification shared by catalog failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogErrorKind {
    /// The `SQLite` database is not at the schema version this binary supports.
    SchemaMismatch,
    /// A uniqueness or referential constraint rejected the write.
    Constraint,
    /// Another failure, with the underlying message preserved.
    Other,
}

/// All catalog crate failures.
#[derive(Debug, Error)]
#[error("{kind:?}: {message}")]
pub struct CatalogError {
    pub kind: CatalogErrorKind,
    pub message: String,
}

impl CatalogError {
    /// Creates a catalog error.
    #[must_use]
    pub fn new(kind: CatalogErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl From<std::io::Error> for CatalogError {
    fn from(error: std::io::Error) -> Self {
        Self::new(CatalogErrorKind::Other, error.to_string())
    }
}

impl From<rusqlite::Error> for CatalogError {
    fn from(error: rusqlite::Error) -> Self {
        let kind = if matches!(
            error,
            rusqlite::Error::SqliteFailure(error, _)
                if error.code == rusqlite::ffi::ErrorCode::ConstraintViolation
        ) {
            CatalogErrorKind::Constraint
        } else {
            CatalogErrorKind::Other
        };
        Self::new(kind, error.to_string())
    }
}
