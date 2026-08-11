use serde_json::json;

use super::*;
use crate::infer_runtime::tests::{
    audio_event_detection_response, audio_event_job_snapshot, audio_fixture,
    candidate4_contract_response, json_response, serve,
};

#[test]
fn event_detection_accepts_stable_ids_policy_and_local_job_evidence() {
    let response = audio_event_detection_response("unknown", 0.12);
    let (base_url, server) = serve(vec![
        candidate4_contract_response(),
        json_response(&response),
        json_response(&audio_event_job_snapshot()),
    ]);
    let source = audio_fixture();
    let client = InferRuntimeClient::new(super::super::InferRuntimeConfig {
        base_url,
        bearer_token: "test-consumer-token".to_owned(),
    });

    let detection = client
        .detect_audio_events(&source, &AudioEventDetectionIntent::default())
        .expect("sound-event evidence is accepted");

    assert_eq!(detection.model, AUDIO_EVENT_DETECTION_INTENT);
    assert_eq!(detection.object, AUDIO_EVENT_DETECTION_OBJECT);
    assert_eq!(detection.events[0].class_id, "/m/015p6");
    assert_eq!(
        detection.speech_presence.status,
        SpeechPresenceStatus::Unknown
    );
    assert_eq!(detection.ontology.class_id_namespace, "audioset_mid");
    assert_eq!(
        detection.runtime.job.deployment,
        "yamnet_audio_events_tfhub_v1"
    );

    std::fs::remove_file(source).expect("fixture removes");
    server.join().expect("server exits");
}

#[test]
fn event_detection_rejects_speech_absence_without_full_coverage() {
    let mut response: serde_json::Value =
        serde_json::from_str(&audio_event_detection_response("absent", 0.02))
            .expect("fixture decodes");
    response["events"] = json!([]);
    response["coverage"]["status"] = json!("partial");
    response["coverage"]["analyzed_end_seconds"] = json!(1.0);
    response["coverage"]["analyzed_seconds"] = json!(1.0);
    response["coverage"]["ratio"] = json!(0.5);
    let (base_url, server) = serve(vec![
        candidate4_contract_response(),
        json_response(&response.to_string()),
    ]);
    let source = audio_fixture();
    let client = InferRuntimeClient::new(super::super::InferRuntimeConfig {
        base_url,
        bearer_token: "test-consumer-token".to_owned(),
    });

    let error = client
        .detect_audio_events(&source, &AudioEventDetectionIntent::default())
        .expect_err("partial coverage cannot establish speech absence");

    assert_eq!(error.kind, InferRuntimeErrorKind::Protocol);
    assert_eq!(error.code, "invalid_audio_event_contract");
    std::fs::remove_file(source).expect("fixture removes");
    server.join().expect("server exits");
}

#[test]
fn event_detection_rejects_job_provenance_from_another_contract() {
    let response = audio_event_detection_response("unknown", 0.12);
    let mut job: serde_json::Value =
        serde_json::from_str(&audio_event_job_snapshot()).expect("job fixture decodes");
    job["consumer_contract_version"] = json!("0.1.0-obsolete");
    let (base_url, server) = serve(vec![
        candidate4_contract_response(),
        json_response(&response),
        json_response(&job.to_string()),
    ]);
    let source = audio_fixture();
    let client = InferRuntimeClient::new(super::super::InferRuntimeConfig {
        base_url,
        bearer_token: "test-consumer-token".to_owned(),
    });

    let error = client
        .detect_audio_events(&source, &AudioEventDetectionIntent::default())
        .expect_err("Job provenance must use the negotiated contract");

    assert_eq!(error.kind, InferRuntimeErrorKind::ContractMismatch);
    assert_eq!(error.code, "job_contract_mismatch");
    std::fs::remove_file(source).expect("fixture removes");
    server.join().expect("server exits");
}
