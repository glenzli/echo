use std::path::Path;

use echo_domain::{
    AdjustmentEffects, AdjustmentGraph, ChannelRepairSettings, CompressorSettings, ContentHash,
    CreativeVfxSettings, DeClickSettings, DeEsserSettings, DeHumSettings, DePlosiveSettings,
    DelayVfxCharacter, EditSegment, EditSegmentState, EditTimeline, EffectChain, EffectMask,
    EffectNodeKind, FadeCurve, FadeCurves, LimiterSettings, ModulationVfxCharacter,
    NoiseReductionSettings, RestorationSettings, ReverbCharacter, ReverbSettings,
    SceneVfxCharacter, TransformVfxCharacter,
};

use super::*;
use crate::{AssetRegistrationInput, RegisterAsset, open_catalog, register_asset};

#[test]
fn revisions_are_append_only_and_identical_saves_are_idempotent() {
    let root = std::env::temp_dir().join(format!("echo-adjustments-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let registered = register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([31; 32]),
                    path: Path::new("/voices/edit.wav"),
                    size_bytes: 100,
                    codec: Some("pcm"),
                    duration_millis: Some(10_000),
                    recorded_at_millis: None,
                    imported_at_millis: 10,
                },
            )?;
            Ok(match registered {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
            })
        })
        .expect("fixture writes");
    assert_eq!(
        catalog
            .with_transaction(|transaction| latest_adjustment_graph(transaction, asset_id))
            .expect("untouched graph reads"),
        None
    );

    let graph = fully_configured_graph();
    let first = catalog
        .with_transaction(|transaction| {
            record_adjustment_graph(transaction, asset_id, graph.clone(), 20)
        })
        .expect("first revision writes");
    let duplicate = catalog
        .with_transaction(|transaction| {
            record_adjustment_graph(transaction, asset_id, graph.clone(), 30)
        })
        .expect("duplicate save reads current");
    assert_eq!(duplicate, first);
    assert_eq!(
        first.graph.equalizer(),
        echo_domain::ParametricEqualizer::from_legacy_gains(300, -150, 225)
    );
    assert_eq!(first.graph.compressor(), graph.compressor());
    assert_eq!(first.graph.de_hum(), graph.de_hum());
    assert_eq!(first.graph.de_click(), graph.de_click());
    assert_eq!(first.graph.channel_repair(), graph.channel_repair());
    assert_eq!(first.graph.reverb(), graph.reverb());
    assert_eq!(first.graph.creative_vfx(), graph.creative_vfx());
    assert_eq!(first.graph.limiter(), graph.limiter());
    assert_eq!(first.graph.effect_chain(), graph.effect_chain());
    assert_eq!(first.graph.edit_timeline(), graph.edit_timeline());
    assert_eq!(first.graph.effect_masks(), graph.effect_masks());

    let second_graph = AdjustmentGraph::new(
        10_000,
        2_000,
        8_000,
        100,
        100,
        AdjustmentEffects::new(
            FadeCurves::new(FadeCurve::Linear, FadeCurve::Smooth),
            0,
            120,
        )
        .with_effect_chain(
            EffectChain::new([EffectNodeKind::Restoration, EffectNodeKind::Master])
                .expect("valid reduced chain"),
        ),
    )
    .expect("second graph validates");
    let second = catalog
        .with_transaction(|transaction| {
            record_adjustment_graph(transaction, asset_id, second_graph.clone(), 40)
        })
        .expect("second revision writes");
    assert!(second.revision_id > first.revision_id);
    assert_eq!(
        catalog
            .with_transaction(|transaction| latest_adjustment_graph(transaction, asset_id))
            .expect("latest reads")
            .expect("revision exists")
            .graph,
        second_graph
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn historical_revisions_are_exact_and_asset_scoped() {
    let root = std::env::temp_dir().join(format!(
        "echo-adjustment-history-scope-{}",
        std::process::id()
    ));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = register_fixture_asset(&catalog, 32, "/voices/history.wav");
    let first = catalog
        .with_transaction(|transaction| {
            record_adjustment_graph(transaction, asset_id, fully_configured_graph(), 20)
        })
        .expect("first revision writes");
    let second = catalog
        .with_transaction(|transaction| {
            record_adjustment_graph(
                transaction,
                asset_id,
                AdjustmentGraph::identity(10_000).expect("identity graph validates"),
                30,
            )
        })
        .expect("second revision writes");

    for expected in [first.clone(), second] {
        assert_eq!(
            catalog
                .with_transaction(|transaction| {
                    adjustment_graph_at_revision(transaction, asset_id, expected.revision_id)
                })
                .expect("historical revision reads"),
            Some(expected)
        );
    }
    assert_eq!(
        catalog
            .with_transaction(|transaction| {
                adjustment_graph_at_revision(transaction, asset_id, i64::MAX)
            })
            .expect("missing historical revision reads"),
        None
    );

    let other_asset_id = register_fixture_asset(&catalog, 34, "/voices/other.wav");
    assert_eq!(
        catalog
            .with_transaction(|transaction| {
                adjustment_graph_at_revision(transaction, other_asset_id, first.revision_id)
            })
            .expect("foreign revision is hidden"),
        None
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn historical_reads_follow_duration_and_persisted_value_failure_policy() {
    let root = std::env::temp_dir().join(format!(
        "echo-adjustment-history-invalid-{}",
        std::process::id()
    ));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let registered = register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([33; 32]),
                    path: Path::new("/voices/history-invalid.wav"),
                    size_bytes: 100,
                    codec: Some("pcm"),
                    duration_millis: Some(10_000),
                    recorded_at_millis: None,
                    imported_at_millis: 10,
                },
            )?;
            Ok(match registered {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
            })
        })
        .expect("fixture writes");
    let revision = catalog
        .with_transaction(|transaction| {
            record_adjustment_graph(transaction, asset_id, fully_configured_graph(), 20)
        })
        .expect("revision writes");

    catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            transaction.execute(
                "UPDATE assets SET duration_millis = NULL WHERE id = ?1",
                [asset_id.to_string()],
            )?;
            Ok(())
        })
        .expect("duration clears");
    assert_eq!(
        catalog
            .with_transaction(|transaction| {
                adjustment_graph_at_revision(transaction, asset_id, revision.revision_id)
            })
            .expect("missing duration follows latest read policy"),
        None
    );

    catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            transaction.execute(
                "UPDATE assets SET duration_millis = -1 WHERE id = ?1",
                [asset_id.to_string()],
            )?;
            Ok(())
        })
        .expect("duration corrupts");
    let duration_error = catalog
        .with_transaction(|transaction| {
            adjustment_graph_at_revision(transaction, asset_id, revision.revision_id)
        })
        .expect_err("negative duration fails closed");
    assert_eq!(duration_error.kind, CatalogErrorKind::Other);
    assert_eq!(duration_error.message, "stored asset duration is invalid");

    catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            transaction.execute(
                "UPDATE assets SET duration_millis = 10000 WHERE id = ?1",
                [asset_id.to_string()],
            )?;
            transaction.execute(
                "UPDATE asset_adjustment_revisions SET parametric_equalizer_json = 'not-json' \
                 WHERE id = ?1",
                [revision.revision_id],
            )?;
            Ok(())
        })
        .expect("stored adjustment corrupts");
    let adjustment_error = catalog
        .with_transaction(|transaction| {
            adjustment_graph_at_revision(transaction, asset_id, revision.revision_id)
        })
        .expect_err("invalid stored adjustment fails closed");
    assert_eq!(adjustment_error.kind, CatalogErrorKind::Other);
    assert!(
        adjustment_error
            .message
            .starts_with("stored parametric equalizer is invalid:")
    );

    let _ = std::fs::remove_dir_all(root);
}

fn register_fixture_asset(catalog: &crate::Catalog, byte: u8, path: &str) -> AssetId {
    catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let registered = register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([byte; 32]),
                    path: Path::new(path),
                    size_bytes: 100,
                    codec: Some("pcm"),
                    duration_millis: Some(10_000),
                    recorded_at_millis: None,
                    imported_at_millis: 10,
                },
            )?;
            Ok(match registered {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
            })
        })
        .expect("fixture asset writes")
}

#[allow(clippy::too_many_lines)] // One complete persistence fixture covers every authored field.
fn fully_configured_graph() -> AdjustmentGraph {
    let mut creative_vfx = CreativeVfxSettings::default();
    creative_vfx.scene.enabled = true;
    creative_vfx.scene.character = SceneVfxCharacter::Underwater;
    creative_vfx.scene.intensity_percent = 72;
    creative_vfx.delay.enabled = true;
    creative_vfx.delay.character = DelayVfxCharacter::Echo;
    creative_vfx.delay.echo.feedback_percent = 42;
    creative_vfx.modulation.enabled = true;
    creative_vfx.modulation.character = ModulationVfxCharacter::Phaser;
    creative_vfx.modulation.phaser.feedback_percent = -18;
    creative_vfx.transform.enabled = true;
    creative_vfx.transform.character = TransformVfxCharacter::Ghost;
    creative_vfx.transform.amount_percent = 68;
    AdjustmentGraph::new(
        10_000,
        1_000,
        9_000,
        250,
        500,
        AdjustmentEffects::new(
            FadeCurves::new(FadeCurve::Smooth, FadeCurve::EqualPower),
            -300,
            80,
        )
        .with_restoration(RestorationSettings {
            enabled: true,
            de_plosive: DePlosiveSettings {
                enabled: true,
                frequency_hertz: 145,
                sensitivity_percent: 61,
                reduction_centibels: 1_250,
                release_millis: 170,
            },
            noise_reduction: NoiseReductionSettings {
                enabled: true,
                reduction_centibels: 1_100,
                sensitivity_percent: 58,
                smoothing_millis: 300,
            },
            de_esser: DeEsserSettings {
                enabled: true,
                frequency_hertz: 7_000,
                threshold_centibels: -2_600,
                reduction_centibels: 700,
            },
        })
        .with_de_hum(DeHumSettings {
            enabled: true,
            fundamental_hertz: 60,
            harmonic_count: 7,
            quality_tenths: 480,
            depth_centibels: 2_100,
        })
        .with_de_click(DeClickSettings {
            enabled: true,
            sensitivity_percent: 72,
            maximum_click_microseconds: 650,
            repair_percent: 88,
        })
        .with_channel_repair(ChannelRepairSettings {
            enabled: true,
            invert_left: true,
            invert_right: false,
            swap_channels: true,
            mono_fold_down: false,
            balance_percent: 24,
        })
        .with_equalizer(echo_domain::ParametricEqualizer::from_legacy_gains(
            300, -150, 225,
        ))
        .with_compressor(CompressorSettings {
            enabled: true,
            threshold_centibels: -2_100,
            ratio_tenths: 40,
            attack_millis: 15,
            release_millis: 180,
            makeup_centibels: 250,
        })
        .with_reverb(ReverbSettings {
            character: ReverbCharacter::Plate,
            enabled: true,
            mix_percent: 24,
            pre_delay_millis: 28,
            decay_millis: 2_400,
            size_percent: 68,
            damping_percent: 52,
            low_cut_hertz: 150,
            high_cut_hertz: 9_000,
        })
        .with_creative_vfx(creative_vfx)
        .with_limiter(LimiterSettings {
            enabled: true,
            ceiling_centibels: -125,
            release_millis: 160,
        })
        .with_effect_chain(
            EffectChain::new([
                EffectNodeKind::DeHum,
                EffectNodeKind::Space,
                EffectNodeKind::Restoration,
                EffectNodeKind::DeClick,
                EffectNodeKind::Equalizer,
                EffectNodeKind::Dynamics,
                EffectNodeKind::ChannelRepair,
                EffectNodeKind::Master,
            ])
            .expect("valid reordered chain"),
        )
        .with_edit_timeline(
            EditTimeline::new(
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
                    .expect("first source segment validates"),
                    EditSegment::new(
                        4_000,
                        5_000,
                        EditSegmentState::Hidden,
                        0,
                        0,
                        0,
                        FadeCurves::linear(),
                        125,
                    )
                    .expect("hidden source segment validates"),
                    EditSegment::new(
                        5_000,
                        9_000,
                        EditSegmentState::Muted,
                        -100,
                        50,
                        75,
                        FadeCurves::new(FadeCurve::Smooth, FadeCurve::EqualPower),
                        0,
                    )
                    .expect("muted source segment validates"),
                ],
            )
            .expect("edit timeline validates"),
        )
        .with_effect_masks(vec![
            EffectMask::new(
                2_000,
                6_000,
                10,
                vec![EffectNodeKind::Restoration, EffectNodeKind::Equalizer],
            )
            .expect("effect mask validates"),
        ]),
    )
    .expect("graph validates")
}
