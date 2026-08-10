use std::path::Path;

use echo_domain::{
    AdjustmentEffects, AdjustmentGraph, CompressorSettings, ContentHash, DeClickSettings,
    DeEsserSettings, DeHumSettings, EffectChain, EffectNodeKind, FadeCurve, FadeCurves,
    LimiterSettings, NoiseReductionSettings, RestorationSettings, ReverbSettings,
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
        .with_transaction(|transaction| record_adjustment_graph(transaction, asset_id, graph, 20))
        .expect("first revision writes");
    let duplicate = catalog
        .with_transaction(|transaction| record_adjustment_graph(transaction, asset_id, graph, 30))
        .expect("duplicate save reads current");
    assert_eq!(duplicate, first);
    assert_eq!(
        first.graph.equalizer(),
        echo_domain::ParametricEqualizer::from_legacy_gains(300, -150, 225)
    );
    assert_eq!(first.graph.compressor(), graph.compressor());
    assert_eq!(first.graph.de_hum(), graph.de_hum());
    assert_eq!(first.graph.de_click(), graph.de_click());
    assert_eq!(first.graph.reverb(), graph.reverb());
    assert_eq!(first.graph.limiter(), graph.limiter());
    assert_eq!(first.graph.effect_chain(), graph.effect_chain());

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
            record_adjustment_graph(transaction, asset_id, second_graph, 40)
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

fn fully_configured_graph() -> AdjustmentGraph {
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
        })
        .with_effect_chain(
            EffectChain::new([
                EffectNodeKind::DeHum,
                EffectNodeKind::Space,
                EffectNodeKind::Restoration,
                EffectNodeKind::DeClick,
                EffectNodeKind::Equalizer,
                EffectNodeKind::Dynamics,
                EffectNodeKind::Master,
            ])
            .expect("valid reordered chain"),
        ),
    )
    .expect("graph validates")
}
