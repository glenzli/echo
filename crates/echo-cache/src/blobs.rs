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
    /// Bounded STFT overview for the spectral repair workspace.
    SpectrogramOverview,
    /// Embedding payload reference.
    Embedding,
    /// Transcript cache.
    Transcript,
    /// Rendered proxy audio.
    RenderProxy,
    /// Canonical 48 kHz planar impulse-response preparation.
    ImpulseResponsePreparation,
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
    fs::create_dir_all(root.join("blobs").join("staging"))?;
    fs::create_dir_all(root.join("quarantine").join("b3"))?;
    Ok(BlobStore {
        root: root.to_owned(),
    })
}

/// Streams one existing file into the content-addressed store without
/// retaining its payload in memory. The source remains owned by the caller.
///
/// # Errors
///
/// Returns a cache error when the source cannot be read or publication cannot
/// be applied atomically.
pub fn put_file(
    store: &BlobStore,
    role: BlobRole,
    source: &Path,
) -> Result<(CachedBlob, PutBlob), CacheError> {
    let staging = store.root.join("blobs").join("staging").join(format!(
        "publish-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos())
    ));
    let result = stream_copy_and_hash(source, &staging).and_then(|(content_hash, size_bytes)| {
        let final_path = blob_path(&store.root, &content_hash);
        if final_path.exists() {
            verify_file(&final_path, &content_hash, size_bytes)?;
            fs::remove_file(&staging)?;
            return Ok((
                CachedBlob {
                    role,
                    content_hash,
                    size_bytes,
                },
                PutBlob::AlreadyPresent,
            ));
        }
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(&staging, &final_path)?;
        Ok((
            CachedBlob {
                role,
                content_hash,
                size_bytes,
            },
            PutBlob::Stored,
        ))
    });
    if result.is_err() {
        let _ = fs::remove_file(staging);
    }
    result
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
    let payload_size = u64::try_from(payload.len())
        .map_err(|error| CacheError::new(CacheErrorKind::TooLarge, error.to_string()))?;
    let final_path = blob_path(&store.root, &content_hash);
    if final_path.exists() {
        verify_file(&final_path, &content_hash, payload_size)?;
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
    if ContentHash::from(blake3::hash(&payload)) != content_hash {
        return Err(CacheError::new(
            CacheErrorKind::Corrupt,
            format!("blob {} failed verification", path.display()),
        ));
    }
    Ok(payload)
}

/// Verifies one cached blob by streaming its bytes without returning the
/// payload to the caller.
///
/// # Errors
///
/// Returns a cache error when the blob is missing, corrupt, or has a different
/// size than its catalog reference.
pub fn verify_blob(
    store: &BlobStore,
    content_hash: ContentHash,
    expected_size: u64,
) -> Result<PathBuf, CacheError> {
    let path = blob_path(&store.root, &content_hash);
    verify_file(&path, &content_hash, expected_size)?;
    Ok(path)
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
    if verify_file(&path, &content_hash, expected_size).is_ok() {
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

fn cached_blob(role: BlobRole, content_hash: ContentHash, size_bytes: usize) -> CachedBlob {
    CachedBlob {
        role,
        content_hash,
        size_bytes: u64::try_from(size_bytes).expect("blob size fits u64"),
    }
}

fn stream_copy_and_hash(
    source: &Path,
    destination: &Path,
) -> Result<(ContentHash, u64), CacheError> {
    let mut input = File::open(source)?;
    let mut output = File::create(destination)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0u8; 64 * 1024];
    let mut size_bytes = 0u64;
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
        size_bytes = size_bytes
            .checked_add(u64::try_from(read).expect("read size fits u64"))
            .ok_or_else(|| CacheError::new(CacheErrorKind::TooLarge, "size overflow"))?;
    }
    output.sync_all().ok();
    Ok((ContentHash::from(hasher.finalize()), size_bytes))
}

fn verify_file(
    path: &Path,
    content_hash: &ContentHash,
    expected_size: u64,
) -> Result<(), CacheError> {
    let actual_size = fs::metadata(path)?.len();
    if actual_size != expected_size {
        return Err(CacheError::new(
            CacheErrorKind::Corrupt,
            format!(
                "blob {} has size {actual_size}, expected {expected_size}",
                path.display()
            ),
        ));
    }
    let mut input = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    if ContentHash::from(hasher.finalize()) != *content_hash {
        return Err(CacheError::new(
            CacheErrorKind::Corrupt,
            format!("blob {} failed verification", path.display()),
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
