//! Bounded metadata extracted from the original audio container. These values
//! are source evidence, separate from user state and model analysis.

use echo_domain::AssetId;
use rusqlite::Transaction;
use serde::{Deserialize, Serialize};

use crate::error::CatalogError;

/// One original container tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMetadataEntry {
    pub key: String,
    pub value: String,
}

/// Technical and embedded metadata from the source container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMetadata {
    pub container_format: String,
    pub sample_rate: u32,
    pub channel_count: u32,
    pub entries: Vec<SourceMetadataEntry>,
}

/// Stores or refreshes source metadata for one asset and promotes an embedded
/// recording time into the descriptive Original projection.
///
/// # Errors
///
/// Returns a catalog failure when JSON encoding or the write fails.
pub fn record_source_metadata(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    metadata: &SourceMetadata,
    recorded_at_millis: Option<i64>,
) -> Result<(), CatalogError> {
    let entries_json = serde_json::to_string(&metadata.entries).map_err(|error| {
        crate::error::CatalogError::new(
            crate::error::CatalogErrorKind::Other,
            format!("cannot encode source metadata: {error}"),
        )
    })?;
    transaction.execute(
        "INSERT INTO asset_source_metadata \
         (asset_id, container_format, sample_rate, channel_count, entries_json) \
         VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(asset_id) DO UPDATE SET \
         container_format = excluded.container_format, sample_rate = excluded.sample_rate, \
         channel_count = excluded.channel_count, entries_json = excluded.entries_json",
        rusqlite::params![
            asset_id.to_string(),
            metadata.container_format,
            i64::from(metadata.sample_rate),
            i64::from(metadata.channel_count),
            entries_json
        ],
    )?;
    if let Some(recorded_at_millis) = recorded_at_millis {
        transaction.execute(
            "UPDATE assets SET recorded_at_millis = ?2 WHERE id = ?1",
            rusqlite::params![asset_id.to_string(), recorded_at_millis],
        )?;
    }
    Ok(())
}

/// Lists present assets whose source container metadata has not been
/// extracted yet.
///
/// # Errors
///
/// Returns a catalog failure when the projection cannot be read.
pub fn list_assets_missing_source_metadata(
    transaction: &Transaction<'_>,
) -> Result<Vec<AssetId>, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT a.id FROM assets a WHERE a.path_status = 'present' AND NOT EXISTS (\
             SELECT 1 FROM asset_source_metadata m WHERE m.asset_id = a.id\
         ) ORDER BY a.imported_at_millis ASC, a.id ASC",
    )?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let mut assets = Vec::new();
    for row in rows {
        let text = row?;
        assets.push(text.parse().map_err(|error| {
            crate::error::CatalogError::new(
                crate::error::CatalogErrorKind::Other,
                format!("invalid stored asset id {text}: {error}"),
            )
        })?);
    }
    Ok(assets)
}

#[cfg(test)]
mod tests;
