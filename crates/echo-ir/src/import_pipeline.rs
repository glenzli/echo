//! Failure-ordered local IR import and canonical preparation.

use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use echo_cache::{
    BlobRole, BlobStore, PutBlob, open_blob_store, put_file, store_root, verify_blob,
};
use echo_domain::ContentHash;
use uuid::Uuid;

use crate::{
    ImportOutcome, IrImportProvenance, IrImportRecord, IrSourceStore, IrStoreError, StoredIrSource,
};

const PREPARED_HEADER_BYTES: usize = 64;

/// Whether canonical prepared bytes were newly cached or already present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationOutcome {
    Stored,
    AlreadyPresent,
}

/// Rebuildable canonical preparation bound to one durable source identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedIrArtifact {
    pub source_hash: ContentHash,
    pub prepared_hash: ContentHash,
    pub preparation_version: u32,
    pub source_sample_rate: u32,
    pub channel_count: u32,
    pub source_frame_count: u64,
    pub prepared_frame_count: u64,
    pub avcodec_version: u32,
    pub swresample_version: u32,
    pub size_bytes: u64,
    pub cache_path: PathBuf,
    pub outcome: PreparationOutcome,
}

/// Complete non-Catalog result for one local WAV import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedIr {
    pub record: IrImportRecord,
    pub source_outcome: ImportOutcome,
    pub preparation: PreparedIrArtifact,
}

/// Coordinates durable source ownership and rebuildable prepared bytes.
#[derive(Debug, Clone)]
pub struct IrImportPipeline {
    source_store: IrSourceStore,
    prepared_cache: BlobStore,
}

impl IrImportPipeline {
    /// Opens the independent durable source and rebuildable cache roots.
    ///
    /// # Errors
    ///
    /// Returns a filesystem or cache error when either store cannot open.
    pub fn open(source_store_root: &Path, cache_root: &Path) -> Result<Self, IrStoreError> {
        Ok(Self {
            source_store: IrSourceStore::open(source_store_root)?,
            prepared_cache: open_blob_store(cache_root)?,
        })
    }

    /// Imports and prepares one local WAV before appending provenance.
    ///
    /// Publication order is durable source, rebuildable preparation, then
    /// immutable provenance. Any failure before the last step leaves no
    /// provenance event. Orphan source/cache objects are safe to garbage
    /// collect later because neither is a user-visible import by itself.
    ///
    /// # Errors
    ///
    /// Fails closed for invalid rights metadata, source I/O/corruption,
    /// unsupported WAV structure/content, or cache/event publication failure.
    pub fn import_local_wav(
        &self,
        source: &Path,
        provenance: IrImportProvenance,
    ) -> Result<ImportedIr, IrStoreError> {
        provenance.validate()?;
        let stored = self.source_store.store_source(source)?;
        let preparation = self.prepare_owned_source(&stored)?;
        let record = self
            .source_store
            .append_provenance(&stored, source, provenance)?;
        Ok(ImportedIr {
            record,
            source_outcome: stored.outcome,
            preparation,
        })
    }

    /// Rebuilds canonical prepared bytes from a verified durable source.
    ///
    /// # Errors
    ///
    /// Fails when the owned source is missing/corrupt, preparation is rejected,
    /// or the rebuildable cache cannot publish/verify the resulting artifact.
    pub fn prepare_owned_source(
        &self,
        stored: &StoredIrSource,
    ) -> Result<PreparedIrArtifact, IrStoreError> {
        let owned_source = self
            .source_store
            .verify_source(stored.source_hash, stored.source_size_bytes)?;
        let staging_root = store_root(&self.prepared_cache).join("ir-preparation-staging");
        fs::create_dir_all(&staging_root)?;
        let temporary = staging_root.join(format!("{}.echoir", Uuid::now_v7()));
        let result = self.prepare_to_cache(stored.source_hash, &owned_source, &temporary);
        let _ = fs::remove_file(&temporary);
        result
    }

    #[must_use]
    pub const fn source_store(&self) -> &IrSourceStore {
        &self.source_store
    }

    fn prepare_to_cache(
        &self,
        source_hash: ContentHash,
        owned_source: &Path,
        temporary: &Path,
    ) -> Result<PreparedIrArtifact, IrStoreError> {
        let evidence = echo_bridge::prepare_impulse_response(owned_source, temporary)?;
        let temporary_size = fs::metadata(temporary)?.len();
        if temporary_size != evidence.size_bytes {
            return Err(IrStoreError::new(
                crate::IrStoreErrorKind::Corrupt,
                "prepared IR size does not match engine evidence",
            ));
        }
        verify_prepared_header(temporary, &evidence)?;
        let (blob, put) = put_file(
            &self.prepared_cache,
            BlobRole::ImpulseResponsePreparation,
            temporary,
        )?;
        let cache_path = verify_blob(&self.prepared_cache, blob.content_hash, blob.size_bytes)?;
        verify_prepared_header(&cache_path, &evidence)?;
        Ok(PreparedIrArtifact {
            source_hash,
            prepared_hash: blob.content_hash,
            preparation_version: evidence.preparation_version,
            source_sample_rate: evidence.source_sample_rate,
            channel_count: evidence.channel_count,
            source_frame_count: evidence.source_frame_count,
            prepared_frame_count: evidence.prepared_frame_count,
            avcodec_version: evidence.avcodec_version,
            swresample_version: evidence.swresample_version,
            size_bytes: evidence.size_bytes,
            cache_path,
            outcome: match put {
                PutBlob::Stored => PreparationOutcome::Stored,
                PutBlob::AlreadyPresent => PreparationOutcome::AlreadyPresent,
            },
        })
    }
}

fn verify_prepared_header(
    path: &Path,
    evidence: &echo_bridge::PreparedImpulseResponse,
) -> Result<(), IrStoreError> {
    let mut header = [0_u8; PREPARED_HEADER_BYTES];
    File::open(path)?.read_exact(&mut header)?;
    let data_bytes = evidence
        .prepared_frame_count
        .checked_mul(u64::from(evidence.channel_count))
        .and_then(|samples| samples.checked_mul(4))
        .ok_or_else(|| {
            IrStoreError::new(
                crate::IrStoreErrorKind::Corrupt,
                "prepared IR sample size overflow",
            )
        })?;
    let expected_size = u64::try_from(PREPARED_HEADER_BYTES)
        .expect("prepared header size fits u64")
        .checked_add(data_bytes)
        .ok_or_else(|| {
            IrStoreError::new(
                crate::IrStoreErrorKind::Corrupt,
                "prepared IR artifact size overflow",
            )
        })?;
    let valid = &header[..8] == b"ECHOIR01"
        && read_u32(&header, 8) == u32::try_from(PREPARED_HEADER_BYTES).expect("header fits u32")
        && read_u32(&header, 12) == evidence.preparation_version
        && read_u32(&header, 16) == 48_000
        && read_u32(&header, 20) == evidence.channel_count
        && read_u64(&header, 24) == evidence.prepared_frame_count
        && read_u32(&header, 32) == evidence.source_sample_rate
        && read_u32(&header, 36) == evidence.channel_count
        && read_u64(&header, 40) == evidence.source_frame_count
        && read_u32(&header, 48) == evidence.avcodec_version
        && read_u32(&header, 52) == evidence.swresample_version
        && read_u64(&header, 56) == data_bytes
        && evidence.size_bytes == expected_size;
    if !valid {
        return Err(IrStoreError::new(
            crate::IrStoreErrorKind::Corrupt,
            "prepared IR header does not match engine evidence",
        ));
    }
    Ok(())
}

fn read_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("four bytes"))
}

fn read_u64(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().expect("eight bytes"))
}

#[cfg(test)]
mod tests;
