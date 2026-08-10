use std::{
    io::{Read, Write},
    net::TcpListener,
    path::PathBuf,
    thread,
};

use serde_json::json;

use super::*;

#[test]
fn transcription_requires_contract_and_job_provenance() {
    let (base_url, server) = serve(vec![
        json_response(r#"{"contract_version":"0.1.0-candidate.3"}"#),
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
fn transcription_accepts_null_segments_from_runtime() {
    let (base_url, server) = serve(vec![
        json_response(r#"{"contract_version":"0.1.0-candidate.3"}"#),
        json_response(
            r#"{"id":"job-echo-null-segments","model":"audio.transcribe","language":"zh","text":"fixture text","segments":null,"usage":{"total_tokens":1}}"#,
        ),
        json_response(
            r#"{"id":"job-echo-null-segments","app_id":"echo","intent":"audio.transcribe","provider":"mlx-audio-local","deployment":"qwen3-asr-mlx","model_profile":"qwen3-asr","model_build":"build-20260809","physical_model":"Qwen3-ASR-1.7B","placement":"local","state":"succeeded","policy":"local-first","priority":"background","attempts":[{"number":1,"provider":"mlx-audio-local","deployment":"qwen3-asr-mlx","outcome":"succeeded","trigger":"initial","error_kind":null}]}"#,
        ),
    ]);
    let source = audio_fixture();
    let client = InferRuntimeClient::new(InferRuntimeConfig {
        base_url,
        bearer_token: "test-consumer-token".to_owned(),
    });

    let payload = client
        .transcribe(&source, &TranscriptionIntent::default())
        .expect("Runtime result with null segments is accepted");

    assert_eq!(payload.text, "fixture text");
    assert!(payload.segments.is_empty());
    assert_eq!(
        payload.runtime.expect("provenance is attached").job.id,
        "job-echo-null-segments"
    );

    std::fs::remove_file(source).expect("fixture removes");
    server.join().expect("server exits");
}

#[test]
fn transcription_keeps_recognized_items_from_openapi_extensions() {
    let (base_url, server) = serve(vec![
        json_response(r#"{"contract_version":"0.1.0-candidate.3"}"#),
        json_response(
            r#"{"id":"job-echo-extension","model":"audio.transcribe","language":{"label":"Chinese"},"text":"fixture text","segments":[{"text":"provider-specific untimed item"},{"text":"timed fixture","start":0.2,"end":0.9}],"usage":{"total_tokens":1}}"#,
        ),
        json_response(
            r#"{"id":"job-echo-extension","app_id":"echo","intent":"audio.transcribe","provider":"mlx-audio-local","deployment":"qwen3-asr-mlx","model_profile":"qwen3-asr","model_build":"build-20260809","physical_model":"Qwen3-ASR-1.7B","placement":"local","state":"succeeded","policy":"local-first","priority":"background","attempts":[{"number":1,"provider":"mlx-audio-local","deployment":"qwen3-asr-mlx","outcome":"succeeded","trigger":"initial","error_kind":null}]}"#,
        ),
    ]);
    let source = audio_fixture();
    let client = InferRuntimeClient::new(InferRuntimeConfig {
        base_url,
        bearer_token: "test-consumer-token".to_owned(),
    });

    let payload = client
        .transcribe(&source, &TranscriptionIntent::default())
        .expect("recognized Runtime extensions are projected");

    assert_eq!(payload.language, None);
    assert_eq!(payload.segments.len(), 1);
    assert_eq!(payload.segments[0].text, "timed fixture");

    std::fs::remove_file(source).expect("fixture removes");
    server.join().expect("server exits");
}

#[test]
fn stable_runtime_error_ignores_provider_message() {
    let (base_url, server) = serve(vec![
        json_response(r#"{"contract_version":"0.1.0-candidate.3"}"#),
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

#[test]
fn contract_redirect_is_not_followed() {
    let (base_url, server) = serve(vec![response_with_headers(
        "302 Found",
        "Location: /redirected\r\n",
        r#"{"error":{"code":"moved","message":"must not follow"}}"#,
    )]);
    let client = InferRuntimeClient::new(InferRuntimeConfig {
        base_url,
        bearer_token: "test-consumer-token".to_owned(),
    });

    let error = client
        .contract_version()
        .expect_err("redirect is returned to the consumer");

    assert_eq!(error.kind, InferRuntimeErrorKind::Rejected);
    assert_eq!(error.code, "moved");
    server.join().expect("server exits after one request");
}

pub(crate) fn serve(responses: Vec<String>) -> (String, thread::JoinHandle<()>) {
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

pub(crate) fn json_response(body: &str) -> String {
    response("200 OK", body)
}

fn response(status: &str, body: &str) -> String {
    response_with_headers(status, "", body)
}

fn response_with_headers(status: &str, headers: &str, body: &str) -> String {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

pub(crate) fn audio_fixture() -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "echo-runtime-test-{}-{}.wav",
        std::process::id(),
        std::thread::current().name().unwrap_or("worker")
    ));
    std::fs::write(&path, b"RIFF test audio").expect("fixture writes");
    path
}

pub(crate) fn audio_event_detection_response(speech_status: &str, speech_score: f64) -> String {
    json!({
        "id": "audio_echo_1",
        "model": AUDIO_EVENT_DETECTION_INTENT,
        "object": "audio.event_detection",
        "events": [{
            "class_id": "/m/015p6",
            "label": "Bird",
            "start_seconds": 0.0,
            "end_seconds": 1.44,
            "score": 0.82
        }, {
            "class_id": "/m/07r04",
            "label": "Truck",
            "start_seconds": 0.48,
            "end_seconds": 2.0,
            "score": 0.61
        }],
        "speech_presence": {"status": speech_status, "max_score": speech_score},
        "coverage": {
            "status": "full",
            "input_duration_seconds": 2.0,
            "analyzed_start_seconds": 0.0,
            "analyzed_end_seconds": 2.0,
            "analyzed_seconds": 2.0,
            "ratio": 1.0,
            "window_count": 4,
            "window_seconds": 0.96,
            "hop_seconds": 0.48
        },
        "ontology": {
            "id": "audioset",
            "revision": "yamnet-class-map@cdf24d193e19",
            "class_id_namespace": "audioset_mid",
            "class_count": 521,
            "artifact_sha256": "cdf24d193e196d9e95912a2667051ae203e92a2ba09449218ccb40ef787c6df2",
            "license_spdx": "CC-BY-SA-4.0"
        },
        "policy": {
            "revision": "yamnet-audioset-event-policy-v1",
            "score_kind": "raw_sigmoid",
            "event_score_threshold": 0.1,
            "smoothing": {"method": "centered_median_edge_padded", "window_frames": 3},
            "max_classes_per_window": 12,
            "speech_present_threshold": 0.3,
            "speech_absent_threshold": 0.05
        },
        "provenance": {
            "model": "google/yamnet/1",
            "model_archive_sha256": "b80da2a1a56926fb0767205051a200dd7b3beaf3ea1ea126c42a53943996e5e0",
            "model_license_spdx": "Apache-2.0",
            "training_data_license_spdx": "CC-BY-4.0",
            "runtime": "tensorflow-saved-model",
            "runtime_version": "2.20.0",
            "decoder": "ffmpeg",
            "decoder_version": "ffmpeg 8.1.2",
            "preprocessing_identity": "ffmpeg_decode_mono_f32le_16khz_then_tfhub_yamnet_waveform_v1"
        }
    })
    .to_string()
}

pub(crate) fn audio_event_job_snapshot() -> String {
    json!({
        "id": "audio_echo_1",
        "app_id": "echo",
        "intent": AUDIO_EVENT_DETECTION_INTENT,
        "provider": "yamnet-local",
        "deployment": "yamnet_audio_events_tfhub_v1",
        "model_profile": "yamnet_tfhub_v1",
        "model_build": "yamnet_tfhub_v1_tensorflow_2_20",
        "physical_model": "google/yamnet/1",
        "placement": "local",
        "quality_grade": "basic",
        "rating_status": "provisional",
        "resource_class": "standard",
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
            "fallback": "none"
        },
        "routing": {"quality_floor": "basic", "candidates": []},
        "attempts": [{
            "number": 1,
            "provider": "yamnet-local",
            "deployment": "yamnet_audio_events_tfhub_v1",
            "outcome": "succeeded",
            "trigger": "initial",
            "error_kind": null
        }]
    })
    .to_string()
}
