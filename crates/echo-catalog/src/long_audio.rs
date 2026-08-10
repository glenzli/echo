//! Resumable long-recording analysis projection.
//!
//! Rows reference rebuildable content-addressed proxies and retain bounded
//! per-segment model results. Immutable Originals and user facts do not live
//! here; changing the plan version replaces this projection only.

use std::str::FromStr;

use echo_domain::{AssetId, ContentHash};
use rusqlite::Transaction;

use crate::CatalogError;

/// Stable plan row for one source time range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LongAudioSegmentPlan {
    pub index: u32,
    pub start_millis: u64,
    pub end_millis: u64,
}

/// Rebuildable proxy identity retained for one segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongAudioProxyRef {
    pub content_hash: ContentHash,
    pub size_bytes: u64,
}

/// Persisted state of one leaf segment.
#[derive(Debug, Clone, PartialEq)]
pub struct LongAudioSegment {
    pub asset_id: AssetId,
    pub plan_version: u32,
    pub index: u32,
    pub start_millis: u64,
    pub end_millis: u64,
    pub proxy: Option<LongAudioProxyRef>,
    pub transcript: Option<serde_json::Value>,
    pub alignment: Option<serde_json::Value>,
    pub contextual: Option<serde_json::Value>,
    pub updated_at_millis: i64,
}

/// One recursively aggregated contextual node.
#[derive(Debug, Clone, PartialEq)]
pub struct LongAudioOutlineNode {
    pub asset_id: AssetId,
    pub plan_version: u32,
    pub level: u32,
    pub index: u32,
    pub start_millis: u64,
    pub end_millis: u64,
    pub contextual: serde_json::Value,
    pub updated_at_millis: i64,
}

/// Per-segment inference stage whose accepted JSON is updated atomically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LongAudioStage {
    Transcript,
    Alignment,
    Contextual,
}

impl LongAudioStage {
    const fn column(self) -> &'static str {
        match self {
            Self::Transcript => "transcript_json",
            Self::Alignment => "alignment_json",
            Self::Contextual => "contextual_json",
        }
    }
}

/// Installs one versioned plan while preserving completed rows of the same
/// version. Other plan versions are discarded as rebuildable projections.
///
/// # Errors
///
/// Returns a catalog failure when rows are invalid or cannot be persisted.
pub fn ensure_long_audio_plan(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    plan_version: u32,
    segments: &[LongAudioSegmentPlan],
    now_millis: i64,
) -> Result<(), CatalogError> {
    validate_plan(segments)?;
    transaction.execute(
        "DELETE FROM long_audio_outline_nodes WHERE asset_id = ?1 AND plan_version <> ?2",
        rusqlite::params![asset_id.to_string(), i64::from(plan_version)],
    )?;
    transaction.execute(
        "DELETE FROM long_audio_segments WHERE asset_id = ?1 AND plan_version <> ?2",
        rusqlite::params![asset_id.to_string(), i64::from(plan_version)],
    )?;
    for segment in segments {
        transaction.execute(
            "INSERT INTO long_audio_segments \
             (asset_id, plan_version, segment_index, start_millis, end_millis, updated_at_millis) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6) ON CONFLICT DO NOTHING",
            rusqlite::params![
                asset_id.to_string(),
                i64::from(plan_version),
                i64::from(segment.index),
                sqlite_u64(segment.start_millis)?,
                sqlite_u64(segment.end_millis)?,
                now_millis,
            ],
        )?;
    }
    Ok(())
}

/// Publishes the cache identity for one generated segment proxy.
///
/// # Errors
///
/// Returns a catalog failure when the segment does not exist or the write is
/// rejected.
pub fn record_long_audio_proxy(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    plan_version: u32,
    segment_index: u32,
    proxy: &LongAudioProxyRef,
    now_millis: i64,
) -> Result<(), CatalogError> {
    let changed = transaction.execute(
        "UPDATE long_audio_segments SET proxy_content_hash = ?1, proxy_size_bytes = ?2, \
         updated_at_millis = ?3 WHERE asset_id = ?4 AND plan_version = ?5 AND segment_index = ?6",
        rusqlite::params![
            proxy.content_hash.to_string(),
            sqlite_u64(proxy.size_bytes)?,
            now_millis,
            asset_id.to_string(),
            i64::from(plan_version),
            i64::from(segment_index),
        ],
    )?;
    require_one_row(changed)
}

/// Commits one accepted segment-stage payload. The payload includes its own
/// sanitized Runtime provenance and is never logged by this owner.
///
/// # Errors
///
/// Returns a catalog failure when serialization or the update fails.
pub fn record_long_audio_stage(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    plan_version: u32,
    segment_index: u32,
    stage: LongAudioStage,
    value: &serde_json::Value,
    now_millis: i64,
) -> Result<(), CatalogError> {
    let encoded = serde_json::to_string(value)
        .map_err(|error| invalid(&format!("cannot encode long-audio stage: {error}")))?;
    let statement = format!(
        "UPDATE long_audio_segments SET {} = ?1, updated_at_millis = ?2 \
         WHERE asset_id = ?3 AND plan_version = ?4 AND segment_index = ?5",
        stage.column()
    );
    let changed = transaction.execute(
        &statement,
        rusqlite::params![
            encoded,
            now_millis,
            asset_id.to_string(),
            i64::from(plan_version),
            i64::from(segment_index),
        ],
    )?;
    require_one_row(changed)
}

/// Reads all leaf segments in source order.
///
/// # Errors
///
/// Returns a catalog failure when stored identities or payloads are invalid.
pub fn list_long_audio_segments(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    plan_version: u32,
) -> Result<Vec<LongAudioSegment>, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT segment_index, start_millis, end_millis, proxy_content_hash, proxy_size_bytes, \
         transcript_json, alignment_json, contextual_json, updated_at_millis \
         FROM long_audio_segments WHERE asset_id = ?1 AND plan_version = ?2 \
         ORDER BY segment_index",
    )?;
    let rows = statement.query_map(
        rusqlite::params![asset_id.to_string(), i64::from(plan_version)],
        |row| {
            let hash = row.get::<_, Option<String>>(3)?;
            let size = row.get::<_, Option<i64>>(4)?;
            let proxy = match (hash, size) {
                (Some(hash), Some(size)) => Some(LongAudioProxyRef {
                    content_hash: ContentHash::from_str(&hash).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?,
                    size_bytes: row_u64(size, 4)?,
                }),
                (None, None) => None,
                _ => unreachable!("schema check keeps proxy identity complete"),
            };
            Ok(LongAudioSegment {
                asset_id,
                plan_version,
                index: row_u32(row.get(0)?, 0)?,
                start_millis: row_u64(row.get(1)?, 1)?,
                end_millis: row_u64(row.get(2)?, 2)?,
                proxy,
                transcript: optional_json(row.get(5)?, 5)?,
                alignment: optional_json(row.get(6)?, 6)?,
                contextual: optional_json(row.get(7)?, 7)?,
                updated_at_millis: row.get(8)?,
            })
        },
    )?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(CatalogError::from)
}

/// Publishes or replaces one derived outline node.
///
/// # Errors
///
/// Returns a catalog failure when serialization or the write fails.
pub fn upsert_long_audio_outline_node(
    transaction: &Transaction<'_>,
    node: &LongAudioOutlineNode,
) -> Result<(), CatalogError> {
    let encoded = serde_json::to_string(&node.contextual)
        .map_err(|error| invalid(&format!("cannot encode long-audio outline: {error}")))?;
    transaction.execute(
        "INSERT INTO long_audio_outline_nodes \
         (asset_id, plan_version, level, node_index, start_millis, end_millis, contextual_json, \
          updated_at_millis) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
         ON CONFLICT(asset_id, plan_version, level, node_index) DO UPDATE SET \
         start_millis = excluded.start_millis, end_millis = excluded.end_millis, \
         contextual_json = excluded.contextual_json, updated_at_millis = excluded.updated_at_millis",
        rusqlite::params![
            node.asset_id.to_string(),
            i64::from(node.plan_version),
            i64::from(node.level),
            i64::from(node.index),
            sqlite_u64(node.start_millis)?,
            sqlite_u64(node.end_millis)?,
            encoded,
            node.updated_at_millis,
        ],
    )?;
    Ok(())
}

/// Reads every outline node, leaf level first and then source order.
///
/// # Errors
///
/// Returns a catalog failure when stored payloads are invalid.
pub fn list_long_audio_outline_nodes(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    plan_version: u32,
) -> Result<Vec<LongAudioOutlineNode>, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT level, node_index, start_millis, end_millis, contextual_json, updated_at_millis \
         FROM long_audio_outline_nodes WHERE asset_id = ?1 AND plan_version = ?2 \
         ORDER BY level, node_index",
    )?;
    let rows = statement.query_map(
        rusqlite::params![asset_id.to_string(), i64::from(plan_version)],
        |row| {
            let contextual: String = row.get(4)?;
            Ok(LongAudioOutlineNode {
                asset_id,
                plan_version,
                level: row_u32(row.get(0)?, 0)?,
                index: row_u32(row.get(1)?, 1)?,
                start_millis: row_u64(row.get(2)?, 2)?,
                end_millis: row_u64(row.get(3)?, 3)?,
                contextual: serde_json::from_str(&contextual).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?,
                updated_at_millis: row.get(5)?,
            })
        },
    )?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(CatalogError::from)
}

fn validate_plan(segments: &[LongAudioSegmentPlan]) -> Result<(), CatalogError> {
    if segments.is_empty() {
        return Err(invalid("long-audio plan must contain at least one segment"));
    }
    let mut expected_start = 0u64;
    for (position, segment) in segments.iter().enumerate() {
        if usize::try_from(segment.index).ok() != Some(position)
            || segment.start_millis != expected_start
            || segment.end_millis <= segment.start_millis
        {
            return Err(invalid("long-audio plan must be contiguous and zero-based"));
        }
        expected_start = segment.end_millis;
    }
    Ok(())
}

fn require_one_row(changed: usize) -> Result<(), CatalogError> {
    if changed == 1 {
        Ok(())
    } else {
        Err(invalid("long-audio segment does not exist"))
    }
}

fn sqlite_u64(value: u64) -> Result<i64, CatalogError> {
    i64::try_from(value).map_err(|error| invalid(&format!("value exceeds SQLite range: {error}")))
}

fn row_u64(value: i64, column: usize) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}

fn row_u32(value: i64, column: usize) -> rusqlite::Result<u32> {
    u32::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}

fn optional_json(
    value: Option<String>,
    column: usize,
) -> rusqlite::Result<Option<serde_json::Value>> {
    value
        .map(|value| {
            serde_json::from_str(&value).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    column,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .transpose()
}

fn invalid(message: &str) -> CatalogError {
    crate::CatalogError::new(crate::CatalogErrorKind::Other, message)
}

#[cfg(test)]
mod tests;
