//! Append-only persistence for user-authored, non-destructive sound
//! adjustments. Originals and analysis evidence are never modified.

use echo_domain::{AdjustmentEffects, AdjustmentGraph, AssetId, FadeCurve, ThreeBandEqualizer};
use rusqlite::{OptionalExtension, Transaction};

use crate::{CatalogError, CatalogErrorKind};

/// One saved adjustment revision for an asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetAdjustmentRevision {
    pub revision_id: i64,
    pub graph: AdjustmentGraph,
    pub created_at_millis: i64,
}

/// Reads the newest saved adjustment, or `None` for an untouched asset.
///
/// # Errors
///
/// Returns a catalog failure when the query cannot be applied or persisted
/// values violate the adjustment contract.
pub fn latest_adjustment_graph(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
) -> Result<Option<AssetAdjustmentRevision>, CatalogError> {
    let duration: Option<i64> = transaction
        .query_row(
            "SELECT duration_millis FROM assets WHERE id = ?1",
            [asset_id.to_string()],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    let Some(duration) = duration else {
        return Ok(None);
    };
    let duration = u64::try_from(duration).map_err(|_| {
        CatalogError::new(CatalogErrorKind::Other, "stored asset duration is invalid")
    })?;
    let stored = transaction
        .query_row(
            "SELECT id, trim_start_millis, trim_end_millis, fade_in_millis, \
             fade_out_millis, fade_in_curve, fade_out_curve, gain_centibels, \
             low_cut_hertz, eq_low_gain_centibels, eq_mid_gain_centibels, \
             eq_high_gain_centibels, created_at_millis \
             FROM asset_adjustment_revisions WHERE asset_id = ?1 \
             ORDER BY id DESC LIMIT 1",
            [asset_id.to_string()],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, i64>(12)?,
                ))
            },
        )
        .optional()?;
    let Some((
        revision_id,
        trim_start,
        trim_end,
        fade_in,
        fade_out,
        fade_in_curve,
        fade_out_curve,
        gain,
        low_cut_hertz,
        eq_low_gain,
        eq_mid_gain,
        eq_high_gain,
        created_at,
    )) = stored
    else {
        return Ok(None);
    };
    let graph = AdjustmentGraph::new(
        duration,
        stored_millis(trim_start)?,
        stored_millis(trim_end)?,
        stored_millis(fade_in)?,
        stored_millis(fade_out)?,
        AdjustmentEffects::new(
            echo_domain::FadeCurves::new(
                stored_curve(fade_in_curve)?,
                stored_curve(fade_out_curve)?,
            ),
            stored_centibels(gain, "adjustment gain")?,
            u16::try_from(low_cut_hertz).map_err(|_| {
                CatalogError::new(
                    CatalogErrorKind::Other,
                    "stored low-cut frequency is invalid",
                )
            })?,
        )
        .with_equalizer(ThreeBandEqualizer::new(
            stored_centibels(eq_low_gain, "low equalizer gain")?,
            stored_centibels(eq_mid_gain, "mid equalizer gain")?,
            stored_centibels(eq_high_gain, "high equalizer gain")?,
        )),
    )
    .map_err(|error| CatalogError::new(CatalogErrorKind::Other, error.to_string()))?;
    Ok(Some(AssetAdjustmentRevision {
        revision_id,
        graph,
        created_at_millis: created_at,
    }))
}

/// Appends a new adjustment revision unless the newest graph is identical.
///
/// # Errors
///
/// Returns a catalog failure when the asset is missing, has no known
/// duration, the graph exceeds that duration, or the write cannot be applied.
pub fn record_adjustment_graph(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    graph: AdjustmentGraph,
    now_millis: i64,
) -> Result<AssetAdjustmentRevision, CatalogError> {
    let duration: Option<i64> = transaction
        .query_row(
            "SELECT duration_millis FROM assets WHERE id = ?1",
            [asset_id.to_string()],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    let Some(duration) = duration else {
        return Err(CatalogError::new(
            CatalogErrorKind::Other,
            "asset must have a known duration before it can be adjusted",
        ));
    };
    let duration = stored_millis(duration)?;
    let validated = AdjustmentGraph::new(
        duration,
        graph.trim_start_millis(),
        graph.trim_end_millis(),
        graph.fade_in_millis(),
        graph.fade_out_millis(),
        AdjustmentEffects::new(
            echo_domain::FadeCurves::new(graph.fade_in_curve(), graph.fade_out_curve()),
            graph.gain_centibels(),
            graph.low_cut_hertz(),
        )
        .with_equalizer(graph.equalizer()),
    )
    .map_err(|error| CatalogError::new(CatalogErrorKind::Other, error.to_string()))?;
    if let Some(current) = latest_adjustment_graph(transaction, asset_id)?
        && current.graph == validated
    {
        return Ok(current);
    }
    transaction.execute(
        "INSERT INTO asset_adjustment_revisions (asset_id, trim_start_millis, \
         trim_end_millis, fade_in_millis, fade_out_millis, fade_in_curve, \
         fade_out_curve, gain_centibels, low_cut_hertz, eq_low_gain_centibels, \
         eq_mid_gain_centibels, eq_high_gain_centibels, created_at_millis) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        rusqlite::params![
            asset_id.to_string(),
            millis_i64(validated.trim_start_millis())?,
            millis_i64(validated.trim_end_millis())?,
            millis_i64(validated.fade_in_millis())?,
            millis_i64(validated.fade_out_millis())?,
            validated.fade_in_curve().catalog_value(),
            validated.fade_out_curve().catalog_value(),
            i64::from(validated.gain_centibels()),
            i64::from(validated.low_cut_hertz()),
            i64::from(validated.equalizer().low_gain_centibels()),
            i64::from(validated.equalizer().mid_gain_centibels()),
            i64::from(validated.equalizer().high_gain_centibels()),
            now_millis,
        ],
    )?;
    Ok(AssetAdjustmentRevision {
        revision_id: transaction.last_insert_rowid(),
        graph: validated,
        created_at_millis: now_millis,
    })
}

fn stored_curve(value: i64) -> Result<FadeCurve, CatalogError> {
    FadeCurve::from_catalog_value(value)
        .map_err(|error| CatalogError::new(CatalogErrorKind::Other, error.to_string()))
}

fn stored_centibels(value: i64, field: &str) -> Result<i16, CatalogError> {
    i16::try_from(value).map_err(|_| {
        CatalogError::new(
            CatalogErrorKind::Other,
            format!("stored {field} is invalid"),
        )
    })
}

fn stored_millis(value: i64) -> Result<u64, CatalogError> {
    u64::try_from(value).map_err(|_| {
        CatalogError::new(
            CatalogErrorKind::Other,
            "stored adjustment milliseconds are invalid",
        )
    })
}

fn millis_i64(value: u64) -> Result<i64, CatalogError> {
    i64::try_from(value).map_err(|_| {
        CatalogError::new(
            CatalogErrorKind::Other,
            "adjustment milliseconds exceed the catalog range",
        )
    })
}

#[cfg(test)]
mod tests;
