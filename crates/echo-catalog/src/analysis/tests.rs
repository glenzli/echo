use std::path::Path;

use echo_domain::{AnalysisKind, AnalysisRecord, ContentHash, ModelIdentity};

use super::*;
use crate::{
    AssetRegistrationInput, RegisterAsset, mark_asset_missing, open_catalog, register_asset,
};

fn register(
    transaction: &rusqlite::Transaction<'_>,
    byte: u8,
    path: &Path,
) -> echo_domain::AssetId {
    match register_asset(
        transaction,
        &AssetRegistrationInput {
            content_hash: ContentHash::new([byte; 32]),
            path,
            size_bytes: 100,
            codec: Some("pcm"),
            duration_millis: Some(1_000),
            recorded_at_millis: None,
            imported_at_millis: i64::from(byte),
        },
    )
    .expect("asset registers")
    {
        RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
    }
}

#[test]
fn missing_analysis_projection_excludes_evidence_and_offline_originals() {
    let root =
        std::env::temp_dir().join(format!("echo-analysis-projection-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let (ready, analyzed, offline) = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let ready = register(transaction, 1, Path::new("/voices/ready.wav"));
            let analyzed = register(transaction, 2, Path::new("/voices/analyzed.wav"));
            let offline = register(transaction, 3, Path::new("/voices/offline.wav"));
            record_analysis(
                transaction,
                &AppendAnalysisRecord {
                    asset_id: analyzed,
                    record: AnalysisRecord::new(
                        AnalysisKind::Transcript,
                        serde_json::json!({ "text": "done" }),
                        ModelIdentity::new("test".into(), "1".into()),
                        None,
                        10,
                    ),
                },
            )?;
            mark_asset_missing(transaction, &offline.to_string())?;
            Ok((ready, analyzed, offline))
        })
        .expect("fixture writes");

    let missing = catalog
        .with_transaction(|transaction| {
            list_assets_missing_analysis(transaction, AnalysisKind::Transcript)
        })
        .expect("projection reads");
    assert_eq!(missing, [ready]);
    assert!(!missing.contains(&analyzed));
    assert!(!missing.contains(&offline));
    let _ = std::fs::remove_dir_all(root);
}

fn record_fixture_analysis(
    transaction: &rusqlite::Transaction<'_>,
    asset_id: echo_domain::AssetId,
    kind: AnalysisKind,
    value: serde_json::Value,
    created_at_millis: i64,
) -> Result<(), crate::CatalogError> {
    record_analysis(
        transaction,
        &AppendAnalysisRecord {
            asset_id,
            record: AnalysisRecord::new(
                kind,
                value,
                ModelIdentity::new("test".into(), "1".into()),
                None,
                created_at_millis,
            ),
        },
    )
}

#[test]
fn contextual_projection_requires_the_newest_current_sound_caption() {
    let root = std::env::temp_dir().join(format!(
        "echo-current-contextual-projection-{}",
        std::process::id()
    ));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let (missing, legacy, current, superseded) = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let missing = register(transaction, 10, Path::new("/voices/missing.wav"));
            let legacy = register(transaction, 11, Path::new("/voices/legacy.wav"));
            let current = register(transaction, 12, Path::new("/voices/current.wav"));
            let superseded = register(transaction, 13, Path::new("/voices/superseded.wav"));
            for (offset, asset_id) in [missing, legacy, current, superseded]
                .into_iter()
                .enumerate()
            {
                let created_at = 100 + i64::try_from(offset).expect("offset fits i64") * 10;
                record_fixture_analysis(
                    transaction,
                    asset_id,
                    AnalysisKind::Transcript,
                    serde_json::json!({ "text": "fixture speech" }),
                    created_at,
                )?;
                record_fixture_analysis(
                    transaction,
                    asset_id,
                    AnalysisKind::Alignment,
                    serde_json::json!({ "text": "fixture speech", "items": [] }),
                    created_at + 1,
                )?;
            }
            record_fixture_analysis(
                transaction,
                legacy,
                AnalysisKind::Contextual,
                serde_json::json!({ "summary": "legacy" }),
                200,
            )?;
            let current_value = serde_json::json!({
                "schema_version": 3,
                "sound_caption": "Voices in a quiet room",
                "summary": ""
            });
            record_fixture_analysis(
                transaction,
                current,
                AnalysisKind::Contextual,
                current_value.clone(),
                210,
            )?;
            record_fixture_analysis(
                transaction,
                superseded,
                AnalysisKind::Contextual,
                current_value,
                220,
            )?;
            record_fixture_analysis(
                transaction,
                superseded,
                AnalysisKind::Contextual,
                serde_json::json!({ "summary": "newer legacy evidence" }),
                221,
            )?;
            Ok((missing, legacy, current, superseded))
        })
        .expect("fixtures write");

    let queued = catalog
        .with_transaction(|transaction| {
            list_assets_with_alignment_missing_current_contextual(transaction, 3)
        })
        .expect("projection reads");
    assert_eq!(queued, [missing, legacy, superseded]);
    assert!(!queued.contains(&current));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn ordinary_backfill_excludes_assets_owned_by_long_audio_pipeline() {
    let root = std::env::temp_dir().join(format!(
        "echo-long-audio-backfill-exclusion-{}",
        std::process::id()
    ));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let (ordinary, long) = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let ordinary = register(transaction, 31, Path::new("/voices/ordinary.wav"));
            let long = register(transaction, 32, Path::new("/voices/long.wav"));
            for asset_id in [ordinary, long] {
                record_fixture_analysis(
                    transaction,
                    asset_id,
                    AnalysisKind::Transcript,
                    serde_json::json!({ "text": "fixture speech" }),
                    100,
                )?;
            }
            crate::ensure_long_audio_plan(
                transaction,
                long,
                1,
                &[crate::LongAudioSegmentPlan {
                    index: 0,
                    start_millis: 0,
                    end_millis: 1_000,
                }],
                101,
            )?;
            Ok((ordinary, long))
        })
        .expect("fixtures write");

    let alignment = catalog
        .with_transaction(list_assets_with_nonempty_transcript_missing_alignment)
        .expect("alignment projection reads");
    assert_eq!(alignment, [ordinary]);
    assert!(!alignment.contains(&long));

    catalog
        .with_transaction(|transaction| -> Result<(), crate::CatalogError> {
            for asset_id in [ordinary, long] {
                record_fixture_analysis(
                    transaction,
                    asset_id,
                    AnalysisKind::Alignment,
                    serde_json::json!({ "text": "fixture speech", "items": [] }),
                    102,
                )?;
            }
            Ok(())
        })
        .expect("alignment fixtures write");
    let contextual = catalog
        .with_transaction(|transaction| {
            list_assets_with_alignment_missing_current_contextual(transaction, 3)
        })
        .expect("contextual projection reads");
    assert_eq!(contextual, [ordinary]);
    assert!(!contextual.contains(&long));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn audio_event_projection_requires_present_empty_asr_and_missing_event_evidence() {
    let root = std::env::temp_dir().join(format!(
        "echo-audio-event-projection-{}",
        std::process::id()
    ));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let (eligible, speaking, analyzed, offline) = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let eligible = register(transaction, 41, Path::new("/sounds/birds.wav"));
            let speaking = register(transaction, 42, Path::new("/sounds/speech.wav"));
            let analyzed = register(transaction, 43, Path::new("/sounds/truck.wav"));
            let offline = register(transaction, 44, Path::new("/sounds/offline.wav"));
            for asset_id in [eligible, analyzed, offline] {
                record_fixture_analysis(
                    transaction,
                    asset_id,
                    AnalysisKind::Transcript,
                    serde_json::json!({ "text": "" }),
                    100,
                )?;
            }
            record_fixture_analysis(
                transaction,
                speaking,
                AnalysisKind::Transcript,
                serde_json::json!({ "text": "recognized speech" }),
                101,
            )?;
            record_fixture_analysis(
                transaction,
                analyzed,
                AnalysisKind::AudioEvents,
                serde_json::json!({ "schema_version": 1, "chunks": [] }),
                102,
            )?;
            mark_asset_missing(transaction, &offline.to_string())?;
            Ok((eligible, speaking, analyzed, offline))
        })
        .expect("fixtures write");

    let projected = catalog
        .with_transaction(list_assets_with_empty_transcript_missing_audio_events)
        .expect("projection reads");

    assert_eq!(projected, [eligible]);
    assert!(!projected.contains(&speaking));
    assert!(!projected.contains(&analyzed));
    assert!(!projected.contains(&offline));
    let _ = std::fs::remove_dir_all(root);
}
