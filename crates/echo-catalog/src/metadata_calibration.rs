//! Append-only user calibration over immutable model analysis.
//!
//! This owner computes a sparse override against the current model projection,
//! persists it as a revision, and refreshes the literal search surfaces that
//! must immediately reflect the user's correction.

use echo_domain::{AssetId, MetadataCalibration, MetadataField, MetadataFields};
use rusqlite::{OptionalExtension, Transaction};

use crate::{CatalogError, CatalogErrorKind, index_transcript, remove_transcript_index};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataCalibrationRevision {
    pub revision_id: i64,
    pub asset_id: AssetId,
    pub calibration: MetadataCalibration,
    pub created_at_millis: i64,
}

/// Appends the exact sparse correction needed to produce `desired` from the
/// current model evidence. A revision containing no fields is an explicit
/// reset to model values.
///
/// # Errors
///
/// Returns a catalog failure for an unknown asset, invalid user values, or a
/// failed atomic write.
pub fn calibrate_asset_metadata(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    desired: MetadataFields,
    explicit_fields: &[MetadataField],
    created_at_millis: i64,
) -> Result<MetadataCalibrationRevision, CatalogError> {
    if created_at_millis < 0 {
        return Err(invalid("metadata calibration time must be non-negative"));
    }
    let model = model_metadata_fields(transaction, asset_id)?;
    let calibration =
        MetadataCalibration::between_with_explicit_fields(model.clone(), desired, explicit_fields)
            .map_err(|error| invalid(error.message()))?;
    let keywords_json = calibration
        .keywords
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|error| invalid(format!("cannot encode calibrated keywords: {error}")))?;
    transaction.execute(
        "INSERT INTO metadata_calibration_revisions (asset_id, sound_caption, summary, \
         event_type, mood, keywords_json, transcript_text, language, created_at_millis) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        rusqlite::params![
            asset_id.to_string(),
            calibration.sound_caption,
            calibration.summary,
            calibration.event_type,
            calibration.mood,
            keywords_json,
            calibration.transcript_text,
            calibration.language,
            created_at_millis,
        ],
    )?;
    let revision = MetadataCalibrationRevision {
        revision_id: transaction.last_insert_rowid(),
        asset_id,
        calibration,
        created_at_millis,
    };
    refresh_literal_search(transaction, &model, &revision)?;
    Ok(revision)
}

/// Returns the newest calibration revision for an asset.
///
/// # Errors
///
/// Returns a catalog failure when the stored revision cannot be decoded.
pub fn latest_metadata_calibration(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
) -> Result<Option<MetadataCalibrationRevision>, CatalogError> {
    transaction
        .query_row(
            "SELECT id, sound_caption, summary, event_type, mood, keywords_json, \
             transcript_text, language, created_at_millis \
             FROM metadata_calibration_revisions WHERE asset_id = ?1 \
             ORDER BY id DESC LIMIT 1",
            [asset_id.to_string()],
            |row| calibration_revision_from_row(row, asset_id),
        )
        .optional()
        .map_err(CatalogError::from)
}

pub(crate) fn model_metadata_fields(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
) -> Result<MetadataFields, CatalogError> {
    transaction
        .query_row(
            "SELECT \
             COALESCE(json_extract(contextual.value, '$.sound_caption'), ''), \
             COALESCE(json_extract(contextual.value, '$.summary'), ''), \
             COALESCE((SELECT f.display_value FROM contextual_browse_facets f \
               WHERE f.asset_id = a.id AND f.facet_kind = 'event' \
               ORDER BY f.analysis_record_id DESC LIMIT 1), ''), \
             COALESCE((SELECT f.display_value FROM contextual_browse_facets f \
               WHERE f.asset_id = a.id AND f.facet_kind = 'mood' \
               ORDER BY f.analysis_record_id DESC LIMIT 1), ''), \
             COALESCE((SELECT json_group_array(display_value) FROM (\
               SELECT f.display_value FROM contextual_browse_facets f \
               WHERE f.asset_id = a.id AND f.facet_kind = 'keyword' \
                 AND f.analysis_record_id = (SELECT MAX(latest.analysis_record_id) \
                   FROM contextual_browse_facets latest WHERE latest.asset_id = a.id \
                   AND latest.facet_kind = 'keyword') ORDER BY f.normalized_value)), '[]'), \
             COALESCE(json_extract(transcript.value, '$.text'), ''), \
             COALESCE(json_extract(transcript.value, '$.language'), '') \
             FROM assets a \
             LEFT JOIN analysis_records contextual ON contextual.id = (SELECT candidate.id \
               FROM analysis_records candidate WHERE candidate.asset_id = a.id \
               AND candidate.kind = 'contextual' ORDER BY candidate.id DESC LIMIT 1) \
             LEFT JOIN analysis_records transcript ON transcript.id = (SELECT candidate.id \
               FROM analysis_records candidate WHERE candidate.asset_id = a.id \
               AND candidate.kind = 'transcript' ORDER BY candidate.id DESC LIMIT 1) \
             WHERE a.id = ?1",
            [asset_id.to_string()],
            |row| {
                let keywords_json: String = row.get(4)?;
                Ok(MetadataFields {
                    sound_caption: row.get(0)?,
                    summary: row.get(1)?,
                    event_type: row.get(2)?,
                    mood: row.get(3)?,
                    keywords: serde_json::from_str(&keywords_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?,
                    transcript_text: row.get(5)?,
                    language: row.get(6)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| invalid(format!("asset {asset_id} does not exist")))?
        .normalized()
        .map_err(|error| invalid(error.message()))
}

pub(crate) fn metadata_fields_from_projection(
    contextual: Option<&serde_json::Value>,
    keywords: Vec<String>,
    mood: Option<String>,
    event_type: Option<String>,
    transcript: Option<&serde_json::Value>,
) -> MetadataFields {
    MetadataFields {
        sound_caption: contextual
            .and_then(|value| value.get("sound_caption"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        summary: contextual
            .and_then(|value| value.get("summary"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        event_type: event_type.unwrap_or_default(),
        mood: mood.unwrap_or_default(),
        keywords,
        transcript_text: transcript
            .and_then(|value| value.get("text"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        language: transcript
            .and_then(|value| value.get("language"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    }
}

pub(crate) fn calibration_revision_from_row(
    row: &rusqlite::Row<'_>,
    asset_id: AssetId,
) -> rusqlite::Result<MetadataCalibrationRevision> {
    let keywords_json = row.get::<_, Option<String>>(5)?;
    Ok(MetadataCalibrationRevision {
        revision_id: row.get(0)?,
        asset_id,
        calibration: MetadataCalibration {
            sound_caption: row.get(1)?,
            summary: row.get(2)?,
            event_type: row.get(3)?,
            mood: row.get(4)?,
            keywords: keywords_json
                .map(|encoded| {
                    serde_json::from_str(&encoded).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            5,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })
                })
                .transpose()?,
            transcript_text: row.get(6)?,
            language: row.get(7)?,
        },
        created_at_millis: row.get(8)?,
    })
}

fn refresh_literal_search(
    transaction: &Transaction<'_>,
    model: &MetadataFields,
    revision: &MetadataCalibrationRevision,
) -> Result<(), CatalogError> {
    let effective = revision.calibration.apply_to(model.clone());
    if effective.transcript_text.is_empty() {
        remove_transcript_index(transaction, &revision.asset_id.to_string())?;
    } else {
        index_transcript(
            transaction,
            &revision.asset_id.to_string(),
            &effective.transcript_text,
        )?;
    }
    transaction.execute(
        "DELETE FROM semantic_documents WHERE asset_id = ?1",
        [revision.asset_id.to_string()],
    )?;
    transaction.execute(
        "DELETE FROM semantic_document_fts WHERE asset_id = ?1",
        [revision.asset_id.to_string()],
    )?;
    if let Some(source) = crate::semantic_source(transaction, revision.asset_id)? {
        crate::index_semantic_source_text(transaction, &source)?;
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> CatalogError {
    CatalogError::new(CatalogErrorKind::Other, message)
}

#[cfg(test)]
mod tests;
