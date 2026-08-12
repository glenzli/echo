use std::path::Path;

use echo_domain::{AnalysisKind, AnalysisRecord, ContentHash, ModelIdentity};

use super::*;
use crate::{
    AppendAnalysisRecord, AssetRegistrationInput, RegisterAsset, open_catalog, record_analysis,
    register_asset,
};

#[test]
fn current_evidence_forms_a_bounded_revisioned_document() {
    let root = std::env::temp_dir().join(format!("echo-semantic-source-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let asset = match register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([61; 32]),
                    path: Path::new("/sounds/train.wav"),
                    size_bytes: 100,
                    codec: Some("pcm"),
                    duration_millis: Some(1_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )? {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset,
            };
            record_analysis(
                transaction,
                &AppendAnalysisRecord {
                    asset_id: asset.id,
                    record: AnalysisRecord::new(
                        AnalysisKind::Transcript,
                        serde_json::json!({"text":"A train arrives beside a rainy platform."}),
                        ModelIdentity::new("asr".into(), "1".into()),
                        None,
                        2,
                    ),
                },
            )?;
            record_analysis(
                transaction,
                &AppendAnalysisRecord {
                    asset_id: asset.id,
                    record: AnalysisRecord::new(
                        AnalysisKind::Contextual,
                        serde_json::json!({
                            "schema_version":3,
                            "sound_caption":"Train at a rainy platform",
                            "summary":"",
                            "keywords":["railway","rain"],
                            "mood":"calm",
                            "place_hint":"station",
                            "event_type":"train arrival",
                            "people_hints":[]
                        }),
                        ModelIdentity::new("llm".into(), "1".into()),
                        None,
                        3,
                    ),
                },
            )?;
            Ok(asset)
        })
        .expect("fixture writes");

    let sources = catalog
        .with_transaction(list_semantic_sources_needing_embedding)
        .expect("source projects");
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].asset_id, asset.id);
    assert!(
        sources[0]
            .revision
            .starts_with("echo:semantic-document:v2:")
    );
    assert!(sources[0].text.contains("railway"));
    assert!(sources[0].text.contains("train arrival"));
    assert!(sources[0].text.len() <= MAX_DOCUMENT_BYTES);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn compact_vectors_rank_only_inside_the_exact_space() {
    let root = std::env::temp_dir().join(format!("echo-semantic-index-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let assets = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let mut assets = Vec::new();
            for byte in [71, 72, 73] {
                let asset = match register_asset(
                    transaction,
                    &AssetRegistrationInput {
                        content_hash: ContentHash::new([byte; 32]),
                        path: Path::new("/sounds/fixture.wav"),
                        size_bytes: 100,
                        codec: Some("pcm"),
                        duration_millis: Some(1_000),
                        recorded_at_millis: None,
                        imported_at_millis: i64::from(byte),
                    },
                )? {
                    RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset,
                };
                assets.push(asset);
            }
            Ok(assets)
        })
        .expect("assets register");
    catalog
        .with_transaction(|transaction| {
            for (index, values, space) in [
                (0, vec![1.0, 0.0, 0.0], "space-a"),
                (1, vec![0.8, 0.2, 0.0], "space-a"),
                (2, vec![1.0, 0.0, 0.0], "space-b"),
            ] {
                let source = SemanticSource {
                    asset_id: assets[index].id,
                    revision: format!("fixture-{index}"),
                    text: format!("document {index}"),
                };
                upsert_semantic_document(
                    transaction,
                    &UpsertSemanticDocument {
                        source: &source,
                        embedding_space: space,
                        values: &values,
                        runtime: &serde_json::json!({"job_id":index}),
                        updated_at_millis: 1,
                    },
                )?;
            }
            Ok::<_, crate::CatalogError>(())
        })
        .expect("vectors publish");

    let hits = catalog
        .with_transaction(|transaction| {
            search_semantic_documents(transaction, &[1.0, 0.0, 0.0], "space-a", 10)
        })
        .expect("search succeeds");
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].asset_id, assets[0].id.to_string());
    assert!(hits[0].score > hits[1].score);

    catalog
        .with_transaction(|transaction| {
            crate::mark_asset_missing(transaction, &assets[0].id.to_string())
        })
        .expect("asset becomes unavailable");
    let present_hits = catalog
        .with_transaction(|transaction| {
            search_semantic_documents(transaction, &[1.0, 0.0, 0.0], "space-a", 10)
        })
        .expect("search excludes offline originals");
    assert_eq!(present_hits.len(), 1);
    assert_eq!(present_hits[0].asset_id, assets[1].id.to_string());

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn obsolete_runtime_vectors_are_removed_without_touching_current_documents() {
    let root = std::env::temp_dir().join(format!("echo-semantic-contract-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let assets = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let mut assets = Vec::new();
            for byte in [81, 82] {
                let asset = match register_asset(
                    transaction,
                    &AssetRegistrationInput {
                        content_hash: ContentHash::new([byte; 32]),
                        path: Path::new("/sounds/contract-fixture.wav"),
                        size_bytes: 100,
                        codec: Some("pcm"),
                        duration_millis: Some(1_000),
                        recorded_at_millis: None,
                        imported_at_millis: i64::from(byte),
                    },
                )? {
                    RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset,
                };
                assets.push(asset);
            }
            Ok(assets)
        })
        .expect("assets register");
    catalog
        .with_transaction(|transaction| {
            for (index, contract) in [
                "infer-runtime.consumer-core@20260813.1",
                "infer-runtime.consumer-core@20260812.1",
            ]
            .into_iter()
            .enumerate()
            {
                let source = SemanticSource {
                    asset_id: assets[index].id,
                    revision: format!("contract-fixture-{index}"),
                    text: format!("document {index}"),
                };
                upsert_semantic_document(
                    transaction,
                    &UpsertSemanticDocument {
                        source: &source,
                        embedding_space: "space-a",
                        values: &[1.0, 0.0, 0.0],
                        runtime: &serde_json::json!({
                            "runtime": {"contract_version": contract}
                        }),
                        updated_at_millis: 1,
                    },
                )?;
            }
            Ok::<_, crate::CatalogError>(())
        })
        .expect("vectors publish");

    let removed = catalog
        .with_transaction(|transaction| {
            remove_semantic_documents_outside_contract(
                transaction,
                "infer-runtime.consumer-core@20260813.1",
            )
        })
        .expect("obsolete vectors remove");
    assert_eq!(removed, 1);
    let hits = catalog
        .with_transaction(|transaction| {
            search_semantic_documents(transaction, &[1.0, 0.0, 0.0], "space-a", 10)
        })
        .expect("current vectors remain searchable");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].asset_id, assets[0].id.to_string());

    let _ = std::fs::remove_dir_all(root);
}
