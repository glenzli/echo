//! Durable exact-byte IR source storage and append-only import provenance.

use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use echo_domain::ContentHash;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{IrStoreError, IrStoreErrorKind};

const MAXIMUM_SOURCE_BYTES: u64 = 32 * 1024 * 1024;
const COPY_BUFFER_BYTES: usize = 64 * 1024;

/// User-declared rights context for one import event.
///
/// Echo records this declaration for provenance; it does not certify that the
/// declaration is legally correct or grant redistribution rights.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IrRightsDeclaration {
    /// A non-empty SPDX expression supplied by the importer.
    Spdx {
        expression: String,
        license_url: Option<String>,
    },
    /// The importer declares that they may use the source locally but Echo may
    /// not redistribute it.
    UserOwnedNoRedistribution,
}

/// Human and rights provenance for one import action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrImportProvenance {
    pub display_name: String,
    pub creator: Option<String>,
    pub source_url: Option<String>,
    pub attribution: Option<String>,
    pub rights: IrRightsDeclaration,
}

/// Immutable append-only record written after source publication succeeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrImportRecord {
    pub import_id: Uuid,
    pub source_hash: ContentHash,
    pub source_size_bytes: u64,
    pub imported_at_millis: u64,
    pub original_path: String,
    pub provenance: IrImportProvenance,
}

/// Whether exact source bytes were newly published or already present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportOutcome {
    Stored,
    AlreadyPresent,
}

/// Durable IR source store rooted independently from Echo's rebuildable cache.
#[derive(Debug, Clone)]
pub struct IrSourceStore {
    root: PathBuf,
}

impl IrSourceStore {
    /// Opens or creates a durable source store.
    ///
    /// # Errors
    ///
    /// Returns a filesystem error when required directories cannot be created.
    pub fn open(root: &Path) -> Result<Self, IrStoreError> {
        fs::create_dir_all(root.join("sources").join("b3"))?;
        fs::create_dir_all(root.join("sources").join("staging"))?;
        fs::create_dir_all(root.join("provenance").join("events"))?;
        Ok(Self {
            root: root.to_owned(),
        })
    }

    /// Imports exact bytes, then appends one immutable provenance event.
    ///
    /// Object publication is deliberately ordered before event publication.
    /// A crash may leave an unreferenced object for later garbage collection,
    /// but never a provenance event which points at missing source bytes.
    ///
    /// # Errors
    ///
    /// Fails closed for invalid provenance, a non-file source, oversized bytes,
    /// corruption at an existing content path, or filesystem/JSON failures.
    pub fn import(
        &self,
        source: &Path,
        provenance: IrImportProvenance,
    ) -> Result<(IrImportRecord, ImportOutcome), IrStoreError> {
        validate_provenance(&provenance)?;
        let metadata = fs::metadata(source)?;
        if !metadata.is_file() {
            return Err(IrStoreError::new(
                IrStoreErrorKind::InvalidInput,
                "impulse response source must be a regular file",
            ));
        }
        if metadata.len() == 0 {
            return Err(IrStoreError::new(
                IrStoreErrorKind::InvalidInput,
                "impulse response source must not be empty",
            ));
        }
        if metadata.len() > MAXIMUM_SOURCE_BYTES {
            return Err(IrStoreError::new(
                IrStoreErrorKind::TooLarge,
                "impulse response source exceeds 32 MiB",
            ));
        }

        let import_id = Uuid::now_v7();
        let staging = self
            .root
            .join("sources")
            .join("staging")
            .join(format!("{import_id}.source"));
        let result = self.publish_source(source, &staging, metadata.len());
        if result.is_err() {
            let _ = fs::remove_file(&staging);
        }
        let (source_hash, source_size_bytes, outcome) = result?;
        let record = IrImportRecord {
            import_id,
            source_hash,
            source_size_bytes,
            imported_at_millis: now_millis()?,
            original_path: source.to_string_lossy().into_owned(),
            provenance,
        };
        self.publish_event(&record)?;
        Ok((record, outcome))
    }

    /// Verifies an owned source object and returns its stable path.
    ///
    /// # Errors
    ///
    /// Returns `Corrupt` when the bytes or size do not match the requested
    /// source identity, and a filesystem error when the object is missing.
    pub fn verify_source(
        &self,
        source_hash: ContentHash,
        expected_size_bytes: u64,
    ) -> Result<PathBuf, IrStoreError> {
        let path = self.source_path(source_hash);
        verify_file(&path, source_hash, expected_size_bytes)?;
        Ok(path)
    }

    #[must_use]
    pub fn source_path(&self, source_hash: ContentHash) -> PathBuf {
        let text = source_hash.to_string();
        let (prefix, remainder) = text.split_at(2);
        self.root
            .join("sources")
            .join("b3")
            .join(prefix)
            .join(remainder)
    }

    #[must_use]
    pub fn provenance_path(&self, import_id: Uuid) -> PathBuf {
        self.root
            .join("provenance")
            .join("events")
            .join(format!("{import_id}.json"))
    }

    fn publish_source(
        &self,
        source: &Path,
        staging: &Path,
        expected_size: u64,
    ) -> Result<(ContentHash, u64, ImportOutcome), IrStoreError> {
        let (source_hash, source_size) = copy_hash_and_sync(source, staging)?;
        if source_size != expected_size {
            return Err(IrStoreError::new(
                IrStoreErrorKind::InvalidInput,
                "impulse response source changed during import",
            ));
        }
        let final_path = self.source_path(source_hash);
        if final_path.exists() {
            verify_file(&final_path, source_hash, source_size)?;
            fs::remove_file(staging)?;
            return Ok((source_hash, source_size, ImportOutcome::AlreadyPresent));
        }
        let parent = final_path.parent().ok_or_else(|| {
            IrStoreError::new(IrStoreErrorKind::Other, "source object has no parent")
        })?;
        fs::create_dir_all(parent)?;
        fs::rename(staging, &final_path)?;
        sync_directory(parent)?;
        Ok((source_hash, source_size, ImportOutcome::Stored))
    }

    fn publish_event(&self, record: &IrImportRecord) -> Result<(), IrStoreError> {
        let final_path = self.provenance_path(record.import_id);
        let parent = final_path.parent().ok_or_else(|| {
            IrStoreError::new(IrStoreErrorKind::Other, "provenance event has no parent")
        })?;
        let temporary = parent.join(format!(".{}.tmp", record.import_id));
        let result = write_event_file(record, &temporary, &final_path, parent);
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

fn write_event_file(
    record: &IrImportRecord,
    temporary: &Path,
    final_path: &Path,
    parent: &Path,
) -> Result<(), IrStoreError> {
    let payload = serde_json::to_vec_pretty(record)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary)?;
    file.write_all(&payload)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    fs::rename(temporary, final_path)?;
    sync_directory(parent)
}

fn validate_provenance(provenance: &IrImportProvenance) -> Result<(), IrStoreError> {
    if provenance.display_name.trim().is_empty() {
        return Err(IrStoreError::new(
            IrStoreErrorKind::InvalidInput,
            "impulse response display name is required",
        ));
    }
    if let IrRightsDeclaration::Spdx { expression, .. } = &provenance.rights
        && expression.trim().is_empty()
    {
        return Err(IrStoreError::new(
            IrStoreErrorKind::InvalidInput,
            "SPDX rights expression is required",
        ));
    }
    Ok(())
}

fn copy_hash_and_sync(
    source: &Path,
    destination: &Path,
) -> Result<(ContentHash, u64), IrStoreError> {
    let mut input = File::open(source)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0_u8; COPY_BUFFER_BYTES];
    let mut size = 0_u64;
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        output.write_all(&buffer[..count])?;
        hasher.update(&buffer[..count]);
        size = size
            .checked_add(u64::try_from(count).map_err(|error| {
                IrStoreError::new(IrStoreErrorKind::TooLarge, error.to_string())
            })?)
            .ok_or_else(|| IrStoreError::new(IrStoreErrorKind::TooLarge, "source size overflow"))?;
        if size > MAXIMUM_SOURCE_BYTES {
            return Err(IrStoreError::new(
                IrStoreErrorKind::TooLarge,
                "impulse response source exceeds 32 MiB",
            ));
        }
    }
    output.sync_all()?;
    Ok((ContentHash::from(hasher.finalize()), size))
}

fn verify_file(
    path: &Path,
    expected_hash: ContentHash,
    expected_size: u64,
) -> Result<(), IrStoreError> {
    let mut file = File::open(path)?;
    let metadata = file.metadata()?;
    if metadata.len() != expected_size {
        return Err(IrStoreError::new(
            IrStoreErrorKind::Corrupt,
            format!("IR source {} has the wrong size", path.display()),
        ));
    }
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0_u8; COPY_BUFFER_BYTES];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    if ContentHash::from(hasher.finalize()) != expected_hash {
        return Err(IrStoreError::new(
            IrStoreErrorKind::Corrupt,
            format!("IR source {} failed content verification", path.display()),
        ));
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), IrStoreError> {
    File::open(path)?.sync_all()?;
    Ok(())
}

fn now_millis() -> Result<u64, IrStoreError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            IrStoreError::new(
                IrStoreErrorKind::Other,
                format!("system clock error: {error}"),
            )
        })?;
    u64::try_from(duration.as_millis())
        .map_err(|error| IrStoreError::new(IrStoreErrorKind::Other, error.to_string()))
}

#[cfg(test)]
mod tests;
