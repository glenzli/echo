use super::*;
use crate::{
    DePlosiveSettings, EditSegment, EditSegmentState, EditTimeline, EffectMask, FadeCurve,
    NoiseReductionSettings, ParametricEqualizerBand, ReverbCharacter,
};
use uuid::Uuid;

fn target_edit_timeline() -> EditTimeline {
    EditTimeline::new(
        700,
        10_500,
        vec![
            EditSegment::new(
                700,
                4_000,
                EditSegmentState::Audible,
                0,
                0,
                0,
                FadeCurves::linear(),
                0,
            )
            .expect("first target segment validates"),
            EditSegment::new(
                4_000,
                5_000,
                EditSegmentState::Hidden,
                0,
                0,
                0,
                FadeCurves::linear(),
                120,
            )
            .expect("hidden target segment validates"),
            EditSegment::new(
                5_000,
                10_500,
                EditSegmentState::Audible,
                -100,
                50,
                80,
                FadeCurves::new(FadeCurve::Smooth, FadeCurve::EqualPower),
                0,
            )
            .expect("last target segment validates"),
        ],
    )
    .expect("target timeline validates")
}

fn target_effect_masks() -> Vec<EffectMask> {
    vec![
        EffectMask::new(2_000, 3_500, 10, vec![EffectNodeKind::Equalizer])
            .expect("target mask validates"),
    ]
}

fn target_channel_repair() -> ChannelRepairSettings {
    ChannelRepairSettings {
        enabled: true,
        invert_left: false,
        invert_right: true,
        swap_channels: true,
        mono_fold_down: false,
        balance_percent: -22,
    }
}

fn source_creative_vfx() -> CreativeVfxSettings {
    let mut creative_vfx = CreativeVfxSettings::default();
    creative_vfx.scene.enabled = true;
    creative_vfx.scene.character = crate::SceneVfxCharacter::Underwater;
    creative_vfx.scene.intensity_percent = 72;
    creative_vfx.delay.enabled = true;
    creative_vfx.delay.character = crate::DelayVfxCharacter::Echo;
    creative_vfx.delay.echo.feedback_percent = 42;
    creative_vfx.modulation.enabled = true;
    creative_vfx.modulation.character = crate::ModulationVfxCharacter::Phaser;
    creative_vfx.modulation.phaser.feedback_percent = -18;
    creative_vfx.transform.enabled = true;
    creative_vfx.transform.character = crate::TransformVfxCharacter::Ghost;
    creative_vfx.transform.amount_percent = 68;
    creative_vfx.digital_degrade.enabled = true;
    creative_vfx.digital_degrade.character = crate::DigitalDegradeVfxCharacter::LoFi;
    creative_vfx.digital_degrade.bitcrusher.bit_depth = 7;
    creative_vfx.drive.enabled = true;
    creative_vfx.drive.character = crate::DriveVfxCharacter::Overdrive;
    creative_vfx.drive.drive_centibels = 2_100;
    creative_vfx.rotary.enabled = true;
    creative_vfx.rotary.speed = crate::RotaryVfxSpeed::Fast;
    creative_vfx.rotary.motion_percent = 83;
    creative_vfx
}

fn source_processing_effects(gain_centibels: i16, low_cut_hertz: u16) -> AdjustmentEffects {
    AdjustmentEffects::new(
        FadeCurves::new(FadeCurve::Smooth, FadeCurve::EqualPower),
        gain_centibels,
        low_cut_hertz,
    )
    .with_restoration(RestorationSettings {
        enabled: true,
        de_plosive: DePlosiveSettings {
            enabled: true,
            frequency_hertz: 155,
            sensitivity_percent: 66,
            reduction_centibels: 1_350,
            release_millis: 190,
        },
        noise_reduction: NoiseReductionSettings {
            enabled: true,
            reduction_centibels: 1_100,
            sensitivity_percent: 64,
            smoothing_millis: 300,
        },
        de_esser: crate::DeEsserSettings {
            enabled: true,
            frequency_hertz: 7_400,
            threshold_centibels: -2_600,
            reduction_centibels: 700,
        },
    })
    .with_de_hum(DeHumSettings {
        enabled: true,
        fundamental_hertz: 60,
        harmonic_count: 5,
        quality_tenths: 360,
        depth_centibels: 1_800,
    })
    .with_de_click(DeClickSettings {
        enabled: true,
        sensitivity_percent: 68,
        maximum_click_microseconds: 700,
        repair_percent: 84,
    })
    .with_channel_repair(target_channel_repair())
    .with_equalizer(ParametricEqualizer::new([
        ParametricEqualizerBand::new(true, crate::EqualizerFilterKind::LowShelf, 90, 80, 250),
        ParametricEqualizerBand::new(false, crate::EqualizerFilterKind::Bell, 250, 100, 0),
        ParametricEqualizerBand::new(true, crate::EqualizerFilterKind::Bell, 900, 120, -175),
        ParametricEqualizerBand::new(false, crate::EqualizerFilterKind::Bell, 3_000, 100, 0),
        ParametricEqualizerBand::new(false, crate::EqualizerFilterKind::Bell, 5_000, 100, 0),
        ParametricEqualizerBand::new(true, crate::EqualizerFilterKind::HighShelf, 9_000, 75, 300),
    ]))
    .with_compressor(CompressorSettings {
        enabled: true,
        threshold_centibels: -2_100,
        ratio_tenths: 40,
        attack_millis: 15,
        release_millis: 180,
        makeup_centibels: 200,
    })
    .with_reverb(ReverbSettings {
        character: ReverbCharacter::Plate,
        enabled: true,
        mix_percent: 21,
        pre_delay_millis: 24,
        decay_millis: 2_200,
        size_percent: 62,
        damping_percent: 48,
        low_cut_hertz: 140,
        high_cut_hertz: 9_400,
    })
    .with_creative_vfx(source_creative_vfx())
    .with_limiter(LimiterSettings {
        enabled: true,
        ceiling_centibels: -125,
        release_millis: 150,
    })
    .with_effect_chain(source_effect_chain())
}

fn source_effect_chain() -> EffectChain {
    EffectChain::new([
        EffectNodeKind::DeHum,
        EffectNodeKind::Restoration,
        EffectNodeKind::Equalizer,
        EffectNodeKind::DeClick,
        EffectNodeKind::Dynamics,
        EffectNodeKind::Space,
        EffectNodeKind::ChannelRepair,
        EffectNodeKind::SceneVfx,
        EffectNodeKind::DelayVfx,
        EffectNodeKind::ModulationVfx,
        EffectNodeKind::TransformVfx,
        EffectNodeKind::DigitalDegradeVfx,
        EffectNodeKind::DriveVfx,
        EffectNodeKind::RotaryVfx,
        EffectNodeKind::Master,
    ])
    .expect("source chain is valid")
}

fn graph_with_processing(
    trim_start_millis: u64,
    trim_end_millis: u64,
    fade_in_millis: u64,
    fade_out_millis: u64,
    gain_centibels: i16,
    low_cut_hertz: u16,
) -> AdjustmentGraph {
    AdjustmentGraph::new(
        20_000,
        trim_start_millis,
        trim_end_millis,
        fade_in_millis,
        fade_out_millis,
        source_processing_effects(gain_centibels, low_cut_hertz),
    )
    .expect("fixture graph is valid")
}

#[test]
fn default_components_are_complete_and_clip_local_controls_are_absent() {
    assert_eq!(DEFAULT_PROCESSING_COMPONENTS.len(), 16);
    for component in DEFAULT_PROCESSING_COMPONENTS.iter().copied() {
        assert_eq!(
            ProcessingComponent::from_wire_value(component.wire_value()),
            Ok(component)
        );
    }
    assert_eq!(
        ProcessingComponent::from_wire_value(16),
        Err(ProcessingComponentValueError)
    );
}

#[test]
fn merge_changes_only_selected_processing_and_preserves_target_clip_state() {
    let source = graph_with_processing(1_000, 18_000, 300, 450, 350, 90);
    let target = AdjustmentGraph::new(
        12_000,
        700,
        10_500,
        125,
        275,
        AdjustmentEffects::new(FadeCurves::linear(), -425, 40)
            .with_restoration(RestorationSettings::standard())
            .with_equalizer(ParametricEqualizer::flat())
            .with_effect_chain(
                EffectChain::new([
                    EffectNodeKind::Dynamics,
                    EffectNodeKind::Equalizer,
                    EffectNodeKind::Space,
                    EffectNodeKind::Master,
                ])
                .expect("target chain is valid"),
            )
            .with_edit_timeline(target_edit_timeline())
            .with_effect_masks(target_effect_masks()),
    )
    .expect("target graph is valid");
    let patch = AdjustmentPatch::from_graph(
        source.clone(),
        &[ProcessingComponent::LowCut, ProcessingComponent::Equalizer],
    )
    .expect("selected patch is valid");

    let merged = patch
        .apply_to(target.clone(), ProcessingMergeMode::Merge)
        .expect("merge is valid");

    assert_eq!(merged.trim_start_millis(), target.trim_start_millis());
    assert_eq!(merged.trim_end_millis(), target.trim_end_millis());
    assert_eq!(merged.fade_in_millis(), target.fade_in_millis());
    assert_eq!(merged.fade_out_millis(), target.fade_out_millis());
    assert_eq!(merged.fade_in_curve(), target.fade_in_curve());
    assert_eq!(merged.fade_out_curve(), target.fade_out_curve());
    assert_eq!(merged.gain_centibels(), target.gain_centibels());
    assert_eq!(merged.edit_timeline(), target.edit_timeline());
    assert_eq!(merged.effect_masks(), target.effect_masks());
    assert_eq!(merged.low_cut_hertz(), source.low_cut_hertz());
    assert_eq!(merged.equalizer(), source.equalizer());
    assert_eq!(merged.restoration(), target.restoration());
    assert_eq!(merged.compressor(), target.compressor());
    assert_eq!(
        merged.effect_chain().nodes(),
        [
            EffectNodeKind::Dynamics,
            EffectNodeKind::Equalizer,
            EffectNodeKind::Space,
            EffectNodeKind::Master,
        ]
    );
}

#[test]
fn replace_copies_processing_but_preserves_target_trim_fades_and_gain() {
    let source = graph_with_processing(1_000, 18_000, 300, 450, 350, 90);
    let target = AdjustmentGraph::new(
        12_000,
        700,
        10_500,
        125,
        275,
        AdjustmentEffects::new(
            FadeCurves::new(FadeCurve::Linear, FadeCurve::Smooth),
            -425,
            40,
        )
        .with_edit_timeline(target_edit_timeline())
        .with_effect_masks(target_effect_masks()),
    )
    .expect("target graph is valid");
    let patch = AdjustmentPatch::from_graph(source.clone(), &DEFAULT_PROCESSING_COMPONENTS)
        .expect("complete patch is valid");

    let replaced = patch
        .apply_to(target.clone(), ProcessingMergeMode::Replace)
        .expect("replacement is valid");

    assert_eq!(replaced.trim_start_millis(), target.trim_start_millis());
    assert_eq!(replaced.trim_end_millis(), target.trim_end_millis());
    assert_eq!(replaced.fade_in_millis(), target.fade_in_millis());
    assert_eq!(replaced.fade_out_millis(), target.fade_out_millis());
    assert_eq!(replaced.fade_in_curve(), target.fade_in_curve());
    assert_eq!(replaced.fade_out_curve(), target.fade_out_curve());
    assert_eq!(replaced.gain_centibels(), target.gain_centibels());
    assert_eq!(replaced.edit_timeline(), target.edit_timeline());
    assert_eq!(replaced.effect_masks(), target.effect_masks());
    assert_eq!(replaced.low_cut_hertz(), source.low_cut_hertz());
    assert_eq!(replaced.restoration(), source.restoration());
    assert_eq!(replaced.de_hum(), source.de_hum());
    assert_eq!(replaced.de_click(), source.de_click());
    assert_eq!(replaced.equalizer(), source.equalizer());
    assert_eq!(replaced.compressor(), source.compressor());
    assert_eq!(replaced.reverb(), source.reverb());
    assert_eq!(replaced.creative_vfx(), source.creative_vfx());
    assert_eq!(replaced.limiter(), source.limiter());
    assert_eq!(replaced.effect_chain(), source.effect_chain());
}

#[test]
fn applying_the_same_patch_twice_is_idempotent() {
    let source = graph_with_processing(1_000, 18_000, 300, 450, 350, 90);
    let target = AdjustmentGraph::identity(8_000).expect("target is valid");
    let patch = AdjustmentPatch::from_graph(source, &DEFAULT_PROCESSING_COMPONENTS)
        .expect("patch is valid");
    let once = patch
        .apply_to(target, ProcessingMergeMode::Merge)
        .expect("first apply succeeds");
    let twice = patch
        .apply_to(once.clone(), ProcessingMergeMode::Merge)
        .expect("second apply succeeds");
    assert_eq!(twice, once);
}

#[test]
fn serde_identity_is_stable_and_invalid_input_fails_closed() {
    let source = graph_with_processing(1_000, 18_000, 300, 450, 350, 90);
    let patch = AdjustmentPatch::from_graph(source, &DEFAULT_PROCESSING_COMPONENTS)
        .expect("patch is valid");
    let revision = ProcessingRecipeRevision::new(
        ProcessingRecipeRevisionId::from_uuid(
            Uuid::parse_str("018f23c0-0f00-7000-8000-000000000002").expect("fixed UUID"),
        ),
        ProcessingRecipeId::from_uuid(
            Uuid::parse_str("018f23c0-0f00-7000-8000-000000000001").expect("fixed UUID"),
        ),
        3,
        patch.clone(),
        42,
    )
    .expect("revision is valid");
    let encoded = serde_json::to_value(&revision).expect("revision encodes");
    assert_eq!(encoded["sequence"], 3);
    assert_eq!(encoded["patch"]["components"][0], "low_cut");
    assert_eq!(
        serde_json::to_string(&ProcessingMergeMode::Merge).expect("merge encodes"),
        r#""merge""#
    );
    assert_eq!(
        serde_json::to_string(&ProcessingMergeMode::Replace).expect("replace encodes"),
        r#""replace_processing""#
    );
    let decoded: ProcessingRecipeRevision =
        serde_json::from_value(encoded).expect("revision decodes");
    assert_eq!(decoded, revision);

    let mut invalid = serde_json::to_value(&patch).expect("patch encodes");
    invalid["low_cut_hertz"] = serde_json::json!(241);
    assert!(serde_json::from_value::<AdjustmentPatch>(invalid).is_err());

    let mut duplicate = serde_json::to_value(&patch).expect("patch encodes");
    duplicate["components"] = serde_json::json!(["low_cut", "low_cut"]);
    assert!(serde_json::from_value::<AdjustmentPatch>(duplicate).is_err());
}

#[test]
fn constructors_reject_empty_duplicate_and_invalid_revision_boundaries() {
    let source = graph_with_processing(1_000, 18_000, 300, 450, 350, 90);
    assert_eq!(
        AdjustmentPatch::from_graph(source.clone(), &[]),
        Err(ProcessingRecipeError::EmptyComponents)
    );
    assert_eq!(
        AdjustmentPatch::from_graph(
            source.clone(),
            &[ProcessingComponent::Space, ProcessingComponent::Space],
        ),
        Err(ProcessingRecipeError::DuplicateComponent(
            ProcessingComponent::Space
        ))
    );

    let patch = AdjustmentPatch::from_graph(source, &[ProcessingComponent::Master])
        .expect("patch is valid");
    assert_eq!(
        ProcessingRecipeRevision::new(
            ProcessingRecipeRevisionId::new(),
            ProcessingRecipeId::new(),
            0,
            patch.clone(),
            0,
        ),
        Err(ProcessingRecipeError::InvalidRevisionSequence)
    );
    assert_eq!(
        ProcessingRecipeRevision::new(
            ProcessingRecipeRevisionId::new(),
            ProcessingRecipeId::new(),
            1,
            patch,
            -1,
        ),
        Err(ProcessingRecipeError::InvalidTimestamp)
    );
}
