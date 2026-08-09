use std::path::Path;

use echo_catalog::{
    AssetRegistrationInput, JobStats, RegisterAsset, job_stats, open_catalog, record_analysis,
    register_asset,
};
use echo_domain::{AnalysisKind, AnalysisRecord, ContentHash, ModelIdentity};

use super::*;

fn register(transaction: &rusqlite::Transaction<'_>, byte: u8) -> echo_domain::AssetId {
    match register_asset(
        transaction,
        &AssetRegistrationInput {
            content_hash: ContentHash::new([byte; 32]),
            path: Path::new(if byte == 1 {
                "/voices/missing-transcript.wav"
            } else {
                "/voices/already-analyzed.wav"
            }),
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
fn backfill_is_evidence_aware_and_job_idempotent() {
    let root = std::env::temp_dir().join(format!("echo-analysis-queue-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<_, echo_catalog::CatalogError> {
            let _missing = register(transaction, 1);
            let analyzed = register(transaction, 2);
            record_analysis(
                transaction,
                &echo_catalog::AppendAnalysisRecord {
                    asset_id: analyzed,
                    record: AnalysisRecord::new(
                        AnalysisKind::Transcript,
                        serde_json::json!({ "text": "already done" }),
                        ModelIdentity::new("test".into(), "1".into()),
                        None,
                        10,
                    ),
                },
            )
        })
        .expect("fixture writes");

    assert_eq!(
        enqueue_missing_transcriptions(&catalog, 20).expect("backfill queues"),
        1
    );
    assert_eq!(
        enqueue_missing_transcriptions(&catalog, 30).expect("backfill repeats"),
        1
    );
    let JobStats { pending, .. } = catalog.with_transaction(job_stats).expect("stats read");
    assert_eq!(pending, 1, "deterministic job identity prevents duplicates");
    let _ = std::fs::remove_dir_all(root);
}
