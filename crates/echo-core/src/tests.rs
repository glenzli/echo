//! Facade contracts for analysis: worker output parsing and evidence
//! recording shape.

use crate::{TranscribeWorker, TranscriptPayload, record_transcript, run_transcribe};
use echo_catalog::{open_catalog, query_analysis};
use echo_domain::AnalysisKind;

const FIXTURE_OUTPUT: &str = r#"{
  "model": "mlx/qwen3-asr",
  "language": "zh",
  "text": "今天天气不错",
  "segments": [
    {"text": "今天天气不错", "start": 0.0, "end": 2.1},
    {"text": "我们一起去坐小火车吧", "start": 2.2, "end": 4.8, "words": [
      {"text": "我们", "start": 2.2, "end": 2.6}
    ]}
  ]
}"#;

#[test]
fn transcript_payload_parses_canonical_schema() {
    let payload: TranscriptPayload = serde_json::from_str(FIXTURE_OUTPUT).expect("fixture parses");
    assert_eq!(payload.language.as_deref(), Some("zh"));
    assert_eq!(payload.segments.len(), 2);
    assert!((payload.segments[0].start - 0.0).abs() < f64::EPSILON);
    assert!((payload.segments[1].end - 4.8).abs() < f64::EPSILON);
    let words = payload.segments[1].words.as_ref().expect("words present");
    assert_eq!(words[0].text, "我们");
}

#[test]
fn worker_rejects_missing_python() {
    let worker = TranscribeWorker {
        python: "/nonexistent/python".into(),
        script: "/nonexistent/transcribe.py".into(),
    };
    let result = run_transcribe(
        std::path::Path::new("/tmp/voice.wav"),
        std::path::Path::new("/tmp/model"),
        &worker,
    );
    assert!(result.is_err(), "missing interpreter must fail cleanly");
}

#[test]
fn recording_transcript_creates_evidence_and_lifts_level() {
    let root = std::env::temp_dir().join(format!(
        "echo-core-analysis-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("t")
    ));
    let catalog_path = root.join("catalog.sqlite");
    let catalog = open_catalog(&catalog_path).expect("catalog opens");
    let payload: TranscriptPayload = serde_json::from_str(FIXTURE_OUTPUT).expect("fixture parses");

    catalog
        .with_transaction(|transaction| {
            echo_catalog::register_asset(
                transaction,
                &echo_catalog::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([3; 32]),
                    path: &root.join("voice.wav"),
                    size_bytes: 1024,
                    codec: None,
                    duration_millis: Some(4800),
                    recorded_at_millis: None,
                    imported_at_millis: 0,
                },
            )
        })
        .expect("registration");
    let asset = catalog
        .with_transaction(|transaction| {
            echo_catalog::find_by_content_hash(transaction, echo_domain::ContentHash::new([3; 32]))
        })
        .expect("lookup");
    let echo_catalog::AssetLookup::Found(asset) = asset else {
        panic!("asset must exist")
    };
    record_transcript(&catalog, asset.id, &payload, "fixture-rev").expect("record");

    let records = catalog
        .with_transaction(|transaction| query_analysis(transaction, asset.id))
        .expect("query");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].kind, AnalysisKind::Transcript);
    assert_eq!(records[0].model.name, "mlx/qwen3-asr");
    assert_eq!(records[0].model.version, "fixture-rev");
    let _ = std::fs::remove_dir_all(root);
}
