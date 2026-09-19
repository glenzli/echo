//! Audio Space projection: assets joined with their newest contextual
//! presentation and latest positive browse facets for the sound-album surface.

use echo_domain::{AssetId, MetadataCalibration, MetadataFields};
use rusqlite::Transaction;

use crate::error::CatalogError;

/// One asset projected for the Audio Space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioSpaceAsset {
    pub id: String,
    pub in_memory: bool,
    pub in_materials: bool,
    pub material_category: String,
    pub assembly_id: String,
    pub assembly_revision_id: i64,
    pub provenance_json: String,
    pub path: std::path::PathBuf,
    pub codec: Option<String>,
    pub duration_millis: Option<u64>,
    pub recorded_at_millis: Option<i64>,
    pub imported_at_millis: i64,
    pub path_status: String,
    pub max_level: u8,
    pub liked: bool,
    pub rating: u8,
    /// Newest meaningful user listening activity, or zero when unheard.
    pub last_listened_at_millis: i64,
    /// Original-time position eligible for explicit continue listening.
    pub resume_position_millis: u64,
    /// Newest user-authored non-destructive adjustment revision.
    pub adjustment: Option<crate::AssetAdjustmentRevision>,
    /// Latest contextual payload JSON (absent when not analyzed yet).
    pub contextual: Option<serde_json::Value>,
    /// Keywords from the newest contextual record that emitted keywords.
    pub contextual_keywords: Vec<String>,
    /// Mood from the newest contextual record that emitted a mood.
    pub contextual_mood: Option<String>,
    /// Event from the newest contextual record that emitted an event.
    pub contextual_event_type: Option<String>,
    /// Latest model-extracted text payload JSON.
    pub transcript: Option<serde_json::Value>,
    /// Model-only descriptive values before user calibration.
    pub model_metadata: MetadataFields,
    /// User-visible values after applying the newest calibration revision.
    pub effective_metadata: MetadataFields,
    /// Newest append-only user calibration revision, including an empty reset.
    pub metadata_calibration: Option<crate::MetadataCalibrationRevision>,
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
         COALESCE((SELECT json_group_array(display_value) FROM (\
             SELECT f.display_value FROM contextual_browse_facets f \
             WHERE f.asset_id = a.id AND f.facet_kind = 'keyword' \
               AND f.analysis_record_id = (\
                   SELECT MAX(latest.analysis_record_id) \
                   FROM contextual_browse_facets latest \
                   WHERE latest.asset_id = a.id AND latest.facet_kind = 'keyword'\
               ) ORDER BY f.normalized_value\
         )), '[]'), \
         (SELECT f.display_value FROM contextual_browse_facets f \
          WHERE f.asset_id = a.id AND f.facet_kind = 'mood' \
          ORDER BY f.analysis_record_id DESC LIMIT 1), \
         (SELECT f.display_value FROM contextual_browse_facets f \
          WHERE f.asset_id = a.id AND f.facet_kind = 'event' \
          ORDER BY f.analysis_record_id DESC LIMIT 1), \
         (SELECT json_set(r.value, '$.text', \
             substr(COALESCE(json_extract(r.value, '$.text'), ''), 1, 2048), \
             '$.segments', json('[]'), '$.runtime', NULL) \
          FROM analysis_records r WHERE r.asset_id = a.id \
          AND r.kind = 'transcript' ORDER BY r.id DESC LIMIT 1), \
         COALESCE(u.liked, 0), COALESCE(u.rating, 0), \
         m.container_format, m.sample_rate, m.channel_count, m.entries_json, \
         adj.id, adj.trim_start_millis, adj.trim_end_millis, adj.fade_in_millis, \
         adj.fade_out_millis, adj.fade_in_curve, adj.fade_out_curve, \
         adj.gain_centibels, adj.low_cut_hertz, adj.parametric_equalizer_json, \
         adj.compressor_enabled, adj.compressor_threshold_centibels, \
         adj.compressor_ratio_tenths, adj.compressor_attack_millis, \
         adj.compressor_release_millis, adj.compressor_makeup_centibels, \
         adj.reverb_json, adj.space_json, adj.restoration_json, adj.de_hum_json, adj.de_click_json, \
         adj.channel_repair_json, adj.effect_chain_json, adj.edit_timeline_json, adj.effect_masks_json, \
         adj.limiter_enabled, adj.limiter_ceiling_centibels, \
         adj.limiter_release_millis, \
         adj.creative_vfx_json, \
         adj.created_at_millis, \
         COALESCE(u.last_listened_at_millis, 0), \
         COALESCE(u.resume_position_millis, 0), \
         calibration.id, calibration.sound_caption, calibration.summary, \
         calibration.event_type, calibration.mood, calibration.keywords_json, \
         calibration.transcript_text, calibration.language, calibration.created_at_millis \
         FROM assets a LEFT JOIN sound_user_state u ON u.asset_id = a.id \
         LEFT JOIN asset_source_metadata m ON m.asset_id = a.id \
         LEFT JOIN asset_adjustment_revisions adj ON adj.id = (\
             SELECT id FROM asset_adjustment_revisions latest_adjustment \
             WHERE latest_adjustment.asset_id = a.id AND NOT EXISTS (SELECT 1 FROM project_adjustment_revisions p WHERE p.revision_id = latest_adjustment.id) ORDER BY id DESC LIMIT 1\
         ) \
         LEFT JOIN metadata_calibration_revisions calibration ON calibration.id = (\
             SELECT id FROM metadata_calibration_revisions latest_calibration \
             WHERE latest_calibration.asset_id = a.id ORDER BY id DESC LIMIT 1\
         ) \
         ORDER BY a.imported_at_millis DESC, a.id DESC",
    )?;
    let rows = statement.query_map([], audio_space_asset_from_row)?;
    let mut assets = Vec::new();
    for row in rows {
        assets.push(row?);
    }
    let memberships = crate::sound_memberships(transaction)?;
    for asset in &mut assets {
        if let Some(membership) = memberships.get(&asset.id) {
            asset.in_memory = membership.in_memory;
            asset.in_materials = membership.in_materials;
            asset
                .material_category
                .clone_from(&membership.material_category);
        }
    }
    for memory in crate::assembly_memories(transaction)? {
        let Some(membership) = memberships.get(&memory.id) else {
            continue;
        };
        assets.push(assembly_memory_summary(memory, membership));
    }
    assets.sort_by(|left, right| {
        right
            .imported_at_millis
            .cmp(&left.imported_at_millis)
            .then_with(|| right.id.cmp(&left.id))
    });
    Ok(assets)
}

fn assembly_memory_summary(
    memory: crate::AssemblyMemory,
    membership: &crate::SoundMembership,
) -> AudioSpaceAsset {
    AudioSpaceAsset {
        id: memory.id.clone(),
        in_memory: membership.in_memory,
        in_materials: membership.in_materials,
        material_category: membership.material_category.clone(),
        assembly_id: memory.id,
        assembly_revision_id: memory.assembly_revision_id,
        provenance_json: memory.provenance_json,
        path_status: if memory.path.is_file() {
            "present"
        } else {
            "missing"
        }
        .to_owned(),
        path: memory.path,
        codec: Some("pcm_s24le".to_owned()),
        duration_millis: Some(memory.duration_millis),
        recorded_at_millis: None,
        imported_at_millis: memory.created_at_millis,
        max_level: 0,
        liked: memory.liked,
        rating: memory.rating,
        last_listened_at_millis: memory.last_listened_at_millis,
        resume_position_millis: memory.resume_position_millis,
        adjustment: None,
        contextual: None,
        contextual_keywords: Vec::new(),
        contextual_mood: None,
        contextual_event_type: None,
        transcript: None,
        model_metadata: MetadataFields::default(),
        effective_metadata: MetadataFields {
            sound_caption: memory.name,
            ..MetadataFields::default()
        },
        metadata_calibration: None,
        source_metadata: Some(crate::SourceMetadata {
            container_format: "wav".to_owned(),
            sample_rate: memory.sample_rate,
            channel_count: u32::from(memory.channel_count),
            entries: Vec::new(),
        }),
    }
}

#[allow(clippy::too_many_lines)] // One SQL row is projected together to preserve positional correspondence.
fn audio_space_asset_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AudioSpaceAsset> {
    let id: String = row.get(0)?;
    let asset_id = id.parse::<AssetId>().map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let duration_millis = row
        .get::<_, Option<i64>>(3)?
        .map(|millis| u64::try_from(millis).expect("stored duration is non-negative"));
    let adjustment = audio_space_adjustment_from_row(row, duration_millis)?;
    let contextual = row
        .get::<_, Option<String>>(8)?
        .map(|json| serde_json::from_str(&json).expect("contextual payload parses"));
    let contextual_keywords: Vec<String> =
        serde_json::from_str(&row.get::<_, String>(9)?).expect("contextual keywords parse");
    let contextual_mood: Option<String> = row.get(10)?;
    let contextual_event_type: Option<String> = row.get(11)?;
    let transcript = row
        .get::<_, Option<String>>(12)?
        .map(|json| serde_json::from_str(&json).expect("transcript payload parses"));
    let model_metadata = crate::metadata_calibration::metadata_fields_from_projection(
        contextual.as_ref(),
        contextual_keywords.clone(),
        contextual_mood.clone(),
        contextual_event_type.clone(),
        transcript.as_ref(),
    );
    let metadata_calibration = row
        .get::<_, Option<i64>>(51)?
        .map(|revision_id| {
            let keywords_json = row.get::<_, Option<String>>(56)?;
            Ok::<_, rusqlite::Error>(crate::MetadataCalibrationRevision {
                revision_id,
                asset_id,
                calibration: MetadataCalibration {
                    sound_caption: row.get(52)?,
                    summary: row.get(53)?,
                    event_type: row.get(54)?,
                    mood: row.get(55)?,
                    keywords: keywords_json
                        .map(|encoded| {
                            serde_json::from_str(&encoded).map_err(|error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    56,
                                    rusqlite::types::Type::Text,
                                    Box::new(error),
                                )
                            })
                        })
                        .transpose()?,
                    transcript_text: row.get(57)?,
                    language: row.get(58)?,
                },
                created_at_millis: row.get(59)?,
            })
        })
        .transpose()?;
    let effective_metadata = metadata_calibration.as_ref().map_or_else(
        || model_metadata.clone(),
        |revision| revision.calibration.apply_to(model_metadata.clone()),
    );
    Ok(AudioSpaceAsset {
        id,
        in_memory: true,
        in_materials: false,
        material_category: String::new(),
        assembly_id: String::new(),
        assembly_revision_id: 0,
        provenance_json: String::new(),
        path: row.get::<_, String>(1)?.into(),
        codec: row.get(2)?,
        duration_millis,
        recorded_at_millis: row.get(4)?,
        imported_at_millis: row.get(5)?,
        path_status: row.get(6)?,
        max_level: row.get(7)?,
        contextual,
        contextual_keywords,
        contextual_mood,
        contextual_event_type,
        transcript,
        model_metadata,
        effective_metadata,
        metadata_calibration,
        liked: row.get::<_, i64>(13)? != 0,
        rating: u8::try_from(row.get::<_, i64>(14)?)
            .expect("stored rating is between zero and five"),
        last_listened_at_millis: row.get(49)?,
        resume_position_millis: u64::try_from(row.get::<_, i64>(50)?)
            .expect("stored resume position is non-negative"),
        adjustment,
        source_metadata: match row.get::<_, Option<String>>(15)? {
            Some(container_format) => Some(crate::SourceMetadata {
                container_format,
                sample_rate: u32::try_from(row.get::<_, i64>(16)?)
                    .expect("stored sample rate is non-negative"),
                channel_count: u32::try_from(row.get::<_, i64>(17)?)
                    .expect("stored channel count is non-negative"),
                entries: serde_json::from_str(&row.get::<_, String>(18)?)
                    .expect("source metadata entries parse"),
            }),
            None => None,
        },
    })
}

fn audio_space_adjustment_from_row(
    row: &rusqlite::Row<'_>,
    duration_millis: Option<u64>,
) -> rusqlite::Result<Option<crate::AssetAdjustmentRevision>> {
    let Some(revision_id) = row.get::<_, Option<i64>>(19)? else {
        return Ok(None);
    };
    let source_duration = duration_millis.expect("adjusted asset has a known duration");
    let trim_start_millis =
        u64::try_from(row.get::<_, i64>(20)?).expect("trim start is non-negative");
    let trim_end_millis = u64::try_from(row.get::<_, i64>(21)?).expect("trim end is non-negative");
    let curves = echo_domain::FadeCurves::new(
        echo_domain::FadeCurve::from_catalog_value(row.get(24)?).expect("fade in curve is valid"),
        echo_domain::FadeCurve::from_catalog_value(row.get(25)?).expect("fade out curve is valid"),
    );
    let effects = echo_domain::AdjustmentEffects::new(
        curves,
        i16::try_from(row.get::<_, i64>(26)?).expect("gain fits centibels"),
        u16::try_from(row.get::<_, i64>(27)?).expect("low cut fits hertz"),
    )
    .with_equalizer(
        serde_json::from_str(&row.get::<_, String>(28)?).expect("stored parametric EQ parses"),
    )
    .with_restoration(
        serde_json::from_str(&row.get::<_, String>(37)?).expect("stored restoration chain parses"),
    )
    .with_de_hum(
        serde_json::from_str(&row.get::<_, String>(38)?).expect("stored de-hum settings parse"),
    )
    .with_de_click(
        serde_json::from_str(&row.get::<_, String>(39)?).expect("stored de-click settings parse"),
    )
    .with_channel_repair(
        serde_json::from_str(&row.get::<_, String>(40)?)
            .expect("stored channel repair settings parse"),
    )
    .with_compressor(echo_domain::CompressorSettings {
        enabled: row.get::<_, i64>(29)? != 0,
        threshold_centibels: i16::try_from(row.get::<_, i64>(30)?)
            .expect("compressor threshold fits centibels"),
        ratio_tenths: u16::try_from(row.get::<_, i64>(31)?).expect("compressor ratio fits tenths"),
        attack_millis: u16::try_from(row.get::<_, i64>(32)?)
            .expect("compressor attack fits milliseconds"),
        release_millis: u16::try_from(row.get::<_, i64>(33)?)
            .expect("compressor release fits milliseconds"),
        makeup_centibels: i16::try_from(row.get::<_, i64>(34)?)
            .expect("compressor makeup fits centibels"),
    })
    .with_reverb(serde_json::from_str(&row.get::<_, String>(35)?).expect("stored reverb parses"))
    .with_space(serde_json::from_str(&row.get::<_, String>(36)?).expect("stored space parses"))
    .with_effect_chain(
        serde_json::from_str(&row.get::<_, String>(41)?).expect("stored effect chain parses"),
    )
    .with_edit_timeline({
        let encoded = row.get::<_, String>(42)?;
        if encoded.trim().is_empty() || encoded.trim() == "[]" {
            echo_domain::EditTimeline::identity(trim_start_millis, trim_end_millis)
                .expect("legacy edit timeline restores")
        } else {
            serde_json::from_str(&encoded).expect("stored edit timeline parses")
        }
    })
    .with_effect_masks({
        let encoded = row.get::<_, String>(43)?;
        if encoded.trim().is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&encoded).expect("stored effect masks parse")
        }
    })
    .with_limiter(echo_domain::LimiterSettings {
        enabled: row.get::<_, i64>(44)? != 0,
        ceiling_centibels: i16::try_from(row.get::<_, i64>(45)?)
            .expect("limiter ceiling fits centibels"),
        release_millis: u16::try_from(row.get::<_, i64>(46)?)
            .expect("limiter release fits milliseconds"),
    })
    .with_creative_vfx(
        serde_json::from_str(&row.get::<_, String>(47)?)
            .expect("stored creative VFX settings parse"),
    );
    let graph = echo_domain::AdjustmentGraph::new(
        source_duration,
        trim_start_millis,
        trim_end_millis,
        u64::try_from(row.get::<_, i64>(22)?).expect("fade in is non-negative"),
        u64::try_from(row.get::<_, i64>(23)?).expect("fade out is non-negative"),
        effects,
    )
    .expect("stored adjustment is valid");
    Ok(Some(crate::AssetAdjustmentRevision {
        revision_id,
        graph,
        created_at_millis: row.get(48)?,
    }))
}

#[cfg(test)]
mod tests;
