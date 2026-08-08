//! Append-only analysis evidence in the catalog.
//!
//! Records are never updated or deleted: a model upgrade writes a new record.
//! Readers project the newest record per kind when a single value is needed.

use echo_domain::{AnalysisKind, AnalysisLevel, AnalysisRecord, AssetId, ModelIdentity};
use rusqlite::Transaction;

use crate::error::CatalogError;

/// A prepared append of one analysis record.
#[derive(Debug, Clone)]
pub struct AppendAnalysisRecord {
    pub asset_id: AssetId,
    pub record: AnalysisRecord,
}

/// Appends one analysis record and lifts the asset's highest satisfied level.
///
/// The caller owns job orchestration: this function only persists evidence and
/// the level projection.
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn record_analysis(
    transaction: &Transaction<'_>,
    append: &AppendAnalysisRecord,
) -> Result<(), CatalogError> {
    transaction.execute(
        "INSERT INTO analysis_records (asset_id, kind, value, model, model_version, confidence, \
         recorded_at_millis) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            append.asset_id.to_string(),
            kind_text(append.record.kind),
            append.record.value.to_string(),
            append.record.model.name,
            append.record.model.version,
            append.record.confidence,
            append.record.recorded_at_millis,
        ],
    )?;
    transaction.execute(
        "UPDATE asset_levels SET max_level = MAX(max_level, ?1) WHERE asset_id = ?2",
        rusqlite::params![
            minimum_level_for(append.record.kind) as u8,
            append.asset_id.to_string()
        ],
    )?;
    Ok(())
}

/// Query failure vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisQueryErrorKind {
    /// The stored JSON payload of a record could not be parsed.
    CorruptPayload,
}

#[derive(Debug, thiserror::Error)]
#[error("analysis query failed: {kind:?}: {message}")]
pub struct AnalysisQueryError {
    pub kind: AnalysisQueryErrorKind,
    pub message: String,
}

impl From<CatalogError> for AnalysisQueryError {
    fn from(error: CatalogError) -> Self {
        Self {
            kind: AnalysisQueryErrorKind::CorruptPayload,
            message: error.message,
        }
    }
}

impl From<rusqlite::Error> for AnalysisQueryError {
    fn from(error: rusqlite::Error) -> Self {
        Self {
            kind: AnalysisQueryErrorKind::CorruptPayload,
            message: error.to_string(),
        }
    }
}

/// Reads every analysis record for an asset, newest first.
///
/// # Errors
///
/// Returns [`AnalysisQueryError`] when stored payloads are not valid JSON.
pub fn query_analysis(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
) -> Result<Vec<AnalysisRecord>, AnalysisQueryError> {
    let mut statement = transaction.prepare(
        "SELECT kind, value, model, model_version, confidence, recorded_at_millis \
         FROM analysis_records WHERE asset_id = ?1 ORDER BY id DESC",
    )?;
    let rows = statement.query_map([asset_id.to_string()], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<f64>>(4)?,
            row.get::<_, i64>(5)?,
        ))
    })?;
    let mut records = Vec::new();
    for row in rows {
        let (kind, value, model, model_version, confidence, recorded_at) = row?;
        let value = serde_json::from_str(&value).map_err(|error| AnalysisQueryError {
            kind: AnalysisQueryErrorKind::CorruptPayload,
            message: error.to_string(),
        })?;
        records.push(AnalysisRecord {
            kind: parse_kind(&kind).ok_or_else(|| AnalysisQueryError {
                kind: AnalysisQueryErrorKind::CorruptPayload,
                message: format!("unknown analysis kind {kind}"),
            })?,
            value,
            model: ModelIdentity::new(model, model_version),
            confidence,
            recorded_at_millis: recorded_at,
        });
    }
    Ok(records)
}

const fn minimum_level_for(kind: AnalysisKind) -> AnalysisLevel {
    match kind {
        AnalysisKind::Transcript => AnalysisLevel::Asr,
        AnalysisKind::Speakers | AnalysisKind::Emotions | AnalysisKind::AudioEvents => {
            AnalysisLevel::Understanding
        }
        AnalysisKind::Embedding | AnalysisKind::Semantic => AnalysisLevel::SemanticIndex,
        AnalysisKind::Contextual => AnalysisLevel::Contextual,
    }
}

const fn kind_text(kind: AnalysisKind) -> &'static str {
    match kind {
        AnalysisKind::Transcript => "transcript",
        AnalysisKind::Speakers => "speakers",
        AnalysisKind::Emotions => "emotions",
        AnalysisKind::AudioEvents => "audio_events",
        AnalysisKind::Embedding => "embedding",
        AnalysisKind::Semantic => "semantic",
        AnalysisKind::Contextual => "contextual",
    }
}

fn parse_kind(text: &str) -> Option<AnalysisKind> {
    match text {
        "transcript" => Some(AnalysisKind::Transcript),
        "speakers" => Some(AnalysisKind::Speakers),
        "emotions" => Some(AnalysisKind::Emotions),
        "audio_events" => Some(AnalysisKind::AudioEvents),
        "embedding" => Some(AnalysisKind::Embedding),
        "semantic" => Some(AnalysisKind::Semantic),
        "contextual" => Some(AnalysisKind::Contextual),
        _ => None,
    }
}
