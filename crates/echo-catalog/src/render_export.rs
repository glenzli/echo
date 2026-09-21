//! Durable provenance for user-published renders. Unlike cache artifacts,
//! these files are user deliverables and are never rebuilt or deleted by
//! cache maintenance.

use std::{path::PathBuf, str::FromStr};

use echo_domain::{AssetId, ContentHash};
use rusqlite::{OptionalExtension, Transaction};

use crate::{CatalogError, CatalogErrorKind, latest_adjustment_graph};

/// Stable on-disk export format identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderExportFormat {
    WavPcm16,
    WavPcm24,
    Flac24,
}

/// Complete evidence supplied after an atomic render publication.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordRenderExport {
    pub asset_id: AssetId,
    /// Zero identifies the immutable original with no saved adjustment.
    pub adjustment_revision_id: i64,
    pub output_path: PathBuf,
    pub format: RenderExportFormat,
    pub sample_rate: u32,
    pub channel_count: u32,
    pub bit_depth: u16,
    pub frame_count: u64,
    pub content_hash: ContentHash,
    pub size_bytes: u64,
    pub integrated_lufs: f32,
    pub true_peak_dbtp: f32,
    pub created_at_millis: i64,
}

/// One durable publication record.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderExportRecord {
    pub id: i64,
    pub evidence: RecordRenderExport,
}

/// Records a delivery rendered from one active post-effect spectral working
/// copy. The snapshot belongs to the publication, not the mutable cache: later
/// destructive operations must not rewrite what an already-exported file means.
pub fn record_rendered_spectral_working_copy_export(
    transaction: &Transaction<'_>,
    evidence: &RecordRenderExport,
    working_copy_id: i64,
) -> Result<RenderExportRecord, CatalogError> {
    if working_copy_id <= 0 {
        return Err(invalid(
            "rendered spectral working-copy identity is invalid",
        ));
    }
    let original_content_hash: String = transaction.query_row(
        "SELECT content_hash FROM assets WHERE id = ?1",
        [evidence.asset_id.to_string()],
        |row| row.get(0),
    )?;
    let (
        parent_adjustment_revision_id,
        parent_render_content_hash,
        working_render_content_hash,
        tile_manifest_json,
        tool_version,
        enabled,
    ): (Option<i64>, String, String, String, String, i64) = transaction.query_row(
        "SELECT parent_adjustment_revision_id, parent_render_content_hash, \
         working_render_content_hash, tile_manifest_json, tool_version, enabled \
         FROM rendered_spectral_working_copies WHERE id = ?1 AND asset_id = ?2",
        rusqlite::params![working_copy_id, evidence.asset_id.to_string()],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            ))
        },
    )?;
    if enabled != 1 || parent_adjustment_revision_id.unwrap_or(0) != evidence.adjustment_revision_id
    {
        return Err(invalid(
            "rendered spectral working copy is not the active current render",
        ));
    }

    let record = record_render_export(transaction, evidence)?;
    let existing = transaction
        .query_row(
            "SELECT working_copy_id, original_content_hash, parent_render_content_hash, \
             working_render_content_hash, tile_manifest_json, tool_version \
             FROM render_export_working_copy_provenance WHERE render_export_id = ?1",
            [record.id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()?;
    let snapshot = (
        working_copy_id,
        original_content_hash,
        parent_render_content_hash,
        working_render_content_hash,
        tile_manifest_json,
        tool_version,
    );
    if let Some(existing) = existing {
        if existing != snapshot {
            return Err(invalid(
                "render export already has different rendered spectral working-copy provenance",
            ));
        }
        return Ok(record);
    }
    transaction.execute(
        "INSERT INTO render_export_working_copy_provenance \
         (render_export_id, working_copy_id, original_content_hash, parent_render_content_hash, \
          working_render_content_hash, tile_manifest_json, tool_version) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            record.id, snapshot.0, snapshot.1, snapshot.2, snapshot.3, snapshot.4, snapshot.5,
        ],
    )?;
    Ok(record)
}

/// Records a render only when its adjustment revision is still the newest
/// saved graph for the source asset.
///
/// # Errors
///
/// Returns a constraint error for stale or foreign adjustment identity and a
/// catalog error when the row cannot be written.
pub fn record_render_export(
    transaction: &Transaction<'_>,
    evidence: &RecordRenderExport,
) -> Result<RenderExportRecord, CatalogError> {
    validate_revision(
        transaction,
        evidence.asset_id,
        evidence.adjustment_revision_id,
    )?;
    if evidence.sample_rate == 0
        || evidence.channel_count == 0
        || !format_matches_bit_depth(evidence.format, evidence.bit_depth)
        || evidence.frame_count == 0
        || evidence.size_bytes == 0
        || !evidence.integrated_lufs.is_finite()
        || !evidence.true_peak_dbtp.is_finite()
    {
        return Err(invalid(
            "render export evidence is outside the supported range",
        ));
    }
    let summary =
        crate::source_disclosure::asset_source_disclosure(transaction, evidence.asset_id)?;
    let disclosure_json = serde_json::to_string(&summary).map_err(|e| invalid(&e.to_string()))?;
    let revision =
        (evidence.adjustment_revision_id != 0).then_some(evidence.adjustment_revision_id);
    let existing = transaction
        .query_row(
            "SELECT id FROM render_exports WHERE asset_id = ?1 \
             AND adjustment_revision_id IS ?2 AND output_path = ?3 AND format = ?4 \
             AND sample_rate = ?5 AND channel_count = ?6 AND bit_depth = ?7 \
             AND frame_count = ?8 AND content_hash = ?9 AND size_bytes = ?10 ORDER BY id DESC LIMIT 1",
            rusqlite::params![
                evidence.asset_id.to_string(),
                revision,
                evidence.output_path.to_string_lossy(),
                format_text(evidence.format),
                i64::from(evidence.sample_rate),
                i64::from(evidence.channel_count),
                i64::from(evidence.bit_depth),
                i64::try_from(evidence.frame_count)
                    .map_err(|_| invalid("frame count is too large"))?,
                evidence.content_hash.to_string(),
                i64::try_from(evidence.size_bytes).map_err(|_| invalid("size is too large"))?,
            ],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    if let Some(id) = existing {
        let previous: Option<String> = transaction
            .query_row(
                "SELECT disclosure_json FROM render_export_disclosures WHERE render_export_id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()?;
        if previous.as_deref() == Some(disclosure_json.as_str()) {
            return Ok(RenderExportRecord {
                id,
                evidence: evidence.clone(),
            });
        }
    }
    transaction.execute(
        "INSERT INTO render_exports (asset_id, adjustment_revision_id, output_path, format, \
         sample_rate, channel_count, bit_depth, frame_count, content_hash, size_bytes, \
         integrated_lufs, true_peak_dbtp, created_at_millis) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        rusqlite::params![
            evidence.asset_id.to_string(),
            revision,
            evidence.output_path.to_string_lossy(),
            format_text(evidence.format),
            i64::from(evidence.sample_rate),
            i64::from(evidence.channel_count),
            i64::from(evidence.bit_depth),
            i64::try_from(evidence.frame_count).map_err(|_| invalid("frame count is too large"))?,
            evidence.content_hash.to_string(),
            i64::try_from(evidence.size_bytes).map_err(|_| invalid("size is too large"))?,
            evidence.integrated_lufs,
            evidence.true_peak_dbtp,
            evidence.created_at_millis,
        ],
    )?;
    let id = transaction.last_insert_rowid();
    transaction.execute(
        "INSERT INTO render_export_disclosures(render_export_id,disclosure_json) VALUES(?1,?2)",
        rusqlite::params![id, disclosure_json],
    )?;
    Ok(RenderExportRecord {
        id,
        evidence: evidence.clone(),
    })
}

/// Lists newest publications for one asset.
///
/// # Errors
///
/// Returns a catalog failure when a stored row is malformed or unreadable.
pub fn list_render_exports(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
) -> Result<Vec<RenderExportRecord>, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT id, adjustment_revision_id, output_path, format, sample_rate, channel_count, \
         bit_depth, frame_count, content_hash, size_bytes, integrated_lufs, true_peak_dbtp, \
         created_at_millis FROM render_exports WHERE asset_id = ?1 \
         ORDER BY created_at_millis DESC, id DESC",
    )?;
    let rows = statement.query_map([asset_id.to_string()], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<i64>>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
            row.get::<_, i64>(5)?,
            row.get::<_, i64>(6)?,
            row.get::<_, i64>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, i64>(9)?,
            row.get::<_, f32>(10)?,
            row.get::<_, f32>(11)?,
            row.get::<_, i64>(12)?,
        ))
    })?;
    rows.map(|row| {
        let (
            id,
            revision,
            output_path,
            format,
            sample_rate,
            channel_count,
            bit_depth,
            frame_count,
            content_hash,
            size_bytes,
            integrated_lufs,
            true_peak_dbtp,
            created_at_millis,
        ) = row?;
        Ok(RenderExportRecord {
            id,
            evidence: RecordRenderExport {
                asset_id,
                adjustment_revision_id: revision.unwrap_or(0),
                output_path: PathBuf::from(output_path),
                format: parse_format(&format)?,
                sample_rate: u32::try_from(sample_rate).map_err(|_| malformed("sample rate"))?,
                channel_count: u32::try_from(channel_count)
                    .map_err(|_| malformed("channel count"))?,
                bit_depth: u16::try_from(bit_depth).map_err(|_| malformed("bit depth"))?,
                frame_count: u64::try_from(frame_count).map_err(|_| malformed("frame count"))?,
                content_hash: ContentHash::from_str(&content_hash)
                    .map_err(|_| malformed("content hash"))?,
                size_bytes: u64::try_from(size_bytes).map_err(|_| malformed("size"))?,
                integrated_lufs,
                true_peak_dbtp,
                created_at_millis,
            },
        })
    })
    .collect()
}

fn validate_revision(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    revision_id: i64,
) -> Result<(), CatalogError> {
    let latest = latest_adjustment_graph(transaction, asset_id)?;
    if matches!((revision_id, latest.as_ref()), (0, None))
        || latest.is_some_and(|revision| revision.revision_id == revision_id)
    {
        return Ok(());
    }
    Err(invalid(
        "render export must reference the newest saved adjustment revision",
    ))
}

const fn format_text(format: RenderExportFormat) -> &'static str {
    match format {
        RenderExportFormat::WavPcm16 => "wav_pcm16",
        RenderExportFormat::WavPcm24 => "wav_pcm24",
        RenderExportFormat::Flac24 => "flac24",
    }
}

fn parse_format(text: &str) -> Result<RenderExportFormat, CatalogError> {
    match text {
        "wav_pcm16" => Ok(RenderExportFormat::WavPcm16),
        "wav_pcm24" => Ok(RenderExportFormat::WavPcm24),
        "flac24" => Ok(RenderExportFormat::Flac24),
        _ => Err(malformed("format")),
    }
}

const fn format_matches_bit_depth(format: RenderExportFormat, bit_depth: u16) -> bool {
    matches!(
        (format, bit_depth),
        (RenderExportFormat::WavPcm16, 16)
            | (
                RenderExportFormat::WavPcm24 | RenderExportFormat::Flac24,
                24
            )
    )
}

fn invalid(message: &str) -> CatalogError {
    CatalogError::new(CatalogErrorKind::Constraint, message)
}

fn malformed(field: &str) -> CatalogError {
    CatalogError::new(
        CatalogErrorKind::Other,
        format!("malformed render export {field}"),
    )
}

#[cfg(test)]
mod tests;
