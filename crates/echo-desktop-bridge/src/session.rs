//! The long-lived Library session: one catalog attachment for the desktop
//! process lifetime.

mod processing_recipe;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

use echo_catalog::{AssetLookup, Catalog, find_by_id, open_catalog, query_analysis};
use echo_core::load_or_build_waveform;
use echo_domain::AssetId;

use crate::ffi::{
    AnalysisStatusWire, AssetListeningStateWire, AssetSummaryWire, EditSegmentWire, EffectMaskWire,
    EqualizerBandWire, ImpulseResponseWire, JobStatsWire, KeywordFacetWire, LongAudioChapterWire,
    RevisitSnapshotWire, ScanRootWire, SearchHitWire, SmartAlbumWire, TranscriptSegmentWire,
    TranscriptWire, UserAlbumWire, WaveformArtifactWire, WaveformLevelWire,
};

pub(crate) fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}

fn encode_asset_ids(asset_ids: Vec<AssetId>) -> Vec<String> {
    asset_ids
        .into_iter()
        .map(|asset_id| asset_id.to_string())
        .collect()
}

fn metadata_entry(entries: &[echo_catalog::SourceMetadataEntry], keys: &[&str]) -> String {
    entries
        .iter()
        .find(|entry| keys.iter().any(|key| entry.key.eq_ignore_ascii_case(key)))
        .map_or_else(String::new, |entry| entry.value.clone())
}

fn metadata_entry_containing(
    entries: &[echo_catalog::SourceMetadataEntry],
    needle: &str,
) -> String {
    entries
        .iter()
        .find(|entry| entry.key.to_ascii_lowercase().contains(needle))
        .map_or_else(String::new, |entry| entry.value.clone())
}

fn analysis_status_wire(status: echo_catalog::AssetAnalysisStatus) -> AnalysisStatusWire {
    let state = status.state_str().to_owned();
    AnalysisStatusWire {
        asset_id: status.asset_id.to_string(),
        stage: status.stage.as_str().to_owned(),
        state,
        recovery: status.recovery.as_str().to_owned(),
        progress: status.progress,
        attempts: status.attempts,
        error_code: status.error_code.unwrap_or_default(),
        runtime_job_id: status.runtime_job_id.unwrap_or_default(),
        contract_version: status.contract_version,
    }
}

// The CXX ABI intentionally flattens independent bypass switches. Keeping the
// internal projection equally shaped makes omissions visible at compile time.
#[allow(clippy::struct_excessive_bools)]
struct AdjustmentWireFields {
    revision: i64,
    trim_start_millis: u64,
    trim_end_millis: u64,
    fade_in_millis: u64,
    fade_out_millis: u64,
    fade_in_curve: u8,
    fade_out_curve: u8,
    gain_centibels: i16,
    low_cut_hertz: u16,
    restoration_enabled: bool,
    de_plosive_enabled: bool,
    de_plosive_frequency_hertz: u16,
    de_plosive_sensitivity_percent: u8,
    de_plosive_reduction_centibels: u16,
    de_plosive_release_millis: u16,
    noise_reduction_enabled: bool,
    noise_reduction_centibels: u16,
    noise_reduction_sensitivity_percent: u8,
    noise_reduction_smoothing_millis: u16,
    de_esser_enabled: bool,
    de_esser_frequency_hertz: u16,
    de_esser_threshold_centibels: i16,
    de_esser_reduction_centibels: u16,
    de_hum_enabled: bool,
    de_hum_fundamental_hertz: u16,
    de_hum_harmonic_count: u8,
    de_hum_quality_tenths: u16,
    de_hum_depth_centibels: u16,
    de_click_enabled: bool,
    de_click_sensitivity_percent: u8,
    de_click_maximum_click_microseconds: u16,
    de_click_repair_percent: u8,
    channel_repair_enabled: bool,
    channel_repair_invert_left: bool,
    channel_repair_invert_right: bool,
    channel_repair_swap_channels: bool,
    channel_repair_mono_fold_down: bool,
    channel_repair_balance_percent: i8,
    equalizer_enabled: bool,
    equalizer_bands: Vec<EqualizerBandWire>,
    compressor_enabled: bool,
    compressor_threshold_centibels: i16,
    compressor_ratio_tenths: u16,
    compressor_attack_millis: u16,
    compressor_release_millis: u16,
    compressor_makeup_centibels: i16,
    reverb_character: u8,
    reverb_enabled: bool,
    reverb_mix_percent: u8,
    reverb_pre_delay_millis: u16,
    reverb_decay_millis: u16,
    reverb_size_percent: u8,
    reverb_damping_percent: u8,
    reverb_low_cut_hertz: u16,
    reverb_high_cut_hertz: u16,
    space_mode: u8,
    impulse_response_import_id: String,
    impulse_response_source_hash: String,
    impulse_response_prepared_hash: String,
    convolution_mix_percent: u8,
    convolution_wet_gain_centibels: i16,
    creative_vfx_json: String,
    limiter_enabled: bool,
    limiter_ceiling_centibels: i16,
    limiter_release_millis: u16,
    effect_chain: Vec<u8>,
    edit_segments: Vec<EditSegmentWire>,
    effect_masks: Vec<EffectMaskWire>,
}

struct SourceMetadataWireFields {
    container_format: String,
    sample_rate: u32,
    channel_count: u32,
    title: String,
    location: String,
    created_at: String,
}

fn source_metadata_wire_fields(
    metadata: Option<&echo_catalog::SourceMetadata>,
) -> SourceMetadataWireFields {
    metadata.map_or_else(
        || SourceMetadataWireFields {
            container_format: String::new(),
            sample_rate: 0,
            channel_count: 0,
            title: String::new(),
            location: String::new(),
            created_at: String::new(),
        },
        |metadata| SourceMetadataWireFields {
            container_format: metadata.container_format.clone(),
            sample_rate: metadata.sample_rate,
            channel_count: metadata.channel_count,
            title: metadata_entry(&metadata.entries, &["title", "stream.title"]),
            location: metadata_entry_containing(&metadata.entries, "location"),
            created_at: metadata_entry(
                &metadata.entries,
                &["creation_time", "stream.creation_time"],
            ),
        },
    )
}

fn equalizer_wire_bands(equalizer: echo_domain::ParametricEqualizer) -> Vec<EqualizerBandWire> {
    equalizer
        .bands()
        .into_iter()
        .map(|band| EqualizerBandWire {
            enabled: band.enabled,
            filter_kind: u8::try_from(band.filter_kind.catalog_value())
                .expect("equalizer filter catalog values fit u8"),
            frequency_hertz: band.frequency_hertz,
            q_hundredths: band.q_hundredths,
            gain_centibels: band.gain_centibels,
        })
        .collect()
}

fn equalizer_from_wire(
    enabled: bool,
    bands: &[EqualizerBandWire],
) -> Result<echo_domain::ParametricEqualizer, SessionError> {
    let authored: Vec<_> = bands
        .iter()
        .map(|band| {
            Ok(echo_domain::ParametricEqualizerBand::new(
                band.enabled,
                echo_domain::EqualizerFilterKind::from_catalog_value(i64::from(band.filter_kind))
                    .map_err(|error| SessionError {
                    message: error.to_string(),
                })?,
                band.frequency_hertz,
                band.q_hundredths,
                band.gain_centibels,
            ))
        })
        .collect::<Result<_, SessionError>>()?;
    let bands = authored.try_into().map_err(|_: Vec<_>| SessionError {
        message: "parametric equalizer must contain exactly six bands".to_owned(),
    })?;
    Ok(echo_domain::ParametricEqualizer::new(bands).with_enabled(enabled))
}

fn effect_chain_wire(chain: echo_domain::EffectChain) -> Vec<u8> {
    chain
        .nodes()
        .iter()
        .copied()
        .map(echo_domain::EffectNodeKind::wire_value)
        .collect()
}

fn effect_chain_from_wire(values: &[u8]) -> Result<echo_domain::EffectChain, SessionError> {
    let nodes: Vec<_> = values
        .iter()
        .copied()
        .map(|value| {
            echo_domain::EffectNodeKind::from_wire_value(value).map_err(|error| SessionError {
                message: error.to_string(),
            })
        })
        .collect::<Result<_, _>>()?;
    echo_domain::EffectChain::from_active_nodes(&nodes).map_err(|error| SessionError {
        message: error.to_string(),
    })
}

fn edit_segment_wire(segment: &echo_domain::EditSegment) -> EditSegmentWire {
    EditSegmentWire {
        source_start_millis: segment.source_start_millis(),
        source_end_millis: segment.source_end_millis(),
        state: segment.state().wire_value(),
        gain_centibels: segment.gain_centibels(),
        fade_in_millis: segment.fade_in_millis(),
        fade_out_millis: segment.fade_out_millis(),
        fade_in_curve: u8::try_from(segment.fade_in_curve().catalog_value())
            .expect("fade curve catalog values fit u8"),
        fade_out_curve: u8::try_from(segment.fade_out_curve().catalog_value())
            .expect("fade curve catalog values fit u8"),
        gap_after_millis: segment.gap_after_millis(),
    }
}

fn edit_timeline_from_wire(
    trim_start_millis: u64,
    trim_end_millis: u64,
    values: &[EditSegmentWire],
) -> Result<echo_domain::EditTimeline, SessionError> {
    if values.is_empty() {
        let segment = echo_domain::EditSegment::new(
            trim_start_millis,
            trim_end_millis,
            echo_domain::EditSegmentState::Audible,
            0,
            0,
            0,
            echo_domain::FadeCurves::new(
                echo_domain::FadeCurve::Linear,
                echo_domain::FadeCurve::Linear,
            ),
            0,
        )
        .map_err(|error| SessionError {
            message: error.to_string(),
        })?;
        return echo_domain::EditTimeline::new(trim_start_millis, trim_end_millis, vec![segment])
            .map_err(|error| SessionError {
                message: error.to_string(),
            });
    }
    let segments = values
        .iter()
        .map(|segment| {
            echo_domain::EditSegment::new(
                segment.source_start_millis,
                segment.source_end_millis,
                echo_domain::EditSegmentState::from_wire_value(segment.state).map_err(|error| {
                    SessionError {
                        message: error.to_string(),
                    }
                })?,
                segment.gain_centibels,
                segment.fade_in_millis,
                segment.fade_out_millis,
                echo_domain::FadeCurves::new(
                    echo_domain::FadeCurve::from_catalog_value(i64::from(segment.fade_in_curve))
                        .map_err(|error| SessionError {
                            message: error.to_string(),
                        })?,
                    echo_domain::FadeCurve::from_catalog_value(i64::from(segment.fade_out_curve))
                        .map_err(|error| SessionError {
                        message: error.to_string(),
                    })?,
                ),
                segment.gap_after_millis,
            )
            .map_err(|error| SessionError {
                message: error.to_string(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    echo_domain::EditTimeline::new(trim_start_millis, trim_end_millis, segments).map_err(|error| {
        SessionError {
            message: error.to_string(),
        }
    })
}

fn effect_mask_wire(mask: &echo_domain::EffectMask) -> EffectMaskWire {
    EffectMaskWire {
        start_millis: mask.start_millis(),
        end_millis: mask.end_millis(),
        feather_millis: mask.feather_millis(),
        effect_nodes: mask
            .effect_nodes()
            .iter()
            .copied()
            .map(echo_domain::EffectNodeKind::wire_value)
            .collect(),
    }
}

fn effect_masks_from_wire(
    values: &[EffectMaskWire],
) -> Result<Vec<echo_domain::EffectMask>, SessionError> {
    values
        .iter()
        .map(|mask| {
            let nodes = mask
                .effect_nodes
                .iter()
                .copied()
                .map(|value| {
                    echo_domain::EffectNodeKind::from_wire_value(value).map_err(|error| {
                        SessionError {
                            message: error.to_string(),
                        }
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            echo_domain::EffectMask::new(
                mask.start_millis,
                mask.end_millis,
                mask.feather_millis,
                nodes,
            )
            .map_err(|error| SessionError {
                message: error.to_string(),
            })
        })
        .collect()
}

fn channel_repair_from_wire(
    adjustment: &crate::ffi::AssetAdjustmentWire,
) -> echo_domain::ChannelRepairSettings {
    echo_domain::ChannelRepairSettings {
        enabled: adjustment.channel_repair_enabled,
        invert_left: adjustment.channel_repair_invert_left,
        invert_right: adjustment.channel_repair_invert_right,
        swap_channels: adjustment.channel_repair_swap_channels,
        mono_fold_down: adjustment.channel_repair_mono_fold_down,
        balance_percent: adjustment.channel_repair_balance_percent,
    }
}

fn space_settings_from_wire(
    adjustment: &crate::ffi::AssetAdjustmentWire,
) -> Result<echo_domain::SpaceSettings, SessionError> {
    let impulse_response = if adjustment.impulse_response_import_id.is_empty()
        && adjustment.impulse_response_source_hash.is_empty()
        && adjustment.impulse_response_prepared_hash.is_empty()
    {
        None
    } else {
        Some(echo_domain::ImpulseResponseSelection {
            import_id: uuid::Uuid::parse_str(&adjustment.impulse_response_import_id).map_err(
                |error| SessionError {
                    message: format!("invalid impulse response import id: {error}"),
                },
            )?,
            source_hash: echo_domain::ContentHash::from_str(
                &adjustment.impulse_response_source_hash,
            )
            .map_err(|error| SessionError {
                message: format!("invalid impulse response source hash: {error}"),
            })?,
            prepared_hash: echo_domain::ContentHash::from_str(
                &adjustment.impulse_response_prepared_hash,
            )
            .map_err(|error| SessionError {
                message: format!("invalid prepared impulse response hash: {error}"),
            })?,
        })
    };

    Ok(echo_domain::SpaceSettings {
        mode: echo_domain::SpaceMode::from_wire_value(adjustment.space_mode).map_err(|error| {
            SessionError {
                message: error.to_string(),
            }
        })?,
        impulse_response,
        convolution_mix_percent: adjustment.convolution_mix_percent,
        convolution_wet_gain_centibels: adjustment.convolution_wet_gain_centibels,
    })
}

fn adjustment_graph_from_wire(
    duration: u64,
    adjustment: &crate::ffi::AssetAdjustmentWire,
) -> Result<echo_domain::AdjustmentGraph, SessionError> {
    echo_domain::AdjustmentGraph::new(
        duration,
        adjustment.trim_start_millis,
        adjustment.trim_end_millis,
        adjustment.fade_in_millis,
        adjustment.fade_out_millis,
        echo_domain::AdjustmentEffects::new(
            echo_domain::FadeCurves::new(
                echo_domain::FadeCurve::from_catalog_value(i64::from(adjustment.fade_in_curve))
                    .map_err(|error| SessionError {
                        message: error.to_string(),
                    })?,
                echo_domain::FadeCurve::from_catalog_value(i64::from(adjustment.fade_out_curve))
                    .map_err(|error| SessionError {
                        message: error.to_string(),
                    })?,
            ),
            adjustment.gain_centibels,
            adjustment.low_cut_hertz,
        )
        .with_restoration(echo_domain::RestorationSettings {
            enabled: adjustment.restoration_enabled,
            de_plosive: echo_domain::DePlosiveSettings {
                enabled: adjustment.de_plosive_enabled,
                frequency_hertz: adjustment.de_plosive_frequency_hertz,
                sensitivity_percent: adjustment.de_plosive_sensitivity_percent,
                reduction_centibels: adjustment.de_plosive_reduction_centibels,
                release_millis: adjustment.de_plosive_release_millis,
            },
            noise_reduction: echo_domain::NoiseReductionSettings {
                enabled: adjustment.noise_reduction_enabled,
                reduction_centibels: adjustment.noise_reduction_centibels,
                sensitivity_percent: adjustment.noise_reduction_sensitivity_percent,
                smoothing_millis: adjustment.noise_reduction_smoothing_millis,
            },
            de_esser: echo_domain::DeEsserSettings {
                enabled: adjustment.de_esser_enabled,
                frequency_hertz: adjustment.de_esser_frequency_hertz,
                threshold_centibels: adjustment.de_esser_threshold_centibels,
                reduction_centibels: adjustment.de_esser_reduction_centibels,
            },
        })
        .with_de_hum(echo_domain::DeHumSettings {
            enabled: adjustment.de_hum_enabled,
            fundamental_hertz: adjustment.de_hum_fundamental_hertz,
            harmonic_count: adjustment.de_hum_harmonic_count,
            quality_tenths: adjustment.de_hum_quality_tenths,
            depth_centibels: adjustment.de_hum_depth_centibels,
        })
        .with_de_click(echo_domain::DeClickSettings {
            enabled: adjustment.de_click_enabled,
            sensitivity_percent: adjustment.de_click_sensitivity_percent,
            maximum_click_microseconds: adjustment.de_click_maximum_click_microseconds,
            repair_percent: adjustment.de_click_repair_percent,
        })
        .with_channel_repair(channel_repair_from_wire(adjustment))
        .with_equalizer(equalizer_from_wire(
            adjustment.equalizer_enabled,
            &adjustment.equalizer_bands,
        )?)
        .with_compressor(echo_domain::CompressorSettings {
            enabled: adjustment.compressor_enabled,
            threshold_centibels: adjustment.compressor_threshold_centibels,
            ratio_tenths: adjustment.compressor_ratio_tenths,
            attack_millis: adjustment.compressor_attack_millis,
            release_millis: adjustment.compressor_release_millis,
            makeup_centibels: adjustment.compressor_makeup_centibels,
        })
        .with_reverb(echo_domain::ReverbSettings {
            character: echo_domain::ReverbCharacter::from_wire_value(adjustment.reverb_character)
                .map_err(|error| SessionError {
                message: error.to_string(),
            })?,
            enabled: adjustment.reverb_enabled,
            mix_percent: adjustment.reverb_mix_percent,
            pre_delay_millis: adjustment.reverb_pre_delay_millis,
            decay_millis: adjustment.reverb_decay_millis,
            size_percent: adjustment.reverb_size_percent,
            damping_percent: adjustment.reverb_damping_percent,
            low_cut_hertz: adjustment.reverb_low_cut_hertz,
            high_cut_hertz: adjustment.reverb_high_cut_hertz,
        })
        .with_space(space_settings_from_wire(adjustment)?)
        .with_creative_vfx(creative_vfx_from_wire(&adjustment.creative_vfx_json)?)
        .with_limiter(echo_domain::LimiterSettings {
            enabled: adjustment.limiter_enabled,
            ceiling_centibels: adjustment.limiter_ceiling_centibels,
            release_millis: adjustment.limiter_release_millis,
        })
        .with_effect_chain(effect_chain_from_wire(&adjustment.effect_chain)?)
        .with_edit_timeline(edit_timeline_from_wire(
            adjustment.trim_start_millis,
            adjustment.trim_end_millis,
            &adjustment.edit_segments,
        )?)
        .with_effect_masks(effect_masks_from_wire(&adjustment.effect_masks)?),
    )
    .map_err(|error| SessionError {
        message: error.to_string(),
    })
}

fn creative_vfx_from_wire(encoded: &str) -> Result<echo_domain::CreativeVfxSettings, SessionError> {
    serde_json::from_str(encoded).map_err(|error| SessionError {
        message: format!("creative VFX settings are invalid: {error}"),
    })
}

#[allow(clippy::too_many_lines)] // Exhaustive flat ABI projection is intentional.
fn adjustment_wire_fields(
    adjustment: Option<echo_catalog::AssetAdjustmentRevision>,
    source_duration_millis: Option<u64>,
) -> AdjustmentWireFields {
    adjustment.map_or_else(
        || AdjustmentWireFields {
            revision: 0,
            trim_start_millis: 0,
            trim_end_millis: source_duration_millis.unwrap_or(0),
            fade_in_millis: 0,
            fade_out_millis: 0,
            fade_in_curve: 0,
            fade_out_curve: 0,
            gain_centibels: 0,
            low_cut_hertz: 0,
            restoration_enabled: true,
            de_plosive_enabled: false,
            de_plosive_frequency_hertz: 140,
            de_plosive_sensitivity_percent: 50,
            de_plosive_reduction_centibels: 1_200,
            de_plosive_release_millis: 160,
            noise_reduction_enabled: false,
            noise_reduction_centibels: 900,
            noise_reduction_sensitivity_percent: 50,
            noise_reduction_smoothing_millis: 240,
            de_esser_enabled: false,
            de_esser_frequency_hertz: 6_500,
            de_esser_threshold_centibels: -2_400,
            de_esser_reduction_centibels: 600,
            de_hum_enabled: false,
            de_hum_fundamental_hertz: 50,
            de_hum_harmonic_count: 4,
            de_hum_quality_tenths: 300,
            de_hum_depth_centibels: 2_400,
            de_click_enabled: false,
            de_click_sensitivity_percent: 50,
            de_click_maximum_click_microseconds: 1_000,
            de_click_repair_percent: 100,
            channel_repair_enabled: false,
            channel_repair_invert_left: false,
            channel_repair_invert_right: false,
            channel_repair_swap_channels: false,
            channel_repair_mono_fold_down: false,
            channel_repair_balance_percent: 0,
            equalizer_enabled: true,
            equalizer_bands: equalizer_wire_bands(echo_domain::ParametricEqualizer::flat()),
            compressor_enabled: false,
            compressor_threshold_centibels: -1_800,
            compressor_ratio_tenths: 30,
            compressor_attack_millis: 10,
            compressor_release_millis: 120,
            compressor_makeup_centibels: 0,
            reverb_character: echo_domain::ReverbCharacter::Room.wire_value(),
            reverb_enabled: false,
            reverb_mix_percent: 18,
            reverb_pre_delay_millis: 20,
            reverb_decay_millis: 1_800,
            reverb_size_percent: 55,
            reverb_damping_percent: 45,
            reverb_low_cut_hertz: 120,
            reverb_high_cut_hertz: 10_000,
            space_mode: echo_domain::SpaceMode::Algorithmic.wire_value(),
            impulse_response_import_id: String::new(),
            impulse_response_source_hash: String::new(),
            impulse_response_prepared_hash: String::new(),
            convolution_mix_percent: 35,
            convolution_wet_gain_centibels: 0,
            creative_vfx_json: serde_json::to_string(&echo_domain::CreativeVfxSettings::default())
                .expect("default creative VFX settings encode"),
            limiter_enabled: false,
            limiter_ceiling_centibels: -100,
            limiter_release_millis: 100,
            effect_chain: effect_chain_wire(echo_domain::EffectChain::standard()),
            edit_segments: source_duration_millis
                .filter(|duration| *duration > 0)
                .map(|duration| EditSegmentWire {
                    source_start_millis: 0,
                    source_end_millis: duration,
                    state: echo_domain::EditSegmentState::Audible.wire_value(),
                    gain_centibels: 0,
                    fade_in_millis: 0,
                    fade_out_millis: 0,
                    fade_in_curve: 0,
                    fade_out_curve: 0,
                    gap_after_millis: 0,
                })
                .into_iter()
                .collect(),
            effect_masks: Vec::new(),
        },
        |revision| AdjustmentWireFields {
            revision: revision.revision_id,
            trim_start_millis: revision.graph.trim_start_millis(),
            trim_end_millis: revision.graph.trim_end_millis(),
            fade_in_millis: revision.graph.fade_in_millis(),
            fade_out_millis: revision.graph.fade_out_millis(),
            fade_in_curve: u8::try_from(revision.graph.fade_in_curve().catalog_value())
                .expect("fade curve catalog values fit u8"),
            fade_out_curve: u8::try_from(revision.graph.fade_out_curve().catalog_value())
                .expect("fade curve catalog values fit u8"),
            gain_centibels: revision.graph.gain_centibels(),
            low_cut_hertz: revision.graph.low_cut_hertz(),
            restoration_enabled: revision.graph.restoration().enabled,
            de_plosive_enabled: revision.graph.restoration().de_plosive.enabled,
            de_plosive_frequency_hertz: revision.graph.restoration().de_plosive.frequency_hertz,
            de_plosive_sensitivity_percent: revision
                .graph
                .restoration()
                .de_plosive
                .sensitivity_percent,
            de_plosive_reduction_centibels: revision
                .graph
                .restoration()
                .de_plosive
                .reduction_centibels,
            de_plosive_release_millis: revision.graph.restoration().de_plosive.release_millis,
            noise_reduction_enabled: revision.graph.restoration().noise_reduction.enabled,
            noise_reduction_centibels: revision
                .graph
                .restoration()
                .noise_reduction
                .reduction_centibels,
            noise_reduction_sensitivity_percent: revision
                .graph
                .restoration()
                .noise_reduction
                .sensitivity_percent,
            noise_reduction_smoothing_millis: revision
                .graph
                .restoration()
                .noise_reduction
                .smoothing_millis,
            de_esser_enabled: revision.graph.restoration().de_esser.enabled,
            de_esser_frequency_hertz: revision.graph.restoration().de_esser.frequency_hertz,
            de_esser_threshold_centibels: revision.graph.restoration().de_esser.threshold_centibels,
            de_esser_reduction_centibels: revision.graph.restoration().de_esser.reduction_centibels,
            de_hum_enabled: revision.graph.de_hum().enabled,
            de_hum_fundamental_hertz: revision.graph.de_hum().fundamental_hertz,
            de_hum_harmonic_count: revision.graph.de_hum().harmonic_count,
            de_hum_quality_tenths: revision.graph.de_hum().quality_tenths,
            de_hum_depth_centibels: revision.graph.de_hum().depth_centibels,
            de_click_enabled: revision.graph.de_click().enabled,
            de_click_sensitivity_percent: revision.graph.de_click().sensitivity_percent,
            de_click_maximum_click_microseconds: revision
                .graph
                .de_click()
                .maximum_click_microseconds,
            de_click_repair_percent: revision.graph.de_click().repair_percent,
            channel_repair_enabled: revision.graph.channel_repair().enabled,
            channel_repair_invert_left: revision.graph.channel_repair().invert_left,
            channel_repair_invert_right: revision.graph.channel_repair().invert_right,
            channel_repair_swap_channels: revision.graph.channel_repair().swap_channels,
            channel_repair_mono_fold_down: revision.graph.channel_repair().mono_fold_down,
            channel_repair_balance_percent: revision.graph.channel_repair().balance_percent,
            equalizer_enabled: revision.graph.equalizer().enabled(),
            equalizer_bands: equalizer_wire_bands(revision.graph.equalizer()),
            compressor_enabled: revision.graph.compressor().enabled,
            compressor_threshold_centibels: revision.graph.compressor().threshold_centibels,
            compressor_ratio_tenths: revision.graph.compressor().ratio_tenths,
            compressor_attack_millis: revision.graph.compressor().attack_millis,
            compressor_release_millis: revision.graph.compressor().release_millis,
            compressor_makeup_centibels: revision.graph.compressor().makeup_centibels,
            reverb_character: revision.graph.reverb().character.wire_value(),
            reverb_enabled: revision.graph.reverb().enabled,
            reverb_mix_percent: revision.graph.reverb().mix_percent,
            reverb_pre_delay_millis: revision.graph.reverb().pre_delay_millis,
            reverb_decay_millis: revision.graph.reverb().decay_millis,
            reverb_size_percent: revision.graph.reverb().size_percent,
            reverb_damping_percent: revision.graph.reverb().damping_percent,
            reverb_low_cut_hertz: revision.graph.reverb().low_cut_hertz,
            reverb_high_cut_hertz: revision.graph.reverb().high_cut_hertz,
            space_mode: revision.graph.space().mode.wire_value(),
            impulse_response_import_id: revision
                .graph
                .space()
                .impulse_response
                .map_or_else(String::new, |selection| selection.import_id.to_string()),
            impulse_response_source_hash: revision
                .graph
                .space()
                .impulse_response
                .map_or_else(String::new, |selection| selection.source_hash.to_string()),
            impulse_response_prepared_hash: revision
                .graph
                .space()
                .impulse_response
                .map_or_else(String::new, |selection| selection.prepared_hash.to_string()),
            convolution_mix_percent: revision.graph.space().convolution_mix_percent,
            convolution_wet_gain_centibels: revision.graph.space().convolution_wet_gain_centibels,
            creative_vfx_json: serde_json::to_string(&revision.graph.creative_vfx())
                .expect("validated creative VFX settings encode"),
            limiter_enabled: revision.graph.limiter().enabled,
            limiter_ceiling_centibels: revision.graph.limiter().ceiling_centibels,
            limiter_release_millis: revision.graph.limiter().release_millis,
            effect_chain: effect_chain_wire(revision.graph.effect_chain()),
            edit_segments: revision
                .graph
                .edit_timeline()
                .segments()
                .iter()
                .map(edit_segment_wire)
                .collect(),
            effect_masks: revision
                .graph
                .effect_masks()
                .iter()
                .map(effect_mask_wire)
                .collect(),
        },
    )
}

fn contextual_preview(value: Option<&serde_json::Value>) -> (String, String) {
    value
        .and_then(|value| {
            serde_json::from_value::<echo_core::ContextualPayload>(value.clone()).ok()
        })
        .map_or((String::new(), String::new()), |payload| {
            (
                if payload.is_current() {
                    payload.sound_caption
                } else {
                    String::new()
                },
                payload.summary,
            )
        })
}

fn transcript_preview(value: Option<&serde_json::Value>) -> String {
    value
        .and_then(|value| {
            serde_json::from_value::<echo_core::TranscriptPayload>(value.clone()).ok()
        })
        .map_or_else(String::new, |payload| payload.text)
}

fn transcript_language(value: Option<&serde_json::Value>) -> String {
    value
        .and_then(|value| {
            serde_json::from_value::<echo_core::TranscriptPayload>(value.clone()).ok()
        })
        .and_then(|payload| payload.language)
        .unwrap_or_default()
}

struct AnalysisWireFields {
    stage: String,
    state: String,
    recovery: String,
    error_code: String,
    progress: u8,
    attempts: u32,
}

fn analysis_wire_fields(status: Option<&echo_catalog::AssetAnalysisStatus>) -> AnalysisWireFields {
    AnalysisWireFields {
        stage: status.map_or_else(String::new, |value| value.stage.as_str().to_owned()),
        state: status.map_or_else(
            || "missing".to_owned(),
            |value| value.state_str().to_owned(),
        ),
        recovery: status.map_or_else(
            || "none".to_owned(),
            |value| value.recovery.as_str().to_owned(),
        ),
        error_code: status
            .and_then(|value| value.error_code.clone())
            .unwrap_or_default(),
        progress: status.map_or(0, |value| value.progress),
        attempts: status.map_or(0, |value| value.attempts),
    }
}

// The flat CXX summary is intentionally exhaustive: keeping every field in
// one constructor makes projection omissions compile-time visible.
#[allow(clippy::too_many_lines)]
fn asset_summary_wire(
    asset: echo_catalog::AudioSpaceAsset,
    analysis: Option<&echo_catalog::AssetAnalysisStatus>,
    cache_root: &Path,
) -> AssetSummaryWire {
    let adjustment = adjustment_wire_fields(asset.adjustment, asset.duration_millis);
    let impulse_response_prepared_path =
        echo_domain::ContentHash::from_str(&adjustment.impulse_response_prepared_hash)
            .map(|hash| {
                echo_cache::blob_path(cache_root, &hash)
                    .to_string_lossy()
                    .into_owned()
            })
            .unwrap_or_default();
    let (sound_caption, summary) = contextual_preview(asset.contextual.as_ref());
    let text_preview = transcript_preview(asset.transcript.as_ref());
    let language = transcript_language(asset.transcript.as_ref());
    let source_metadata = source_metadata_wire_fields(asset.source_metadata.as_ref());
    let analysis = analysis_wire_fields(analysis);
    AssetSummaryWire {
        id: asset.id,
        path: asset.path.to_string_lossy().into_owned(),
        codec: asset.codec.unwrap_or_else(|| "unknown".to_owned()),
        duration_millis: asset.duration_millis.unwrap_or(0),
        recorded_at_millis: asset.recorded_at_millis.unwrap_or(0),
        imported_at_millis: asset.imported_at_millis,
        max_level: asset.max_level,
        path_status: asset.path_status,
        sound_caption,
        summary,
        event_type: asset.contextual_event_type.unwrap_or_default(),
        mood: asset.contextual_mood.unwrap_or_default(),
        keywords: asset.contextual_keywords,
        text_preview,
        language,
        analysis_stage: analysis.stage,
        analysis_state: analysis.state,
        analysis_recovery: analysis.recovery,
        analysis_error_code: analysis.error_code,
        analysis_progress: analysis.progress,
        analysis_attempts: analysis.attempts,
        liked: asset.liked,
        rating: asset.rating,
        last_listened_at_millis: asset.last_listened_at_millis,
        resume_position_millis: asset.resume_position_millis,
        adjustment_revision: adjustment.revision,
        trim_start_millis: adjustment.trim_start_millis,
        trim_end_millis: adjustment.trim_end_millis,
        fade_in_millis: adjustment.fade_in_millis,
        fade_out_millis: adjustment.fade_out_millis,
        fade_in_curve: adjustment.fade_in_curve,
        fade_out_curve: adjustment.fade_out_curve,
        gain_centibels: adjustment.gain_centibels,
        low_cut_hertz: adjustment.low_cut_hertz,
        restoration_enabled: adjustment.restoration_enabled,
        de_plosive_enabled: adjustment.de_plosive_enabled,
        de_plosive_frequency_hertz: adjustment.de_plosive_frequency_hertz,
        de_plosive_sensitivity_percent: adjustment.de_plosive_sensitivity_percent,
        de_plosive_reduction_centibels: adjustment.de_plosive_reduction_centibels,
        de_plosive_release_millis: adjustment.de_plosive_release_millis,
        noise_reduction_enabled: adjustment.noise_reduction_enabled,
        noise_reduction_centibels: adjustment.noise_reduction_centibels,
        noise_reduction_sensitivity_percent: adjustment.noise_reduction_sensitivity_percent,
        noise_reduction_smoothing_millis: adjustment.noise_reduction_smoothing_millis,
        de_esser_enabled: adjustment.de_esser_enabled,
        de_esser_frequency_hertz: adjustment.de_esser_frequency_hertz,
        de_esser_threshold_centibels: adjustment.de_esser_threshold_centibels,
        de_esser_reduction_centibels: adjustment.de_esser_reduction_centibels,
        de_hum_enabled: adjustment.de_hum_enabled,
        de_hum_fundamental_hertz: adjustment.de_hum_fundamental_hertz,
        de_hum_harmonic_count: adjustment.de_hum_harmonic_count,
        de_hum_quality_tenths: adjustment.de_hum_quality_tenths,
        de_hum_depth_centibels: adjustment.de_hum_depth_centibels,
        de_click_enabled: adjustment.de_click_enabled,
        de_click_sensitivity_percent: adjustment.de_click_sensitivity_percent,
        de_click_maximum_click_microseconds: adjustment.de_click_maximum_click_microseconds,
        de_click_repair_percent: adjustment.de_click_repair_percent,
        channel_repair_enabled: adjustment.channel_repair_enabled,
        channel_repair_invert_left: adjustment.channel_repair_invert_left,
        channel_repair_invert_right: adjustment.channel_repair_invert_right,
        channel_repair_swap_channels: adjustment.channel_repair_swap_channels,
        channel_repair_mono_fold_down: adjustment.channel_repair_mono_fold_down,
        channel_repair_balance_percent: adjustment.channel_repair_balance_percent,
        equalizer_enabled: adjustment.equalizer_enabled,
        equalizer_bands: adjustment.equalizer_bands,
        compressor_enabled: adjustment.compressor_enabled,
        compressor_threshold_centibels: adjustment.compressor_threshold_centibels,
        compressor_ratio_tenths: adjustment.compressor_ratio_tenths,
        compressor_attack_millis: adjustment.compressor_attack_millis,
        compressor_release_millis: adjustment.compressor_release_millis,
        compressor_makeup_centibels: adjustment.compressor_makeup_centibels,
        reverb_character: adjustment.reverb_character,
        reverb_enabled: adjustment.reverb_enabled,
        reverb_mix_percent: adjustment.reverb_mix_percent,
        reverb_pre_delay_millis: adjustment.reverb_pre_delay_millis,
        reverb_decay_millis: adjustment.reverb_decay_millis,
        reverb_size_percent: adjustment.reverb_size_percent,
        reverb_damping_percent: adjustment.reverb_damping_percent,
        reverb_low_cut_hertz: adjustment.reverb_low_cut_hertz,
        reverb_high_cut_hertz: adjustment.reverb_high_cut_hertz,
        space_mode: adjustment.space_mode,
        impulse_response_import_id: adjustment.impulse_response_import_id,
        impulse_response_source_hash: adjustment.impulse_response_source_hash,
        impulse_response_prepared_hash: adjustment.impulse_response_prepared_hash,
        impulse_response_prepared_path,
        convolution_mix_percent: adjustment.convolution_mix_percent,
        convolution_wet_gain_centibels: adjustment.convolution_wet_gain_centibels,
        creative_vfx_json: adjustment.creative_vfx_json,
        limiter_enabled: adjustment.limiter_enabled,
        limiter_ceiling_centibels: adjustment.limiter_ceiling_centibels,
        limiter_release_millis: adjustment.limiter_release_millis,
        effect_chain: adjustment.effect_chain,
        edit_segments: adjustment.edit_segments,
        effect_masks: adjustment.effect_masks,
        container_format: source_metadata.container_format,
        sample_rate: source_metadata.sample_rate,
        channel_count: source_metadata.channel_count,
        source_title: source_metadata.title,
        source_location: source_metadata.location,
        source_created_at: source_metadata.created_at,
    }
}

/// One catalog attachment. Sessions are created on the Qt main thread and
/// reused; the catalog serializes its own writes.
#[derive(Debug)]
pub struct LibrarySession {
    catalog: std::sync::Arc<Catalog>,
    catalog_path: PathBuf,
    cache_root: PathBuf,
    ir_source_root: PathBuf,
    workers: std::sync::Mutex<Option<echo_core::WorkerPool>>,
}

/// Error vocabulary for session operations.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct SessionError {
    pub message: String,
}

impl From<echo_catalog::CatalogError> for SessionError {
    fn from(error: echo_catalog::CatalogError) -> Self {
        Self {
            message: error.to_string(),
        }
    }
}

/// Opens (creating if needed) the catalog at `path` with the cache root at
/// `cache_root`.
///
/// # Errors
///
/// Returns [`SessionError`] when the catalog cannot be opened.
pub fn open_session(path: &str, cache_root: &str) -> Result<LibrarySession, SessionError> {
    let catalog_path = PathBuf::from(path);
    let catalog = open_catalog(&catalog_path).map_err(|error| SessionError {
        message: error.to_string(),
    })?;
    let ir_source_root = catalog_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
        .join("impulse-responses");
    Ok(LibrarySession {
        catalog: std::sync::Arc::new(catalog),
        catalog_path,
        cache_root: PathBuf::from(cache_root),
        ir_source_root,
        workers: std::sync::Mutex::new(None),
    })
}

impl Drop for LibrarySession {
    fn drop(&mut self) {
        if let Some(workers) = self.workers.lock().expect("worker mutex poisoned").take() {
            workers.stop();
        }
    }
}

impl LibrarySession {
    fn ir_pipeline(&self) -> Result<echo_ir::IrImportPipeline, SessionError> {
        echo_ir::IrImportPipeline::open(&self.ir_source_root, &self.cache_root).map_err(|error| {
            SessionError {
                message: error.to_string(),
            }
        })
    }

    fn prepared_ir_path(
        &self,
        record: &echo_catalog::ImpulseResponseRecord,
    ) -> Result<PathBuf, SessionError> {
        let store =
            echo_cache::open_blob_store(&self.cache_root).map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        if let Ok(path) =
            echo_cache::verify_blob(&store, record.prepared_hash, record.prepared_size_bytes)
        {
            return Ok(path);
        }
        let pipeline = self.ir_pipeline()?;
        let rebuilt = pipeline
            .prepare_owned_source(&echo_ir::StoredIrSource {
                source_hash: record.source_hash,
                source_size_bytes: record.source_size_bytes,
                outcome: echo_ir::ImportOutcome::AlreadyPresent,
            })
            .map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        if rebuilt.prepared_hash != record.prepared_hash
            || rebuilt.preparation_version != record.preparation_version
        {
            return Err(SessionError {
                message: "rebuilt impulse response does not match its Catalog evidence".to_owned(),
            });
        }
        Ok(rebuilt.cache_path)
    }

    fn impulse_response_wire(
        &self,
        record: echo_catalog::ImpulseResponseRecord,
    ) -> Result<ImpulseResponseWire, SessionError> {
        let prepared_path = self.prepared_ir_path(&record)?;
        let (rights_kind, spdx_expression, license_url) = match record.rights {
            echo_catalog::ImpulseResponseRights::Spdx {
                expression,
                license_url,
            } => (
                "spdx".to_owned(),
                expression,
                license_url.unwrap_or_default(),
            ),
            echo_catalog::ImpulseResponseRights::UserOwnedNoRedistribution => (
                "user_owned_no_redistribution".to_owned(),
                String::new(),
                String::new(),
            ),
        };
        Ok(ImpulseResponseWire {
            import_id: record.import_id.to_string(),
            source_hash: record.source_hash.to_string(),
            prepared_hash: record.prepared_hash.to_string(),
            prepared_path: prepared_path.to_string_lossy().into_owned(),
            display_name: record.display_name,
            creator: record.creator.unwrap_or_default(),
            source_url: record.source_url.unwrap_or_default(),
            attribution: record.attribution.unwrap_or_default(),
            rights_kind,
            spdx_expression,
            license_url,
            imported_at_millis: record.imported_at_millis,
            source_sample_rate: record.source_sample_rate,
            channel_count: record.channel_count,
            prepared_frame_count: record.prepared_frame_count,
        })
    }

    pub fn impulse_responses(&self) -> Result<Vec<ImpulseResponseWire>, SessionError> {
        let records = self
            .catalog
            .with_transaction(echo_catalog::list_impulse_responses)
            .map_err(SessionError::from)?;
        records
            .into_iter()
            .map(|record| self.impulse_response_wire(record))
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn import_impulse_response(
        &self,
        source_path: &str,
        display_name: &str,
        creator: &str,
        source_url: &str,
        attribution: &str,
        rights_kind: &str,
        spdx_expression: &str,
        license_url: &str,
    ) -> Result<ImpulseResponseWire, SessionError> {
        let optional = |value: &str| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_owned())
        };
        let rights = match rights_kind {
            "spdx" => echo_ir::IrRightsDeclaration::Spdx {
                expression: spdx_expression.trim().to_owned(),
                license_url: optional(license_url),
            },
            "user_owned_no_redistribution" => {
                echo_ir::IrRightsDeclaration::UserOwnedNoRedistribution
            }
            _ => {
                return Err(SessionError {
                    message: "impulse response rights declaration is invalid".to_owned(),
                });
            }
        };
        let imported = self
            .ir_pipeline()?
            .import_local_wav(
                Path::new(source_path),
                echo_ir::IrImportProvenance {
                    display_name: display_name.trim().to_owned(),
                    creator: optional(creator),
                    source_url: optional(source_url),
                    attribution: optional(attribution),
                    rights,
                },
            )
            .map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        let rights = match imported.record.provenance.rights {
            echo_ir::IrRightsDeclaration::Spdx {
                expression,
                license_url,
            } => echo_catalog::ImpulseResponseRights::Spdx {
                expression,
                license_url,
            },
            echo_ir::IrRightsDeclaration::UserOwnedNoRedistribution => {
                echo_catalog::ImpulseResponseRights::UserOwnedNoRedistribution
            }
        };
        let record = echo_catalog::ImpulseResponseRecord {
            import_id: imported.record.import_id,
            source_hash: imported.record.source_hash,
            source_size_bytes: imported.record.source_size_bytes,
            prepared_hash: imported.preparation.prepared_hash,
            prepared_size_bytes: imported.preparation.size_bytes,
            preparation_version: imported.preparation.preparation_version,
            source_sample_rate: imported.preparation.source_sample_rate,
            channel_count: imported.preparation.channel_count,
            source_frame_count: imported.preparation.source_frame_count,
            prepared_frame_count: imported.preparation.prepared_frame_count,
            avcodec_version: imported.preparation.avcodec_version,
            swresample_version: imported.preparation.swresample_version,
            imported_at_millis: imported.record.imported_at_millis,
            original_path: imported.record.original_path,
            display_name: imported.record.provenance.display_name,
            creator: imported.record.provenance.creator,
            source_url: imported.record.provenance.source_url,
            attribution: imported.record.provenance.attribution,
            rights,
        };
        self.catalog
            .with_transaction(|transaction| {
                echo_catalog::record_impulse_response(transaction, &record)
            })
            .map_err(SessionError::from)?;
        self.impulse_response_wire(record)
    }

    /// Lists registered assets, newest import first.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the catalog read fails.
    pub fn list_assets(&self) -> Result<Vec<AssetSummaryWire>, SessionError> {
        let (projection, statuses) = self
            .catalog
            .with_transaction(|transaction| -> Result<_, echo_catalog::CatalogError> {
                Ok((
                    echo_catalog::list_audio_space(transaction)?,
                    echo_catalog::list_asset_analysis_statuses(
                        transaction,
                        echo_core::CONTEXTUAL_SCHEMA_VERSION,
                        echo_core::CONTEXTUAL_JOB_REVISION,
                        echo_core::LONG_AUDIO_PLAN_VERSION,
                    )?,
                ))
            })
            .map_err(SessionError::from)?;
        let mut statuses = statuses
            .into_iter()
            .map(|status| (status.asset_id.to_string(), status))
            .collect::<HashMap<_, _>>();
        Ok(projection
            .into_iter()
            .map(|asset| {
                let status = statuses.remove(&asset.id);
                asset_summary_wire(asset, status.as_ref(), &self.cache_root)
            })
            .collect())
    }

    /// Lists contextual keyword facets using the Catalog's latest-evidence
    /// projection rather than aggregating model strings in QML.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the aggregate query fails.
    pub fn keyword_facets(&self) -> Result<Vec<KeywordFacetWire>, SessionError> {
        let facets = self
            .catalog
            .with_transaction(echo_catalog::list_contextual_keyword_facets)
            .map_err(SessionError::from)?;
        Ok(facets
            .into_iter()
            .map(|facet| KeywordFacetWire {
                key: facet.key,
                label: facet.label,
                count: facet.count,
            })
            .collect())
    }

    /// Lists smart album candidates projected by the Catalog. Membership is
    /// explicit so QML never needs to reinterpret evidence strings.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the candidate projection fails.
    pub fn smart_albums(&self) -> Result<Vec<SmartAlbumWire>, SessionError> {
        let candidates = self
            .catalog
            .with_transaction(echo_catalog::list_smart_album_candidates)
            .map_err(SessionError::from)?;
        Ok(candidates
            .into_iter()
            .map(|candidate| SmartAlbumWire {
                key: candidate.key,
                label: candidate.label,
                facet: candidate.facet.as_str().to_owned(),
                evidence: candidate.evidence.as_str().to_owned(),
                count: u64::try_from(candidate.member_asset_ids.len()).unwrap_or(u64::MAX),
                member_asset_ids: candidate
                    .member_asset_ids
                    .into_iter()
                    .map(|asset_id| asset_id.to_string())
                    .collect(),
            })
            .collect())
    }

    /// Lists user-authored albums and their explicit member identities.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the Catalog projection fails.
    pub fn user_albums(&self) -> Result<Vec<UserAlbumWire>, SessionError> {
        let albums = self
            .catalog
            .with_transaction(echo_catalog::list_user_albums)
            .map_err(SessionError::from)?;
        Ok(albums
            .into_iter()
            .map(|album| UserAlbumWire {
                id: album.id,
                name: album.name,
                cover_asset_id: album
                    .cover_asset_id
                    .map_or_else(String::new, |asset_id| asset_id.to_string()),
                count: u64::try_from(album.member_asset_ids.len()).unwrap_or(u64::MAX),
                member_asset_ids: album
                    .member_asset_ids
                    .into_iter()
                    .map(|asset_id| asset_id.to_string())
                    .collect(),
                created_at_millis: album.created_at_millis,
                updated_at_millis: album.updated_at_millis,
            })
            .collect())
    }

    /// Projects bounded Revisit sections without exposing listening policy to
    /// QML or loading unbounded history into the home surface.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the Catalog projection fails.
    pub fn revisit_snapshot(&self, now_millis: i64) -> Result<RevisitSnapshotWire, SessionError> {
        let snapshot = self
            .catalog
            .with_transaction(|transaction| echo_catalog::revisit_snapshot(transaction, now_millis))
            .map_err(SessionError::from)?;
        Ok(RevisitSnapshotWire {
            continue_listening: encode_asset_ids(snapshot.continue_listening_asset_ids),
            recently_listened: encode_asset_ids(snapshot.recently_listened_asset_ids),
            on_this_day: encode_asset_ids(snapshot.on_this_day_asset_ids),
            recently_added: encode_asset_ids(snapshot.recently_added_asset_ids),
        })
    }

    /// Creates an empty user album or atomically snapshots suggested members.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the name, an asset identity, or the
    /// Catalog write is invalid.
    pub fn create_user_album(
        &self,
        name: &str,
        member_asset_ids: &[String],
    ) -> Result<i64, SessionError> {
        let members = member_asset_ids
            .iter()
            .map(|asset_id| {
                AssetId::from_str(asset_id).map_err(|error| SessionError {
                    message: format!("invalid album member id {asset_id}: {error}"),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.catalog
            .with_transaction(|transaction| {
                echo_catalog::create_user_album(
                    transaction,
                    echo_catalog::CreateUserAlbum {
                        name,
                        member_asset_ids: &members,
                    },
                    now_millis(),
                )
            })
            .map_err(SessionError::from)
    }

    /// Renames one user album.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the album, name, or write is invalid.
    pub fn rename_user_album(&self, album_id: i64, name: &str) -> Result<(), SessionError> {
        self.catalog
            .with_transaction(|transaction| {
                echo_catalog::rename_user_album(transaction, album_id, name, now_millis())
            })
            .map_err(SessionError::from)
    }

    /// Deletes one user album without touching any asset.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the album is unknown or the write fails.
    pub fn delete_user_album(&self, album_id: i64) -> Result<(), SessionError> {
        self.catalog
            .with_transaction(|transaction| echo_catalog::delete_user_album(transaction, album_id))
            .map_err(SessionError::from)
    }

    /// Adds or removes one explicit album member.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when an identity is invalid or the write fails.
    pub fn set_user_album_membership(
        &self,
        album_id: i64,
        asset_id: &str,
        included: bool,
    ) -> Result<bool, SessionError> {
        let asset_id = AssetId::from_str(asset_id).map_err(|error| SessionError {
            message: format!("invalid album member id {asset_id}: {error}"),
        })?;
        self.catalog
            .with_transaction(|transaction| {
                echo_catalog::set_user_album_membership(
                    transaction,
                    album_id,
                    asset_id,
                    included,
                    now_millis(),
                )
            })
            .map_err(SessionError::from)
    }

    /// Stores user-owned Like and rating state for one sound.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the asset identity or write is invalid.
    pub fn set_asset_affinity(
        &self,
        asset_id: &str,
        liked: bool,
        rating: u8,
    ) -> Result<(), SessionError> {
        let asset_id = AssetId::from_str(asset_id).map_err(|error| SessionError {
            message: format!("invalid asset id {asset_id}: {error}"),
        })?;
        self.catalog
            .with_transaction(|transaction| {
                echo_catalog::set_asset_affinity(
                    transaction,
                    asset_id,
                    echo_catalog::AssetAffinity { liked, rating },
                    now_millis(),
                )
            })
            .map_err(SessionError::from)
    }

    /// Records one bounded listening checkpoint without touching affinity or
    /// authored sound adjustments.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the asset identity, interval, position,
    /// or catalog write is invalid.
    pub fn record_listening_progress(
        &self,
        asset_id: &str,
        position_millis: u64,
        playback_start_millis: u64,
        playback_end_millis: u64,
    ) -> Result<AssetListeningStateWire, SessionError> {
        let asset_id = AssetId::from_str(asset_id).map_err(|error| SessionError {
            message: format!("invalid asset id {asset_id}: {error}"),
        })?;
        self.catalog
            .with_transaction(|transaction| {
                echo_catalog::record_asset_listening_progress(
                    transaction,
                    asset_id,
                    position_millis,
                    playback_start_millis,
                    playback_end_millis,
                    now_millis(),
                )
            })
            .map(|state| AssetListeningStateWire {
                last_listened_at_millis: state.last_listened_at_millis,
                resume_position_millis: state.resume_position_millis,
            })
            .map_err(SessionError::from)
    }

    /// Appends one validated, non-destructive adjustment graph.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the asset is unknown, lacks a duration,
    /// or the authored range, fades, or gain violate the product contract.
    pub fn set_asset_adjustment(
        &self,
        asset_id: &str,
        adjustment: &crate::ffi::AssetAdjustmentWire,
    ) -> Result<(), SessionError> {
        let asset_id = AssetId::from_str(asset_id).map_err(|error| SessionError {
            message: format!("invalid asset id {asset_id}: {error}"),
        })?;
        let duration = self.catalog.with_transaction(|transaction| {
            match find_by_id(transaction, asset_id) {
                Ok(AssetLookup::Found(asset)) => {
                    asset.original.duration_millis.ok_or_else(|| SessionError {
                        message: "asset duration is not available".to_owned(),
                    })
                }
                Ok(AssetLookup::NotFound) => Err(SessionError {
                    message: format!("asset {asset_id} not found"),
                }),
                Err(error) => Err(SessionError {
                    message: error.to_string(),
                }),
            }
        })?;
        let graph = adjustment_graph_from_wire(duration, adjustment)?;
        self.catalog
            .with_transaction(|transaction| {
                echo_catalog::record_adjustment_graph(transaction, asset_id, graph, now_millis())
            })
            .map(|_| ())
            .map_err(SessionError::from)
    }

    /// Returns the waveform artifact for an asset, building and caching it
    /// when absent.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the asset is unknown or the artifact
    /// cannot be built, read, or decoded.
    pub fn waveform_artifact(&self, asset_id: &str) -> Result<WaveformArtifactWire, SessionError> {
        let id = AssetId::from_str(asset_id).map_err(|error| SessionError {
            message: format!("invalid asset id {asset_id}: {error}"),
        })?;
        let source =
            self.catalog
                .with_transaction(|transaction| match find_by_id(transaction, id) {
                    Ok(AssetLookup::Found(asset)) => Ok(asset.original.path),
                    Ok(AssetLookup::NotFound) => Err(SessionError {
                        message: format!("asset {asset_id} not found"),
                    }),
                    Err(error) => Err(SessionError {
                        message: error.to_string(),
                    }),
                })?;
        let payload = load_or_build_waveform(&self.catalog, id, &source, &self.cache_root, 8)
            .map_err(|error| SessionError {
                message: format!("cannot build waveform for {}: {error}", source.display()),
            })?;
        Ok(WaveformArtifactWire {
            canonical_sample_rate: payload.canonical_sample_rate,
            levels: payload
                .levels
                .into_iter()
                .map(|level| WaveformLevelWire {
                    samples_per_bucket: level.samples_per_bucket,
                    mins: level.mins,
                    maxs: level.maxs,
                })
                .collect(),
        })
    }

    /// Returns every transcript evidence record for an asset, newest first.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the catalog read fails.
    pub fn transcripts(&self, asset_id: &str) -> Result<Vec<TranscriptWire>, SessionError> {
        let id = AssetId::from_str(asset_id).map_err(|error| SessionError {
            message: format!("invalid asset id {asset_id}: {error}"),
        })?;
        let records = self
            .catalog
            .with_transaction(|transaction| query_analysis(transaction, id))
            .map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        let mut wires = Vec::new();
        let alignment = records
            .iter()
            .find(|record| record.kind == echo_domain::AnalysisKind::Alignment)
            .and_then(|record| {
                serde_json::from_value::<echo_core::AlignmentPayload>(record.value.clone()).ok()
            });
        for record in records {
            if record.kind != echo_domain::AnalysisKind::Transcript {
                continue;
            }
            let Ok(payload) = serde_json::from_value::<echo_core::TranscriptPayload>(record.value)
            else {
                continue;
            };
            let segments = aligned_segments(&payload, alignment.as_ref());
            wires.push(TranscriptWire {
                model: record.model.name,
                model_version: record.model.version,
                language: payload.language.unwrap_or_default(),
                text: payload.text,
                segments: segments
                    .into_iter()
                    .map(|segment| TranscriptSegmentWire {
                        text: segment.text,
                        start: segment.start,
                        end: segment.end,
                    })
                    .collect(),
            });
        }
        Ok(wires)
    }

    /// Returns every persisted long-recording outline node. Level zero is the
    /// leaf/chapter view; higher levels are progressively coarser summaries.
    pub fn long_audio_chapters(
        &self,
        asset_id: &str,
    ) -> Result<Vec<LongAudioChapterWire>, SessionError> {
        let asset_id = AssetId::from_str(asset_id).map_err(|error| SessionError {
            message: error.to_string(),
        })?;
        let nodes = self
            .catalog
            .with_transaction(
                |transaction| -> Result<Vec<_>, echo_catalog::CatalogError> {
                    echo_catalog::list_long_audio_outline_nodes(
                        transaction,
                        asset_id,
                        echo_core::LONG_AUDIO_PLAN_VERSION,
                    )
                },
            )
            .map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        nodes
            .into_iter()
            .map(|node| {
                let payload =
                    node.contextual
                        .get("payload")
                        .cloned()
                        .ok_or_else(|| SessionError {
                            message: "long-audio outline lacks contextual payload".to_owned(),
                        })?;
                let payload: echo_core::ContextualPayload = serde_json::from_value(payload)
                    .map_err(|error| SessionError {
                        message: format!("long-audio outline is invalid: {error}"),
                    })?;
                Ok(LongAudioChapterWire {
                    level: node.level,
                    index: node.index,
                    start_millis: node.start_millis,
                    end_millis: node.end_millis,
                    sound_caption: payload.sound_caption,
                    summary: payload.summary,
                })
            })
            .collect()
    }

    /// Starts the background worker pool (idempotent).
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the pool cannot start.
    pub fn start_workers(&self, runtime_endpoint: &str) -> Result<(), SessionError> {
        let mut workers = self.workers.lock().expect("worker mutex poisoned");
        if workers.is_some() {
            return Ok(());
        }
        // Missing or unsafe credentials must not prevent structural Library
        // jobs from running. Inference then fails through its sanitized
        // authentication path while scanning and waveform work continues.
        let bearer_token = echo_core::load_infer_runtime_credential().map_or_else(
            |_| String::new(),
            echo_core::InferRuntimeCredential::into_bearer_token,
        );
        let config = echo_core::WorkerConfig {
            cache_root: self.cache_root.clone(),
            infer_runtime: echo_core::InferRuntimeConfig {
                base_url: runtime_endpoint.to_owned(),
                bearer_token,
            },
        };
        let pool = echo_core::WorkerPool::start(&self.catalog, &config, 2).map_err(|error| {
            SessionError {
                message: format!("cannot start workers: {error}"),
            }
        })?;
        *workers = Some(pool);
        Ok(())
    }

    /// Returns the current process-local worker transition revision.
    #[must_use]
    pub fn worker_state_revision(&self) -> u64 {
        self.workers
            .lock()
            .expect("worker mutex poisoned")
            .as_ref()
            .map_or(0, echo_core::WorkerPool::state_revision)
    }

    /// Returns the product stage and sanitized Runtime linkage for one asset.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the asset identity or catalog projection
    /// is invalid.
    pub fn analysis_status(&self, asset_id: &str) -> Result<AnalysisStatusWire, SessionError> {
        let asset_id = AssetId::from_str(asset_id).map_err(|error| SessionError {
            message: format!("invalid asset id {asset_id}: {error}"),
        })?;
        self.analysis_statuses()?
            .into_iter()
            .find(|status| status.asset_id == asset_id.to_string())
            .ok_or_else(|| SessionError {
                message: format!("analysis status is unavailable for {asset_id}"),
            })
    }

    /// Returns payload-free stage and recovery state for every asset.
    pub fn analysis_statuses(&self) -> Result<Vec<AnalysisStatusWire>, SessionError> {
        let statuses = self
            .catalog
            .with_transaction(|transaction| {
                echo_catalog::list_asset_analysis_statuses(
                    transaction,
                    echo_core::CONTEXTUAL_SCHEMA_VERSION,
                    echo_core::CONTEXTUAL_JOB_REVISION,
                    echo_core::LONG_AUDIO_PLAN_VERSION,
                )
            })
            .map_err(SessionError::from)?;
        Ok(statuses.into_iter().map(analysis_status_wire).collect())
    }

    /// Requeues the failed product analysis stage for one asset.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when no failed stage can be retried.
    pub fn retry_analysis(&self, asset_id: &str) -> Result<(), SessionError> {
        let id = AssetId::from_str(asset_id).map_err(|error| SessionError {
            message: format!("invalid asset id {asset_id}: {error}"),
        })?;
        let status = self
            .catalog
            .with_transaction(|transaction| {
                echo_catalog::list_asset_analysis_statuses(
                    transaction,
                    echo_core::CONTEXTUAL_SCHEMA_VERSION,
                    echo_core::CONTEXTUAL_JOB_REVISION,
                    echo_core::LONG_AUDIO_PLAN_VERSION,
                )
            })
            .map_err(SessionError::from)?
            .into_iter()
            .find(|status| status.asset_id == id)
            .ok_or_else(|| SessionError {
                message: format!("analysis status is unavailable for {id}"),
            })?;
        let retried = self
            .catalog
            .with_transaction(|transaction| {
                echo_catalog::retry_job(transaction, &status.job_id, now_millis())
            })
            .map_err(SessionError::from)?;
        if retried {
            Ok(())
        } else {
            Err(SessionError {
                message: "analysis stage is not failed or cancelled".to_owned(),
            })
        }
    }

    /// Requeues all current manually recoverable stages with present sources.
    pub fn retry_failed_analysis(&self) -> Result<u64, SessionError> {
        self.catalog
            .with_transaction(|transaction| {
                echo_catalog::requeue_manual_analysis(
                    transaction,
                    echo_core::CONTEXTUAL_SCHEMA_VERSION,
                    echo_core::CONTEXTUAL_JOB_REVISION,
                    echo_core::LONG_AUDIO_PLAN_VERSION,
                    now_millis(),
                )
            })
            .map_err(SessionError::from)
    }

    /// Queues scans for every enabled root (incremental detection).
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when queueing fails.
    pub fn queue_scans(&self) -> Result<u64, SessionError> {
        echo_core::queue_scans_for_enabled_roots(&self.catalog, now_millis()).map_err(|error| {
            SessionError {
                message: error.to_string(),
            }
        })
    }

    /// Reads aggregate job statistics.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the read fails.
    pub fn job_stats(&self) -> Result<JobStatsWire, SessionError> {
        let stats = self
            .catalog
            .with_transaction(echo_catalog::job_stats)
            .map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        Ok(JobStatsWire {
            pending: stats.pending,
            running: stats.running,
            done: stats.done,
            failed: stats.failed,
        })
    }

    /// Lists configured scan roots.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the read fails.
    pub fn list_roots(&self) -> Result<Vec<ScanRootWire>, SessionError> {
        let roots = self
            .catalog
            .with_transaction(echo_catalog::list_scan_roots)
            .map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        Ok(roots
            .into_iter()
            .map(|root| ScanRootWire {
                id: root.id,
                root: root.root.to_string_lossy().into_owned(),
                enabled: root.enabled,
            })
            .collect())
    }

    /// Adds a scan root and queues its scan.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the write fails.
    pub fn add_root(&self, root: &str) -> Result<(), SessionError> {
        echo_core::add_root_and_scan(&self.catalog, std::path::Path::new(root), now_millis())
            .map_err(|error| SessionError {
                message: error.to_string(),
            })
    }

    /// Removes a scan root by id.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the write fails.
    pub fn remove_root(&self, id: i64) -> Result<(), SessionError> {
        self.catalog
            .with_transaction(|transaction| echo_catalog::remove_scan_root(transaction, id))
            .map_err(|error| SessionError {
                message: error.to_string(),
            })
    }

    /// Full-text search over indexed transcripts. Each hit carries the best
    /// matching segment's start time so the UI can jump straight to it.
    ///
    /// # Errors
    ///
    /// Returns [`SessionError`] when the search fails.
    pub fn search(&self, query: &str, limit: u64) -> Result<Vec<SearchHitWire>, SessionError> {
        let hits = self
            .catalog
            .with_transaction(
                |transaction| -> Result<Vec<_>, echo_catalog::CatalogError> {
                    let mut hits = echo_catalog::search_semantic_text(transaction, query, limit)?;
                    let mut seen = hits
                        .iter()
                        .map(|hit| hit.asset_id.clone())
                        .collect::<std::collections::BTreeSet<_>>();
                    for hit in echo_catalog::search_transcripts(transaction, query, limit)? {
                        if seen.insert(hit.asset_id.clone()) {
                            hits.push(hit);
                        }
                    }
                    hits.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
                    Ok(hits)
                },
            )
            .map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        let mut wires = Vec::new();
        for hit in hits {
            let id = AssetId::from_str(&hit.asset_id).map_err(|error| SessionError {
                message: format!("bad asset id {}: {error}", hit.asset_id),
            })?;
            let asset =
                self.catalog
                    .with_transaction(|transaction| match find_by_id(transaction, id) {
                        Ok(AssetLookup::Found(asset)) => Ok(asset),
                        Ok(AssetLookup::NotFound) => Err(SessionError {
                            message: format!("asset {} not found", hit.asset_id),
                        }),
                        Err(error) => Err(SessionError {
                            message: error.to_string(),
                        }),
                    })?;
            let start_millis = self.best_segment_start_millis(id, query)?;
            wires.push(SearchHitWire {
                asset_id: hit.asset_id,
                path: asset.original.path.to_string_lossy().into_owned(),
                codec: asset
                    .original
                    .codec
                    .clone()
                    .unwrap_or_else(|| "unknown".to_owned()),
                snippet: hit.snippet,
                start_millis,
            });
        }
        Ok(wires)
    }

    /// Finds the transcript segment whose text best overlaps the query and
    /// returns its start time in milliseconds.
    fn best_segment_start_millis(
        &self,
        asset_id: AssetId,
        query: &str,
    ) -> Result<u64, SessionError> {
        let records = self
            .catalog
            .with_transaction(|transaction| query_analysis(transaction, asset_id))
            .map_err(|error| SessionError {
                message: error.to_string(),
            })?;
        let transcript = records
            .iter()
            .find(|record| record.kind == echo_domain::AnalysisKind::Transcript)
            .and_then(|record| {
                serde_json::from_value::<echo_core::TranscriptPayload>(record.value.clone()).ok()
            });
        let Some(transcript) = transcript else {
            return Ok(0);
        };
        let query_chars: Vec<char> = query.chars().filter(|c| !c.is_whitespace()).collect();
        let mut best_start = None;
        let mut best_overlap = 0usize;
        for segment in &transcript.segments {
            let overlap = query_chars
                .iter()
                .filter(|c| segment.text.contains(**c))
                .count();
            if overlap > best_overlap {
                best_overlap = overlap;
                best_start = Some(segment.start);
            }
        }
        // Segment starts are seconds bounded by recording length; the
        // float-to-int conversion cannot lose meaningful precision here.
        #[allow(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss
        )]
        let start_millis = (best_start.unwrap_or(0.0).max(0.0) * 1000.0) as u64;
        Ok(start_millis)
    }

    /// Shared catalog attachment for adjacent session workflow owners.
    pub(crate) fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// Total registered asset count.
    #[must_use]
    pub fn asset_count(&self) -> u64 {
        self.catalog.stats().map_or(0, |stats| stats.asset_count)
    }

    /// The catalog file path.
    #[must_use]
    pub fn catalog_path(&self) -> &Path {
        &self.catalog_path
    }

    /// The cache root path.
    #[must_use]
    pub fn cache_root(&self) -> &Path {
        &self.cache_root
    }
}

pub(crate) fn aligned_segments(
    transcript: &echo_core::TranscriptPayload,
    alignment: Option<&echo_core::AlignmentPayload>,
) -> Vec<echo_core::TranscriptSegment> {
    let Some(alignment) = alignment.filter(|alignment| {
        normalized_text(&alignment.text) == normalized_text(&transcript.text)
            && !alignment.items.is_empty()
    }) else {
        return transcript.segments.clone();
    };
    let mut cursor = 0;
    transcript
        .segments
        .iter()
        .map(|segment| {
            let target = normalized_text(&segment.text);
            let start_cursor = cursor;
            let mut collected = String::new();
            while cursor < alignment.items.len()
                && collected.chars().count() < target.chars().count()
            {
                collected.push_str(&normalized_text(&alignment.items[cursor].text));
                cursor += 1;
            }
            if !target.is_empty() && collected == target && cursor > start_cursor {
                let mut refined = segment.clone();
                refined.start = alignment.items[start_cursor].start;
                refined.end = alignment.items[cursor - 1].end;
                refined
            } else {
                cursor = start_cursor;
                segment.clone()
            }
        })
        .collect()
}

fn normalized_text(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}
