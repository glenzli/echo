//! Durable references from assets to rebuildable cache artifacts.
//!
//! Artifact bytes remain in the content-addressed cache. The catalog stores
//! only enough identity and schema information to reuse or rebuild them.

use std::str::FromStr;

use echo_domain::{AssetId, ContentHash};
use rusqlite::{OptionalExtension, Transaction};

use crate::{CatalogError, CatalogErrorKind};

/// Rebuildable artifact family owned by a producer workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DerivedArtifactKind {
    /// Multi-resolution min/max waveform pyramid.
    WaveformPyramid,
}

/// One catalog reference to content-addressed artifact bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedArtifactRecord {
    pub asset_id: AssetId,
    pub kind: DerivedArtifactKind,
    pub schema_version: u32,
    pub content_hash: ContentHash,
    pub size_bytes: u64,
    pub created_at_millis: i64,
}

/// Reads the current reference for one artifact schema.
///
/// # Errors
///
/// Returns a catalog failure when the row is malformed or cannot be read.
pub fn find_derived_artifact(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    kind: DerivedArtifactKind,
    schema_version: u32,
) -> Result<Option<DerivedArtifactRecord>, CatalogError> {
    let row: Option<(String, i64, String, i64, i64)> = transaction
        .query_row(
            "SELECT kind, schema_version, content_hash, size_bytes, created_at_millis \
             FROM derived_artifacts WHERE asset_id = ?1 AND kind = ?2 AND schema_version = ?3",
            rusqlite::params![
                asset_id.to_string(),
                kind_text(kind),
                i64::from(schema_version)
            ],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .optional()?;
    let Some((kind_text, schema_version, content_hash, size_bytes, created_at_millis)) = row else {
        return Ok(None);
    };
    Ok(Some(DerivedArtifactRecord {
        asset_id,
        kind: parse_kind(&kind_text)?,
        schema_version: u32::try_from(schema_version).map_err(|_| malformed("schema version"))?,
        content_hash: ContentHash::from_str(&content_hash)
            .map_err(|_| malformed("content hash"))?,
        size_bytes: u64::try_from(size_bytes).map_err(|_| malformed("size"))?,
        created_at_millis,
    }))
}

/// Publishes or replaces the current reference for one artifact schema.
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn upsert_derived_artifact(
    transaction: &Transaction<'_>,
    record: &DerivedArtifactRecord,
) -> Result<(), CatalogError> {
    transaction.execute(
        "INSERT INTO derived_artifacts \
         (asset_id, kind, schema_version, content_hash, size_bytes, created_at_millis) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6) \
         ON CONFLICT(asset_id, kind, schema_version) DO UPDATE SET \
         content_hash = excluded.content_hash, size_bytes = excluded.size_bytes, \
         created_at_millis = excluded.created_at_millis",
        rusqlite::params![
            record.asset_id.to_string(),
            kind_text(record.kind),
            i64::from(record.schema_version),
            record.content_hash.to_string(),
            i64::try_from(record.size_bytes).map_err(|_| malformed("size"))?,
            record.created_at_millis,
        ],
    )?;
    Ok(())
}

/// Removes a stale reference so the next read rebuilds the artifact.
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn remove_derived_artifact(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    kind: DerivedArtifactKind,
    schema_version: u32,
) -> Result<(), CatalogError> {
    transaction.execute(
        "DELETE FROM derived_artifacts WHERE asset_id = ?1 AND kind = ?2 AND schema_version = ?3",
        rusqlite::params![
            asset_id.to_string(),
            kind_text(kind),
            i64::from(schema_version)
        ],
    )?;
    Ok(())
}

const fn kind_text(kind: DerivedArtifactKind) -> &'static str {
    match kind {
        DerivedArtifactKind::WaveformPyramid => "waveform_pyramid",
    }
}

fn parse_kind(text: &str) -> Result<DerivedArtifactKind, CatalogError> {
    match text {
        "waveform_pyramid" => Ok(DerivedArtifactKind::WaveformPyramid),
        _ => Err(malformed("kind")),
    }
}

fn malformed(field: &str) -> CatalogError {
    CatalogError::new(
        CatalogErrorKind::Other,
        format!("malformed derived artifact {field}"),
    )
}

#[cfg(test)]
mod tests;
