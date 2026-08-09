//! User-owned affinity for one sound: Like and rating are durable user facts,
//! not model analysis or immutable Original metadata.

use echo_domain::AssetId;
use rusqlite::Transaction;

use crate::error::{CatalogError, CatalogErrorKind};

/// User affinity projected for a sound card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AssetAffinity {
    pub liked: bool,
    pub rating: u8,
}

/// Reads the current affinity; an untouched asset has the default state.
///
/// # Errors
///
/// Returns a catalog failure when the query cannot be applied.
pub fn asset_affinity(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
) -> Result<AssetAffinity, CatalogError> {
    let mut statement =
        transaction.prepare("SELECT liked, rating FROM asset_user_state WHERE asset_id = ?1")?;
    let mut rows = statement.query([asset_id.to_string()])?;
    let Some(row) = rows.next()? else {
        return Ok(AssetAffinity::default());
    };
    Ok(AssetAffinity {
        liked: row.get::<_, i64>(0)? != 0,
        rating: u8::try_from(row.get::<_, i64>(1)?).map_err(|_| {
            CatalogError::new(CatalogErrorKind::Other, "stored asset rating is invalid")
        })?,
    })
}

/// Atomically stores Like and rating for one asset.
///
/// # Errors
///
/// Returns a catalog failure when `rating` exceeds five or the write cannot
/// be applied.
pub fn set_asset_affinity(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    affinity: AssetAffinity,
    now_millis: i64,
) -> Result<(), CatalogError> {
    if affinity.rating > 5 {
        return Err(CatalogError::new(
            CatalogErrorKind::Other,
            "asset rating must be between zero and five",
        ));
    }
    transaction.execute(
        "INSERT INTO asset_user_state (asset_id, liked, rating, updated_at_millis) \
         VALUES (?1, ?2, ?3, ?4) ON CONFLICT(asset_id) DO UPDATE SET \
         liked = excluded.liked, rating = excluded.rating, \
         updated_at_millis = excluded.updated_at_millis",
        rusqlite::params![
            asset_id.to_string(),
            i64::from(affinity.liked),
            i64::from(affinity.rating),
            now_millis
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests;
