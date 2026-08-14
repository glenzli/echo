use infer_runtime_client::AudioEventDetectionResponse as SdkAudioEventDetectionResponse;
use serde_json::json;

use super::*;
use crate::{
    InferRuntimeConfig, InferRuntimeErrorKind,
    infer_runtime::tests::{FakeTransport, audio_fixture, sdk_job_fixture},
    infer_runtime_credential_path,
};

#[test]
fn typed_sdk_audio_event_response_preserves_evidence_and_provenance() {
    let response: SdkAudioEventDetectionResponse = serde_json::from_value(json!({
        "id": "audio-events-1",
        "model": "audio.detect_events",
        "object": "audio.event_detection",
        "events": [{
            "class_id": "/m/06d_3",
            "label": "Rain",
            "start_seconds": 0.0,
            "end_seconds": 1.0,
            "score": 0.88
        }],
        "speech_presence": { "status": "absent", "max_score": 0.01 },
        "coverage": {
            "status": "full",
            "input_duration_seconds": 1.0,
            "analyzed_start_seconds": 0.0,
            "analyzed_end_seconds": 1.0,
            "analyzed_seconds": 1.0,
            "ratio": 1.0,
            "window_count": 1,
            "window_seconds": 0.96,
            "hop_seconds": 0.48
        },
        "ontology": {
            "id": "audioset",
            "revision": "audioset@20260813.2",
            "class_id_namespace": "audioset_mid",
            "class_count": 521,
            "artifact_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "license_spdx": "CC-BY-4.0"
        },
        "policy": {
            "revision": "yamnet-policy@20260813.2",
            "score_kind": "raw_sigmoid",
            "event_score_threshold": 0.1,
            "smoothing": { "method": "median", "window_frames": 3 },
            "max_classes_per_window": 12,
            "max_events": 64,
            "speech_class_set_revision": "audioset-speech@20260813.2",
            "speech_present_threshold": 0.3,
            "speech_absent_threshold": 0.05,
            "max_audio_seconds": 600
        },
        "provenance": {
            "model": "yamnet",
            "model_archive_sha256": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "artifact_set_sha256": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "model_license_spdx": "Apache-2.0",
            "training_data_license_spdx": "CC-BY-4.0",
            "runtime": "tensorflow",
            "runtime_version": "2.20",
            "decoder": "ffmpeg",
            "decoder_version": "8",
            "preprocessing_identity": "canonical"
        }
    }))
    .expect("official SDK response shape parses");
    let job = sdk_job_fixture(
        AUDIO_EVENT_DETECTION_INTENT,
        super::super::AUDIO_EVENT_DETECTION_CAPABILITY,
    );
    let client =
        InferRuntimeClient::with_transport(FakeTransport::audio_event_detection(response, job));
    let detection = client
        .detect_audio_events(&audio_fixture(), &AudioEventDetectionIntent::default())
        .expect("typed capability is accepted");

    assert_eq!(detection.events[0].class_id, "/m/06d_3");
    assert_eq!(
        detection.speech_presence.status,
        SpeechPresenceStatus::Absent
    );
    assert_eq!(detection.policy.max_audio_seconds, 600);
    assert_eq!(
        detection.runtime.job.capability_contract.as_deref(),
        Some(super::super::AUDIO_EVENT_DETECTION_CAPABILITY)
    );
}

#[test]
fn audio_event_detection_rejects_wrong_product_intent_without_transport() {
    let client = InferRuntimeClient::with_transport(std::sync::Arc::new(FakeTransport::default()));
    let error = client
        .detect_audio_events(
            &audio_fixture(),
            &AudioEventDetectionIntent {
                model: "audio.transcribe".to_owned(),
                ..AudioEventDetectionIntent::default()
            },
        )
        .expect_err("wrong intent fails before transport");
    assert_eq!(error.kind, InferRuntimeErrorKind::Rejected);
    assert_eq!(error.code, "invalid_audio_event_detection_intent");
}

#[test]
#[ignore = "requires a healthy local Infer Runtime and Echo's managed credential"]
fn live_sdk_audio_event_detection_accepts_a_public_non_speech_sample() {
    let source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../local-audio-library/public-domain-non-speech/01-rain.ogg");
    let client = InferRuntimeClient::new(InferRuntimeConfig {
        base_url: String::new(),
        credential_path: infer_runtime_credential_path().expect("Echo credential path resolves"),
    });
    let detection = client
        .detect_audio_events(&source, &AudioEventDetectionIntent::default())
        .expect("official local Runtime accepts typed event detection");

    assert_eq!(detection.model, AUDIO_EVENT_DETECTION_INTENT);
    assert_eq!(detection.object, "audio.event_detection");
    assert_eq!(detection.runtime.job.app_id, "echo");
    assert_eq!(
        detection.runtime.job.capability_contract.as_deref(),
        Some(super::super::AUDIO_EVENT_DETECTION_CAPABILITY)
    );
    assert_eq!(detection.runtime.job.placement, "local");
    assert_eq!(detection.runtime.job.priority, "background");
}
