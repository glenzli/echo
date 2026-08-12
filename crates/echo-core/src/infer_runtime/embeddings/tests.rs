use infer_runtime_client::TextEmbeddingResponse;
use serde_json::json;

use super::*;
use crate::infer_runtime::tests::{FakeTransport, sdk_job_fixture};

#[test]
fn fake_sdk_embedding_preserves_space_and_redacted_provenance() {
    let mut values = vec![0.0_f32; TEXT_EMBEDDING_DIMENSIONS];
    values[0] = 1.0;
    let response: TextEmbeddingResponse = serde_json::from_value(json!({
        "id":"embed-1","object":"vision.text_embedding","created_at":1_786_212_000,
        "status":"completed","query_revision":"query-1","language":"zh",
        "embedding":{"values":values,"dimensions":768,"normalized":true,
        "distance_metric":"cosine","space":"siglip2-so400m-text@v1"},
        "provenance":{"job_id":"embed-1","provider":"local-example",
        "deployment":"example-small","model_build":"example-build-v1",
        "artifact_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "preprocessing_identity":"tokenizer-v1","postprocessing_identity":"l2-v1",
        "runtime":"onnxruntime","requested_execution_provider":"CoreMLExecutionProvider",
        "actual_execution_provider":"CoreMLExecutionProvider",
        "execution_provider_fallback_reason":null,"precision":"fp32",
        "tokenizer":{"id":"fixture"}}
    }))
    .unwrap();
    let job = sdk_job_fixture(TEXT_EMBEDDING_INTENT, TEXT_EMBEDDING_CAPABILITY);
    let client = InferRuntimeClient::with_transport(FakeTransport::embedding(response, job));
    let result = client
        .embed_text("雨声", &TextEmbeddingIntent::new("query-1"))
        .unwrap();
    assert_eq!(result.space, "siglip2-so400m-text@v1");
    assert_eq!(result.values.len(), TEXT_EMBEDDING_DIMENSIONS);
    assert_eq!(result.provider.tokenizer, Some(json!({"id":"fixture"})));
    assert_eq!(
        result.runtime.job.capability_contract.as_deref(),
        Some(TEXT_EMBEDDING_CAPABILITY)
    );
}

#[test]
fn empty_embedding_input_fails_before_transport() {
    let client = InferRuntimeClient::with_transport(std::sync::Arc::new(FakeTransport::default()));
    let error = client
        .embed_text(" ", &TextEmbeddingIntent::new("query-1"))
        .unwrap_err();
    assert_eq!(error.code, "text_embedding_input_size");
}
