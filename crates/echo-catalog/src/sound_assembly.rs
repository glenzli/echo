//! Immutable revisions and render provenance for multi-asset sound assembly.
//!
//! The authored document remains a domain value. This owner validates every
//! clip against the exact linear asset revision it names, then persists the
//! document and its source references atomically. Originals and asset
//! adjustment revisions are never rewritten.

use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use echo_domain::{ContentHash, SoundAssembly, SoundAssemblyId};
use rusqlite::{OptionalExtension, Transaction, params};
use serde_json::json;

use crate::{CatalogError, CatalogErrorKind, adjustment_graph_at_revision};

/// One immutable authored revision of an assembly document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundAssemblyRevision {
    pub revision_id: i64,
    pub revision_number: u32,
    pub assembly: SoundAssembly,
    pub created_at_millis: i64,
}

/// Compact top-level workspace projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundAssemblySummary {
    pub assembly_id: SoundAssemblyId,
    pub name: String,
    pub revision_id: i64,
    pub revision_number: u32,
    pub duration_millis: u64,
    pub track_count: usize,
    pub clip_count: usize,
    pub updated_at_millis: i64,
}

/// Supported durable assembly publication format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundAssemblyExportFormat {
    WavPcm24,
}

impl SoundAssemblyExportFormat {
    const fn stored_name(self) -> &'static str {
        match self {
            Self::WavPcm24 => "wav_pcm24",
        }
    }
}

/// Engine-verified facts for one completed assembly publication.
#[derive(Debug, Clone)]
pub struct RecordSoundAssemblyExport<'a> {
    pub assembly_id: SoundAssemblyId,
    pub assembly_revision_id: i64,
    pub output_path: &'a Path,
    pub format: SoundAssemblyExportFormat,
    pub sample_rate: u32,
    pub channel_count: u16,
    pub frame_count: u64,
    pub content_hash: ContentHash,
    pub size_bytes: u64,
    pub integrated_lufs: f64,
    pub true_peak_dbtp: f64,
    pub created_at_millis: i64,
}

/// Durable readback of one assembly publication and its source snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundAssemblyExportRecord {
    pub export_id: i64,
    pub assembly_id: SoundAssemblyId,
    pub assembly_revision_id: i64,
    pub output_path: PathBuf,
    pub format: SoundAssemblyExportFormat,
    pub sample_rate: u32,
    pub channel_count: u16,
    pub frame_count: u64,
    pub content_hash: ContentHash,
    pub size_bytes: u64,
    pub integrated_lufs: f64,
    pub true_peak_dbtp: f64,
    pub provenance_json: String,
    pub created_at_millis: i64,
}

/// Lists active assembly documents by most recent authored change.
///
/// # Errors
///
/// Returns a Catalog error when stored rows cannot be queried or decoded.
pub fn list_sound_assemblies(
    transaction: &Transaction<'_>,
) -> Result<Vec<SoundAssemblySummary>, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT a.id, r.name, r.id, r.revision_number, r.duration_millis, \
         r.track_count, r.clip_count, a.updated_at_millis \
         FROM sound_assemblies a \
         JOIN sound_assembly_revisions r ON r.id = ( \
             SELECT newest.id FROM sound_assembly_revisions newest \
             WHERE newest.assembly_id = a.id \
             ORDER BY newest.revision_number DESC LIMIT 1) \
         WHERE a.archived_at_millis IS NULL \
         ORDER BY a.updated_at_millis DESC, a.id DESC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, i64>(7)?,
        ))
    })?;
    rows.map(|row| {
        let (id, name, revision_id, revision_number, duration, tracks, clips, updated) = row?;
        Ok(SoundAssemblySummary {
            assembly_id: parse_assembly_id(&id)?,
            name,
            revision_id,
            revision_number: stored_u32(revision_number, "assembly revision number")?,
            duration_millis: stored_u64(duration, "assembly duration")?,
            track_count: stored_usize(tracks, "assembly track count")?,
            clip_count: stored_usize(clips, "assembly clip count")?,
            updated_at_millis: updated,
        })
    })
    .collect()
}

/// Reads the newest immutable revision of one active or archived assembly.
///
/// # Errors
///
/// Returns a Catalog error when the stored revision cannot be queried or decoded.
pub fn latest_sound_assembly(
    transaction: &Transaction<'_>,
    assembly_id: SoundAssemblyId,
) -> Result<Option<SoundAssemblyRevision>, CatalogError> {
    sound_assembly_query(
        transaction,
        "SELECT id, revision_number, document_json, created_at_millis \
         FROM sound_assembly_revisions WHERE assembly_id = ?1 \
         ORDER BY revision_number DESC LIMIT 1",
        params![assembly_id.to_string()],
    )
}

/// Reads one exact immutable assembly revision scoped by its assembly.
///
/// # Errors
///
/// Returns a Catalog error when the stored revision cannot be queried or decoded.
pub fn sound_assembly_at_revision(
    transaction: &Transaction<'_>,
    assembly_id: SoundAssemblyId,
    revision_id: i64,
) -> Result<Option<SoundAssemblyRevision>, CatalogError> {
    sound_assembly_query(
        transaction,
        "SELECT id, revision_number, document_json, created_at_millis \
         FROM sound_assembly_revisions WHERE assembly_id = ?1 AND id = ?2",
        params![assembly_id.to_string(), revision_id],
    )
}

/// Appends an immutable revision, or returns the current revision unchanged
/// when the submitted document is byte-equivalent after canonical encoding.
///
/// # Errors
///
/// Returns a Catalog error when the document or its exact clip sources are
/// invalid, the assembly is archived, or persistence fails.
pub fn record_sound_assembly(
    transaction: &Transaction<'_>,
    assembly: &SoundAssembly,
    now_millis: i64,
) -> Result<SoundAssemblyRevision, CatalogError> {
    if now_millis < 0 {
        return Err(assembly_error("assembly timestamp must not be negative"));
    }
    validate_clip_sources(transaction, assembly)?;
    let document_json = serde_json::to_string(assembly)
        .map_err(|error| assembly_error(format!("assembly document cannot be encoded: {error}")))?;
    let existing = transaction
        .query_row(
            "SELECT archived_at_millis FROM sound_assemblies WHERE id = ?1",
            [assembly.id().to_string()],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()?;
    if existing.flatten().is_some() {
        return Err(assembly_error(
            "archived assembly cannot accept new revisions",
        ));
    }
    if let Some(current) = latest_sound_assembly(transaction, assembly.id())? {
        let current_json = serde_json::to_string(&current.assembly)
            .map_err(|error| assembly_error(error.to_string()))?;
        if current_json == document_json {
            return Ok(current);
        }
    }

    transaction.execute(
        "INSERT INTO sound_assemblies (id, created_at_millis, updated_at_millis) \
         VALUES (?1, ?2, ?2) ON CONFLICT(id) DO UPDATE SET updated_at_millis = excluded.updated_at_millis",
        params![assembly.id().to_string(), now_millis],
    )?;
    let revision_number: i64 = transaction.query_row(
        "SELECT COALESCE(MAX(revision_number), 0) + 1 \
         FROM sound_assembly_revisions WHERE assembly_id = ?1",
        [assembly.id().to_string()],
        |row| row.get(0),
    )?;
    transaction.execute(
        "INSERT INTO sound_assembly_revisions \
         (assembly_id, revision_number, name, document_json, duration_millis, \
          track_count, clip_count, created_at_millis) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            assembly.id().to_string(),
            revision_number,
            assembly.name(),
            document_json,
            sqlite_u64(assembly.duration_millis(), "assembly duration")?,
            i64::try_from(assembly.tracks().len()).map_err(sqlite_overflow)?,
            i64::try_from(assembly.clip_count()).map_err(sqlite_overflow)?,
            now_millis,
        ],
    )?;
    let revision_id = transaction.last_insert_rowid();
    for track in assembly.tracks() {
        for clip in track.clips() {
            transaction.execute(
                "INSERT INTO sound_assembly_clip_sources \
                 (assembly_revision_id, track_id, clip_id, asset_id, adjustment_revision_id, \
                  source_start_millis, source_end_millis, timeline_start_millis) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    revision_id,
                    track.id().to_string(),
                    clip.id().to_string(),
                    clip.asset_id().to_string(),
                    (clip.adjustment_revision_id() > 0).then_some(clip.adjustment_revision_id()),
                    sqlite_u64(clip.source_start_millis(), "clip source start")?,
                    sqlite_u64(clip.source_end_millis(), "clip source end")?,
                    sqlite_u64(clip.timeline_start_millis(), "clip timeline start")?,
                ],
            )?;
        }
    }
    Ok(SoundAssemblyRevision {
        revision_id,
        revision_number: stored_u32(revision_number, "assembly revision number")?,
        assembly: assembly.clone(),
        created_at_millis: now_millis,
    })
}

/// Archives a document without deleting its revision or export history.
///
/// # Errors
///
/// Returns a Catalog error when the active assembly does not exist or the
/// archive update fails.
pub fn archive_sound_assembly(
    transaction: &Transaction<'_>,
    assembly_id: SoundAssemblyId,
    now_millis: i64,
) -> Result<(), CatalogError> {
    let changed = transaction.execute(
        "UPDATE sound_assemblies SET archived_at_millis = ?1, updated_at_millis = ?1 \
         WHERE id = ?2 AND archived_at_millis IS NULL",
        params![now_millis, assembly_id.to_string()],
    )?;
    if changed == 0 {
        return Err(assembly_error("active assembly does not exist"));
    }
    Ok(())
}

/// Records one completed mixdown with provenance generated from the exact
/// stored revision and immutable source hashes.
///
/// # Errors
///
/// Returns a Catalog error when the evidence or revision is invalid, source
/// provenance cannot be resolved, or persistence fails.
pub fn record_sound_assembly_export(
    transaction: &Transaction<'_>,
    input: &RecordSoundAssemblyExport<'_>,
) -> Result<SoundAssemblyExportRecord, CatalogError> {
    validate_export(input)?;
    let revision =
        sound_assembly_at_revision(transaction, input.assembly_id, input.assembly_revision_id)?
            .ok_or_else(|| assembly_error("assembly revision does not exist"))?;
    let provenance_json = assembly_provenance(transaction, &revision)?;
    transaction.execute(
        "INSERT INTO sound_assembly_exports \
         (assembly_id, assembly_revision_id, output_path, format, sample_rate, channel_count, \
          bit_depth, frame_count, content_hash, size_bytes, integrated_lufs, true_peak_dbtp, \
          provenance_json, created_at_millis) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 24, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            input.assembly_id.to_string(),
            input.assembly_revision_id,
            input.output_path.to_string_lossy(),
            input.format.stored_name(),
            i64::from(input.sample_rate),
            i64::from(input.channel_count),
            sqlite_u64(input.frame_count, "assembly export frame count")?,
            input.content_hash.to_string(),
            sqlite_u64(input.size_bytes, "assembly export size")?,
            input.integrated_lufs,
            input.true_peak_dbtp,
            provenance_json,
            input.created_at_millis,
        ],
    )?;
    Ok(SoundAssemblyExportRecord {
        export_id: transaction.last_insert_rowid(),
        assembly_id: input.assembly_id,
        assembly_revision_id: input.assembly_revision_id,
        output_path: input.output_path.to_owned(),
        format: input.format,
        sample_rate: input.sample_rate,
        channel_count: input.channel_count,
        frame_count: input.frame_count,
        content_hash: input.content_hash,
        size_bytes: input.size_bytes,
        integrated_lufs: input.integrated_lufs,
        true_peak_dbtp: input.true_peak_dbtp,
        provenance_json,
        created_at_millis: input.created_at_millis,
    })
}

fn validate_clip_sources(
    transaction: &Transaction<'_>,
    assembly: &SoundAssembly,
) -> Result<(), CatalogError> {
    for clip in assembly
        .tracks()
        .iter()
        .flat_map(echo_domain::AssemblyTrack::clips)
    {
        let source_duration = if clip.adjustment_revision_id() == 0 {
            transaction
                .query_row(
                    "SELECT duration_millis FROM assets WHERE id = ?1",
                    [clip.asset_id().to_string()],
                    |row| row.get::<_, Option<i64>>(0),
                )
                .optional()?
                .flatten()
                .map(|duration| stored_u64(duration, "asset duration"))
                .transpose()?
                .ok_or_else(|| assembly_error("clip asset is missing or has no known duration"))?
        } else {
            adjustment_graph_at_revision(
                transaction,
                clip.asset_id(),
                clip.adjustment_revision_id(),
            )?
            .ok_or_else(|| assembly_error("clip adjustment revision does not belong to its asset"))?
            .graph
            .edit_timeline()
            .output_duration_millis()
        };
        if clip.source_end_millis() > source_duration {
            return Err(assembly_error(
                "clip source range exceeds its exact asset revision",
            ));
        }
    }
    Ok(())
}

fn sound_assembly_query<P: rusqlite::Params>(
    transaction: &Transaction<'_>,
    sql: &str,
    parameters: P,
) -> Result<Option<SoundAssemblyRevision>, CatalogError> {
    let stored = transaction
        .query_row(sql, parameters, |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })
        .optional()?;
    stored
        .map(
            |(revision_id, revision_number, document_json, created_at_millis)| {
                let assembly =
                    serde_json::from_str::<SoundAssembly>(&document_json).map_err(|error| {
                        assembly_error(format!("stored assembly document is invalid: {error}"))
                    })?;
                assembly.validate().map_err(|error| {
                    assembly_error(format!(
                        "stored assembly document violates its contract: {error}"
                    ))
                })?;
                Ok(SoundAssemblyRevision {
                    revision_id,
                    revision_number: stored_u32(revision_number, "assembly revision number")?,
                    assembly,
                    created_at_millis,
                })
            },
        )
        .transpose()
}

fn assembly_provenance(
    transaction: &Transaction<'_>,
    revision: &SoundAssemblyRevision,
) -> Result<String, CatalogError> {
    let mut sources = Vec::new();
    for track in revision.assembly.tracks() {
        for clip in track.clips() {
            let content_hash: String = transaction.query_row(
                "SELECT content_hash FROM assets WHERE id = ?1",
                [clip.asset_id().to_string()],
                |row| row.get(0),
            )?;
            sources.push(json!({
                "trackId": track.id().to_string(),
                "clipId": clip.id().to_string(),
                "assetId": clip.asset_id().to_string(),
                "originalContentHash": content_hash,
                "sourceRole": clip.source_role(),
                "adjustmentRevisionId": (clip.adjustment_revision_id() > 0)
                    .then_some(clip.adjustment_revision_id()),
                "sourceStartMillis": clip.source_start_millis(),
                "sourceEndMillis": clip.source_end_millis(),
                "timelineStartMillis": clip.timeline_start_millis(),
            }));
        }
    }
    serde_json::to_string(&json!({
        "schema": "echo.sound-assembly-export-provenance.v1",
        "assemblyId": revision.assembly.id().to_string(),
        "assemblyRevisionId": revision.revision_id,
        "assemblyRevisionNumber": revision.revision_number,
        "document": revision.assembly,
        "sourceDisclosure": crate::assembly_source_disclosure(&revision.assembly, &crate::source_disclosures(transaction)?),
        "sources": sources,
    }))
    .map_err(|error| assembly_error(format!("assembly provenance cannot be encoded: {error}")))
}

fn validate_export(input: &RecordSoundAssemblyExport<'_>) -> Result<(), CatalogError> {
    if input.sample_rate == 0
        || !matches!(input.channel_count, 1 | 2)
        || input.frame_count == 0
        || input.size_bytes == 0
        || input.output_path.as_os_str().is_empty()
        || input.created_at_millis < 0
        || !input.integrated_lufs.is_finite()
        || !input.true_peak_dbtp.is_finite()
    {
        return Err(assembly_error("assembly export facts are invalid"));
    }
    Ok(())
}

fn parse_assembly_id(value: &str) -> Result<SoundAssemblyId, CatalogError> {
    SoundAssemblyId::from_str(value)
        .map_err(|_| assembly_error("stored assembly identity is invalid"))
}

fn stored_u64(value: i64, label: &str) -> Result<u64, CatalogError> {
    u64::try_from(value).map_err(|_| assembly_error(format!("stored {label} is invalid")))
}

fn stored_u32(value: i64, label: &str) -> Result<u32, CatalogError> {
    u32::try_from(value).map_err(|_| assembly_error(format!("stored {label} is invalid")))
}

fn stored_usize(value: i64, label: &str) -> Result<usize, CatalogError> {
    usize::try_from(value).map_err(|_| assembly_error(format!("stored {label} is invalid")))
}

fn sqlite_u64(value: u64, label: &str) -> Result<i64, CatalogError> {
    i64::try_from(value).map_err(|_| assembly_error(format!("{label} exceeds SQLite range")))
}

fn sqlite_overflow(error: std::num::TryFromIntError) -> CatalogError {
    assembly_error(format!("value exceeds SQLite range: {error}"))
}

fn assembly_error(message: impl Into<String>) -> CatalogError {
    CatalogError::new(CatalogErrorKind::Other, message)
}

#[cfg(test)]
mod tests;
