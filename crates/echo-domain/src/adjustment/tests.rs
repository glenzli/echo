use super::*;
use crate::{EditSegment, EditSegmentState};

#[test]
#[allow(clippy::too_many_lines)] // One complete authored graph contract fixture.
fn graph_preserves_authored_millisecond_and_centibel_units() {
    let graph = AdjustmentGraph::new(
        10_000,
        1_000,
        9_000,
        250,
        500,
        AdjustmentEffects::new(
            FadeCurves::new(FadeCurve::Smooth, FadeCurve::EqualPower),
            -350,
            80,
        )
        .with_restoration(RestorationSettings {
            enabled: true,
            de_plosive: DePlosiveSettings {
                enabled: true,
                frequency_hertz: 150,
                sensitivity_percent: 65,
                reduction_centibels: 1_300,
                release_millis: 180,
            },
            noise_reduction: NoiseReductionSettings {
                enabled: true,
                reduction_centibels: 1_200,
                sensitivity_percent: 62,
                smoothing_millis: 320,
            },
            de_esser: DeEsserSettings {
                enabled: true,
                frequency_hertz: 7_200,
                threshold_centibels: -2_800,
                reduction_centibels: 750,
            },
        })
        .with_de_hum(DeHumSettings {
            enabled: true,
            fundamental_hertz: 60,
            harmonic_count: 6,
            quality_tenths: 420,
            depth_centibels: 1_800,
        })
        .with_de_click(DeClickSettings {
            enabled: true,
            sensitivity_percent: 64,
            maximum_click_microseconds: 750,
            repair_percent: 85,
        })
        .with_channel_repair(ChannelRepairSettings {
            enabled: true,
            invert_left: true,
            invert_right: false,
            swap_channels: true,
            mono_fold_down: false,
            balance_percent: 18,
        })
        .with_equalizer(ParametricEqualizer::from_legacy_gains(250, -175, 400))
        .with_effect_chain(
            EffectChain::new([
                EffectNodeKind::DeHum,
                EffectNodeKind::Equalizer,
                EffectNodeKind::Restoration,
                EffectNodeKind::DeClick,
                EffectNodeKind::Space,
                EffectNodeKind::Dynamics,
                EffectNodeKind::Master,
            ])
            .expect("valid reordered chain"),
        )
        .with_compressor(CompressorSettings {
            enabled: true,
            threshold_centibels: -2_000,
            ratio_tenths: 40,
            attack_millis: 12,
            release_millis: 180,
            makeup_centibels: 250,
        })
        .with_reverb(ReverbSettings {
            character: ReverbCharacter::Hall,
            enabled: true,
            mix_percent: 24,
            pre_delay_millis: 28,
            decay_millis: 2_400,
            size_percent: 68,
            damping_percent: 52,
            low_cut_hertz: 150,
            high_cut_hertz: 9_000,
        })
        .with_limiter(LimiterSettings {
            enabled: true,
            ceiling_centibels: -125,
            release_millis: 160,
        }),
    )
    .expect("valid graph builds");
    assert_eq!(graph.trim_start_millis(), 1_000);
    assert_eq!(graph.trim_end_millis(), 9_000);
    assert_eq!(graph.fade_in_millis(), 250);
    assert_eq!(graph.fade_out_millis(), 500);
    assert_eq!(graph.fade_in_curve(), FadeCurve::Smooth);
    assert_eq!(graph.fade_out_curve(), FadeCurve::EqualPower);
    assert_eq!(graph.gain_centibels(), -350);
    assert_eq!(graph.low_cut_hertz(), 80);
    assert_eq!(
        graph.restoration().noise_reduction.reduction_centibels,
        1_200
    );
    assert_eq!(graph.restoration().de_esser.frequency_hertz, 7_200);
    assert_eq!(graph.de_hum().fundamental_hertz, 60);
    assert_eq!(graph.de_hum().harmonic_count, 6);
    assert_eq!(graph.de_click().maximum_click_microseconds, 750);
    assert_eq!(graph.de_click().repair_percent, 85);
    assert!(graph.channel_repair().invert_left);
    assert!(graph.channel_repair().swap_channels);
    assert_eq!(graph.channel_repair().balance_percent, 18);
    assert_eq!(
        graph.equalizer(),
        ParametricEqualizer::from_legacy_gains(250, -175, 400)
    );
    assert!(graph.compressor().enabled);
    assert_eq!(graph.compressor().ratio_tenths, 40);
    assert_eq!(graph.reverb().decay_millis, 2_400);
    assert_eq!(graph.reverb().mix_percent, 24);
    assert_eq!(graph.reverb().character, ReverbCharacter::Hall);
    assert_eq!(graph.limiter().ceiling_centibels, -125);
    assert_eq!(graph.effect_chain().nodes()[0], EffectNodeKind::DeHum);
}

#[test]
fn effect_chain_has_bounded_singleton_identity_and_fixed_master_tail() {
    let reordered = EffectChain::new([
        EffectNodeKind::Space,
        EffectNodeKind::Equalizer,
        EffectNodeKind::Restoration,
        EffectNodeKind::Dynamics,
        EffectNodeKind::Master,
    ])
    .expect("valid reorder");
    assert!(reordered.is_valid());
    assert_eq!(EffectNodeKind::Space.wire_value(), 3);
    assert_eq!(EffectNodeKind::DeHum.wire_value(), 5);
    assert_eq!(EffectNodeKind::DeClick.wire_value(), 6);
    assert_eq!(EffectNodeKind::ChannelRepair.wire_value(), 7);
    assert_eq!(EffectNodeKind::SceneVfx.wire_value(), 8);
    assert_eq!(EffectNodeKind::DelayVfx.wire_value(), 9);
    assert_eq!(EffectNodeKind::ModulationVfx.wire_value(), 10);
    assert_eq!(EffectNodeKind::TransformVfx.wire_value(), 11);
    assert_eq!(EffectNodeKind::DigitalDegradeVfx.wire_value(), 12);
    assert_eq!(EFFECT_NODE_COUNT, 13);
    assert_eq!(
        EffectNodeKind::from_wire_value(3),
        Ok(EffectNodeKind::Space)
    );
    assert_eq!(
        EffectNodeKind::from_wire_value(13),
        Err(EffectNodeKindValueError)
    );

    assert_eq!(
        EffectChain::new([
            EffectNodeKind::Restoration,
            EffectNodeKind::Equalizer,
            EffectNodeKind::Dynamics,
            EffectNodeKind::Space,
            EffectNodeKind::Restoration,
        ]),
        Err(EffectChainError)
    );
    assert_eq!(
        EffectChain::new([
            EffectNodeKind::Master,
            EffectNodeKind::Equalizer,
            EffectNodeKind::Dynamics,
            EffectNodeKind::Space,
            EffectNodeKind::Restoration,
        ]),
        Err(EffectChainError)
    );

    let reduced = EffectChain::new([
        EffectNodeKind::Restoration,
        EffectNodeKind::Dynamics,
        EffectNodeKind::Master,
    ])
    .expect("optional insert nodes may be omitted");
    assert_eq!(
        reduced.nodes(),
        [
            EffectNodeKind::Restoration,
            EffectNodeKind::Dynamics,
            EffectNodeKind::Master,
        ]
    );
    let encoded = serde_json::to_string(&reduced).expect("reduced chain encodes");
    let encoded_value: serde_json::Value =
        serde_json::from_str(&encoded).expect("encoded chain is JSON");
    assert_eq!(encoded_value["nodes"].as_array().map(Vec::len), Some(13));
    assert_eq!(encoded_value["active_count"], 3);
    let decoded: EffectChain = serde_json::from_str(&encoded).expect("reduced chain decodes");
    assert_eq!(decoded, reduced);

    let legacy: EffectChain = serde_json::from_str(
        r#"{"nodes":["restoration","equalizer","dynamics","space","master"]}"#,
    )
    .expect("legacy full chain decodes");
    assert_eq!(legacy, EffectChain::standard());

    let legacy_reduced: EffectChain = serde_json::from_str(
        r#"{"nodes":["restoration","dynamics","master","equalizer","space"],"active_count":3}"#,
    )
    .expect("legacy reduced chain decodes");
    assert_eq!(
        legacy_reduced.nodes(),
        [
            EffectNodeKind::Restoration,
            EffectNodeKind::Dynamics,
            EffectNodeKind::Master,
        ]
    );
    let normalized = serde_json::to_value(legacy_reduced).expect("legacy chain normalizes");
    assert_eq!(normalized["nodes"].as_array().map(Vec::len), Some(13));
    assert_eq!(normalized["active_count"], 3);

    assert_eq!(
        EffectChain::standard().nodes(),
        [
            EffectNodeKind::Restoration,
            EffectNodeKind::Equalizer,
            EffectNodeKind::Dynamics,
            EffectNodeKind::Space,
            EffectNodeKind::Master,
        ]
    );
}

#[test]
fn twelve_node_chain_json_migrates_with_disabled_digital_degrade_tail() {
    let mut stored = serde_json::to_value(EffectChain::standard()).expect("chain encodes");
    let nodes = stored
        .get_mut("nodes")
        .and_then(serde_json::Value::as_array_mut)
        .expect("nodes array");
    assert_eq!(nodes.pop(), Some(serde_json::json!("digital_degrade_vfx")));
    let restored: EffectChain = serde_json::from_value(stored).expect("legacy chain decodes");
    assert!(restored.is_valid());
    assert_eq!(restored.nodes(), EffectChain::standard().nodes());
}

#[test]
fn legacy_graphs_default_creative_vfx_to_disabled_identity() {
    let graph = AdjustmentGraph::identity(4_000).expect("identity graph validates");
    let mut legacy = serde_json::to_value(&graph).expect("graph encodes");
    legacy
        .as_object_mut()
        .expect("graph is object")
        .remove("creative_vfx");
    let restored: AdjustmentGraph = serde_json::from_value(legacy).expect("legacy graph decodes");
    assert_eq!(restored.creative_vfx(), CreativeVfxSettings::default());
}

#[test]
fn reverb_character_has_stable_wire_values() {
    assert_eq!(ReverbCharacter::Room.wire_value(), 0);
    assert_eq!(ReverbCharacter::Hall.wire_value(), 1);
    assert_eq!(ReverbCharacter::Plate.wire_value(), 2);
    assert_eq!(ReverbCharacter::Spring.wire_value(), 3);
    assert_eq!(
        ReverbCharacter::from_wire_value(1),
        Ok(ReverbCharacter::Hall)
    );
    assert_eq!(
        ReverbCharacter::from_wire_value(4),
        Err(ReverbCharacterValueError)
    );
}

#[test]
fn legacy_effect_json_defaults_whole_node_bypass_to_enabled() {
    let restoration: RestorationSettings = serde_json::from_str(
        r#"{"noise_reduction":{"enabled":false,"reduction_centibels":900,"sensitivity_percent":50,"smoothing_millis":240},"de_esser":{"enabled":false,"frequency_hertz":6500,"threshold_centibels":-2400,"reduction_centibels":600}}"#,
    )
    .expect("legacy restoration decodes");
    assert!(restoration.enabled);

    let bands = serde_json::to_string(&ParametricEqualizer::flat().bands()).expect("bands encode");
    let equalizer: ParametricEqualizer =
        serde_json::from_str(&format!(r#"{{"bands":{bands}}}"#)).expect("legacy equalizer decodes");
    assert!(equalizer.enabled());
}

#[test]
fn fade_curve_catalog_values_are_stable_and_closed() {
    for curve in [FadeCurve::Linear, FadeCurve::Smooth, FadeCurve::EqualPower] {
        assert_eq!(
            FadeCurve::from_catalog_value(curve.catalog_value()),
            Ok(curve)
        );
    }
    assert_eq!(FadeCurve::from_catalog_value(3), Err(FadeCurveValueError));
}

#[test]
fn graph_rejects_out_of_source_and_overlapping_envelopes() {
    assert_eq!(
        AdjustmentGraph::new(1_000, 900, 1_100, 0, 0, AdjustmentEffects::default()),
        Err(AdjustmentGraphError::InvalidTrimRange)
    );
    assert_eq!(
        AdjustmentGraph::new(1_000, 100, 900, 500, 400, AdjustmentEffects::default()),
        Err(AdjustmentGraphError::OverlappingFades)
    );
    assert_eq!(
        AdjustmentGraph::new(
            1_000,
            0,
            1_000,
            0,
            0,
            AdjustmentEffects::new(FadeCurves::linear(), MAX_GAIN_CENTIBELS + 1, 0),
        ),
        Err(AdjustmentGraphError::GainOutOfRange)
    );
    assert_eq!(
        AdjustmentGraph::new(
            1_000,
            0,
            1_000,
            0,
            0,
            AdjustmentEffects::new(FadeCurves::linear(), 0, MIN_LOW_CUT_HERTZ - 1),
        ),
        Err(AdjustmentGraphError::LowCutOutOfRange)
    );
    assert_eq!(
        AdjustmentGraph::new(
            1_000,
            0,
            1_000,
            0,
            0,
            AdjustmentEffects::new(FadeCurves::linear(), 0, 0).with_equalizer(
                ParametricEqualizer::from_legacy_gains(0, MAX_EQ_GAIN_CENTIBELS + 1, 0,)
            ),
        ),
        Err(AdjustmentGraphError::EqualizerBandOutOfRange)
    );
    assert_eq!(
        AdjustmentGraph::new(
            1_000,
            0,
            1_000,
            0,
            0,
            AdjustmentEffects::default().with_compressor(CompressorSettings {
                ratio_tenths: MAX_COMPRESSOR_RATIO_TENTHS + 1,
                ..CompressorSettings::default()
            }),
        ),
        Err(AdjustmentGraphError::CompressorOutOfRange)
    );
    assert_eq!(
        AdjustmentGraph::new(
            1_000,
            0,
            1_000,
            0,
            0,
            AdjustmentEffects::default().with_reverb(ReverbSettings {
                decay_millis: MIN_REVERB_DECAY_MILLIS - 1,
                ..ReverbSettings::default()
            }),
        ),
        Err(AdjustmentGraphError::ReverbOutOfRange)
    );
    assert_eq!(
        AdjustmentGraph::new(
            1_000,
            0,
            1_000,
            0,
            0,
            AdjustmentEffects::default().with_limiter(LimiterSettings {
                ceiling_centibels: MIN_LIMITER_CEILING_CENTIBELS - 1,
                ..LimiterSettings::default()
            }),
        ),
        Err(AdjustmentGraphError::LimiterOutOfRange)
    );
}

#[test]
fn graph_rejects_restoration_parameters_outside_the_authored_contract() {
    let invalid_de_plosive = AdjustmentEffects::default().with_restoration(RestorationSettings {
        de_plosive: DePlosiveSettings {
            frequency_hertz: MIN_DE_PLOSIVE_FREQUENCY_HERTZ - 1,
            ..DePlosiveSettings::default()
        },
        ..RestorationSettings::default()
    });
    assert_eq!(
        AdjustmentGraph::new(1_000, 0, 1_000, 0, 0, invalid_de_plosive),
        Err(AdjustmentGraphError::DePlosiveOutOfRange)
    );

    let invalid_noise = AdjustmentEffects::default().with_restoration(RestorationSettings {
        noise_reduction: NoiseReductionSettings {
            smoothing_millis: MIN_NOISE_REDUCTION_SMOOTHING_MILLIS - 1,
            ..NoiseReductionSettings::default()
        },
        ..RestorationSettings::default()
    });
    assert_eq!(
        AdjustmentGraph::new(1_000, 0, 1_000, 0, 0, invalid_noise),
        Err(AdjustmentGraphError::NoiseReductionOutOfRange)
    );

    let invalid_de_esser = AdjustmentEffects::default().with_restoration(RestorationSettings {
        de_esser: DeEsserSettings {
            frequency_hertz: MIN_DE_ESSER_FREQUENCY_HERTZ - 1,
            ..DeEsserSettings::default()
        },
        ..RestorationSettings::default()
    });
    assert_eq!(
        AdjustmentGraph::new(1_000, 0, 1_000, 0, 0, invalid_de_esser),
        Err(AdjustmentGraphError::DeEsserOutOfRange)
    );
}

#[test]
fn graph_rejects_de_hum_and_de_click_parameters_outside_the_authored_contract() {
    for de_hum in [
        DeHumSettings {
            fundamental_hertz: 55,
            ..DeHumSettings::default()
        },
        DeHumSettings {
            harmonic_count: 0,
            ..DeHumSettings::default()
        },
        DeHumSettings {
            quality_tenths: MAX_DE_HUM_QUALITY_TENTHS + 1,
            ..DeHumSettings::default()
        },
        DeHumSettings {
            depth_centibels: MAX_DE_HUM_DEPTH_CENTIBELS + 1,
            ..DeHumSettings::default()
        },
    ] {
        assert_eq!(
            AdjustmentGraph::new(
                1_000,
                0,
                1_000,
                0,
                0,
                AdjustmentEffects::default().with_de_hum(de_hum),
            ),
            Err(AdjustmentGraphError::DeHumOutOfRange)
        );
    }

    for de_click in [
        DeClickSettings {
            sensitivity_percent: MAX_DE_CLICK_SENSITIVITY_PERCENT + 1,
            ..DeClickSettings::default()
        },
        DeClickSettings {
            maximum_click_microseconds: MIN_DE_CLICK_DURATION_MICROSECONDS - 1,
            ..DeClickSettings::default()
        },
        DeClickSettings {
            repair_percent: MAX_DE_CLICK_REPAIR_PERCENT + 1,
            ..DeClickSettings::default()
        },
    ] {
        assert_eq!(
            AdjustmentGraph::new(
                1_000,
                0,
                1_000,
                0,
                0,
                AdjustmentEffects::default().with_de_click(de_click),
            ),
            Err(AdjustmentGraphError::DeClickOutOfRange)
        );
    }
}

#[test]
fn channel_repair_defaults_to_identity_and_bounds_balance() {
    let graph = AdjustmentGraph::identity(1_000).expect("identity graph validates");
    assert_eq!(graph.channel_repair(), ChannelRepairSettings::identity());
    assert!(
        !graph
            .effect_chain()
            .nodes()
            .contains(&EffectNodeKind::ChannelRepair)
    );

    let encoded = serde_json::to_string(&graph).expect("graph encodes");
    let decoded: AdjustmentGraph = serde_json::from_str(&encoded).expect("graph decodes");
    assert_eq!(decoded.channel_repair(), ChannelRepairSettings::identity());

    for balance_percent in [-101, 101] {
        assert_eq!(
            AdjustmentGraph::new(
                1_000,
                0,
                1_000,
                0,
                0,
                AdjustmentEffects::default().with_channel_repair(ChannelRepairSettings {
                    balance_percent,
                    ..ChannelRepairSettings::identity()
                }),
            ),
            Err(AdjustmentGraphError::ChannelRepairOutOfRange)
        );
    }
}

#[test]
fn graph_bounds_effect_masks_to_trim_and_active_supported_inserts() {
    let timeline = EditTimeline::new(
        1_000,
        9_000,
        vec![
            EditSegment::new(
                1_000,
                4_000,
                EditSegmentState::Audible,
                0,
                0,
                0,
                FadeCurves::linear(),
                0,
            )
            .expect("first segment validates"),
            EditSegment::new(
                4_000,
                9_000,
                EditSegmentState::Hidden,
                0,
                0,
                0,
                FadeCurves::linear(),
                250,
            )
            .expect("second segment validates"),
        ],
    )
    .expect("timeline validates");
    let mask = EffectMask::new(
        2_000,
        6_000,
        10,
        vec![EffectNodeKind::Equalizer, EffectNodeKind::Dynamics],
    )
    .expect("mask validates");
    let graph = AdjustmentGraph::new(
        10_000,
        1_000,
        9_000,
        0,
        0,
        AdjustmentEffects::default()
            .with_edit_timeline(timeline.clone())
            .with_effect_masks(vec![mask.clone()]),
    )
    .expect("mask targets active inserts");
    assert_eq!(graph.edit_timeline(), &timeline);
    assert_eq!(graph.effect_masks(), std::slice::from_ref(&mask));

    let outside_trim =
        EffectMask::new(500, 2_000, 10, vec![EffectNodeKind::Equalizer]).expect("valid mask");
    assert_eq!(
        AdjustmentGraph::new(
            10_000,
            1_000,
            9_000,
            0,
            0,
            AdjustmentEffects::default()
                .with_edit_timeline(timeline.clone())
                .with_effect_masks(vec![outside_trim]),
        ),
        Err(AdjustmentGraphError::InvalidEffectMasks)
    );
    assert_eq!(
        AdjustmentGraph::new(
            10_000,
            1_000,
            9_000,
            0,
            0,
            AdjustmentEffects::default()
                .with_edit_timeline(timeline.clone())
                .with_effect_masks(vec![mask.clone(); MAX_EFFECT_MASKS + 1]),
        ),
        Err(AdjustmentGraphError::InvalidEffectMasks)
    );

    let inactive = EffectMask::new(2_000, 3_000, 10, vec![EffectNodeKind::Space])
        .expect("mask is structurally valid");
    assert_eq!(
        AdjustmentGraph::new(
            10_000,
            1_000,
            9_000,
            0,
            0,
            AdjustmentEffects::default()
                .with_effect_chain(
                    EffectChain::new([EffectNodeKind::Equalizer, EffectNodeKind::Master])
                        .expect("chain validates"),
                )
                .with_edit_timeline(timeline)
                .with_effect_masks(vec![inactive]),
        ),
        Err(AdjustmentGraphError::InvalidEffectMasks)
    );
}

#[test]
fn graph_accepts_digital_degrade_effect_mask() {
    let mask = EffectMask::new(100, 900, 10, vec![EffectNodeKind::DigitalDegradeVfx])
        .expect("zero-latency Digital Degrade mask validates");
    let graph = AdjustmentGraph::new(
        1_000,
        0,
        1_000,
        0,
        0,
        AdjustmentEffects::default()
            .with_effect_chain(
                EffectChain::new([EffectNodeKind::DigitalDegradeVfx, EffectNodeKind::Master])
                    .expect("digital mask chain validates"),
            )
            .with_effect_masks(vec![mask.clone()]),
    )
    .expect("Digital Degrade is a maskable zero-latency insert");
    assert_eq!(graph.effect_masks(), &[mask]);
}

#[test]
fn legacy_graph_json_restores_one_audible_identity_segment() {
    let graph = AdjustmentGraph::identity(4_000).expect("identity graph validates");
    let mut legacy = serde_json::to_value(&graph).expect("graph encodes");
    legacy
        .as_object_mut()
        .expect("graph is object")
        .remove("edit_timeline");
    legacy
        .as_object_mut()
        .expect("graph is object")
        .remove("effect_masks");
    let restored: AdjustmentGraph = serde_json::from_value(legacy).expect("legacy graph decodes");
    assert_eq!(restored.effect_masks(), &[]);
    assert_eq!(restored.edit_timeline().segments().len(), 1);
    assert_eq!(
        restored.edit_timeline().segments()[0].state(),
        EditSegmentState::Audible
    );
}
