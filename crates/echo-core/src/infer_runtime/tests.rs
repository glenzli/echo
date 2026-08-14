use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
    sync::Mutex,
};

use infer_runtime_client::{
    AlignmentResponse as SdkAlignmentResponse,
    AudioEventDetectionResponse as SdkAudioEventDetectionResponse, JobSnapshot as SdkJobSnapshot,
    ResponsesRequest, ResponsesResult, TextEmbeddingRequest, TextEmbeddingResponse,
    TranscriptionResponse as SdkTranscriptionResponse,
};
use serde_json::json;

use super::*;

/// Fake at Echo's product/SDK seam. It returns the official SDK's public
/// response types and Job fixture shape without opening a daemon or socket.
#[derive(Debug, Default)]
pub(crate) struct FakeTransport {
    transcriptions:
        Mutex<VecDeque<Result<(SdkTranscriptionResponse, SdkJobSnapshot), InferRuntimeError>>>,
    alignments: Mutex<VecDeque<Result<(SdkAlignmentResponse, SdkJobSnapshot), InferRuntimeError>>>,
    audio_event_detections: Mutex<
        VecDeque<Result<(SdkAudioEventDetectionResponse, SdkJobSnapshot), InferRuntimeError>>,
    >,
    contextual: Mutex<VecDeque<Result<(ResponsesResult, SdkJobSnapshot), InferRuntimeError>>>,
    embeddings: Mutex<VecDeque<Result<(TextEmbeddingResponse, SdkJobSnapshot), InferRuntimeError>>>,
}

impl FakeTransport {
    pub(crate) fn transcription(
        response: SdkTranscriptionResponse,
        job: SdkJobSnapshot,
    ) -> Arc<Self> {
        Arc::new(Self {
            transcriptions: Mutex::new(VecDeque::from([Ok((response, job))])),
            ..Self::default()
        })
    }

    pub(crate) fn alignment(response: SdkAlignmentResponse, job: SdkJobSnapshot) -> Arc<Self> {
        Arc::new(Self {
            alignments: Mutex::new(VecDeque::from([Ok((response, job))])),
            ..Self::default()
        })
    }

    pub(crate) fn audio_event_detection(
        response: SdkAudioEventDetectionResponse,
        job: SdkJobSnapshot,
    ) -> Arc<Self> {
        Arc::new(Self {
            audio_event_detections: Mutex::new(VecDeque::from([Ok((response, job))])),
            ..Self::default()
        })
    }

    pub(crate) fn contextual(response: ResponsesResult, job: SdkJobSnapshot) -> Arc<Self> {
        Arc::new(Self {
            contextual: Mutex::new(VecDeque::from([Ok((response, job))])),
            ..Self::default()
        })
    }

    pub(crate) fn embedding(response: TextEmbeddingResponse, job: SdkJobSnapshot) -> Arc<Self> {
        Arc::new(Self {
            embeddings: Mutex::new(VecDeque::from([Ok((response, job))])),
            ..Self::default()
        })
    }
}

impl RuntimeTransport for FakeTransport {
    fn contract(&self) -> Result<(), InferRuntimeError> {
        Ok(())
    }

    fn transcribe(
        &self,
        _source: &Path,
        _language: Option<&str>,
        metadata: &BTreeMap<String, String>,
    ) -> Result<(SdkTranscriptionResponse, SdkJobSnapshot), InferRuntimeError> {
        assert_echo_constraints(metadata);
        pop(&self.transcriptions)
    }

    fn align(
        &self,
        _source: &Path,
        text: &str,
        _language: Option<&str>,
        metadata: &BTreeMap<String, String>,
    ) -> Result<(SdkAlignmentResponse, SdkJobSnapshot), InferRuntimeError> {
        assert!(!text.is_empty());
        assert_echo_constraints(metadata);
        pop(&self.alignments)
    }

    fn detect_audio_events(
        &self,
        _source: &Path,
        metadata: &BTreeMap<String, String>,
    ) -> Result<(SdkAudioEventDetectionResponse, SdkJobSnapshot), InferRuntimeError> {
        assert_echo_constraints(metadata);
        pop(&self.audio_event_detections)
    }

    fn contextualize(
        &self,
        request: &ResponsesRequest,
    ) -> Result<(ResponsesResult, SdkJobSnapshot), InferRuntimeError> {
        assert_eq!(request.model, CONTEXTUAL_INTENT);
        assert!(!request.stream);
        assert!(!request.background);
        assert_echo_constraints(&request.metadata);
        pop(&self.contextual)
    }

    fn embed_text(
        &self,
        request: &TextEmbeddingRequest,
    ) -> Result<(TextEmbeddingResponse, SdkJobSnapshot), InferRuntimeError> {
        assert_eq!(request.model, TEXT_EMBEDDING_INTENT);
        assert_echo_constraints(&request.metadata);
        pop(&self.embeddings)
    }
}

fn pop<T>(queue: &Mutex<VecDeque<Result<T, InferRuntimeError>>>) -> Result<T, InferRuntimeError> {
    queue
        .lock()
        .expect("fake queue lock")
        .pop_front()
        .unwrap_or_else(|| Err(protocol("unexpected_fake_transport_call")))
}

fn assert_echo_constraints(metadata: &BTreeMap<String, String>) {
    assert_eq!(
        metadata.get("infer.policy").map(String::as_str),
        Some("local-first")
    );
    assert_eq!(
        metadata.get("infer.priority").map(String::as_str),
        Some("background")
    );
    assert_eq!(
        metadata.get("infer.placement").map(String::as_str),
        Some("local_only")
    );
    assert_eq!(
        metadata.get("infer.offline_required").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        metadata.get("infer.fallback").map(String::as_str),
        Some("none")
    );
    assert_eq!(
        metadata.get("infer.max_cost_usd").map(String::as_str),
        Some("0")
    );
}

pub(crate) fn sdk_job_fixture(intent: &str, capability: &str) -> SdkJobSnapshot {
    // Shape copied verbatim from the frozen SDK Core fixture, then scoped to
    // Echo and the capability exercised by the product test.
    let mut value: serde_json::Value = serde_json::from_str(
        r#"{
          "id":"resp_example","app_id":"sample-app","intent":"text.summarize",
          "consumer_core_contract":"infer-runtime.consumer-core@20260813.1",
          "capability_contract":"infer.responses@20260812.1","provider":"local-example",
          "deployment":"example-small","model_profile":"example-text-small",
          "model_build":"example-build-v1","physical_model":"example/model:small",
          "placement":"local","capability_level":"foundational",
          "evaluation_status":"provisional","resource_class":"light","state":"succeeded",
          "policy":"local-first","priority":"normal",
          "constraints":{"policy":"local-first","priority":"normal","provider_access_class":"standard",
          "placement":"local_only","prefer":"local","offline_required":true,
          "capability_floor":"foundational","latency":"balanced","max_cost_usd":0.0,
          "fallback":"none","deadline_ms":null,"named_route":null},
          "routing":{"capability_floor":"foundational","named_route":null,"candidates":[{
          "deployment":"example-small","provider":"local-example","status":"eligible","rank":0,
          "reason_codes":[]}]},"attempts":[{"number":1,"provider":"local-example",
          "deployment":"example-small","outcome":"succeeded","trigger":"initial"}],"error":null
        }"#,
    )
    .expect("official SDK Job fixture parses");
    value["app_id"] = json!(EXPECTED_APP_ID);
    value["intent"] = json!(intent);
    value["capability_contract"] = json!(capability);
    value["priority"] = json!("background");
    value["constraints"]["priority"] = json!("background");
    value["constraints"]["latency"] = json!("throughput");
    serde_json::from_value(value).expect("Echo-scoped SDK Job fixture parses")
}

pub(crate) fn audio_fixture() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "echo-sdk-audio-fixture-{}-{}.wav",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    std::fs::write(&path, b"RIFFecho-sdk-fixture").expect("write audio fixture");
    path
}

#[test]
fn frozen_sdk_identities_replace_candidate_headers() {
    assert_eq!(
        EXPECTED_CONTRACT_VERSION,
        "infer-runtime.consumer-core@20260813.1"
    );
    assert_eq!(
        infer_runtime_client::CONSUMER_CORE_HEADER,
        "Infer-Consumer-Contract"
    );
    assert_eq!(
        infer_runtime_client::CAPABILITY_CONTRACT_HEADER,
        "Infer-Capability-Contract"
    );
    assert_eq!(
        infer_runtime_client::CAPABILITY_CATALOG_VERSION,
        "20260813.1"
    );
}

#[test]
fn legacy_job_field_is_read_only_compatible_and_serializes_as_core_contract() {
    let mapped = validate_succeeded_job(
        sdk_job_fixture(TRANSCRIPTION_INTENT, TRANSCRIPTION_CAPABILITY),
        TRANSCRIPTION_INTENT,
        TRANSCRIPTION_CAPABILITY,
    )
    .unwrap();
    let mut value = serde_json::to_value(mapped).unwrap();
    let object = value.as_object_mut().unwrap();
    let core_contract = object.remove("consumer_core_contract").unwrap();
    object.insert("consumer_contract_version".to_owned(), core_contract);
    object.remove("capability_contract");
    let legacy: RuntimeJobSnapshot = serde_json::from_value(value).unwrap();
    assert_eq!(legacy.capability_contract, None);
    let encoded = serde_json::to_value(legacy).unwrap();
    assert!(encoded.get("consumer_core_contract").is_some());
    assert!(encoded.get("consumer_contract_version").is_none());
}

#[test]
fn fake_sdk_transcription_preserves_echo_provenance() {
    let response: SdkTranscriptionResponse = serde_json::from_value(json!({
        "id":"transcribe-1","text":"你好","language":"zh",
        "segments":[{"text":"你好","start":0.1,"end":0.8}],"usage":{}
    }))
    .unwrap();
    let job = sdk_job_fixture(TRANSCRIPTION_INTENT, TRANSCRIPTION_CAPABILITY);
    let client = InferRuntimeClient::with_transport(FakeTransport::transcription(response, job));
    let payload = client
        .transcribe(&audio_fixture(), &TranscriptionIntent::default())
        .unwrap();
    assert_eq!(payload.text, "你好");
    assert_eq!(payload.segments.len(), 1);
    let runtime = payload.runtime.unwrap();
    assert_eq!(runtime.contract_version, EXPECTED_CONTRACT_VERSION);
    assert_eq!(
        runtime.job.capability_contract.as_deref(),
        Some(TRANSCRIPTION_CAPABILITY)
    );
}

#[test]
fn transcription_accepts_a_known_capability_above_the_requested_floor() {
    let response: SdkTranscriptionResponse = serde_json::from_value(json!({
        "id":"transcribe-capable","text":"","language":null,"segments":[],"usage":{}
    }))
    .unwrap();
    let mut job = sdk_job_fixture(TRANSCRIPTION_INTENT, TRANSCRIPTION_CAPABILITY);
    job.capability_level = "capable".to_owned();
    let client = InferRuntimeClient::with_transport(FakeTransport::transcription(response, job));
    assert!(
        client
            .transcribe(&audio_fixture(), &TranscriptionIntent::default())
            .is_ok()
    );
}

#[test]
fn transcription_rejects_an_unknown_capability_level() {
    let response: SdkTranscriptionResponse = serde_json::from_value(json!({
        "id":"transcribe-unknown-level","text":"","language":null,"segments":[],"usage":{}
    }))
    .unwrap();
    let mut job = sdk_job_fixture(TRANSCRIPTION_INTENT, TRANSCRIPTION_CAPABILITY);
    job.capability_level = "unrecognized".to_owned();
    let client = InferRuntimeClient::with_transport(FakeTransport::transcription(response, job));
    let error = client
        .transcribe(&audio_fixture(), &TranscriptionIntent::default())
        .unwrap_err();
    assert_eq!(error.code, "inconsistent_transcription_constraints");
}

#[test]
#[ignore = "requires a healthy local Infer Runtime and Echo's managed credential"]
fn live_sdk_transcription_job_satisfies_echo_local_constraints() {
    let source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../local-audio-library/public-domain-non-speech/01-rain.ogg");
    let intent = TranscriptionIntent::default();
    let client = InferRuntimeClient::new(InferRuntimeConfig {
        base_url: String::new(),
        credential_path: crate::infer_runtime_credential_path()
            .expect("Echo credential path resolves"),
    });
    let (_, job) = client
        .transport()
        .expect("SDK transport builds")
        .transcribe(&source, None, &intent.metadata)
        .expect("official SDK transcribes public non-speech source");
    let job = validate_succeeded_job(job, TRANSCRIPTION_INTENT, TRANSCRIPTION_CAPABILITY)
        .expect("SDK Job is Echo scoped and successful");
    assert!(
        validate_local_only_job(&job, "inconsistent_transcription_constraints").is_ok(),
        "sanitized Job constraints: {job:#?}"
    );
}

#[test]
fn typed_mixed_language_evidence_keeps_transcription_usable() {
    let response: SdkTranscriptionResponse = serde_json::from_value(json!({
        "id":"transcribe-mixed","text":"你好 hello","language":null,
        "language_evidence": {
            "kind":"input_set", "source":"provider_reported",
            "languages":["Chinese", "English"]
        },
        "segments":[],"usage":{}
    }))
    .unwrap();
    assert!(matches!(
        response.language_evidence.as_ref(),
        Some(infer_runtime_client::TranscriptionLanguageEvidence::InputSet { languages, .. })
            if languages == &["Chinese".to_owned(), "English".to_owned()]
    ));
    let job = sdk_job_fixture(TRANSCRIPTION_INTENT, TRANSCRIPTION_CAPABILITY);
    let client = InferRuntimeClient::with_transport(FakeTransport::transcription(response, job));
    let payload = client
        .transcribe(&audio_fixture(), &TranscriptionIntent::default())
        .unwrap();
    assert_eq!(payload.language, None);
    assert_eq!(payload.text, "你好 hello");
}

#[test]
fn transcription_rejects_runtime_fallback_evidence() {
    let response: SdkTranscriptionResponse = serde_json::from_value(json!({
        "id":"transcribe-fallback","text":"你好","language":"zh",
        "segments":[],"usage":{}
    }))
    .unwrap();
    let mut job = sdk_job_fixture(TRANSCRIPTION_INTENT, TRANSCRIPTION_CAPABILITY);
    job.constraints["fallback"] = json!("provider");
    let client = InferRuntimeClient::with_transport(FakeTransport::transcription(response, job));
    let error = client
        .transcribe(&audio_fixture(), &TranscriptionIntent::default())
        .unwrap_err();
    assert_eq!(error.kind, InferRuntimeErrorKind::Protocol);
    assert_eq!(error.code, "inconsistent_transcription_constraints");
}

#[test]
fn fake_sdk_alignment_preserves_typed_items() {
    let response: SdkAlignmentResponse = serde_json::from_value(json!({
        "id":"align-1","text":"你好","language":"zh",
        "items":[{"text":"你好","start":0.1,"end":0.8}]
    }))
    .unwrap();
    let job = sdk_job_fixture(ALIGNMENT_INTENT, ALIGNMENT_CAPABILITY);
    let client = InferRuntimeClient::with_transport(FakeTransport::alignment(response, job));
    let payload = client
        .align(&audio_fixture(), "你好", &AlignmentIntent::default())
        .unwrap();
    assert_eq!(payload.items[0].text, "你好");
    assert_eq!(
        payload.runtime.job.consumer_core_contract,
        EXPECTED_CONTRACT_VERSION
    );
}

#[test]
fn oversized_audio_fails_before_sdk_transport() {
    let path = std::env::temp_dir().join(format!("echo-sdk-large-{}", std::process::id()));
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(MAX_AUDIO_UPLOAD_BYTES + 1).unwrap();
    let client = InferRuntimeClient::with_transport(Arc::new(FakeTransport::default()));
    let error = client
        .transcribe(&path, &TranscriptionIntent::default())
        .unwrap_err();
    assert_eq!(error.kind, InferRuntimeErrorKind::SourceTooLarge);
    let _ = std::fs::remove_file(path);
}

#[test]
fn debug_output_never_contains_credential_contents() {
    let config = InferRuntimeConfig {
        base_url: String::new(),
        credential_path: PathBuf::from("/private/echo/infer-runtime.token"),
    };
    let debug = format!("{config:?}");
    assert!(debug.contains("infer-runtime.token"));
    assert!(!debug.contains("Bearer"));
}
