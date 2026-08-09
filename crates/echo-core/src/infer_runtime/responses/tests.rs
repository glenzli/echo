use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

use serde_json::{Value, json};

use super::*;

#[test]
fn contextual_response_requires_explicit_local_constraints_and_job_evidence() {
    let product_json = json!({
        "schema_version": 2,
        "sound_caption": "Fixture voices in a quiet room",
        "summary": "fixture summary",
        "keywords": ["fixture", "speech"],
        "mood": null,
        "place_hint": null,
        "event_type": "conversation",
        "people_hints": []
    })
    .to_string();
    let response_body = json!({
        "id": "resp_echo_context",
        "object": "response",
        "created_at": 1,
        "model": CONTEXTUAL_INTENT,
        "output": [{
            "type": "message",
            "content": [{"type": "output_text", "text": product_json}]
        }]
    })
    .to_string();
    let (base_url, server) = serve(vec![
        json_response(r#"{"contract_version":"0.1.0-candidate.1"}"#),
        json_response(&response_body),
        json_response(&job_snapshot("initial")),
    ]);
    let client = InferRuntimeClient::new(super::super::InferRuntimeConfig {
        base_url,
        bearer_token: "test-consumer-token".to_owned(),
    });

    let response = client
        .contextualize("fixture transcript", &ContextualIntent::default())
        .expect("contextual response is accepted");

    assert!(response.output_text.contains("fixture summary"));
    assert_eq!(response.runtime.job.intent, CONTEXTUAL_INTENT);
    assert_eq!(
        response.runtime.job.constraints.placement.as_deref(),
        Some("local_only")
    );
    assert_eq!(response.runtime.job.routing.quality_floor, "basic");

    let requests = server.join().expect("server exits");
    let request: Value = serde_json::from_slice(&requests[1]).expect("request JSON decodes");
    assert_eq!(request["model"], CONTEXTUAL_INTENT);
    assert_eq!(request["background"], false);
    assert_eq!(request["stream"], false);
    assert_eq!(request["metadata"]["infer.policy"], "local-first");
    assert_eq!(request["metadata"]["infer.priority"], "background");
    assert_eq!(request["metadata"]["infer.placement"], "local_only");
    assert_eq!(request["metadata"]["infer.prefer"], "local");
    assert_eq!(request["metadata"]["infer.offline_required"], "true");
    assert_eq!(request["metadata"]["infer.quality_floor"], "basic");
    assert_eq!(request["metadata"]["infer.latency"], "throughput");
    assert_eq!(request["metadata"]["infer.fallback"], "none");
    assert_eq!(request["metadata"]["infer.max_cost_usd"], "0");
    assert!(
        request["instructions"]
            .as_str()
            .expect("instructions are text")
            .contains("sound_caption")
    );
    assert!(
        request["instructions"]
            .as_str()
            .expect("instructions are text")
            .contains("schema_version must be the JSON integer 2")
    );
}

#[test]
fn contextual_response_rejects_fallback_attempt_evidence() {
    let response_body = json!({
        "id": "resp_echo_context",
        "object": "response",
        "created_at": 1,
        "model": CONTEXTUAL_INTENT,
        "output_text": "{}"
    })
    .to_string();
    let (base_url, server) = serve(vec![
        json_response(r#"{"contract_version":"0.1.0-candidate.1"}"#),
        json_response(&response_body),
        json_response(&job_snapshot("fallback")),
    ]);
    let client = InferRuntimeClient::new(super::super::InferRuntimeConfig {
        base_url,
        bearer_token: "test-consumer-token".to_owned(),
    });

    let error = client
        .contextualize("fixture transcript", &ContextualIntent::default())
        .expect_err("fallback evidence is rejected");

    assert_eq!(error.kind, InferRuntimeErrorKind::Protocol);
    assert_eq!(error.code, "inconsistent_contextual_constraints");
    server.join().expect("server exits");
}

fn job_snapshot(trigger: &str) -> String {
    json!({
        "id": "resp_echo_context",
        "app_id": "echo",
        "intent": CONTEXTUAL_INTENT,
        "provider": "ollama-local",
        "deployment": "qwen3-5-2b",
        "model_profile": "qwen3_5_2b",
        "model_build": "qwen3.5:2b-mlx",
        "physical_model": "qwen3.5:2b-mlx",
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
                "deployment": "qwen3-5-2b",
                "provider": "ollama-local",
                "status": "eligible",
                "rank": 0,
                "reason_codes": []
            }]
        },
        "attempts": [{
            "number": 1,
            "provider": "ollama-local",
            "deployment": "qwen3-5-2b",
            "outcome": "succeeded",
            "trigger": trigger,
            "error_kind": null
        }],
        "error": null
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
