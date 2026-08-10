use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

use serde_json::{Value, json};

use super::*;

#[test]
fn text_embedding_requires_local_constraints_and_matching_provenance() {
    let (base_url, server) = serve(vec![
        json_response(r#"{"contract_version":"0.1.0-candidate.2"}"#),
        json_response(&embedding_response("space-v1", 768)),
        json_response(&job_snapshot()),
    ]);
    let client = InferRuntimeClient::new(super::super::InferRuntimeConfig {
        base_url,
        bearer_token: "test-consumer-token".to_owned(),
    });
    let intent = TextEmbeddingIntent::new("source-revision-1");

    let payload = client
        .embed_text("fixture semantic document", &intent)
        .expect("text embedding is accepted");

    assert_eq!(payload.space, "space-v1");
    assert_eq!(payload.values.len(), TEXT_EMBEDDING_DIMENSIONS);
    assert_eq!(payload.provider.job_id, "embed_echo_1");
    assert_eq!(payload.runtime.job.app_id, "echo");
    assert_eq!(payload.runtime.job.intent, TEXT_EMBEDDING_INTENT);

    let requests = server.join().expect("server exits");
    let request: Value = serde_json::from_slice(&requests[1]).expect("request JSON decodes");
    assert_eq!(request["model"], TEXT_EMBEDDING_INTENT);
    assert_eq!(request["text"], "fixture semantic document");
    assert_eq!(request["query_revision"], "source-revision-1");
    assert_eq!(request["metadata"]["infer.policy"], "local-first");
    assert_eq!(request["metadata"]["infer.priority"], "background");
    assert_eq!(request["metadata"]["infer.placement"], "local_only");
    assert_eq!(request["metadata"]["infer.prefer"], "local");
    assert_eq!(request["metadata"]["infer.offline_required"], "true");
    assert_eq!(request["metadata"]["infer.fallback"], "none");
    assert_eq!(request["metadata"]["infer.max_cost_usd"], "0");
    assert_eq!(request["metadata"].as_object().unwrap().len(), 9);
}

#[test]
fn text_embedding_rejects_a_wrong_vector_contract() {
    let (base_url, server) = serve(vec![
        json_response(r#"{"contract_version":"0.1.0-candidate.2"}"#),
        json_response(&embedding_response("space-v1", 767)),
    ]);
    let client = InferRuntimeClient::new(super::super::InferRuntimeConfig {
        base_url,
        bearer_token: "test-consumer-token".to_owned(),
    });

    let error = client
        .embed_text(
            "fixture semantic document",
            &TextEmbeddingIntent::new("source-revision-1"),
        )
        .expect_err("wrong dimensions are rejected");

    assert_eq!(error.kind, InferRuntimeErrorKind::Protocol);
    assert_eq!(error.code, "invalid_text_embedding_payload");
    server.join().expect("server exits");
}

fn embedding_response(space: &str, dimensions: usize) -> String {
    let mut values = vec![0.0_f32; dimensions];
    values[0] = 1.0;
    json!({
        "id": "embed_echo_1",
        "object": "vision.text_embedding",
        "status": "completed",
        "query_revision": "source-revision-1",
        "embedding": {
            "values": values,
            "dimensions": dimensions,
            "normalized": true,
            "distance_metric": "cosine",
            "space": space
        },
        "provenance": {
            "job_id": "embed_echo_1",
            "provider": "onnx-local",
            "deployment": "siglip2-text",
            "model_build": "siglip2-build",
            "artifact_sha256": "fixture-sha256",
            "preprocessing_identity": "lowercase64",
            "postprocessing_identity": "l2_768_fp32",
            "tokenizer": null,
            "runtime": "onnxruntime",
            "requested_execution_provider": "CoreMLExecutionProvider",
            "actual_execution_provider": "CoreMLExecutionProvider",
            "execution_provider_fallback_reason": null,
            "precision": "fp32"
        }
    })
    .to_string()
}

fn job_snapshot() -> String {
    json!({
        "id": "embed_echo_1",
        "app_id": "echo",
        "intent": TEXT_EMBEDDING_INTENT,
        "provider": "onnx-local",
        "deployment": "siglip2-text",
        "model_profile": "siglip2_text",
        "model_build": "siglip2-build",
        "physical_model": "siglip2-base-patch16-224",
        "placement": "local",
        "quality_grade": "basic",
        "rating_status": "provisional",
        "resource_class": "light",
        "state": "succeeded",
        "policy": "local-first",
        "priority": "background",
        "constraints": {
            "policy": "local-first",
            "priority": "background",
            "placement": "local_only",
            "prefer": "local",
            "offline_required": true,
            "quality_floor": "basic",
            "latency": "throughput",
            "max_cost_usd": 0.0,
            "fallback": "none",
            "deadline_ms": null
        },
        "routing": {
            "quality_floor": "basic",
            "candidates": [{
                "deployment": "siglip2-text",
                "provider": "onnx-local",
                "status": "eligible",
                "rank": 0,
                "reason_codes": []
            }]
        },
        "attempts": [{
            "number": 1,
            "provider": "onnx-local",
            "deployment": "siglip2-text",
            "outcome": "succeeded",
            "trigger": "initial",
            "error_kind": null
        }]
    })
    .to_string()
}

fn serve(responses: Vec<String>) -> (String, thread::JoinHandle<Vec<Vec<u8>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test listener binds");
    let address = listener.local_addr().expect("address is known");
    let handle = thread::spawn(move || {
        let mut requests = Vec::new();
        for response in responses {
            let (mut stream, _) = listener.accept().expect("request arrives");
            requests.push(read_request(&mut stream));
            stream
                .write_all(response.as_bytes())
                .expect("response writes");
        }
        requests
    });
    (format!("http://{address}"), handle)
}

fn read_request(stream: &mut std::net::TcpStream) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 4096];
    let header_end = loop {
        let read = stream.read(&mut buffer).expect("request reads");
        assert!(read > 0, "request includes headers");
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(index) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            break index + 4;
        }
    };
    let headers = String::from_utf8_lossy(&bytes[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            line.strip_prefix("content-length: ")
                .or_else(|| line.strip_prefix("Content-Length: "))
        })
        .and_then(|value| value.trim().parse::<usize>().ok())
        .unwrap_or(0);
    while bytes.len() - header_end < content_length {
        let read = stream.read(&mut buffer).expect("request body reads");
        assert!(read > 0, "request body is complete");
        bytes.extend_from_slice(&buffer[..read]);
    }
    bytes[header_end..header_end + content_length].to_vec()
}

fn json_response(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}
