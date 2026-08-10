//! The idempotent import path: hash the source bytes, register the asset,
//! then probe the audio engine for descriptive metadata.
//!
//! Import never modifies the source file. The audio engine probe is optional:
//! a source that the engine cannot open still registers as an asset so the
//! Library remains complete; its codec/duration stay unknown until a later
//! probe.

use std::{
    fs,
    io::Read,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use echo_catalog::{
    AssetLookup, AssetRegistrationInput, RegisterAsset, find_by_content_hash, open_catalog,
    register_asset,
};
use echo_domain::{AudioAsset, ContentHash};

use crate::error::{CoreError, CoreErrorKind};

/// Descriptive metadata from the audio engine probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioProbe {
    pub codec: Option<String>,
    pub duration_millis: Option<u64>,
    pub recorded_at_millis: Option<i64>,
    pub source_metadata: Option<echo_catalog::SourceMetadata>,
}

/// Outcome of one import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportOutcome {
    /// A new asset was created for previously unknown content.
    Imported(AudioAsset),
    /// The content already existed; its path was refreshed.
    AlreadyPresent(AudioAsset),
}

/// Imports `path` into `catalog_path`, probing the audio engine for
/// descriptive metadata when the source has an audio stream.
///
/// # Errors
///
/// Returns [`CoreErrorKind::SourceUnavailable`] when the file cannot be read,
/// [`CoreErrorKind::AudioEngineRejected`] when the probe refuses the source,
/// and [`CoreErrorKind::Catalog`] when registration fails.
pub fn import_asset(catalog_path: &Path, source: &Path) -> Result<ImportOutcome, CoreError> {
    import_asset_with_probe(catalog_path, source, &audio_engine_probe)
}

fn audio_engine_probe(source: &Path) -> Result<Option<AudioProbe>, CoreError> {
    let result = echo_bridge::probe(source)
        .map_err(|error| CoreError::new(CoreErrorKind::AudioEngineRejected, error.message))?;
    if !result.has_audio {
        return Ok(Some(AudioProbe {
            codec: None,
            duration_millis: None,
            recorded_at_millis: None,
            source_metadata: None,
        }));
    }
    Ok(Some(AudioProbe {
        codec: Some(result.codec_name),
        duration_millis: (result.duration_millis > 0).then_some(result.duration_millis),
        recorded_at_millis: (result.recorded_at_millis > 0).then_some(result.recorded_at_millis),
        source_metadata: Some(echo_catalog::SourceMetadata {
            container_format: result.container_format,
            sample_rate: result.sample_rate,
            channel_count: result.channel_count,
            entries: result
                .metadata
                .into_iter()
                .map(|entry| echo_catalog::SourceMetadataEntry {
                    key: entry.key,
                    value: entry.value,
                })
                .collect(),
        }),
    }))
}

/// Imports `path`, using `probe` for descriptive metadata when the source is
/// readable.
///
/// # Errors
///
/// Returns [`CoreErrorKind::SourceUnavailable`] when the file cannot be read,
/// [`CoreErrorKind::AudioEngineRejected`] when the probe refuses the source,
/// and [`CoreErrorKind::Catalog`] when registration fails.
pub fn import_asset_with_probe(
    catalog_path: &Path,
    source: &Path,
    probe: &dyn Fn(&Path) -> Result<Option<AudioProbe>, CoreError>,
) -> Result<ImportOutcome, CoreError> {
    let content_hash = hash_file(source)?;
    let size_bytes = fs::metadata(source)
        .map_err(|error| {
            CoreError::new(
                CoreErrorKind::SourceUnavailable,
                format!("cannot stat {}: {error}", source.display()),
            )
        })?
        .len();
    let imported_at_millis = now_millis();
    let catalog = open_catalog(catalog_path).map_err(CoreError::from)?;
    let outcome = catalog.with_transaction(|transaction| -> Result<ImportOutcome, CoreError> {
        let existing = find_by_content_hash(transaction, content_hash)?;
        match existing {
            AssetLookup::Found(asset) => {
                let asset = refresh_path(transaction, &asset, source)?;
                Ok(ImportOutcome::AlreadyPresent(asset))
            }
            AssetLookup::NotFound => {
                let probe_result = probe(source)?;
                let AudioProbe {
                    codec,
                    duration_millis,
                    recorded_at_millis,
                    source_metadata,
                } = probe_result.unwrap_or(AudioProbe {
                    codec: None,
                    duration_millis: None,
                    recorded_at_millis: None,
                    source_metadata: None,
                });
                let registration = register_asset(
                    transaction,
                    &AssetRegistrationInput {
                        content_hash,
                        path: source,
                        size_bytes,
                        codec: codec.as_deref(),
                        duration_millis,
                        recorded_at_millis,
                        imported_at_millis,
                    },
                )?;
                match registration {
                    RegisterAsset::Created(asset) => {
                        if let Some(metadata) = source_metadata {
                            echo_catalog::record_source_metadata(
                                transaction,
                                asset.id,
                                &metadata,
                                recorded_at_millis,
                            )?;
                        }
                        Ok(ImportOutcome::Imported(asset))
                    }
                    RegisterAsset::Existed(_) => unreachable!("hash was absent above"),
                }
            }
        }
    })?;
    Ok(outcome)
}

fn refresh_path(
    transaction: &rusqlite::Transaction<'_>,
    asset: &AudioAsset,
    source: &Path,
) -> Result<AudioAsset, CoreError> {
    if asset.original.path != source {
        transaction
            .execute(
                "UPDATE assets SET path = ?1 WHERE id = ?2",
                rusqlite::params![source.to_string_lossy(), asset.id.to_string()],
            )
            .map_err(|error| CoreError::new(CoreErrorKind::Catalog, error.to_string()))?;
    }
    let mut refreshed = asset.clone();
    source.clone_into(&mut refreshed.original.path);
    Ok(refreshed)
}

/// Streams a file into Echo's canonical BLAKE3 content identity.
///
/// # Errors
///
/// Returns [`CoreErrorKind::SourceUnavailable`] when the file cannot be read.
pub fn hash_file(source: &Path) -> Result<ContentHash, CoreError> {
    let mut file = fs::File::open(source).map_err(|error| {
        CoreError::new(
            CoreErrorKind::SourceUnavailable,
            format!("cannot open {}: {error}", source.display()),
        )
    })?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0u8; 256 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| {
            CoreError::new(
                CoreErrorKind::SourceUnavailable,
                format!("cannot read {}: {error}", source.display()),
            )
        })?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(ContentHash::from(hasher.finalize()))
}

pub(crate) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}
