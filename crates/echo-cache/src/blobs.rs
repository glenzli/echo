//! Content-addressed blob storage.
//!
//! Identity is BLAKE3-256 over the exact payload bytes. Files carry no source
//! filename or extension:
//!
//! ```text
//! <cache-root>/blobs/b3/<first-two-hex>/<remaining-hex>
//! ```
//!
//! Writes use a same-directory temporary file followed by an atomic rename.
//! `read_verified` re-hashes lazy reads before bytes reach a consumer; a
//! corrupt blob is never silently accepted. `quarantine_corrupt` re-verifies
//! and then moves the bytes to `quarantine/b3` rather than deleting them,
//! freeing the canonical digest path for regeneration.

use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use echo_domain::ContentHash;

use crate::error::{CacheError, CacheErrorKind};

/// What a cached blob represents. The cache never interprets payload bytes;
/// the role is descriptive metadata for inspection and policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlobRole {
    /// Multi-level waveform peak pyramid.
    WaveformPyramid,
    /// Embedding payload reference.
    Embedding,
    /// Transcript cache.
    Transcript,
    /// Rendered proxy audio.
    RenderProxy,
}

/// A blob stored in the cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedBlob {
    pub role: BlobRole,
    pub content_hash: ContentHash,
    pub size_bytes: u64,
}

/// Handle to the blob store rooted at one directory.
#[derive(Debug, Clone)]
pub struct BlobStore {
    root: PathBuf,
}

/// Opens (creating if needed) the blob store under `root`.
///
/// # Errors
///
/// Returns a cache error when the root cannot be created.
pub fn open_blob_store(root: &Path) -> Result<BlobStore, CacheError> {
    fs::create_dir_all(root.join("blobs").join("b3"))?;
    fs::create_dir_all(root.join("quarantine").join("b3"))?;
    Ok(BlobStore {
        root: root.to_owned(),
    })
}

/// Canonical on-disk path for a content hash.
#[must_use]
pub fn blob_path(root: &Path, content_hash: &ContentHash) -> PathBuf {
    let text = content_hash.to_string();
    let (first, rest) = text.split_at(2);
    root.join("blobs").join("b3").join(first).join(rest)
}

/// Outcome of publishing a payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PutBlob {
    /// The payload was newly stored.
    Stored,
    /// A blob with the identical digest already existed.
    AlreadyPresent,
}

/// Publishes `payload` under its content hash. Idempotent: existing content is
/// re-hashed before reuse, so the caller's digest must match the bytes.
///
/// # Errors
///
/// Returns a cache error when the write cannot be applied atomically.
pub fn put_blob(
    store: &BlobStore,
    role: BlobRole,
    payload: &[u8],
) -> Result<(CachedBlob, PutBlob), CacheError> {
    let content_hash = ContentHash::from(blake3::hash(payload));
    let final_path = blob_path(&store.root, &content_hash);
    if final_path.exists() {
        verify_bytes(&final_path, &content_hash, payload.len())?;
        return Ok((
            cached_blob(role, content_hash, payload.len()),
            PutBlob::AlreadyPresent,
        ));
    }
    if let Some(parent) = final_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary_path = temporary_path(&final_path);
    let mut file = File::create(&temporary_path)?;
    file.write_all(payload)?;
    file.sync_all().ok(); // Rebuildable content: best-effort durability.
    fs::rename(&temporary_path, &final_path)?;
    Ok((
        cached_blob(role, content_hash, payload.len()),
        PutBlob::Stored,
    ))
}

/// Reads a blob after verifying its bytes match the requested digest.
///
/// # Errors
///
/// Returns [`CacheErrorKind::Corrupt`] when the stored bytes do not match the
/// digest, and [`CacheErrorKind::TooLarge`] when the payload exceeds `max_bytes`.
pub fn read_verified(
    store: &BlobStore,
    content_hash: ContentHash,
    max_bytes: u64,
) -> Result<Vec<u8>, CacheError> {
    let path = blob_path(&store.root, &content_hash);
    let payload = read_bounded(&path, max_bytes)?;
    verify_bytes(&path, &content_hash, payload.len())?;
    Ok(payload)
}

/// Re-verifies a blob and, when corrupt, moves it out of the canonical tree.
///
/// # Errors
///
/// Returns a cache error when the payload cannot be read at all or the
/// quarantine move itself fails.
pub fn quarantine_corrupt(
    store: &BlobStore,
    content_hash: ContentHash,
    expected_size: u64,
) -> Result<(), CacheError> {
    let path = blob_path(&store.root, &content_hash);
    let payload = read_bounded(&path, expected_size)?;
    if blake3::hash(&payload) == digest_bytes(&content_hash) {
        return Ok(());
    }
    let quarantine_path = quarantine_path(&store.root, &content_hash);
    if let Some(parent) = quarantine_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(&path, &quarantine_path)?;
    Ok(())
}

/// Exposes the store root.
#[must_use]
pub fn store_root(store: &BlobStore) -> &Path {
    &store.root
}

fn digest_bytes(content_hash: &ContentHash) -> blake3::Hash {
    blake3::Hash::from_bytes(*content_hash.as_bytes())
}

fn cached_blob(role: BlobRole, content_hash: ContentHash, size_bytes: usize) -> CachedBlob {
    CachedBlob {
        role,
        content_hash,
        size_bytes: u64::try_from(size_bytes).expect("blob size fits u64"),
    }
}

fn verify_bytes(
    path: &Path,
    content_hash: &ContentHash,
    expected_size: usize,
) -> Result<(), CacheError> {
    let file = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0u8; 64 * 1024];
    let mut file = file.take(u64::try_from(expected_size).expect("size fits u64"));
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    let digest = ContentHash::from(hasher.finalize());
    if digest != *content_hash {
        return Err(CacheError::new(
            CacheErrorKind::Corrupt,
            format!(
                "blob {} failed verification (bytes read {expected_size})",
                path.display()
            ),
        ));
    }
    Ok(())
}

fn read_bounded(path: &Path, max_bytes: u64) -> Result<Vec<u8>, CacheError> {
    let mut file = File::open(path)?;
    let mut payload = Vec::new();
    let mut buffer = vec![0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total = total
            .checked_add(u64::try_from(read).expect("read fits u64"))
            .ok_or_else(|| CacheError::new(CacheErrorKind::TooLarge, "size overflow"))?;
        if total > max_bytes {
            return Err(CacheError::new(
                CacheErrorKind::TooLarge,
                format!(
                    "blob {} exceeds bounded read of {max_bytes} bytes",
                    path.display()
                ),
            ));
        }
        payload.extend_from_slice(&buffer[..read]);
    }
    Ok(payload)
}

fn temporary_path(final_path: &Path) -> PathBuf {
    let file_name = final_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("blob");
    let mut path = final_path.with_file_name(format!(".{file_name}.tmp-{}", std::process::id()));
    if path.exists() {
        path = final_path.with_file_name(format!(
            ".{file_name}.tmp-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
    }
    path
}

fn quarantine_path(root: &Path, content_hash: &ContentHash) -> PathBuf {
    let text = content_hash.to_string();
    let (first, rest) = text.split_at(2);
    root.join("quarantine").join("b3").join(first).join(rest)
}
