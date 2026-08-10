//! Append-only persistence for user-authored, non-destructive sound
//! adjustments. Originals and analysis evidence are never modified.

use echo_domain::{
    AdjustmentEffects, AdjustmentGraph, AssetId, CompressorSettings, FadeCurve, LimiterSettings,
    ParametricEqualizer, RestorationSettings, ReverbSettings,
};
use rusqlite::{OptionalExtension, Transaction};

use crate::{CatalogError, CatalogErrorKind};

/// One saved adjustment revision for an asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetAdjustmentRevision {
    pub revision_id: i64,
    pub graph: AdjustmentGraph,
    pub created_at_millis: i64,
}

struct StoredAdjustment {
    revision_id: i64,
    trim_start: i64,
    trim_end: i64,
    fade_in: i64,
    fade_out: i64,
    fade_in_curve: i64,
    fade_out_curve: i64,
    gain: i64,
    low_cut_hertz: i64,
    equalizer_json: String,
    compressor_enabled: i64,
    compressor_threshold: i64,
    compressor_ratio: i64,
    compressor_attack: i64,
    compressor_release: i64,
    compressor_makeup: i64,
    reverb_json: String,
    restoration_json: String,
    limiter_enabled: i64,
    limiter_ceiling: i64,
    limiter_release: i64,
    created_at: i64,
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
             low_cut_hertz, parametric_equalizer_json, compressor_enabled, \
             compressor_threshold_centibels, compressor_ratio_tenths, \
             compressor_attack_millis, compressor_release_millis, \
             compressor_makeup_centibels, reverb_json, restoration_json, limiter_enabled, \
             limiter_ceiling_centibels, limiter_release_millis, created_at_millis \
             FROM asset_adjustment_revisions WHERE asset_id = ?1 \
             ORDER BY id DESC LIMIT 1",
            [asset_id.to_string()],
            stored_adjustment_from_row,
        )
        .optional()?;
    stored
        .map(|stored| restore_adjustment_graph(duration, &stored))
        .transpose()
}

fn stored_adjustment_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredAdjustment> {
    Ok(StoredAdjustment {
        revision_id: row.get(0)?,
        trim_start: row.get(1)?,
        trim_end: row.get(2)?,
        fade_in: row.get(3)?,
        fade_out: row.get(4)?,
        fade_in_curve: row.get(5)?,
        fade_out_curve: row.get(6)?,
        gain: row.get(7)?,
        low_cut_hertz: row.get(8)?,
        equalizer_json: row.get(9)?,
        compressor_enabled: row.get(10)?,
        compressor_threshold: row.get(11)?,
        compressor_ratio: row.get(12)?,
        compressor_attack: row.get(13)?,
        compressor_release: row.get(14)?,
        compressor_makeup: row.get(15)?,
        reverb_json: row.get(16)?,
        restoration_json: row.get(17)?,
        limiter_enabled: row.get(18)?,
        limiter_ceiling: row.get(19)?,
        limiter_release: row.get(20)?,
        created_at: row.get(21)?,
    })
}

fn restore_adjustment_graph(
    duration: u64,
    stored: &StoredAdjustment,
) -> Result<AssetAdjustmentRevision, CatalogError> {
    let graph = AdjustmentGraph::new(
        duration,
        stored_millis(stored.trim_start)?,
        stored_millis(stored.trim_end)?,
        stored_millis(stored.fade_in)?,
        stored_millis(stored.fade_out)?,
        AdjustmentEffects::new(
            echo_domain::FadeCurves::new(
                stored_curve(stored.fade_in_curve)?,
                stored_curve(stored.fade_out_curve)?,
            ),
            stored_centibels(stored.gain, "adjustment gain")?,
            u16::try_from(stored.low_cut_hertz).map_err(|_| {
                CatalogError::new(
                    CatalogErrorKind::Other,
                    "stored low-cut frequency is invalid",
                )
            })?,
        )
        .with_equalizer(stored_equalizer(&stored.equalizer_json)?)
        .with_restoration(stored_restoration(&stored.restoration_json)?)
        .with_compressor(CompressorSettings {
            enabled: stored.compressor_enabled != 0,
            threshold_centibels: stored_centibels(
                stored.compressor_threshold,
                "compressor threshold",
            )?,
            ratio_tenths: stored_u16(stored.compressor_ratio, "compressor ratio")?,
            attack_millis: stored_u16(stored.compressor_attack, "compressor attack")?,
            release_millis: stored_u16(stored.compressor_release, "compressor release")?,
            makeup_centibels: stored_centibels(stored.compressor_makeup, "compressor makeup")?,
        })
        .with_reverb(stored_reverb(&stored.reverb_json)?)
        .with_limiter(LimiterSettings {
            enabled: stored.limiter_enabled != 0,
            ceiling_centibels: stored_centibels(stored.limiter_ceiling, "limiter ceiling")?,
            release_millis: stored_u16(stored.limiter_release, "limiter release")?,
        }),
    )
    .map_err(|error| CatalogError::new(CatalogErrorKind::Other, error.to_string()))?;
    Ok(AssetAdjustmentRevision {
        revision_id: stored.revision_id,
        graph,
        created_at_millis: stored.created_at,
    })
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
        .with_equalizer(graph.equalizer())
        .with_restoration(graph.restoration())
        .with_compressor(graph.compressor())
        .with_reverb(graph.reverb())
        .with_limiter(graph.limiter()),
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
         eq_mid_gain_centibels, eq_high_gain_centibels, parametric_equalizer_json, \
         compressor_enabled, \
         compressor_threshold_centibels, compressor_ratio_tenths, \
         compressor_attack_millis, compressor_release_millis, \
         compressor_makeup_centibels, reverb_json, restoration_json, limiter_enabled, limiter_ceiling_centibels, \
         limiter_release_millis, created_at_millis) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, \
                 ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25)",
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
            i64::from(validated.equalizer().bands()[0].gain_centibels),
            i64::from(validated.equalizer().bands()[2].gain_centibels),
            i64::from(validated.equalizer().bands()[5].gain_centibels),
            serde_json::to_string(&validated.equalizer()).map_err(|error| CatalogError::new(
                CatalogErrorKind::Other,
                format!("cannot encode parametric equalizer: {error}"),
            ))?,
            i64::from(validated.compressor().enabled),
            i64::from(validated.compressor().threshold_centibels),
            i64::from(validated.compressor().ratio_tenths),
            i64::from(validated.compressor().attack_millis),
            i64::from(validated.compressor().release_millis),
            i64::from(validated.compressor().makeup_centibels),
            serde_json::to_string(&validated.reverb()).map_err(|error| CatalogError::new(
                CatalogErrorKind::Other,
                format!("cannot encode reverb: {error}"),
            ))?,
            serde_json::to_string(&validated.restoration()).map_err(|error| CatalogError::new(
                CatalogErrorKind::Other,
                format!("cannot encode restoration chain: {error}"),
            ))?,
            i64::from(validated.limiter().enabled),
            i64::from(validated.limiter().ceiling_centibels),
            i64::from(validated.limiter().release_millis),
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

fn stored_equalizer(value: &str) -> Result<ParametricEqualizer, CatalogError> {
    serde_json::from_str(value).map_err(|error| {
        CatalogError::new(
            CatalogErrorKind::Other,
            format!("stored parametric equalizer is invalid: {error}"),
        )
    })
}

fn stored_reverb(value: &str) -> Result<ReverbSettings, CatalogError> {
    serde_json::from_str(value).map_err(|error| {
        CatalogError::new(
            CatalogErrorKind::Other,
            format!("stored reverb is invalid: {error}"),
        )
    })
}

fn stored_restoration(value: &str) -> Result<RestorationSettings, CatalogError> {
    serde_json::from_str(value).map_err(|error| {
        CatalogError::new(
            CatalogErrorKind::Other,
            format!("stored restoration chain is invalid: {error}"),
        )
    })
}

fn stored_centibels(value: i64, field: &str) -> Result<i16, CatalogError> {
    i16::try_from(value).map_err(|_| {
        CatalogError::new(
            CatalogErrorKind::Other,
            format!("stored {field} is invalid"),
        )
    })
}

fn stored_u16(value: i64, field: &str) -> Result<u16, CatalogError> {
    u16::try_from(value).map_err(|_| {
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
