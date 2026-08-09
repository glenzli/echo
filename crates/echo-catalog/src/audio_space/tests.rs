use std::path::Path;

use echo_domain::{AnalysisKind, AnalysisRecord, ContentHash, ModelIdentity};

use super::*;
use crate::{
    AppendAnalysisRecord, AppendContextualAnalysis, AssetAffinity, AssetRegistrationInput,
    RegisterAsset, open_catalog, record_adjustment_graph, record_analysis,
    record_contextual_analysis, register_asset, set_asset_affinity,
};

#[test]
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
                    duration_millis: Some(1_000),
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
            record_adjustment_graph(
                transaction,
                asset_id,
                echo_domain::AdjustmentGraph::new(
                    1_000,
                    100,
                    900,
                    50,
                    100,
                    echo_domain::FadeCurves::new(
                        echo_domain::FadeCurve::Smooth,
                        echo_domain::FadeCurve::EqualPower,
                    ),
                    -200,
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
    let adjustment = projected.adjustment.expect("adjustment projects");
    assert_eq!(adjustment.graph.trim_start_millis(), 100);
    assert_eq!(adjustment.graph.trim_end_millis(), 900);
    assert_eq!(
        projected.transcript.expect("text evidence")["text"],
        "旧房子的窗户朝南"
    );
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
