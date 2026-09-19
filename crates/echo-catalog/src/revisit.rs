//! Bounded, source-anchored collections for the Revisit home surface.
//!
//! Revisit is a read-only projection over user listening facts and immutable
//! source dates. It does not create album membership or reinterpret analysis
//! evidence, and every row is capped before crossing the desktop bridge.

use std::str::FromStr;

use echo_domain::AssetId;
use rusqlite::{Transaction, params};

use crate::{CatalogError, CatalogErrorKind};

const MAX_CONTINUE_LISTENING: i64 = 8;
const MAX_RECENTLY_LISTENED: i64 = 12;
const MAX_ON_THIS_DAY: i64 = 12;
const MAX_RECENTLY_ADDED: i64 = 12;

/// One bounded home snapshot. Identities remain source assets; this projection
/// owns only deterministic ordering and admission into Revisit sections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisitSnapshot {
    pub continue_listening_asset_ids: Vec<AssetId>,
    pub recently_listened_asset_ids: Vec<AssetId>,
    pub on_this_day_asset_ids: Vec<AssetId>,
    pub recently_added_asset_ids: Vec<AssetId>,
}

/// Projects the current Revisit home at `now_millis`.
///
/// Continue Listening and Recently Listened are disjoint: a resumable sound
/// stays in the first section until its source-time checkpoint is cleared.
/// "On this day" uses the immutable recording date and excludes the current
/// year, so a recent import cannot masquerade as a memory.
///
/// # Errors
///
/// Returns a catalog failure when a bounded read fails or a stored asset
/// identity is invalid.
pub fn revisit_snapshot(
    transaction: &Transaction<'_>,
    now_millis: i64,
) -> Result<RevisitSnapshot, CatalogError> {
    if now_millis < 0 {
        return Err(CatalogError::new(
            CatalogErrorKind::Other,
            "revisit projection time must be non-negative",
        ));
    }

    Ok(RevisitSnapshot {
        continue_listening_asset_ids: query_ids(
            transaction,
            "SELECT a.id FROM memory_sources a \
             JOIN sound_user_state u ON u.asset_id = a.id \
             WHERE a.path_status = 'present' AND u.resume_position_millis > 0 \
             ORDER BY u.last_listened_at_millis DESC, a.id DESC LIMIT ?1",
            MAX_CONTINUE_LISTENING,
        )?,
        recently_listened_asset_ids: query_ids(
            transaction,
            "SELECT a.id FROM memory_sources a \
             JOIN sound_user_state u ON u.asset_id = a.id \
             WHERE a.path_status = 'present' AND u.last_listened_at_millis > 0 \
               AND u.resume_position_millis = 0 \
             ORDER BY u.last_listened_at_millis DESC, a.id DESC LIMIT ?1",
            MAX_RECENTLY_LISTENED,
        )?,
        on_this_day_asset_ids: query_ids_with_now(
            transaction,
            "SELECT a.id FROM memory_sources a \
             WHERE a.path_status = 'present' AND a.recorded_at_millis > 0 \
               AND strftime('%m-%d', a.recorded_at_millis / 1000, 'unixepoch', 'localtime') = \
                   strftime('%m-%d', ?1 / 1000, 'unixepoch', 'localtime') \
               AND strftime('%Y', a.recorded_at_millis / 1000, 'unixepoch', 'localtime') != \
                   strftime('%Y', ?1 / 1000, 'unixepoch', 'localtime') \
             ORDER BY a.recorded_at_millis DESC, a.id DESC LIMIT ?2",
            now_millis,
            MAX_ON_THIS_DAY,
        )?,
        recently_added_asset_ids: query_ids(
            transaction,
            "SELECT a.id FROM memory_sources a \
             WHERE a.path_status = 'present' \
               AND NOT EXISTS ( \
                   SELECT 1 FROM sound_user_state u \
                   WHERE u.asset_id = a.id AND u.last_listened_at_millis > 0 \
               ) \
             ORDER BY a.imported_at_millis DESC, a.id DESC LIMIT ?1",
            MAX_RECENTLY_ADDED,
        )?,
    })
}

fn query_ids(
    transaction: &Transaction<'_>,
    sql: &str,
    limit: i64,
) -> Result<Vec<AssetId>, CatalogError> {
    let mut statement = transaction.prepare(sql)?;
    let rows = statement.query_map(params![limit], |row| row.get::<_, String>(0))?;
    parse_rows(rows)
}

fn query_ids_with_now(
    transaction: &Transaction<'_>,
    sql: &str,
    now_millis: i64,
    limit: i64,
) -> Result<Vec<AssetId>, CatalogError> {
    let mut statement = transaction.prepare(sql)?;
    let rows = statement.query_map(params![now_millis, limit], |row| row.get::<_, String>(0))?;
    parse_rows(rows)
}

fn parse_rows(
    rows: rusqlite::MappedRows<'_, impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<String>>,
) -> Result<Vec<AssetId>, CatalogError> {
    rows.map(|row| {
        let encoded = row?;
        AssetId::from_str(&encoded).map_err(|error| {
            CatalogError::new(
                CatalogErrorKind::Other,
                format!("invalid stored asset id during Revisit projection: {error}"),
            )
        })
    })
    .collect()
}

#[cfg(test)]
mod tests;
