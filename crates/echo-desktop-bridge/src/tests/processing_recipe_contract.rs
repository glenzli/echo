//! Processing recipes cross the desktop facade as immutable shared intent and
//! materialize back into asset-local adjustment revisions.

use crate::{ffi::AssetAdjustmentWire, session::open_session};

use super::{fixture_catalog, test_equalizer_bands};

#[test]
fn recipe_create_list_and_apply_preserve_target_clip_edits() {
    let root = fixture_catalog();
    let session = open_session(
        root.join("catalog.sqlite").to_str().expect("utf8"),
        root.join("cache").to_str().expect("utf8"),
    )
    .expect("session opens");
    let source = register(&session, [71; 32], &root.join("source.wav"), 10_000);
    let target = register(&session, [72; 32], &root.join("target.wav"), 8_000);

    let mut source_adjustment = adjustment(10_000);
    source_adjustment.trim_start_millis = 500;
    source_adjustment.trim_end_millis = 9_500;
    source_adjustment.fade_in_millis = 150;
    source_adjustment.fade_out_millis = 220;
    source_adjustment.gain_centibels = -200;
    source_adjustment.low_cut_hertz = 90;
    source_adjustment.compressor_enabled = true;
    source_adjustment.compressor_threshold_centibels = -2_400;
    session
        .set_asset_adjustment(&source.to_string(), &source_adjustment)
        .expect("source adjustment saves");

    let mut target_adjustment = adjustment(8_000);
    target_adjustment.trim_start_millis = 1_000;
    target_adjustment.trim_end_millis = 7_000;
    target_adjustment.fade_in_millis = 320;
    target_adjustment.fade_out_millis = 480;
    target_adjustment.fade_in_curve = 1;
    target_adjustment.fade_out_curve = 2;
    target_adjustment.gain_centibels = -475;
    session
        .set_asset_adjustment(&target.to_string(), &target_adjustment)
        .expect("target adjustment saves");

    let recipe_id = session
        .create_processing_recipe(
            "Dialogue cleanup",
            &source.to_string(),
            &[
                echo_domain::ProcessingComponent::LowCut.wire_value(),
                echo_domain::ProcessingComponent::Dynamics.wire_value(),
            ],
        )
        .expect("recipe creates");
    let recipes = session.processing_recipes().expect("recipes list");
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].id, recipe_id);
    assert_eq!(recipes[0].name, "Dialogue cleanup");
    assert_eq!(recipes[0].components, vec![0, 5]);

    let receipt = session
        .apply_processing_recipe(&recipe_id, &[target.to_string()], 0)
        .expect("recipe applies");
    assert_eq!(receipt.updated_count, 1);
    assert_eq!(receipt.unchanged_count, 0);
    assert_eq!(receipt.failed_count, 0);
    assert_eq!(receipt.results[0].outcome, "updated");

    let applied = session
        .catalog()
        .with_transaction(|transaction| echo_catalog::latest_adjustment_graph(transaction, target))
        .expect("target adjustment reads")
        .expect("target adjustment exists")
        .graph;
    assert_eq!(applied.trim_start_millis(), 1_000);
    assert_eq!(applied.trim_end_millis(), 7_000);
    assert_eq!(applied.fade_in_millis(), 320);
    assert_eq!(applied.fade_out_millis(), 480);
    assert_eq!(applied.fade_in_curve(), echo_domain::FadeCurve::Smooth);
    assert_eq!(applied.fade_out_curve(), echo_domain::FadeCurve::EqualPower);
    assert_eq!(applied.gain_centibels(), -475);
    assert_eq!(applied.low_cut_hertz(), 90);
    assert!(applied.compressor().enabled);
    assert_eq!(applied.compressor().threshold_centibels, -2_400);

    let repeated = session
        .apply_processing_recipe(&recipe_id, &[target.to_string()], 0)
        .expect("recipe repeat applies");
    assert_eq!(repeated.updated_count, 0);
    assert_eq!(repeated.unchanged_count, 1);
    assert_eq!(repeated.failed_count, 0);
    let _ = std::fs::remove_dir_all(root);
}

fn register(
    session: &crate::session::LibrarySession,
    hash: [u8; 32],
    path: &std::path::Path,
    duration_millis: u64,
) -> echo_domain::AssetId {
    let registered = session
        .catalog()
        .with_transaction(|transaction| {
            echo_catalog::register_asset(
                transaction,
                &echo_catalog::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new(hash),
                    path,
                    size_bytes: 1_024,
                    codec: Some("pcm"),
                    duration_millis: Some(duration_millis),
                    recorded_at_millis: None,
                    imported_at_millis: i64::from(hash[0]),
                },
            )
        })
        .expect("asset registers");
    match registered {
        echo_catalog::RegisterAsset::Created(asset)
        | echo_catalog::RegisterAsset::Existed(asset) => asset.id,
    }
}

fn adjustment(duration_millis: u64) -> AssetAdjustmentWire {
    AssetAdjustmentWire {
        trim_start_millis: 0,
        trim_end_millis: duration_millis,
        fade_in_millis: 0,
        fade_out_millis: 0,
        fade_in_curve: 0,
        fade_out_curve: 0,
        gain_centibels: 0,
        low_cut_hertz: 0,
        restoration_enabled: true,
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
        equalizer_enabled: false,
        equalizer_bands: test_equalizer_bands(),
        compressor_enabled: false,
        compressor_threshold_centibels: -1_800,
        compressor_ratio_tenths: 30,
        compressor_attack_millis: 10,
        compressor_release_millis: 120,
        compressor_makeup_centibels: 0,
        reverb_enabled: false,
        reverb_mix_percent: 18,
        reverb_pre_delay_millis: 20,
        reverb_decay_millis: 1_800,
        reverb_size_percent: 55,
        reverb_damping_percent: 45,
        reverb_low_cut_hertz: 120,
        reverb_high_cut_hertz: 10_000,
        limiter_enabled: false,
        limiter_ceiling_centibels: -100,
        limiter_release_millis: 100,
        effect_chain: vec![0, 1, 2, 3, 4],
    }
}
