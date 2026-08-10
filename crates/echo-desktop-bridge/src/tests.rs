//! Session contract tests: transcripts query against a real catalog.

mod smart_album_contract;
mod user_album_contract;

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

fn test_equalizer_bands() -> Vec<crate::ffi::EqualizerBandWire> {
    let equalizer = echo_domain::ParametricEqualizer::from_legacy_gains(250, -175, 300);
    equalizer
        .bands()
        .into_iter()
        .map(|band| crate::ffi::EqualizerBandWire {
            enabled: band.enabled,
            filter_kind: u8::try_from(band.filter_kind.catalog_value())
                .expect("filter values fit u8"),
            frequency_hertz: band.frequency_hertz,
            q_hundredths: band.q_hundredths,
            gain_centibels: band.gain_centibels,
        })
        .collect()
}

fn record_current_contextual_fixture(
    catalog: &echo_catalog::Catalog,
    asset_id: echo_domain::AssetId,
) {
    let keywords = vec!["Rain".to_owned(), "Train platform".to_owned()];
    catalog
        .with_transaction(|transaction| -> Result<(), echo_catalog::CatalogError> {
            echo_catalog::record_contextual_analysis(
                transaction,
                &echo_catalog::AppendContextualAnalysis {
                    analysis: echo_catalog::AppendAnalysisRecord {
                        asset_id,
                        record: echo_domain::AnalysisRecord::new(
                            echo_domain::AnalysisKind::Contextual,
                            serde_json::json!({
                                "schema_version":3,
                                "sound_caption":"Rain across a station platform",
                                "summary":"",
                                "keywords":keywords,
                                "mood":"calm",
                                "place_hint":null,
                                "event_type":"rainfall",
                                "people_hints":[]
                            }),
                            echo_domain::ModelIdentity::new("qwen".into(), "build".into()),
                            None,
                            2,
                        ),
                    },
                },
            )?;
            echo_catalog::record_contextual_analysis(
                transaction,
                &echo_catalog::AppendContextualAnalysis {
                    analysis: echo_catalog::AppendAnalysisRecord {
                        asset_id,
                        record: echo_domain::AnalysisRecord::new(
                            echo_domain::AnalysisKind::Contextual,
                            serde_json::json!({
                                "schema_version":3,
                                "sound_caption":"Rain across a station platform",
                                "summary":"",
                                "keywords":[],
                                "mood":null,
                                "place_hint":null,
                                "event_type":null,
                                "people_hints":[]
                            }),
                            echo_domain::ModelIdentity::new("qwen".into(), "new-build".into()),
                            None,
                            3,
                        ),
                    },
                },
            )
        })
        .expect("contextual evidence records");
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
fn adjustment_revision_round_trips_through_the_live_session() {
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
                    content_hash: echo_domain::ContentHash::new([41; 32]),
                    path: &root.join("adjust.wav"),
                    size_bytes: 1024,
                    codec: Some("pcm"),
                    duration_millis: Some(10_000),
                    recorded_at_millis: None,
                    imported_at_millis: 0,
                },
            )
        })
        .expect("registration");
    let echo_catalog::RegisterAsset::Created(asset) = asset else {
        panic!("fixture must create")
    };

    let adjustment = crate::ffi::AssetAdjustmentWire {
        trim_start_millis: 1_000,
        trim_end_millis: 9_000,
        fade_in_millis: 250,
        fade_out_millis: 500,
        fade_in_curve: 1,
        fade_out_curve: 2,
        gain_centibels: -350,
        low_cut_hertz: 80,
        equalizer_bands: test_equalizer_bands(),
        compressor_enabled: true,
        compressor_threshold_centibels: -2_000,
        compressor_ratio_tenths: 40,
        compressor_attack_millis: 12,
        compressor_release_millis: 160,
        compressor_makeup_centibels: 225,
        reverb_enabled: true,
        reverb_mix_percent: 24,
        reverb_pre_delay_millis: 28,
        reverb_decay_millis: 2_400,
        reverb_size_percent: 68,
        reverb_damping_percent: 52,
        reverb_low_cut_hertz: 150,
        reverb_high_cut_hertz: 9_000,
        limiter_enabled: true,
        limiter_ceiling_centibels: -125,
        limiter_release_millis: 160,
    };
    session
        .set_asset_adjustment(&asset.id.to_string(), &adjustment)
        .expect("adjustment saves");
    let projected = session.list_assets().expect("assets project");
    assert_eq!(projected.len(), 1);
    assert!(projected[0].adjustment_revision > 0);
    assert_eq!(projected[0].trim_start_millis, 1_000);
    assert_eq!(projected[0].trim_end_millis, 9_000);
    assert_eq!(projected[0].fade_in_millis, 250);
    assert_eq!(projected[0].fade_out_millis, 500);
    assert_eq!(projected[0].fade_in_curve, 1);
    assert_eq!(projected[0].fade_out_curve, 2);
    assert_eq!(projected[0].gain_centibels, -350);
    assert_eq!(projected[0].low_cut_hertz, 80);
    assert_eq!(projected[0].equalizer_bands.len(), 6);
    assert_eq!(projected[0].equalizer_bands[0].gain_centibels, 250);
    assert_eq!(projected[0].equalizer_bands[2].gain_centibels, -175);
    assert_eq!(projected[0].equalizer_bands[5].gain_centibels, 300);
    assert!(projected[0].compressor_enabled);
    assert_eq!(projected[0].compressor_threshold_centibels, -2_000);
    assert_eq!(projected[0].compressor_ratio_tenths, 40);
    assert_eq!(projected[0].compressor_attack_millis, 12);
    assert_eq!(projected[0].compressor_release_millis, 160);
    assert_eq!(projected[0].compressor_makeup_centibels, 225);
    assert!(projected[0].reverb_enabled);
    assert_eq!(projected[0].reverb_mix_percent, 24);
    assert_eq!(projected[0].reverb_pre_delay_millis, 28);
    assert_eq!(projected[0].reverb_decay_millis, 2_400);
    assert_eq!(projected[0].reverb_size_percent, 68);
    assert_eq!(projected[0].reverb_damping_percent, 52);
    assert_eq!(projected[0].reverb_low_cut_hertz, 150);
    assert_eq!(projected[0].reverb_high_cut_hertz, 9_000);
    assert!(projected[0].limiter_enabled);
    assert_eq!(projected[0].limiter_ceiling_centibels, -125);
    assert_eq!(projected[0].limiter_release_millis, 160);
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
            "contract_version": "0.1.0-candidate.2",
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

    record_current_contextual_fixture(session.catalog(), asset.id);
    let complete = session
        .analysis_status(&asset.id.to_string())
        .expect("complete status reads");
    assert_eq!(complete.stage, "complete");
    assert_eq!(complete.state, "done");
    let assets = session.list_assets().expect("assets project");
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].sound_caption, "Rain across a station platform");
    assert!(assets[0].summary.is_empty());
    assert_eq!(assets[0].keywords, ["Rain", "Train platform"]);
    assert_eq!(assets[0].mood, "calm");
    assert_eq!(assets[0].event_type, "rainfall");
    let facets = session.keyword_facets().expect("facets project");
    assert_eq!(facets.len(), 2);
    assert_eq!(facets[0].count, 1);
    assert!(facets.iter().any(|facet| facet.key == "rain"));
    let _ = std::fs::remove_dir_all(root);
}
