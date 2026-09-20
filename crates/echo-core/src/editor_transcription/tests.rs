use super::*;
use crate::{TranscriptSegment, TranscriptWord};

fn evidence() -> SelectionTranscript {
    SelectionTranscript { schema_version: 1, alignment: None, source_hash: ContentHash::new([1; 32]), start_millis: 1000, end_millis: 3000,
        transcript: TranscriptPayload { model: "audio.transcribe".into(), language: Some("en".into()), text: "one sentence".into(),
            segments: vec![TranscriptSegment { text: "one sentence".into(), start: 1.2, end: 2.8,
                words: Some(vec![TranscriptWord {text: "one".into(),start:1.2,end:1.8}]) }],
            runtime: Some(serde_json::from_value(serde_json::json!({"contract_version":"test", "job":{
                "id":"test-job","consumer_core_contract":"test","app_id":"echo","intent":"audio.transcribe",
                "provider":"test","deployment":"test","model_profile":"test","model_build":"immutable-build",
                "physical_model":"test-asr","placement":"local","state":"succeeded","policy":"local-first","priority":"background"
            }})).expect("provenance")) } }
}
#[test]
fn timing_and_provenance_are_required_before_editing() {
    let original = evidence();
    assert!(original.validate().is_ok());
    let mut value = original.clone();
    value.transcript.segments[0].end = 3.01;
    assert!(value.validate().is_err());
    value = original.clone();
    value.transcript.segments[0].start = f64::NAN;
    assert!(value.validate().is_err());
    value = original.clone();
    value.transcript.runtime = None;
    assert!(value.validate().is_err());
    value = original;
    value.end_millis = 301_001;
    assert!(value.validate().is_err());
}
#[test]
fn selection_evidence_does_not_complete_whole_source_asr_or_enqueue_jobs() {
    let root = std::env::temp_dir().join(format!("echo-selection-{}", AssetId::new()));
    let catalog = echo_catalog::open_catalog(&root.join("catalog.sqlite")).expect("catalog");
    let id = catalog
        .with_transaction(|tx| -> Result<_, echo_catalog::CatalogError> {
            let result = echo_catalog::register_asset(
                tx,
                &echo_catalog::AssetRegistrationInput {
                    content_hash: ContentHash::new([1; 32]),
                    path: Path::new("source.wav"),
                    size_bytes: 100,
                    codec: Some("wav"),
                    duration_millis: Some(10_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )?;
            Ok(match result {
                echo_catalog::RegisterAsset::Created(a)
                | echo_catalog::RegisterAsset::Existed(a) => a.id,
            })
        })
        .expect("asset");
    record_selection_transcript(&catalog, id, &evidence()).expect("store");
    catalog
        .with_transaction(|tx| -> Result<(), echo_catalog::CatalogError> {
            let missing = echo_catalog::list_assets_missing_analysis(tx, AnalysisKind::Transcript)?;
            assert!(missing.contains(&id));
            let jobs: i64 = tx.query_row("SELECT count(*) FROM jobs", [], |row| row.get(0))?;
            assert_eq!(jobs, 0);
            Ok(())
        })
        .expect("scope");
    let mut wrong = evidence();
    wrong.source_hash = ContentHash::new([2; 32]);
    assert!(record_selection_transcript(&catalog, id, &wrong).is_err());
    drop(catalog);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[ignore = "requires a live authorized Infer Runtime and an explicit fixture path"]
fn live_selection_transcription_retains_original_time_and_model_identity() {
    let input = std::env::var("ECHO_SELECTION_LIVE_SOURCE").expect("fixture source");
    let root = std::env::temp_dir().join(format!("echo-live-selection-{}", AssetId::new()));
    let path = root.join("catalog.sqlite");
    let imported = crate::import_asset(&path, Path::new(&input)).expect("fixture import");
    let asset = match imported {
        crate::ImportOutcome::Imported(a) | crate::ImportOutcome::AlreadyPresent(a) => a,
    };
    let catalog = echo_catalog::open_catalog(&path).expect("catalog");
    let end = asset
        .original
        .duration_millis
        .expect("duration")
        .min(10_000);
    let evidence = transcribe_selection(
        &catalog,
        &root.join("cache"),
        asset.id,
        1000,
        end,
        InferRuntimeConfig {
            base_url: String::new(),
            credential_path: crate::infer_runtime_credential_path()
                .expect("managed Echo credential"),
        },
    )
    .expect("real inference");
    assert!(!evidence.transcript.text.trim().is_empty());
    assert!(!evidence.transcript.segments.is_empty());
    assert!(
        evidence
            .transcript
            .segments
            .iter()
            .all(|s| s.start >= 1.0 && s.end <= std::time::Duration::from_millis(end).as_secs_f64())
    );
    record_selection_transcript(&catalog, asset.id, &evidence).expect("accept");
    if let Ok(output) = std::env::var("ECHO_SELECTION_LIVE_REPORT") {
        std::fs::write(output, serde_json::to_vec_pretty(&evidence).expect("json"))
            .expect("report");
    }
    drop(catalog);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn one_zero_duration_alignment_unit_does_not_invalidate_other_words() {
    let mut value = evidence();
    let mut runtime = value.transcript.runtime.clone().expect("provenance");
    runtime.job.intent = "audio.align".into();
    value.alignment = Some(crate::AlignmentPayload {
        text: value.transcript.text.clone(),
        language: Some("en".into()),
        runtime,
        items: vec![
            crate::AlignmentItem {
                text: "one".into(),
                start: 1.2,
                end: 1.8,
            },
            crate::AlignmentItem {
                text: "sentence".into(),
                start: 1.8,
                end: 1.8,
            },
        ],
    });
    assert!(value.validate().is_ok());
    value.alignment.as_mut().unwrap().items[1].end = 1.7;
    assert!(value.validate().is_err());
}
