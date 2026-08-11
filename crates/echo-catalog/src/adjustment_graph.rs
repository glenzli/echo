//! Append-only persistence for user-authored, non-destructive sound
//! adjustments. Originals and analysis evidence are never modified.

use echo_domain::{
    AdjustmentEffects, AdjustmentGraph, AssetId, ChannelRepairSettings, CompressorSettings,
    CreativeVfxSettings, DeClickSettings, DeHumSettings, EditTimeline, EffectChain, EffectMask,
    FadeCurve, LimiterSettings, ParametricEqualizer, RestorationSettings, ReverbSettings,
};
use rusqlite::{OptionalExtension, Transaction};

use crate::{CatalogError, CatalogErrorKind};

/// One saved adjustment revision for an asset.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    de_hum_json: String,
    de_click_json: String,
    channel_repair_json: String,
    effect_chain_json: String,
    edit_timeline_json: String,
    effect_masks_json: String,
    limiter_enabled: i64,
    limiter_ceiling: i64,
    limiter_release: i64,
    creative_vfx_json: String,
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
    let Some(duration) = asset_duration(transaction, asset_id)? else {
        return Ok(None);
    };
    let stored = transaction
        .query_row(
            "SELECT id, trim_start_millis, trim_end_millis, fade_in_millis, \
             fade_out_millis, fade_in_curve, fade_out_curve, gain_centibels, \
             low_cut_hertz, parametric_equalizer_json, compressor_enabled, \
             compressor_threshold_centibels, compressor_ratio_tenths, \
             compressor_attack_millis, compressor_release_millis, \
             compressor_makeup_centibels, reverb_json, restoration_json, de_hum_json, \
             de_click_json, channel_repair_json, effect_chain_json, edit_timeline_json, effect_masks_json, limiter_enabled, \
             limiter_ceiling_centibels, limiter_release_millis, creative_vfx_json, created_at_millis \
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

/// Reads one saved adjustment revision belonging to the requested asset.
///
/// Revision identity is scoped by `asset_id`: a revision owned by another
/// asset is indistinguishable from a missing revision and returns `None`.
/// The graph is rebuilt against the asset's current immutable source duration
/// using the same validation contract as [`latest_adjustment_graph`].
///
/// # Errors
///
/// Returns a catalog failure when the query cannot be applied, the asset has
/// an invalid stored duration, or persisted adjustment values violate the
/// adjustment contract.
pub fn adjustment_graph_at_revision(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    revision_id: i64,
) -> Result<Option<AssetAdjustmentRevision>, CatalogError> {
    let Some(duration) = asset_duration(transaction, asset_id)? else {
        return Ok(None);
    };
    let stored = transaction
        .query_row(
            "SELECT id, trim_start_millis, trim_end_millis, fade_in_millis, \
             fade_out_millis, fade_in_curve, fade_out_curve, gain_centibels, \
             low_cut_hertz, parametric_equalizer_json, compressor_enabled, \
             compressor_threshold_centibels, compressor_ratio_tenths, \
             compressor_attack_millis, compressor_release_millis, \
             compressor_makeup_centibels, reverb_json, restoration_json, de_hum_json, \
             de_click_json, channel_repair_json, effect_chain_json, edit_timeline_json, effect_masks_json, limiter_enabled, \
             limiter_ceiling_centibels, limiter_release_millis, creative_vfx_json, created_at_millis \
             FROM asset_adjustment_revisions WHERE asset_id = ?1 AND id = ?2",
            rusqlite::params![asset_id.to_string(), revision_id],
            stored_adjustment_from_row,
        )
        .optional()?;
    stored
        .map(|stored| restore_adjustment_graph(duration, &stored))
        .transpose()
}

fn asset_duration(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
) -> Result<Option<u64>, CatalogError> {
    let duration: Option<i64> = transaction
        .query_row(
            "SELECT duration_millis FROM assets WHERE id = ?1",
            [asset_id.to_string()],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    duration
        .map(|duration| {
            u64::try_from(duration).map_err(|_| {
                CatalogError::new(CatalogErrorKind::Other, "stored asset duration is invalid")
            })
        })
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
        de_hum_json: row.get(18)?,
        de_click_json: row.get(19)?,
        channel_repair_json: row.get(20)?,
        effect_chain_json: row.get(21)?,
        edit_timeline_json: row.get(22)?,
        effect_masks_json: row.get(23)?,
        limiter_enabled: row.get(24)?,
        limiter_ceiling: row.get(25)?,
        limiter_release: row.get(26)?,
        creative_vfx_json: row.get(27)?,
        created_at: row.get(28)?,
    })
}

fn restore_adjustment_graph(
    duration: u64,
    stored: &StoredAdjustment,
) -> Result<AssetAdjustmentRevision, CatalogError> {
    let trim_start = stored_millis(stored.trim_start)?;
    let trim_end = stored_millis(stored.trim_end)?;
    let graph = AdjustmentGraph::new(
        duration,
        trim_start,
        trim_end,
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
        .with_de_hum(stored_de_hum(&stored.de_hum_json)?)
        .with_de_click(stored_de_click(&stored.de_click_json)?)
        .with_channel_repair(stored_channel_repair(&stored.channel_repair_json)?)
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
        .with_creative_vfx(stored_creative_vfx(&stored.creative_vfx_json)?)
        .with_effect_chain(stored_effect_chain(&stored.effect_chain_json)?)
        .with_edit_timeline(stored_edit_timeline(
            &stored.edit_timeline_json,
            trim_start,
            trim_end,
        )?)
        .with_effect_masks(stored_effect_masks(&stored.effect_masks_json)?)
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
    let validated = validated_adjustment_graph(duration, graph)?;
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
         compressor_makeup_centibels, reverb_json, restoration_json, de_hum_json, \
         de_click_json, channel_repair_json, effect_chain_json, edit_timeline_json, effect_masks_json, \
         limiter_enabled, limiter_ceiling_centibels, \
         limiter_release_millis, creative_vfx_json, created_at_millis) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, \
                 ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, \
                 ?27, ?28, ?29, ?30, ?31, ?32)",
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
            serde_json::to_string(&validated.de_hum()).map_err(|error| CatalogError::new(
                CatalogErrorKind::Other,
                format!("cannot encode de-hum settings: {error}"),
            ))?,
            serde_json::to_string(&validated.de_click()).map_err(|error| CatalogError::new(
                CatalogErrorKind::Other,
                format!("cannot encode de-click settings: {error}"),
            ))?,
            encode_channel_repair(validated.channel_repair())?,
            serde_json::to_string(&validated.effect_chain()).map_err(|error| CatalogError::new(
                CatalogErrorKind::Other,
                format!("cannot encode effect chain: {error}"),
            ))?,
            serde_json::to_string(validated.edit_timeline()).map_err(|error| CatalogError::new(
                CatalogErrorKind::Other,
                format!("cannot encode edit timeline: {error}"),
            ))?,
            serde_json::to_string(validated.effect_masks()).map_err(|error| CatalogError::new(
                CatalogErrorKind::Other,
                format!("cannot encode effect masks: {error}"),
            ))?,
            i64::from(validated.limiter().enabled),
            i64::from(validated.limiter().ceiling_centibels),
            i64::from(validated.limiter().release_millis),
            encoded_creative_vfx(validated.creative_vfx())?,
            now_millis,
        ],
    )?;
    Ok(AssetAdjustmentRevision {
        revision_id: transaction.last_insert_rowid(),
        graph: validated,
        created_at_millis: now_millis,
    })
}

fn encode_channel_repair(settings: ChannelRepairSettings) -> Result<String, CatalogError> {
    serde_json::to_string(&settings).map_err(|error| {
        CatalogError::new(
            CatalogErrorKind::Other,
            format!("cannot encode channel repair settings: {error}"),
        )
    })
}

fn validated_adjustment_graph(
    duration: u64,
    graph: AdjustmentGraph,
) -> Result<AdjustmentGraph, CatalogError> {
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
        .with_de_hum(graph.de_hum())
        .with_de_click(graph.de_click())
        .with_channel_repair(graph.channel_repair())
        .with_compressor(graph.compressor())
        .with_reverb(graph.reverb())
        .with_creative_vfx(graph.creative_vfx())
        .with_limiter(graph.limiter())
        .with_effect_chain(graph.effect_chain())
        .with_edit_timeline(graph.edit_timeline().clone())
        .with_effect_masks(graph.effect_masks().to_vec()),
    );
    drop(graph);
    validated.map_err(|error| CatalogError::new(CatalogErrorKind::Other, error.to_string()))
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

fn stored_creative_vfx(value: &str) -> Result<CreativeVfxSettings, CatalogError> {
    serde_json::from_str(value).map_err(|error| {
        CatalogError::new(
            CatalogErrorKind::Other,
            format!("stored creative VFX settings are invalid: {error}"),
        )
    })
}

fn encoded_creative_vfx(value: CreativeVfxSettings) -> Result<String, CatalogError> {
    serde_json::to_string(&value).map_err(|error| {
        CatalogError::new(
            CatalogErrorKind::Other,
            format!("cannot encode creative VFX settings: {error}"),
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

fn stored_de_hum(value: &str) -> Result<DeHumSettings, CatalogError> {
    serde_json::from_str(value).map_err(|error| {
        CatalogError::new(
            CatalogErrorKind::Other,
            format!("stored de-hum settings are invalid: {error}"),
        )
    })
}

fn stored_de_click(value: &str) -> Result<DeClickSettings, CatalogError> {
    serde_json::from_str(value).map_err(|error| {
        CatalogError::new(
            CatalogErrorKind::Other,
            format!("stored de-click settings are invalid: {error}"),
        )
    })
}

fn stored_channel_repair(value: &str) -> Result<ChannelRepairSettings, CatalogError> {
    serde_json::from_str(value).map_err(|error| {
        CatalogError::new(
            CatalogErrorKind::Other,
            format!("stored channel repair settings are invalid: {error}"),
        )
    })
}

fn stored_effect_chain(value: &str) -> Result<EffectChain, CatalogError> {
    let chain: EffectChain = serde_json::from_str(value).map_err(|error| {
        CatalogError::new(
            CatalogErrorKind::Other,
            format!("stored effect chain is invalid: {error}"),
        )
    })?;
    if !chain.is_valid() {
        return Err(CatalogError::new(
            CatalogErrorKind::Other,
            "stored effect chain violates the singleton chain contract",
        ));
    }
    Ok(chain)
}

fn stored_edit_timeline(
    value: &str,
    trim_start_millis: u64,
    trim_end_millis: u64,
) -> Result<EditTimeline, CatalogError> {
    if value.trim().is_empty() || value.trim() == "[]" {
        return EditTimeline::identity(trim_start_millis, trim_end_millis).map_err(|error| {
            CatalogError::new(
                CatalogErrorKind::Other,
                format!("cannot restore legacy edit timeline: {error}"),
            )
        });
    }
    serde_json::from_str(value).map_err(|error| {
        CatalogError::new(
            CatalogErrorKind::Other,
            format!("stored edit timeline is invalid: {error}"),
        )
    })
}

fn stored_effect_masks(value: &str) -> Result<Vec<EffectMask>, CatalogError> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(value).map_err(|error| {
        CatalogError::new(
            CatalogErrorKind::Other,
            format!("stored effect masks are invalid: {error}"),
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
