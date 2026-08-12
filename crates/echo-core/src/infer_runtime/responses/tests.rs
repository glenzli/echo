use infer_runtime_client::ResponsesResult;
use serde_json::json;

use super::*;
use crate::infer_runtime::tests::{FakeTransport, sdk_job_fixture};

#[test]
fn fake_sdk_contextual_response_preserves_product_output_and_job() {
    let response: ResponsesResult = serde_json::from_value(json!({
        "id":"contextual-1","object":"response","created_at":1_786_212_000,
        "model":"text.summarize","status":"completed","output":[{
            "type":"message","content":[{"type":"output_text","text":"{\"schema_version\":3,\"sound_caption\":\"雨声窗边\",\"summary\":\"\",\"keywords\":[\"雨声\"],\"mood\":null,\"place_hint\":null,\"event_type\":\"rain\",\"people_hints\":[]}"}]
        }]
    }))
    .unwrap();
    let job = sdk_job_fixture(CONTEXTUAL_INTENT, RESPONSES_CAPABILITY);
    let client = InferRuntimeClient::with_transport(FakeTransport::contextual(response, job));
    let result = client
        .contextualize("窗边持续下雨。", &ContextualIntent::default())
        .unwrap();
    assert!(result.output_text.contains("雨声窗边"));
    assert_eq!(
        result.runtime.job.capability_contract.as_deref(),
        Some(RESPONSES_CAPABILITY)
    );
}

#[test]
fn contextual_input_is_bounded_before_transport() {
    let client = InferRuntimeClient::with_transport(std::sync::Arc::new(FakeTransport::default()));
    let error = client
        .contextualize(
            &"x".repeat(MAX_CONTEXTUAL_INPUT_BYTES + 1),
            &ContextualIntent::default(),
        )
        .unwrap_err();
    assert_eq!(error.code, "contextual_input_too_large");
}
