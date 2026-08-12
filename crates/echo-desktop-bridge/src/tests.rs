//! Session contract tests: transcripts query against a real catalog.

mod listening_state_contract;
mod long_audio_contract;
mod processing_recipe_contract;
mod revisit_contract;
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

fn pcm16_mono_wav(sample_rate: u32, samples: &[i16]) -> Vec<u8> {
    let data_bytes = u32::try_from(samples.len() * 2).expect("bounded fixture");
    let mut wav = Vec::with_capacity(44 + samples.len() * 2);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
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
    let assets = session.list_assets().expect("asset summaries project");
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].language, "zh");

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn metadata_calibration_round_trips_without_rewriting_model_evidence() {
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

    let revision = session
        .calibrate_asset_metadata(
            &asset.id.to_string(),
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
            .transcripts(&asset.id.to_string())
            .expect("model evidence reads")[0]
            .text,
        "rain on the train platform"
    );

    session
        .calibrate_asset_metadata(
            &asset.id.to_string(),
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

#[test]
#[allow(clippy::too_many_lines)] // One full cross-language adjustment contract fixture.
fn adjustment_revision_round_trips_through_the_live_session() {
    let root = fixture_catalog();
    let session = open_session(
        root.join("catalog.sqlite").to_str().expect("utf8"),
        root.join("cache").to_str().expect("utf8"),
    )
    .expect("session opens");
    let impulse_path = root.join("small-room-ir.wav");
    let mut impulse_samples = vec![0_i16; 2_400];
    impulse_samples[0] = 28_000;
    impulse_samples[317] = -10_000;
    std::fs::write(&impulse_path, pcm16_mono_wav(24_000, &impulse_samples))
        .expect("impulse fixture writes");
    let impulse = session
        .import_impulse_response(
            impulse_path.to_str().expect("utf8"),
            "Small room",
            "Echo fixture",
            "",
            "Recorded for the bridge contract",
            "user_owned_no_redistribution",
            "",
            "",
        )
        .expect("impulse response imports");
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
    let mut creative_vfx = echo_domain::CreativeVfxSettings::default();
    creative_vfx.scene.enabled = true;
    creative_vfx.scene.character = echo_domain::SceneVfxCharacter::Underwater;
    creative_vfx.scene.intensity_percent = 72;
    creative_vfx.delay.enabled = true;
    creative_vfx.delay.character = echo_domain::DelayVfxCharacter::Echo;
    creative_vfx.delay.echo.feedback_percent = 42;
    creative_vfx.modulation.enabled = true;
    creative_vfx.modulation.character = echo_domain::ModulationVfxCharacter::Phaser;
    creative_vfx.modulation.phaser.feedback_percent = -18;
    creative_vfx.transform.enabled = true;
    creative_vfx.transform.character = echo_domain::TransformVfxCharacter::Ghost;
    creative_vfx.transform.amount_percent = 68;
    creative_vfx.digital_degrade.enabled = true;
    creative_vfx.digital_degrade.character = echo_domain::DigitalDegradeVfxCharacter::LoFi;
    creative_vfx.digital_degrade.bitcrusher.bit_depth = 7;
    creative_vfx.drive.enabled = true;
    creative_vfx.drive.character = echo_domain::DriveVfxCharacter::Fuzz;
    creative_vfx.drive.drive_centibels = 2_400;
    creative_vfx.rotary.enabled = true;
    creative_vfx.rotary.speed = echo_domain::RotaryVfxSpeed::Fast;
    creative_vfx.rotary.motion_percent = 74;
    let mut adjustment = crate::ffi::AssetAdjustmentWire {
        trim_start_millis: 1_000,
        trim_end_millis: 9_000,
        fade_in_millis: 250,
        fade_out_millis: 500,
        fade_in_curve: 1,
        fade_out_curve: 2,
        gain_centibels: -350,
        low_cut_hertz: 80,
        restoration_enabled: true,
        de_plosive_enabled: true,
        de_plosive_frequency_hertz: 150,
        de_plosive_sensitivity_percent: 67,
        de_plosive_reduction_centibels: 1_350,
        de_plosive_release_millis: 190,
        noise_reduction_enabled: true,
        noise_reduction_centibels: 1_200,
        noise_reduction_sensitivity_percent: 62,
        noise_reduction_smoothing_millis: 320,
        de_esser_enabled: true,
        de_esser_frequency_hertz: 7_200,
        de_esser_threshold_centibels: -2_800,
        de_esser_reduction_centibels: 750,
        de_hum_enabled: true,
        de_hum_fundamental_hertz: 60,
        de_hum_harmonic_count: 6,
        de_hum_quality_tenths: 420,
        de_hum_depth_centibels: 1_900,
        de_click_enabled: true,
        de_click_sensitivity_percent: 68,
        de_click_maximum_click_microseconds: 720,
        de_click_repair_percent: 86,
        channel_repair_enabled: true,
        channel_repair_invert_left: true,
        channel_repair_invert_right: false,
        channel_repair_swap_channels: true,
        channel_repair_mono_fold_down: false,
        channel_repair_balance_percent: -24,
        equalizer_enabled: false,
        equalizer_bands: test_equalizer_bands(),
        compressor_enabled: true,
        compressor_threshold_centibels: -2_000,
        compressor_ratio_tenths: 40,
        compressor_attack_millis: 12,
        compressor_release_millis: 160,
        compressor_makeup_centibels: 225,
        reverb_character: echo_domain::ReverbCharacter::Spring.wire_value(),
        reverb_enabled: true,
        reverb_mix_percent: 24,
        reverb_pre_delay_millis: 28,
        reverb_decay_millis: 2_400,
        reverb_size_percent: 68,
        reverb_damping_percent: 52,
        reverb_low_cut_hertz: 150,
        reverb_high_cut_hertz: 9_000,
        space_mode: echo_domain::SpaceMode::Convolution.wire_value(),
        impulse_response_import_id: impulse.import_id.clone(),
        impulse_response_source_hash: impulse.source_hash.clone(),
        impulse_response_prepared_hash: impulse.prepared_hash.clone(),
        convolution_mix_percent: 46,
        convolution_wet_gain_centibels: -175,
        creative_vfx_json: serde_json::to_string(&creative_vfx).expect("creative VFX encodes"),
        limiter_enabled: true,
        limiter_ceiling_centibels: -125,
        limiter_release_millis: 160,
        effect_chain: vec![5, 7, 3, 0, 6, 1, 8, 9, 10, 11, 12, 13, 14, 2, 4],
        edit_segments: vec![
            crate::ffi::EditSegmentWire {
                source_start_millis: 1_000,
                source_end_millis: 4_000,
                state: 0,
                gain_centibels: 125,
                fade_in_millis: 40,
                fade_out_millis: 60,
                fade_in_curve: 1,
                fade_out_curve: 2,
                gap_after_millis: 200,
            },
            crate::ffi::EditSegmentWire {
                source_start_millis: 4_000,
                source_end_millis: 9_000,
                state: 1,
                gain_centibels: 0,
                fade_in_millis: 0,
                fade_out_millis: 0,
                fade_in_curve: 0,
                fade_out_curve: 0,
                gap_after_millis: 0,
            },
        ],
        effect_masks: vec![crate::ffi::EffectMaskWire {
            start_millis: 2_000,
            end_millis: 3_000,
            feather_millis: 12,
            effect_nodes: vec![1, 2],
        }],
    };
    session
        .set_asset_adjustment(&asset.id.to_string(), &adjustment)
        .expect("adjustment saves");
    let stored = session
        .catalog()
        .with_transaction(|transaction| {
            echo_catalog::latest_adjustment_graph(transaction, asset.id)
        })
        .expect("stored adjustment reads")
        .expect("stored adjustment exists");
    assert!(stored.graph.de_hum().enabled);
    assert!(stored.graph.de_click().enabled);
    assert!(stored.graph.channel_repair().enabled);
    assert_eq!(
        stored.graph.reverb().character,
        echo_domain::ReverbCharacter::Spring
    );
    assert_eq!(stored.graph.creative_vfx(), creative_vfx);
    assert_eq!(
        stored.graph.space().mode,
        echo_domain::SpaceMode::Convolution
    );
    assert_eq!(
        stored
            .graph
            .space()
            .impulse_response
            .expect("IR selection remains")
            .import_id
            .to_string(),
        impulse.import_id
    );
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
    assert!(projected[0].restoration_enabled);
    assert!(stored.graph.restoration().de_plosive.enabled);
    assert!(projected[0].de_plosive_enabled);
    assert_eq!(projected[0].de_plosive_frequency_hertz, 150);
    assert_eq!(projected[0].de_plosive_sensitivity_percent, 67);
    assert_eq!(projected[0].de_plosive_reduction_centibels, 1_350);
    assert_eq!(projected[0].de_plosive_release_millis, 190);
    assert!(projected[0].noise_reduction_enabled);
    assert_eq!(projected[0].noise_reduction_centibels, 1_200);
    assert_eq!(projected[0].noise_reduction_sensitivity_percent, 62);
    assert_eq!(projected[0].noise_reduction_smoothing_millis, 320);
    assert!(projected[0].de_esser_enabled);
    assert_eq!(projected[0].de_esser_frequency_hertz, 7_200);
    assert_eq!(projected[0].de_esser_threshold_centibels, -2_800);
    assert_eq!(projected[0].de_esser_reduction_centibels, 750);
    assert!(projected[0].de_hum_enabled);
    assert_eq!(projected[0].de_hum_fundamental_hertz, 60);
    assert_eq!(projected[0].de_hum_harmonic_count, 6);
    assert_eq!(projected[0].de_hum_quality_tenths, 420);
    assert_eq!(projected[0].de_hum_depth_centibels, 1_900);
    assert!(projected[0].de_click_enabled);
    assert_eq!(projected[0].de_click_sensitivity_percent, 68);
    assert_eq!(projected[0].de_click_maximum_click_microseconds, 720);
    assert_eq!(projected[0].de_click_repair_percent, 86);
    assert!(projected[0].channel_repair_enabled);
    assert!(projected[0].channel_repair_invert_left);
    assert!(!projected[0].channel_repair_invert_right);
    assert!(projected[0].channel_repair_swap_channels);
    assert!(!projected[0].channel_repair_mono_fold_down);
    assert_eq!(projected[0].channel_repair_balance_percent, -24);
    assert!(!projected[0].equalizer_enabled);
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
    assert_eq!(
        projected[0].reverb_character,
        echo_domain::ReverbCharacter::Spring.wire_value()
    );
    assert!(projected[0].reverb_enabled);
    assert_eq!(projected[0].reverb_mix_percent, 24);
    assert_eq!(projected[0].reverb_pre_delay_millis, 28);
    assert_eq!(projected[0].reverb_decay_millis, 2_400);
    assert_eq!(projected[0].reverb_size_percent, 68);
    assert_eq!(projected[0].reverb_damping_percent, 52);
    assert_eq!(projected[0].reverb_low_cut_hertz, 150);
    assert_eq!(projected[0].reverb_high_cut_hertz, 9_000);
    assert_eq!(
        projected[0].space_mode,
        echo_domain::SpaceMode::Convolution.wire_value()
    );
    assert_eq!(projected[0].impulse_response_import_id, impulse.import_id);
    assert_eq!(
        projected[0].impulse_response_source_hash,
        impulse.source_hash
    );
    assert_eq!(
        projected[0].impulse_response_prepared_hash,
        impulse.prepared_hash
    );
    assert_eq!(
        projected[0].impulse_response_prepared_path,
        impulse.prepared_path
    );
    assert_eq!(projected[0].convolution_mix_percent, 46);
    assert_eq!(projected[0].convolution_wet_gain_centibels, -175);
    assert_eq!(
        serde_json::from_str::<echo_domain::CreativeVfxSettings>(&projected[0].creative_vfx_json)
            .expect("projected creative VFX decodes"),
        creative_vfx
    );
    assert!(projected[0].limiter_enabled);
    assert_eq!(projected[0].limiter_ceiling_centibels, -125);
    assert_eq!(projected[0].limiter_release_millis, 160);
    assert_eq!(
        projected[0].effect_chain,
        [5, 7, 3, 0, 6, 1, 8, 9, 10, 11, 12, 13, 14, 2, 4]
    );
    assert_eq!(projected[0].edit_segments.len(), 2);
    assert_eq!(projected[0].edit_segments[0].source_start_millis, 1_000);
    assert_eq!(projected[0].edit_segments[0].gap_after_millis, 200);
    assert_eq!(projected[0].edit_segments[1].state, 1);
    assert_eq!(projected[0].effect_masks.len(), 1);
    assert_eq!(projected[0].effect_masks[0].effect_nodes, [1, 2]);
    adjustment.reverb_character = u8::MAX;
    let error = session
        .set_asset_adjustment(&asset.id.to_string(), &adjustment)
        .expect_err("unknown reverb character must fail closed");
    assert_eq!(
        error.message,
        "reverb character must be room, hall, plate, or spring"
    );
    adjustment.reverb_character = echo_domain::ReverbCharacter::Room.wire_value();
    creative_vfx.transform.amount_percent = 101;
    adjustment.creative_vfx_json =
        serde_json::to_string(&creative_vfx).expect("invalid creative VFX still encodes");
    let error = session
        .set_asset_adjustment(&asset.id.to_string(), &adjustment)
        .expect_err("invalid creative VFX range must fail closed");
    assert_eq!(
        error.message,
        "creative VFX parameters are outside the supported range"
    );
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
            "contract_version": "0.1.0-candidate.4",
            "job": {
                "id": "align-1", "consumer_contract_version": "0.1.0-candidate.4",
                "app_id": "echo", "intent": "audio.align",
                "provider": "mlx-audio-local", "deployment": "aligner",
                "model_profile": "aligner", "model_build": "build",
                "physical_model": "Qwen3-ForcedAligner", "placement": "local",
                "capability_level": "foundational", "evaluation_status": "provisional",
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
    assert_eq!(assets[0].language, "en");
    let facets = session.keyword_facets().expect("facets project");
    assert_eq!(facets.len(), 2);
    assert_eq!(facets[0].count, 1);
    assert!(facets.iter().any(|facet| facet.key == "rain"));
    let _ = std::fs::remove_dir_all(root);
}
