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
    pub imported_at_millis: i64,
    pub path_status: String,
    pub max_level: u8,
    /// Latest contextual payload JSON (absent when not analyzed yet).
    pub contextual: Option<serde_json::Value>,
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
        "SELECT a.id, a.path, a.codec, a.duration_millis, a.imported_at_millis, \
         a.path_status, (SELECT max_level FROM asset_levels WHERE asset_id = a.id), \
         (SELECT value FROM analysis_records r WHERE r.asset_id = a.id \
          AND r.kind = 'contextual' ORDER BY r.id DESC LIMIT 1) \
         FROM assets a ORDER BY a.imported_at_millis DESC, a.id DESC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(AudioSpaceAsset {
            id: row.get(0)?,
            path: row.get::<_, String>(1)?.into(),
            codec: row.get(2)?,
            duration_millis: row
                .get::<_, Option<i64>>(3)?
                .map(|millis| u64::try_from(millis).expect("stored duration is non-negative")),
            imported_at_millis: row.get(4)?,
            path_status: row.get(5)?,
            max_level: row.get(6)?,
            contextual: row
                .get::<_, Option<String>>(7)?
                .map(|json| serde_json::from_str(&json).expect("contextual payload parses")),
        })
    })?;
    let mut assets = Vec::new();
    for row in rows {
        assets.push(row?);
    }
    Ok(assets)
}
