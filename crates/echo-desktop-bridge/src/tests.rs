//! Session contract tests: transcripts query against a real catalog.

use crate::session::{open_session, transcribe_asset};

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

    // Missing models must fail with the download hint, not crash.
    let error = transcribe_asset(
        catalog_path.to_str().expect("utf8"),
        &asset.id.to_string(),
        &root.join("empty-model-root").to_string_lossy(),
        "python3",
        "tools/asr/transcribe.py",
    )
    .expect_err("missing model must fail");
    assert!(
        error.message.contains("hf download"),
        "error must hint the download: {}",
        error.message
    );
    let _ = std::fs::remove_dir_all(root);
}
