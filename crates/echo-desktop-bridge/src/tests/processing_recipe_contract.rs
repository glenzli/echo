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

    let source_adjustment = recipe_source_adjustment();
    session
        .set_asset_adjustment(&source.to_string(), &source_adjustment)
        .expect("source adjustment saves");

    let target_adjustment = recipe_target_adjustment();
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
    assert_target_clip_edits(&applied);
    assert_eq!(applied.low_cut_hertz(), 90);
    assert!(applied.compressor().enabled);
    assert_eq!(applied.compressor().threshold_centibels, -2_400);

    let repeated = session
        .apply_processing_recipe(&recipe_id, &[target.to_string()], 0)
        .expect("recipe repeat applies");
    assert_eq!(repeated.updated_count, 0);
    assert_eq!(repeated.unchanged_count, 1);
    assert_eq!(repeated.failed_count, 0);
    let history = session
        .processing_recipe_history()
        .expect("processing history lists");
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].batch_id, repeated.batch_id);
    assert_eq!(history[0].recipe_name, "Dialogue cleanup");
    assert_eq!(history[0].recipe_revision_number, 1);
    assert_eq!(history[0].merge_mode, "merge");
    assert_eq!(history[0].target_count, 1);
    assert_eq!(history[0].unchanged_count, 1);
    assert!(!history[0].reverted);
    let _ = std::fs::remove_dir_all(root);
}

fn recipe_source_adjustment() -> AssetAdjustmentWire {
    let mut source = adjustment(10_000);
    source.trim_start_millis = 500;
    source.trim_end_millis = 9_500;
    source.fade_in_millis = 150;
    source.fade_out_millis = 220;
    source.gain_centibels = -200;
    source.low_cut_hertz = 90;
    source.compressor_enabled = true;
    source.compressor_threshold_centibels = -2_400;
    source.edit_segments = vec![crate::ffi::EditSegmentWire {
        source_start_millis: 500,
        source_end_millis: 9_500,
        state: 0,
        gain_centibels: 0,
        fade_in_millis: 0,
        fade_out_millis: 0,
        fade_in_curve: 0,
        fade_out_curve: 0,
        gap_after_millis: 0,
    }];
    source
}

fn recipe_target_adjustment() -> AssetAdjustmentWire {
    let mut target = adjustment(8_000);
    target.trim_start_millis = 1_000;
    target.trim_end_millis = 7_000;
    target.fade_in_millis = 320;
    target.fade_out_millis = 480;
    target.fade_in_curve = 1;
    target.fade_out_curve = 2;
    target.gain_centibels = -475;
    target.edit_segments = vec![
        crate::ffi::EditSegmentWire {
            source_start_millis: 1_000,
            source_end_millis: 3_500,
            state: 0,
            gain_centibels: 175,
            fade_in_millis: 25,
            fade_out_millis: 50,
            fade_in_curve: 1,
            fade_out_curve: 2,
            gap_after_millis: 120,
        },
        crate::ffi::EditSegmentWire {
            source_start_millis: 3_500,
            source_end_millis: 7_000,
            state: 2,
            gain_centibels: 0,
            fade_in_millis: 0,
            fade_out_millis: 0,
            fade_in_curve: 0,
            fade_out_curve: 0,
            gap_after_millis: 0,
        },
    ];
    target.effect_masks = vec![crate::ffi::EffectMaskWire {
        start_millis: 1_500,
        end_millis: 2_500,
        feather_millis: 10,
        effect_nodes: vec![1],
    }];
    target
}

fn assert_target_clip_edits(applied: &echo_domain::AdjustmentGraph) {
    assert_eq!(applied.trim_start_millis(), 1_000);
    assert_eq!(applied.trim_end_millis(), 7_000);
    assert_eq!(applied.fade_in_millis(), 320);
    assert_eq!(applied.fade_out_millis(), 480);
    assert_eq!(applied.fade_in_curve(), echo_domain::FadeCurve::Smooth);
    assert_eq!(applied.fade_out_curve(), echo_domain::FadeCurve::EqualPower);
    assert_eq!(applied.gain_centibels(), -475);
    assert_eq!(applied.edit_timeline().segments().len(), 2);
    assert_eq!(
        applied.edit_timeline().segments()[0].gap_after_millis(),
        120
    );
    assert_eq!(
        applied.edit_timeline().segments()[1].state(),
        echo_domain::EditSegmentState::Hidden
    );
    assert_eq!(applied.effect_masks().len(), 1);
    assert_eq!(
        applied.effect_masks()[0].effect_nodes(),
        [echo_domain::EffectNodeKind::Equalizer]
    );
}

#[test]
fn recipe_management_and_revert_preserve_later_sound_edits() {
    let root = fixture_catalog();
    let session = open_session(
        root.join("catalog.sqlite").to_str().expect("utf8"),
        root.join("cache").to_str().expect("utf8"),
    )
    .expect("session opens");
    let source = register(&session, [73; 32], &root.join("source-managed.wav"), 10_000);
    let target = register(&session, [74; 32], &root.join("target-managed.wav"), 8_000);

    let mut source_adjustment = adjustment(10_000);
    source_adjustment.low_cut_hertz = 85;
    session
        .set_asset_adjustment(&source.to_string(), &source_adjustment)
        .expect("source adjustment saves");
    let recipe_id = session
        .create_processing_recipe(
            "Field cleanup",
            &source.to_string(),
            &[echo_domain::ProcessingComponent::LowCut.wire_value()],
        )
        .expect("recipe creates");
    session
        .rename_processing_recipe(&recipe_id, "Field restoration")
        .expect("recipe renames");

    source_adjustment.low_cut_hertz = 110;
    session
        .set_asset_adjustment(&source.to_string(), &source_adjustment)
        .expect("updated source adjustment saves");
    assert_eq!(
        session
            .update_processing_recipe(
                &recipe_id,
                &source.to_string(),
                &[echo_domain::ProcessingComponent::LowCut.wire_value()],
            )
            .expect("recipe revision appends"),
        2
    );
    let recipes = session.processing_recipes().expect("recipes list");
    assert_eq!(recipes[0].name, "Field restoration");
    assert_eq!(recipes[0].revision_number, 2);

    let first_apply = session
        .apply_processing_recipe(&recipe_id, &[target.to_string()], 0)
        .expect("recipe applies");
    let first_revert = session
        .revert_processing_recipe_application(&first_apply.batch_id)
        .expect("application reverts");
    assert_eq!(first_revert.restored_count, 1);
    assert_eq!(first_revert.conflict_count, 0);
    let restored_revision = first_revert.results[0].adjustment_revision;
    let repeated = session
        .revert_processing_recipe_application(&first_apply.batch_id)
        .expect("revert is idempotent");
    assert_eq!(repeated.revert_id, first_revert.revert_id);
    assert_eq!(repeated.results[0].adjustment_revision, restored_revision);

    let second_apply = session
        .apply_processing_recipe(&recipe_id, &[target.to_string()], 0)
        .expect("recipe reapplies");
    let mut later_adjustment = adjustment(8_000);
    later_adjustment.low_cut_hertz = 150;
    session
        .set_asset_adjustment(&target.to_string(), &later_adjustment)
        .expect("later user adjustment saves");
    let conflict = session
        .revert_processing_recipe_application(&second_apply.batch_id)
        .expect("conflicting revert returns receipt");
    assert_eq!(conflict.restored_count, 0);
    assert_eq!(conflict.conflict_count, 1);
    let current = session
        .catalog()
        .with_transaction(|transaction| echo_catalog::latest_adjustment_graph(transaction, target))
        .expect("target reads")
        .expect("target adjustment exists");
    assert_eq!(current.graph.low_cut_hertz(), 150);

    session
        .archive_processing_recipe(&recipe_id)
        .expect("recipe archives");
    assert!(
        session
            .processing_recipes()
            .expect("active recipes list")
            .is_empty()
    );
    assert!(
        session
            .apply_processing_recipe(&recipe_id, &[target.to_string()], 0)
            .is_err()
    );
    let history = session
        .processing_recipe_history()
        .expect("archived processing history lists");
    assert_managed_history(&history, &first_apply.batch_id, &second_apply.batch_id);
    let _ = std::fs::remove_dir_all(root);
}

fn assert_managed_history(
    history: &[crate::ffi::ProcessingRecipeHistoryWire],
    first_batch_id: &str,
    second_batch_id: &str,
) {
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].batch_id, second_batch_id);
    assert_eq!(history[0].recipe_name, "Field restoration");
    assert_eq!(history[0].recipe_revision_number, 2);
    assert!(history[0].reverted);
    assert_eq!(history[0].restored_count, 0);
    assert_eq!(history[0].conflict_count, 1);
    assert_eq!(history[1].batch_id, first_batch_id);
    assert!(history[1].reverted);
    assert_eq!(history[1].restored_count, 1);
    assert_eq!(history[1].conflict_count, 0);
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
        edit_segments: vec![crate::ffi::EditSegmentWire {
            source_start_millis: 0,
            source_end_millis: duration_millis,
            state: 0,
            gain_centibels: 0,
            fade_in_millis: 0,
            fade_out_millis: 0,
            fade_in_curve: 0,
            fade_out_curve: 0,
            gap_after_millis: 0,
        }],
        effect_masks: Vec::new(),
    }
}
