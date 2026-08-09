//! Audio Space projection: assets joined with their newest contextual
//! evidence (summary, keywords, mood, place, event) for the sound-album
//! surface.

use rusqlite::Transaction;

use crate::error::CatalogError;

/// One asset projected for the Audio Space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioSpaceAsset {
    pub id: String,
    pub path: std::path::PathBuf,
    pub codec: Option<String>,
    pub duration_millis: Option<u64>,
    pub recorded_at_millis: Option<i64>,
    pub imported_at_millis: i64,
    pub path_status: String,
    pub max_level: u8,
    pub liked: bool,
    pub rating: u8,
    /// Latest contextual payload JSON (absent when not analyzed yet).
    pub contextual: Option<serde_json::Value>,
    /// Latest model-extracted text payload JSON.
    pub transcript: Option<serde_json::Value>,
    /// Metadata extracted from the immutable source container.
    pub source_metadata: Option<crate::SourceMetadata>,
}

/// Lists every asset with its newest contextual evidence, newest import
/// first.
///
/// # Panics
///
/// Panics when a stored duration is negative or a contextual payload is not
/// valid JSON (corrupt rows are treated as fatal in this projection).
///
/// # Errors
///
/// Returns a catalog failure when the read cannot be applied.
pub fn list_audio_space(
    transaction: &Transaction<'_>,
) -> Result<Vec<AudioSpaceAsset>, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT a.id, a.path, a.codec, a.duration_millis, a.recorded_at_millis, \
         a.imported_at_millis, a.path_status, \
         (SELECT max_level FROM asset_levels WHERE asset_id = a.id), \
         (SELECT value FROM analysis_records r WHERE r.asset_id = a.id \
          AND r.kind = 'contextual' ORDER BY r.id DESC LIMIT 1), \
         (SELECT value FROM analysis_records r WHERE r.asset_id = a.id \
          AND r.kind = 'transcript' ORDER BY r.id DESC LIMIT 1), \
         COALESCE(u.liked, 0), COALESCE(u.rating, 0), \
         m.container_format, m.sample_rate, m.channel_count, m.entries_json \
         FROM assets a LEFT JOIN asset_user_state u ON u.asset_id = a.id \
         LEFT JOIN asset_source_metadata m ON m.asset_id = a.id \
         ORDER BY a.imported_at_millis DESC, a.id DESC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(AudioSpaceAsset {
            id: row.get(0)?,
            path: row.get::<_, String>(1)?.into(),
            codec: row.get(2)?,
            duration_millis: row
                .get::<_, Option<i64>>(3)?
                .map(|millis| u64::try_from(millis).expect("stored duration is non-negative")),
            recorded_at_millis: row.get(4)?,
            imported_at_millis: row.get(5)?,
            path_status: row.get(6)?,
            max_level: row.get(7)?,
            contextual: row
                .get::<_, Option<String>>(8)?
                .map(|json| serde_json::from_str(&json).expect("contextual payload parses")),
            transcript: row
                .get::<_, Option<String>>(9)?
                .map(|json| serde_json::from_str(&json).expect("transcript payload parses")),
            liked: row.get::<_, i64>(10)? != 0,
            rating: u8::try_from(row.get::<_, i64>(11)?)
                .expect("stored rating is between zero and five"),
            source_metadata: match row.get::<_, Option<String>>(12)? {
                Some(container_format) => Some(crate::SourceMetadata {
                    container_format,
                    sample_rate: u32::try_from(row.get::<_, i64>(13)?)
                        .expect("stored sample rate is non-negative"),
                    channel_count: u32::try_from(row.get::<_, i64>(14)?)
                        .expect("stored channel count is non-negative"),
                    entries: serde_json::from_str(&row.get::<_, String>(15)?)
                        .expect("source metadata entries parse"),
                }),
                None => None,
            },
        })
    })?;
    let mut assets = Vec::new();
    for row in rows {
        assets.push(row?);
    }
    Ok(assets)
}

#[cfg(test)]
mod tests;
