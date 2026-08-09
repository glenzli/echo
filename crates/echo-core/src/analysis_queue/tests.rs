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
            let empty = register(transaction, 3);
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
            )?;
            record_analysis(
                transaction,
                &echo_catalog::AppendAnalysisRecord {
                    asset_id: empty,
                    record: AnalysisRecord::new(
                        AnalysisKind::Transcript,
                        serde_json::json!({ "text": "" }),
                        ModelIdentity::new("test".into(), "1".into()),
                        None,
                        11,
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

    assert_eq!(
        enqueue_missing_alignments(&catalog, 40).expect("alignment backfill queues"),
        1
    );
    let JobStats { pending, .. } = catalog.with_transaction(job_stats).expect("stats read");
    assert_eq!(pending, 2, "only transcript evidence admits alignment");

    let empty = catalog
        .with_transaction(|transaction| {
            echo_catalog::find_by_content_hash(transaction, ContentHash::new([3; 32]))
        })
        .expect("empty transcript asset reads");
    let echo_catalog::AssetLookup::Found(empty) = empty else {
        panic!("empty transcript asset exists")
    };
    catalog
        .with_transaction(|transaction| enqueue_alignment(transaction, empty.id, 50))
        .expect("legacy empty alignment queues");
    assert_eq!(
        settle_empty_transcript_alignments(&catalog, 60).expect("empty alignment settles"),
        1
    );
    let job = catalog
        .with_transaction(|transaction| job_by_id(transaction, &alignment_job_id(empty.id)))
        .expect("job reads")
        .expect("job exists");
    assert_eq!(job.state, echo_catalog::JobState::Done);
    let _ = std::fs::remove_dir_all(root);
}
