//! Asset path health: missing detection and relinking.
//!
//! Audio identity is the content hash, so relink is a plain hash lookup: when
//! a scan finds a file whose hash matches a `missing` asset, the path is
//! restored and the asset becomes present again. No EXIF or representation
//! matching is needed.

use std::path::Path;

use rusqlite::{OptionalExtension, Transaction};

use crate::error::CatalogError;

/// Path health of one asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetPathStatus {
    Present,
    Missing,
}

/// Marks an asset missing because its path no longer exists.
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn mark_asset_missing(
    transaction: &Transaction<'_>,
    asset_id: &str,
) -> Result<(), CatalogError> {
    transaction.execute(
        "UPDATE assets SET path_status = 'missing' WHERE id = ?1",
        [asset_id],
    )?;
    Ok(())
}

/// Relinks a `missing` asset to a freshly found path with the same content
/// hash; assets already present keep their path.
///
/// Returns `true` when the path was updated.
///
/// # Errors
///
/// Returns a catalog failure when the read or write cannot be applied.
pub fn relink_asset_by_hash(
    transaction: &Transaction<'_>,
    content_hash: &str,
    path: &Path,
) -> Result<bool, CatalogError> {
    let id: Option<String> = transaction
        .query_row(
            "SELECT id FROM assets WHERE content_hash = ?1 AND path_status = 'missing'",
            [content_hash],
            |row| row.get(0),
        )
        .optional()?;
    let Some(id) = id else {
        return Ok(false);
    };
    transaction.execute(
        "UPDATE assets SET path = ?1, path_status = 'present' WHERE id = ?2",
        rusqlite::params![path.to_string_lossy(), id],
    )?;
    Ok(true)
}
