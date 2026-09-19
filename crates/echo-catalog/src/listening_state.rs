//! Durable user-owned listening continuity. This is Revisit state, not model
//! evidence, immutable source metadata, or an authored adjustment revision.

use echo_domain::AssetId;
use rusqlite::Transaction;

use crate::error::{CatalogError, CatalogErrorKind};

const MINIMUM_RESUMABLE_DURATION_MILLIS: u64 = 60_000;
const MINIMUM_RESUME_OFFSET_MILLIS: u64 = 10_000;
const MINIMUM_REMAINING_MILLIS: u64 = 10_000;

/// Current user-owned listening continuity for one sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AssetListeningState {
    pub last_listened_at_millis: i64,
    pub resume_position_millis: u64,
}

/// Reads current listening continuity; an unheard sound returns defaults.
///
/// # Errors
///
/// Returns a catalog failure when the query cannot be applied.
pub fn asset_listening_state(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
) -> Result<AssetListeningState, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT last_listened_at_millis, resume_position_millis \
         FROM sound_user_state WHERE asset_id = ?1",
    )?;
    let mut rows = statement.query([asset_id.to_string()])?;
    let Some(row) = rows.next()? else {
        return Ok(AssetListeningState::default());
    };
    Ok(AssetListeningState {
        last_listened_at_millis: row.get(0)?,
        resume_position_millis: u64::try_from(row.get::<_, i64>(1)?).map_err(|_| {
            CatalogError::new(CatalogErrorKind::Other, "stored resume position is invalid")
        })?,
    })
}

/// Records meaningful listening and returns the normalized continuity state.
///
/// Resume is retained only for a playable interval of at least one minute,
/// after ten seconds have been heard and while at least ten seconds remain.
/// Reaching 95% also clears resume so completed recordings do not linger.
///
/// # Errors
///
/// Returns a catalog failure when the source interval, position, time, asset,
/// or write is invalid.
pub fn record_asset_listening_progress(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    position_millis: u64,
    playback_start_millis: u64,
    playback_end_millis: u64,
    now_millis: i64,
) -> Result<AssetListeningState, CatalogError> {
    if now_millis < 0
        || playback_end_millis <= playback_start_millis
        || position_millis < playback_start_millis
        || position_millis > playback_end_millis
    {
        return Err(CatalogError::new(
            CatalogErrorKind::Other,
            "listening progress is outside the playable source interval",
        ));
    }

    let playable_duration = playback_end_millis - playback_start_millis;
    let played = position_millis - playback_start_millis;
    let remaining = playback_end_millis - position_millis;
    let near_end = u128::from(played) * 100 >= u128::from(playable_duration) * 95;
    let resumable = playable_duration >= MINIMUM_RESUMABLE_DURATION_MILLIS
        && played >= MINIMUM_RESUME_OFFSET_MILLIS
        && remaining >= MINIMUM_REMAINING_MILLIS
        && !near_end;
    let state = AssetListeningState {
        last_listened_at_millis: now_millis,
        resume_position_millis: if resumable { position_millis } else { 0 },
    };

    transaction.execute(
        "INSERT INTO sound_user_state (asset_id, last_listened_at_millis, \
         resume_position_millis, updated_at_millis) VALUES (?1, ?2, ?3, ?2) \
         ON CONFLICT(asset_id) DO UPDATE SET \
         last_listened_at_millis = excluded.last_listened_at_millis, \
         resume_position_millis = excluded.resume_position_millis, \
         updated_at_millis = excluded.updated_at_millis",
        rusqlite::params![
            asset_id.to_string(),
            now_millis,
            i64::try_from(state.resume_position_millis).map_err(|_| CatalogError::new(
                CatalogErrorKind::Other,
                "resume position does not fit the catalog",
            ))?
        ],
    )?;
    Ok(state)
}

#[cfg(test)]
mod tests;
