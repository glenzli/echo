//! Metadata calibration crosses the live desktop session without rewriting
//! immutable model evidence.

use super::{fixture_catalog, open_session, record_current_contextual_fixture};

fn calibration_fixture() -> (
    std::path::PathBuf,
    crate::session::LibrarySession,
    echo_domain::AssetId,
) {
    let root = fixture_catalog();
    let session = open_session(
        root.join("catalog.sqlite").to_str().expect("utf8"),
        root.join("cache").to_str().expect("utf8"),
    )
    .expect("session opens");
    let asset = session
        .catalog()
        .with_transaction(|transaction| {
            echo_catalog::register_asset(
                transaction,
                &echo_catalog::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([23; 32]),
                    path: &root.join("calibration.wav"),
                    size_bytes: 1024,
                    codec: Some("pcm"),
                    duration_millis: Some(2_100),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )
        })
        .expect("asset registers");
    let echo_catalog::RegisterAsset::Created(asset) = asset else {
        panic!("fixture must create")
    };
    echo_core::record_transcript(
        session.catalog(),
        asset.id,
        &echo_core::TranscriptPayload {
            model: "audio.transcribe".into(),
            language: Some("en".into()),
            text: "rain on the train platform".into(),
            segments: Vec::new(),
            runtime: None,
        },
        "fixture",
    )
    .expect("transcript records");
    record_current_contextual_fixture(session.catalog(), asset.id);
    (root, session, asset.id)
}

#[test]
fn metadata_calibration_round_trips_without_rewriting_model_evidence() {
    let (root, session, asset_id) = calibration_fixture();
    let revision = session
        .calibrate_asset_metadata(
            &asset_id.to_string(),
            "Morning rain by the kitchen",
            "",
            "daily routine",
            "quiet",
            r#"["rain","kitchen"]"#,
            "Rain taps the window while water boils.",
            "en",
            r#"["sound_caption","summary","event_type","mood","keywords","transcript_text","language"]"#,
        )
        .expect("calibration saves");
    assert!(revision > 0);
    let projected = session.list_assets().expect("assets project");
    assert_eq!(projected[0].sound_caption, "Morning rain by the kitchen");
    assert_eq!(
        projected[0].model_sound_caption,
        "Rain across a station platform"
    );
    assert_eq!(projected[0].keywords, ["rain", "kitchen"]);
    assert_eq!(projected[0].model_keywords, ["Rain", "Train platform"]);
    assert_eq!(
        projected[0].text_preview,
        "Rain taps the window while water boils."
    );
    assert_eq!(
        projected[0].model_text_preview,
        "rain on the train platform"
    );
    assert!(
        projected[0]
            .calibrated_fields
            .iter()
            .any(|field| field == "transcript_text")
    );
    assert!(
        projected[0]
            .calibrated_fields
            .iter()
            .any(|field| field == "language")
    );
    assert_eq!(
        session
            .transcripts(&asset_id.to_string())
            .expect("model evidence reads")[0]
            .text,
        "rain on the train platform"
    );

    session
        .calibrate_asset_metadata(
            &asset_id.to_string(),
            "Rain across a station platform",
            "",
            "rainfall",
            "calm",
            r#"["Rain","Train platform"]"#,
            "rain on the train platform",
            "en",
            "[]",
        )
        .expect("model reset saves");
    let reset = session.list_assets().expect("reset projects");
    assert!(reset[0].calibrated_fields.is_empty());
    assert_eq!(reset[0].sound_caption, reset[0].model_sound_caption);
    let _ = std::fs::remove_dir_all(root);
}
