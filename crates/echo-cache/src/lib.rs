//! Rebuildable, content-addressed blob storage for waveform pyramids,
//! embeddings, transcript caches, and render proxies.
//!
//! Start with [`blobs`] for identity, atomic publication, verified reads, and
//! corrupt-blob quarantine. The cache is never the source of truth: every blob
//! can be regenerated, so no `fsync` is forced on publication.

mod blobs;
mod error;

pub use blobs::{
    BlobRole, BlobStore, CachedBlob, PutBlob, blob_path, open_blob_store, put_blob,
    quarantine_corrupt, read_verified, store_root,
};
pub use error::{CacheError, CacheErrorKind};

#[cfg(test)]
mod tests;
