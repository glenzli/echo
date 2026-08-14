use std::path::Path;

use echo_catalog::{
    AssetLookup, AssetRegistrationInput, RegisterAsset, find_by_content_hash, list_audio_space,
    list_contextual_keyword_facets, open_catalog, query_analysis, register_asset,
};
use echo_domain::{AnalysisKind, ContentHash};

use super::*;
use crate::DetectedAudioEvent;

#[test]
fn audio_events_preserve_stable_ids_and_drive_existing_browse_projections() {
    let root =
        std::env::temp_dir().join(format!("echo-sound-event-evidence-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, echo_catalog::CatalogError> {
            let registered = register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([77; 32]),
                    path: Path::new("/sounds/bird-and-truck.wav"),
                    size_bytes: 512,
                    codec: Some("pcm"),
                    duration_millis: Some(2_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )?;
            let asset = match registered {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset,
            };
            Ok(asset.id)
        })
        .expect("fixture registers");
    let evidence = AudioEventsEvidence::from_detection(detection(), 0.0);

    assert!(
        record_audio_events(&catalog, asset_id, &evidence).expect("evidence records"),
        "detected events publish a browse presentation"
    );

    let records = catalog
        .with_transaction(|transaction| {
            query_analysis(transaction, asset_id).map_err(|error| {
                echo_catalog::CatalogError::new(
                    echo_catalog::CatalogErrorKind::Other,
                    error.to_string(),
                )
            })
        })
        .expect("analysis reads");
    let events = records
        .iter()
        .find(|record| record.kind == AnalysisKind::AudioEvents)
        .expect("raw AudioEvents evidence exists");
    assert_eq!(
        events.value["chunks"][0]["detection"]["events"][0]["class_id"],
        "/m/015p6"
    );
    assert_eq!(
        events.value["chunks"][0]["detection"]["speech_presence"]["status"],
        "absent"
    );

    let audio_space = catalog
        .with_transaction(list_audio_space)
        .expect("audio-space projection reads");
    assert_eq!(audio_space.len(), 1);
    assert_eq!(
        audio_space[0].contextual.as_ref().unwrap()["sound_caption"],
        "Bird · Truck"
    );
    assert_eq!(audio_space[0].contextual_keywords, ["Bird", "Truck"]);
    assert_eq!(
        audio_space[0].contextual_event_type.as_deref(),
        Some("Bird")
    );
    let facets = catalog
        .with_transaction(list_contextual_keyword_facets)
        .expect("facets read");
    assert_eq!(facets.len(), 2);

    let lookup = catalog
        .with_transaction(|transaction| {
            find_by_content_hash(transaction, ContentHash::new([77; 32]))
        })
        .expect("asset reads");
    assert!(matches!(lookup, AssetLookup::Found(_)));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn repeated_class_ids_rank_once_by_their_strongest_score() {
    let mut evidence = AudioEventsEvidence::from_detection(detection(), 0.0);
    evidence.chunks[0]
        .detection
        .events
        .push(DetectedAudioEvent {
            class_id: "/m/015p6".to_owned(),
            label: "Bird vocalization".to_owned(),
            start_seconds: 1.44,
            end_seconds: 1.92,
            score: 0.91,
        });

    let payload = presentation(&evidence).expect("events produce presentation");

    assert_eq!(payload.event_type.as_deref(), Some("Bird vocalization"));
    assert_eq!(payload.keywords, ["Bird vocalization", "Truck"]);
    assert_eq!(payload.sound_caption, "Bird vocalization · Truck");
}

#[test]
fn evidence_envelope_rejects_mixed_builds_and_overlapping_chunks() {
    let first = detection();
    let mut second = detection();
    second.runtime.job.model_build = "different-build".to_owned();
    let evidence = AudioEventsEvidence {
        schema_version: AUDIO_EVENTS_SCHEMA_VERSION,
        chunks: vec![
            AudioEventChunk {
                source_start_seconds: 0.0,
                source_end_seconds: 2.0,
                detection: first,
            },
            AudioEventChunk {
                source_start_seconds: 1.0,
                source_end_seconds: 3.0,
                detection: second,
            },
        ],
    };

    assert!(validate_evidence(&evidence).is_err());
}

#[test]
fn evidence_from_another_runtime_contract_is_rejected() {
    let mut evidence = AudioEventsEvidence::from_detection(detection(), 0.0);
    evidence.chunks[0].detection.runtime.contract_version = "0.1.0-obsolete".to_owned();

    assert!(validate_evidence(&evidence).is_err());
}

fn detection() -> AudioEventDetection {
    AudioEventDetection {
        id: "audio_echo_1".to_owned(),
        model: crate::AUDIO_EVENT_DETECTION_INTENT.to_owned(),
        object: "audio.event_detection".to_owned(),
        events: vec![
            DetectedAudioEvent {
                class_id: "/m/015p6".to_owned(),
                label: "Bird".to_owned(),
                start_seconds: 0.0,
                end_seconds: 1.44,
                score: 0.82,
            },
            DetectedAudioEvent {
                class_id: "/m/07r04".to_owned(),
                label: "Truck".to_owned(),
                start_seconds: 0.48,
                end_seconds: 2.0,
                score: 0.61,
            },
        ],
        speech_presence: crate::SpeechPresence {
            status: crate::SpeechPresenceStatus::Absent,
            max_score: 0.02,
        },
        coverage: crate::AudioAnalysisCoverage {
            status: crate::AudioCoverageStatus::Full,
            input_duration_seconds: 2.0,
            analyzed_start_seconds: 0.0,
            analyzed_end_seconds: 2.0,
            analyzed_seconds: 2.0,
            ratio: 1.0,
            window_count: 4,
            window_seconds: 0.96,
            hop_seconds: 0.48,
        },
        ontology: crate::SoundEventOntology {
            id: "audioset".to_owned(),
            revision: "yamnet-class-map@cdf24d193e19".to_owned(),
            class_id_namespace: "audioset_mid".to_owned(),
            class_count: 521,
            artifact_sha256: "cdf24d193e196d9e95912a2667051ae203e92a2ba09449218ccb40ef787c6df2"
                .to_owned(),
            license_spdx: "CC-BY-SA-4.0".to_owned(),
        },
        policy: crate::SoundEventDetectionPolicy {
            revision: "yamnet-audioset-event-policy-v1".to_owned(),
            score_kind: "raw_sigmoid".to_owned(),
            event_score_threshold: 0.1,
            smoothing: crate::SoundEventSmoothingPolicy {
                method: "centered_median_edge_padded".to_owned(),
                window_frames: 3,
            },
            max_classes_per_window: 12,
            max_events: 64,
            speech_class_set_revision: "audioset-speech@20260813.2".to_owned(),
            speech_present_threshold: 0.3,
            speech_absent_threshold: 0.05,
            max_audio_seconds: 600,
        },
        provenance: crate::SoundEventProvenance {
            model: "google/yamnet/1".to_owned(),
            model_archive_sha256:
                "b80da2a1a56926fb0767205051a200dd7b3beaf3ea1ea126c42a53943996e5e0".to_owned(),
            artifact_set_sha256: "8d0db0c8e4aafefc0d41344d3eb5f3e34ff2f58ad7c32a018d3be90f27f211d2"
                .to_owned(),
            model_license_spdx: "Apache-2.0".to_owned(),
            training_data_license_spdx: "CC-BY-4.0".to_owned(),
            runtime: "tensorflow-saved-model".to_owned(),
            runtime_version: "2.20.0".to_owned(),
            decoder: "ffmpeg".to_owned(),
            decoder_version: "ffmpeg 8.1.2".to_owned(),
            preprocessing_identity: "ffmpeg_decode_mono_f32le_16khz_then_tfhub_yamnet_waveform_v1"
                .to_owned(),
        },
        runtime: crate::RuntimeProvenance {
            contract_version: crate::EXPECTED_CONTRACT_VERSION.to_owned(),
            job: crate::RuntimeJobSnapshot {
                id: "audio_echo_1".to_owned(),
                consumer_core_contract: crate::EXPECTED_CONTRACT_VERSION.to_owned(),
                capability_contract: None,
                app_id: "echo".to_owned(),
                intent: crate::AUDIO_EVENT_DETECTION_INTENT.to_owned(),
                provider: "yamnet-local".to_owned(),
                deployment: "yamnet_audio_events_tfhub_v1".to_owned(),
                model_profile: "yamnet_tfhub_v1".to_owned(),
                model_build: "yamnet_tfhub_v1_tensorflow_2_20".to_owned(),
                physical_model: "google/yamnet/1".to_owned(),
                placement: "local".to_owned(),
                capability_level: "foundational".to_owned(),
                evaluation_status: "provisional".to_owned(),
                resource_class: "standard".to_owned(),
                state: "succeeded".to_owned(),
                policy: "local-first".to_owned(),
                priority: "background".to_owned(),
                constraints: crate::RuntimeJobConstraints::default(),
                routing: crate::RuntimeRoutingDecision::default(),
                attempts: Vec::new(),
            },
        },
    }
}
