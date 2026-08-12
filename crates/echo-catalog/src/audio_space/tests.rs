use std::path::Path;

use echo_domain::{AnalysisKind, AnalysisRecord, ContentHash, ModelIdentity};

use super::*;
use crate::{
    AppendAnalysisRecord, AppendContextualAnalysis, AssetAffinity, AssetRegistrationInput,
    RegisterAsset, open_catalog, record_adjustment_graph, record_analysis,
    record_asset_listening_progress, record_contextual_analysis, register_asset,
    set_asset_affinity,
};

#[test]
#[allow(clippy::too_many_lines)] // One sound-wall row fixture exercises the full adjustment projection.
fn sound_wall_projection_keeps_text_and_user_affinity_distinct() {
    let root = std::env::temp_dir().join(format!("echo-sound-wall-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let registered = register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([5; 32]),
                    path: Path::new("/voices/memory.wav"),
                    size_bytes: 100,
                    codec: Some("pcm"),
                    duration_millis: Some(120_000),
                    recorded_at_millis: None,
                    imported_at_millis: 10,
                },
            )?;
            let asset_id = match registered {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
            };
            record_analysis(
                transaction,
                &AppendAnalysisRecord {
                    asset_id,
                    record: AnalysisRecord::new(
                        AnalysisKind::Transcript,
                        serde_json::json!({ "text": "旧房子的窗户朝南", "segments": [] }),
                        ModelIdentity::new("test".into(), "1".into()),
                        None,
                        20,
                    ),
                },
            )?;
            set_asset_affinity(
                transaction,
                asset_id,
                AssetAffinity {
                    liked: true,
                    rating: 5,
                },
                30,
            )?;
            record_asset_listening_progress(transaction, asset_id, 70_000, 0, 120_000, 31)?;
            record_adjustment_graph(
                transaction,
                asset_id,
                echo_domain::AdjustmentGraph::new(
                    120_000,
                    100,
                    900,
                    50,
                    100,
                    echo_domain::AdjustmentEffects::new(
                        echo_domain::FadeCurves::new(
                            echo_domain::FadeCurve::Smooth,
                            echo_domain::FadeCurve::EqualPower,
                        ),
                        -200,
                        80,
                    )
                    .with_de_hum(echo_domain::DeHumSettings {
                        enabled: true,
                        fundamental_hertz: 60,
                        harmonic_count: 5,
                        quality_tenths: 360,
                        depth_centibels: 1_700,
                    })
                    .with_de_click(echo_domain::DeClickSettings {
                        enabled: true,
                        sensitivity_percent: 66,
                        maximum_click_microseconds: 800,
                        repair_percent: 90,
                    })
                    .with_edit_timeline(
                        echo_domain::EditTimeline::new(
                            100,
                            900,
                            vec![
                                echo_domain::EditSegment::new(
                                    100,
                                    400,
                                    echo_domain::EditSegmentState::Audible,
                                    0,
                                    0,
                                    0,
                                    echo_domain::FadeCurves::linear(),
                                    0,
                                )
                                .expect("audible edit segment validates"),
                                echo_domain::EditSegment::new(
                                    400,
                                    900,
                                    echo_domain::EditSegmentState::Hidden,
                                    0,
                                    0,
                                    0,
                                    echo_domain::FadeCurves::linear(),
                                    120,
                                )
                                .expect("hidden edit segment validates"),
                            ],
                        )
                        .expect("edit timeline validates"),
                    )
                    .with_effect_masks(vec![
                        echo_domain::EffectMask::new(
                            200,
                            600,
                            10,
                            vec![echo_domain::EffectNodeKind::Restoration],
                        )
                        .expect("effect mask validates"),
                    ]),
                )
                .expect("adjustment validates"),
                31,
            )?;
            Ok(asset_id)
        })
        .expect("fixture writes");

    let assets = catalog
        .with_transaction(list_audio_space)
        .expect("projection reads");
    let projected = assets
        .into_iter()
        .find(|asset| asset.id == asset_id.to_string())
        .expect("asset projects");
    assert!(projected.liked);
    assert_eq!(projected.rating, 5);
    assert_eq!(projected.last_listened_at_millis, 31);
    assert_eq!(projected.resume_position_millis, 70_000);
    let adjustment = projected.adjustment.expect("adjustment projects");
    assert_eq!(adjustment.graph.trim_start_millis(), 100);
    assert_eq!(adjustment.graph.trim_end_millis(), 900);
    assert_eq!(adjustment.graph.de_hum().fundamental_hertz, 60);
    assert_eq!(adjustment.graph.de_hum().harmonic_count, 5);
    assert_eq!(adjustment.graph.de_click().sensitivity_percent, 66);
    assert_eq!(adjustment.graph.de_click().maximum_click_microseconds, 800);
    assert_eq!(adjustment.graph.edit_timeline().segments().len(), 2);
    assert_eq!(
        adjustment.graph.edit_timeline().segments()[1].state(),
        echo_domain::EditSegmentState::Hidden
    );
    assert_eq!(adjustment.graph.effect_masks().len(), 1);
    assert_eq!(
        adjustment.graph.effect_masks()[0].effect_nodes(),
        &[echo_domain::EffectNodeKind::Restoration]
    );
    assert_eq!(
        projected.transcript.expect("text evidence")["text"],
        "旧房子的窗户朝南"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn sound_wall_projection_bounds_long_text_and_omits_segments() {
    let root = std::env::temp_dir().join(format!("echo-bounded-wall-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let registered = register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([0x33; 32]),
                    path: Path::new("/voices/long.wav"),
                    size_bytes: 100,
                    codec: Some("pcm"),
                    duration_millis: Some(1_000_000),
                    recorded_at_millis: None,
                    imported_at_millis: 10,
                },
            )?;
            let asset_id = match registered {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
            };
            record_analysis(
                transaction,
                &AppendAnalysisRecord {
                    asset_id,
                    record: AnalysisRecord::new(
                        AnalysisKind::Transcript,
                        serde_json::json!({
                            "model":"fixture",
                            "text":"x".repeat(10_000),
                            "segments":[{"text":"private leaf","start":0,"end":1}],
                            "runtime":{"large":"provenance"}
                        }),
                        ModelIdentity::new("test".into(), "1".into()),
                        None,
                        20,
                    ),
                },
            )?;
            Ok(asset_id)
        })
        .expect("fixture writes");
    let projected = catalog
        .with_transaction(list_audio_space)
        .expect("projection reads")
        .into_iter()
        .find(|asset| asset.id == asset_id.to_string())
        .expect("asset projects")
        .transcript
        .expect("preview exists");
    assert_eq!(projected["text"].as_str().unwrap().len(), 2048);
    assert_eq!(projected["segments"], serde_json::json!([]));
    assert!(projected["runtime"].is_null());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn sound_wall_projection_keeps_latest_positive_facets_across_empty_refresh() {
    let root = std::env::temp_dir().join(format!("echo-positive-facets-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let registered = register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([6; 32]),
                    path: Path::new("/voices/window.wav"),
                    size_bytes: 100,
                    codec: Some("pcm"),
                    duration_millis: Some(1_000),
                    recorded_at_millis: None,
                    imported_at_millis: 10,
                },
            )?;
            let asset_id = match registered {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
            };
            append_contextual_fixture(transaction, asset_id, 2, 21)?;
            append_contextual_fixture(transaction, asset_id, 3, 22)?;
            Ok(asset_id)
        })
        .expect("fixture writes");

    let assets = catalog
        .with_transaction(list_audio_space)
        .expect("projection reads");
    let projected = assets
        .into_iter()
        .find(|asset| asset.id == asset_id.to_string())
        .expect("asset projects");
    assert_eq!(
        projected.contextual.expect("latest contextual evidence")["sound_caption"],
        "Sunlight through old windows"
    );
    assert_eq!(projected.contextual_keywords, ["old house", "window"]);
    assert_eq!(projected.contextual_mood.as_deref(), Some("calm"));
    assert_eq!(
        projected.contextual_event_type.as_deref(),
        Some("recollection")
    );
    let _ = std::fs::remove_dir_all(root);
}

fn append_contextual_fixture(
    transaction: &rusqlite::Transaction<'_>,
    asset_id: echo_domain::AssetId,
    schema_version: u8,
    created_at_millis: i64,
) -> Result<(), crate::CatalogError> {
    let payload = if schema_version == 2 {
        serde_json::json!({
            "schema_version": 2,
            "sound_caption": "Window memory",
            "summary": "fixture",
            "keywords": ["old house", "window"],
            "mood": "calm",
            "place_hint": null,
            "event_type": "recollection",
            "people_hints": []
        })
    } else {
        serde_json::json!({
            "schema_version": 3,
            "sound_caption": "Sunlight through old windows",
            "summary": "",
            "keywords": [],
            "mood": null,
            "place_hint": null,
            "event_type": null,
            "people_hints": []
        })
    };
    record_contextual_analysis(
        transaction,
        &AppendContextualAnalysis {
            analysis: AppendAnalysisRecord {
                asset_id,
                record: AnalysisRecord::new(
                    AnalysisKind::Contextual,
                    payload,
                    ModelIdentity::new("test".into(), schema_version.to_string()),
                    None,
                    created_at_millis,
                ),
            },
        },
    )?;
    Ok(())
}
