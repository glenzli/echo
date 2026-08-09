use super::{TranscriptPayload, TranscriptSegment, TranscriptWord};

#[test]
fn transcript_payload_keeps_optional_word_timing() {
    let payload = TranscriptPayload {
        model: "audio.transcribe".to_owned(),
        language: Some("zh".to_owned()),
        text: "你好".to_owned(),
        segments: vec![TranscriptSegment {
            text: "你好".to_owned(),
            start: 0.1,
            end: 0.8,
            words: Some(vec![TranscriptWord {
                text: "你好".to_owned(),
                start: 0.1,
                end: 0.8,
            }]),
        }],
        runtime: None,
    };
    let decoded: TranscriptPayload =
        serde_json::from_value(serde_json::to_value(payload).expect("payload serializes"))
            .expect("payload decodes");
    assert!((decoded.segments[0].words.as_ref().unwrap()[0].end - 0.8).abs() < f64::EPSILON);
}
