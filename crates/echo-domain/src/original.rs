//! The immutable original reference: Echo never modifies the source file.
//!
//! Content identity is a BLAKE3-256 digest over the exact source bytes. Two
//! imports of the same bytes register as one asset; provenance (`path`,
//! `codec`, timestamps) is descriptive and may be corrected, but the content
//! hash is the durable identity.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// BLAKE3-256 content identity over the exact original bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// Wraps a 32-byte digest.
    #[must_use]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Exposes the raw digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for ContentHash {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in &self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl From<blake3::Hash> for ContentHash {
    fn from(hash: blake3::Hash) -> Self {
        Self(*hash.as_bytes())
    }
}

/// Descriptive reference to the immutable original source of an asset.
///
/// `path` and the metadata fields may change over time (files move, codecs get
/// re-detected); `content_hash` is the stable identity and never changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OriginalRef {
    /// Current location of the original file on disk.
    pub path: PathBuf,
    /// Stable content identity (BLAKE3-256 over the exact bytes).
    pub content_hash: ContentHash,
    /// File size in bytes at import time.
    pub size_bytes: u64,
    /// Best-known container/codec name (e.g. `aac`, `flac`); filled by the
    /// audio engine probe when available.
    pub codec: Option<String>,
    /// Best-known duration in milliseconds; filled by the audio engine probe
    /// when available.
    pub duration_millis: Option<u64>,
    /// Capture timestamp if the source metadata exposes one, else import time.
    pub recorded_at_millis: Option<i64>,
    /// Wall-clock import time in unix milliseconds.
    pub imported_at_millis: i64,
}
