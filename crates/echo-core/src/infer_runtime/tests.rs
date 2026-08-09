use std::{
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    thread,
};

use super::*;

#[test]
fn transcription_requires_contract_and_job_provenance() {
    let (base_url, server) = serve(vec![
        json_response(r#"{"contract_version":"0.1.0-candidate.1"}"#),
        json_response(
            r#"{"id":"job-echo-1","model":"audio.transcribe","language":"zh","text":"你好","segments":[{"text":"你好","start_time":0.1,"end_time":0.8,"words":[{"text":"你好","start_time":0.1,"end_time":0.8}]}]}"#,
        ),
        json_response(
            r#"{"id":"job-echo-1","app_id":"echo","intent":"audio.transcribe","provider":"mlx-audio-local","deployment":"qwen3-asr-mlx","model_profile":"qwen3-asr","model_build":"build-20260809","physical_model":"Qwen3-ASR-1.7B","placement":"local","state":"succeeded","policy":"local-first","priority":"background","attempts":[{"number":1,"provider":"mlx-audio-local","deployment":"qwen3-asr-mlx","outcome":"succeeded","trigger":"initial","error_kind":null}]}"#,
        ),
    ]);
    let source = audio_fixture();
    let client = InferRuntimeClient::new(InferRuntimeConfig {
        base_url,
        bearer_token: "test-consumer-token".to_owned(),
    });

    let payload = client
        .transcribe(&source, &TranscriptionIntent::default())
        .expect("Runtime result is accepted");

    assert_eq!(payload.text, "你好");
    assert!((payload.segments[0].start - 0.1).abs() < f64::EPSILON);
    assert!((payload.segments[0].words.as_ref().unwrap()[0].end - 0.8).abs() < f64::EPSILON);
    let provenance = payload.runtime.expect("provenance is attached");
    assert_eq!(provenance.contract_version, EXPECTED_CONTRACT_VERSION);
    assert_eq!(provenance.job.physical_model, "Qwen3-ASR-1.7B");
    assert_eq!(provenance.job.attempts.len(), 1);

    std::fs::remove_file(source).expect("fixture removes");
    server.join().expect("server exits");
}

#[test]
fn stable_runtime_error_ignores_provider_message() {
    let (base_url, server) = serve(vec![
        json_response(r#"{"contract_version":"0.1.0-candidate.1"}"#),
        response(
            "503 Service Unavailable",
            r#"{"error":{"code":"provider_unavailable","message":"sensitive provider detail"}}"#,
        ),
    ]);
    let source = audio_fixture();
    let client = InferRuntimeClient::new(InferRuntimeConfig {
        base_url,
        bearer_token: "test-consumer-token".to_owned(),
    });

    let error = client
        .transcribe(&source, &TranscriptionIntent::default())
        .expect_err("capacity failure is classified");

    assert_eq!(error.kind, InferRuntimeErrorKind::Capacity);
    assert_eq!(error.code, "provider_unavailable");
    assert!(!error.to_string().contains("sensitive"));
    assert!(error.retryable());

    std::fs::remove_file(source).expect("fixture removes");
    server.join().expect("server exits");
}

fn serve(responses: Vec<String>) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test listener binds");
    let address = listener.local_addr().expect("address is known");
    let handle = thread::spawn(move || {
        for response in responses {
            let (mut stream, _) = listener.accept().expect("request arrives");
            read_request(&mut stream);
            stream
                .write_all(response.as_bytes())
                .expect("response writes");
        }
    });
    (format!("http://{address}"), handle)
}

fn read_request(stream: &mut std::net::TcpStream) {
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
}

fn json_response(body: &str) -> String {
    response("200 OK", body)
}

fn response(status: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

fn audio_fixture() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "echo-runtime-test-{}-{}.wav",
        std::process::id(),
        std::thread::current().name().unwrap_or("worker")
    ));
    std::fs::write(&path, b"RIFF test audio").expect("fixture writes");
    path
}
