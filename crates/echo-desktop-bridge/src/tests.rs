//! Session contract tests: transcripts query against a real catalog.

mod smart_album_contract;

use crate::session::{aligned_segments, open_session};

fn fixture_catalog() -> std::path::PathBuf {
    // Tests create their own catalog; the echo-tts demo catalog lives in the
    // user's temp area and is exercised by the desktop smoke instead.
    std::env::temp_dir().join(format!(
        "echo-desktop-session-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("t")
    ))
}

#[test]
fn transcripts_round_trip_through_a_live_catalog() {
    let root = fixture_catalog();
    let catalog_path = root.join("catalog.sqlite");
    let cache_root = root.join("cache");

    let session = open_session(
        catalog_path.to_str().expect("utf8"),
        cache_root.to_str().expect("utf8"),
    )
    .expect("session opens");

    // Register an asset and record a transcript payload matching the worker
    // schema, then read it back through the same query the desktop uses.
    let payload = echo_core::TranscriptPayload {
        model: "mlx/qwen3-asr".to_owned(),
        language: Some("zh".to_owned()),
        text: "今天天气不错".to_owned(),
        segments: vec![echo_core::TranscriptSegment {
            text: "今天天气不错".to_owned(),
            start: 0.0,
            end: 2.1,
            words: None,
        }],
        runtime: None,
    };
    let asset = session
        .catalog()
        .with_transaction(|transaction| {
            echo_catalog::register_asset(
                transaction,
                &echo_catalog::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([11; 32]),
                    path: &root.join("voice.wav"),
                    size_bytes: 1024,
                    codec: None,
                    duration_millis: Some(2100),
                    recorded_at_millis: None,
                    imported_at_millis: 0,
                },
            )
        })
        .expect("registration");
    let echo_catalog::RegisterAsset::Created(asset) = asset else {
        panic!("fixture must create")
    };
    echo_core::record_transcript(session.catalog(), asset.id, &payload, "fixture-rev")
        .expect("record");

    let wires = session.transcripts(&asset.id.to_string()).expect("query");
    assert_eq!(wires.len(), 1);
    assert_eq!(wires[0].text, "今天天气不错");
    assert_eq!(wires[0].language, "zh");
    assert_eq!(wires[0].segments.len(), 1);
    assert!((wires[0].segments[0].end - 2.1).abs() < f64::EPSILON);

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn alignment_refines_matching_segment_boundaries_without_mutating_text() {
    let transcript = echo_core::TranscriptPayload {
        model: "audio.transcribe".to_owned(),
        language: Some("zh".to_owned()),
        text: "你好 世界".to_owned(),
        segments: vec![
            echo_core::TranscriptSegment {
                text: "你好".to_owned(),
                start: 0.0,
                end: 1.0,
                words: None,
            },
            echo_core::TranscriptSegment {
                text: "世界".to_owned(),
                start: 1.0,
                end: 2.0,
                words: None,
            },
        ],
        runtime: None,
    };
    let alignment: echo_core::AlignmentPayload = serde_json::from_value(serde_json::json!({
        "text": "你好世界",
        "language": "zh",
        "items": [
            { "text": "你", "start": 0.12, "end": 0.31 },
            { "text": "好", "start": 0.32, "end": 0.55 },
            { "text": "世", "start": 0.91, "end": 1.11 },
            { "text": "界", "start": 1.12, "end": 1.44 }
        ],
        "runtime": {
            "contract_version": "0.1.0-candidate.1",
            "job": {
                "id": "align-1", "app_id": "echo", "intent": "audio.align",
                "provider": "mlx-audio-local", "deployment": "aligner",
                "model_profile": "aligner", "model_build": "build",
                "physical_model": "Qwen3-ForcedAligner", "placement": "local",
                "state": "succeeded", "policy": "local-first",
                "priority": "background", "attempts": []
            }
        }
    }))
    .expect("alignment fixture parses");

    let refined = aligned_segments(&transcript, Some(&alignment));

    assert_eq!(refined[0].text, "你好");
    assert!((refined[0].start - 0.12).abs() < f64::EPSILON);
    assert!((refined[0].end - 0.55).abs() < f64::EPSILON);
    assert!((refined[1].start - 0.91).abs() < f64::EPSILON);
    assert!((refined[1].end - 1.44).abs() < f64::EPSILON);
}

#[test]
fn contextual_stage_and_keyword_facets_project_through_the_live_session() {
    let root = fixture_catalog();
    let catalog_path = root.join("catalog.sqlite");
    let cache_root = root.join("cache");
    let session = open_session(
        catalog_path.to_str().expect("utf8"),
        cache_root.to_str().expect("utf8"),
    )
    .expect("session opens");
    let asset = session
        .catalog()
        .with_transaction(|transaction| {
            echo_catalog::register_asset(
                transaction,
                &echo_catalog::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([21; 32]),
                    path: &root.join("context.wav"),
                    size_bytes: 1024,
                    codec: None,
                    duration_millis: Some(2100),
                    recorded_at_millis: None,
                    imported_at_millis: 0,
                },
            )
        })
        .expect("registration");
    let echo_catalog::RegisterAsset::Created(asset) = asset else {
        panic!("fixture must create")
    };
    let transcript = echo_core::TranscriptPayload {
        model: "audio.transcribe".to_owned(),
        language: Some("en".to_owned()),
        text: "rain on the train platform".to_owned(),
        segments: vec![],
        runtime: None,
    };
    echo_core::record_transcript(session.catalog(), asset.id, &transcript, "fixture")
        .expect("transcript records");
    session
        .catalog()
        .with_transaction(|transaction| {
            echo_catalog::record_analysis(
                transaction,
                &echo_catalog::AppendAnalysisRecord {
                    asset_id: asset.id,
                    record: echo_domain::AnalysisRecord::new(
                        echo_domain::AnalysisKind::Alignment,
                        serde_json::json!({"text":"rain on the train platform","items":[]}),
                        echo_domain::ModelIdentity::new("aligner".into(), "1".into()),
                        None,
                        1,
                    ),
                },
            )
        })
        .expect("alignment records");
    let pending = session
        .analysis_status(&asset.id.to_string())
        .expect("status reads");
    assert_eq!(pending.stage, "contextual");
    assert_eq!(pending.state, "missing");

    let keywords = vec!["Rain".to_owned(), "Train platform".to_owned()];
    session
        .catalog()
        .with_transaction(|transaction| {
            echo_catalog::record_contextual_analysis(
                transaction,
                &echo_catalog::AppendContextualAnalysis {
                    analysis: echo_catalog::AppendAnalysisRecord {
                        asset_id: asset.id,
                        record: echo_domain::AnalysisRecord::new(
                            echo_domain::AnalysisKind::Contextual,
                            serde_json::json!({
                                "summary":"Rain at a station",
                                "keywords":keywords,
                                "mood":null,
                                "place_hint":null,
                                "event_type":null,
                                "people_hints":[]
                            }),
                            echo_domain::ModelIdentity::new("qwen".into(), "build".into()),
                            None,
                            2,
                        ),
                    },
                },
            )
        })
        .expect("contextual evidence records");
    let complete = session
        .analysis_status(&asset.id.to_string())
        .expect("complete status reads");
    assert_eq!(complete.stage, "complete");
    assert_eq!(complete.state, "done");
    let facets = session.keyword_facets().expect("facets project");
    assert_eq!(facets.len(), 2);
    assert_eq!(facets[0].count, 1);
    assert!(facets.iter().any(|facet| facet.key == "rain"));
    let _ = std::fs::remove_dir_all(root);
}
