use super::{TRANSCRIPTION_INTENT, direct_request};

#[test]
fn direct_request_preserves_the_infer_transcription_contract() {
    let request = direct_request(
        std::path::Path::new("/recordings/memory.wav"),
        std::path::Path::new("/models/qwen3-asr"),
    );
    let value = serde_json::to_value(request).expect("request serializes");

    assert_eq!(value["operation"], "transcribe");
    assert_eq!(value["intent"]["model"], TRANSCRIPTION_INTENT);
    assert_eq!(value["intent"]["response_format"], "verbose_json");
    assert_eq!(value["intent"]["metadata"]["infer.policy"], "local-first");
    assert_eq!(value["intent"]["metadata"]["infer.placement"], "local_only");
    assert_eq!(value["model"], "/models/qwen3-asr");
    assert_eq!(value["audio_path"], "/recordings/memory.wav");
}
