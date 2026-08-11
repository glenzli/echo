use std::path::Path;

use echo_catalog::{
    AppendAnalysisRecord, AssetRegistrationInput, JobKind, RegisterAsset, job_by_id, open_catalog,
    record_analysis, register_asset, semantic_source,
};
use echo_domain::{AnalysisKind, AnalysisRecord, ContentHash, ModelIdentity};

use super::*;

#[test]
fn evidence_revision_enqueues_one_idempotent_embedding_job() {
    let root = std::env::temp_dir().join(format!(
        "echo-semantic-queue-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("t")
    ));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| {
            let asset_id = register(transaction, &root.join("voice.wav"));
            record_fixture_evidence(transaction, asset_id, 1)?;
            Ok::<_, echo_catalog::CatalogError>(asset_id)
        })
        .expect("fixture records");

    assert_eq!(
        enqueue_missing_documents(&catalog, 10).expect("semantic backfill queues"),
        1
    );
    assert_eq!(
        enqueue_missing_documents(&catalog, 20).expect("semantic backfill repeats"),
        1,
        "missing evidence remains eligible while stable job identity deduplicates"
    );
    let first_source = catalog
        .with_transaction(|transaction| semantic_source(transaction, asset_id))
        .expect("source reads")
        .expect("source exists");
    let literal_hits = catalog
        .with_transaction(|transaction| {
            echo_catalog::search_semantic_text(transaction, "station", 10)
        })
        .expect("literal source index searches before Runtime vector publication");
    assert_eq!(literal_hits.len(), 1);
    let first_job = document_job_id(&first_source);
    assert!(first_job.starts_with("embed-text-v2-"));
    let job = catalog
        .with_transaction(|transaction| job_by_id(transaction, &first_job))
        .expect("job reads")
        .expect("job exists");
    assert_eq!(job.kind, JobKind::EmbedText);
    assert_eq!(job.payload["source_revision"], first_source.revision);

    catalog
        .with_transaction(|transaction| record_fixture_evidence(transaction, asset_id, 2))
        .expect("new evidence records");
    enqueue_current_document(&catalog, asset_id, 30).expect("new revision queues");
    let second_source = catalog
        .with_transaction(|transaction| semantic_source(transaction, asset_id))
        .expect("source reads")
        .expect("source exists");
    assert_ne!(first_source.revision, second_source.revision);
    assert_ne!(first_job, document_job_id(&second_source));

    drop(catalog);
    let _ = std::fs::remove_dir_all(root);
}

fn register(transaction: &rusqlite::Transaction<'_>, path: &Path) -> AssetId {
    match register_asset(
        transaction,
        &AssetRegistrationInput {
            content_hash: ContentHash::new([71; 32]),
            path,
            size_bytes: 1_024,
            codec: Some("pcm"),
            duration_millis: Some(2_000),
            recorded_at_millis: None,
            imported_at_millis: 1,
        },
    )
    .expect("asset registers")
    {
        RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
    }
}

fn record_fixture_evidence(
    transaction: &rusqlite::Transaction<'_>,
    asset_id: AssetId,
    revision: i64,
) -> Result<(), echo_catalog::CatalogError> {
    if revision == 1 {
        record_analysis(
            transaction,
            &AppendAnalysisRecord {
                asset_id,
                record: AnalysisRecord::new(
                    AnalysisKind::Transcript,
                    serde_json::json!({"text":"quiet rain near a station"}),
                    ModelIdentity::new("asr".into(), "1".into()),
                    None,
                    1,
                ),
            },
        )?;
    }
    record_analysis(
        transaction,
        &AppendAnalysisRecord {
            asset_id,
            record: AnalysisRecord::new(
                AnalysisKind::Contextual,
                serde_json::json!({
                    "sound_caption": format!("Rain at station {revision}"),
                    "summary": "",
                    "keywords": ["rain", "station"],
                    "mood": "calm",
                    "place_hint": "station",
                    "event_type": "rainfall",
                    "people_hints": []
                }),
                ModelIdentity::new("context".into(), revision.to_string()),
                None,
                revision + 1,
            ),
        },
    )
}
