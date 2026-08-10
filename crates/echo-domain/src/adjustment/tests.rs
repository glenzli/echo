use super::*;

#[test]
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
        .with_equalizer(ParametricEqualizer::from_legacy_gains(250, -175, 400))
        .with_effect_chain(
            EffectChain::new([
                EffectNodeKind::Equalizer,
                EffectNodeKind::Restoration,
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
    assert_eq!(
        graph.equalizer(),
        ParametricEqualizer::from_legacy_gains(250, -175, 400)
    );
    assert!(graph.compressor().enabled);
    assert_eq!(graph.compressor().ratio_tenths, 40);
    assert_eq!(graph.reverb().decay_millis, 2_400);
    assert_eq!(graph.reverb().mix_percent, 24);
    assert_eq!(graph.limiter().ceiling_centibels, -125);
    assert_eq!(graph.effect_chain().nodes()[0], EffectNodeKind::Equalizer);
}

#[test]
fn effect_chain_has_closed_identity_and_fixed_master_tail() {
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
    assert_eq!(
        EffectNodeKind::from_wire_value(3),
        Ok(EffectNodeKind::Space)
    );
    assert_eq!(
        EffectNodeKind::from_wire_value(9),
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
