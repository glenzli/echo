use super::{
    CONTEXTUAL_SCHEMA_VERSION, ContextualPayload, decode_contextual_outline_output,
    decode_contextual_output,
};

#[test]
fn contextual_evidence_keeps_optional_product_fields() {
    let payload: ContextualPayload = serde_json::from_value(serde_json::json!({
        "summary": "雨声中的对话",
        "keywords": ["雨声", "对话"]
    }))
    .expect("payload parses");

    assert_eq!(payload.schema_version, 1);
    assert!(payload.sound_caption.is_empty());
    assert!(!payload.is_current());
    assert_eq!(payload.summary, "雨声中的对话");
    assert_eq!(payload.keywords.len(), 2);
    assert!(payload.mood.is_none());
}

#[test]
fn model_output_requires_the_complete_exact_schema() {
    let payload = decode_contextual_output(
        r#"{
            "schema_version":3,
            "sound_caption":"雨夜窗边的轻声交谈",
            "summary":"",
            "keywords":["雨声","对话"],
            "mood":"平静",
            "place_hint":null,
            "event_type":"conversation",
            "people_hints":[]
        }"#,
        "雨夜里两个人正在窗边说话，外面一直下着雨。",
    )
    .expect("strict payload parses");
    assert_eq!(payload.schema_version, CONTEXTUAL_SCHEMA_VERSION);
    assert_eq!(payload.sound_caption, "雨夜窗边的轻声交谈");
    assert!(payload.is_current());
    assert!(payload.summary.is_empty());
    assert_eq!(payload.keywords, ["雨声", "对话"]);

    let missing = decode_contextual_output(
        r#"{"schema_version":3,"sound_caption":"brief scene","summary":"","keywords":["x"],"mood":null,"place_hint":null,"event_type":null}"#,
        "example transcript",
    )
    .expect_err("missing field is rejected");
    assert_eq!(missing.code, "unexpected_fields");

    let extra = decode_contextual_output(
        r#"{"schema_version":3,"sound_caption":"brief scene","summary":"","keywords":["x"],"mood":null,"place_hint":null,"event_type":null,"people_hints":[],"confidence":1}"#,
        "example transcript",
    )
    .expect_err("extra field is rejected");
    assert_eq!(extra.code, "unexpected_fields");
}

#[test]
fn model_output_rejects_markdown_and_unbounded_fields() {
    let fenced = decode_contextual_output(
        "```json\n{\"schema_version\":3,\"sound_caption\":\"brief scene\",\"summary\":\"\",\"keywords\":[\"x\"],\"mood\":null,\"place_hint\":null,\"event_type\":null,\"people_hints\":[]}\n```",
        "example transcript",
    )
    .expect_err("markdown is not repaired");
    assert_eq!(fenced.code, "invalid_json");

    let too_many = serde_json::json!({
        "schema_version": 3,
        "sound_caption": "Rain at the window",
        "summary": "",
        "keywords": ["1", "2", "3", "4", "5", "6", "7", "8", "9"],
        "mood": null,
        "place_hint": null,
        "event_type": null,
        "people_hints": []
    });
    let error = decode_contextual_output(
        &too_many.to_string(),
        "Rain keeps falling outside the window.",
    )
    .expect_err("keyword bound is enforced");
    assert_eq!(error.code, "invalid_keyword_count");
}

#[test]
fn model_output_allows_keywords_to_be_empty_when_evidence_is_absent() {
    let payload = decode_contextual_output(
        r#"{"schema_version":3,"sound_caption":"Silence before voices","summary":"","keywords":[],"mood":null,"place_hint":null,"event_type":null,"people_hints":[]}"#,
        "There is a quiet pause before speech resumes.",
    )
    .expect("absence remains empty instead of being invented");
    assert!(payload.keywords.is_empty());
}

#[test]
fn sound_caption_rejects_language_drift_meta_and_copy_then_hard_compacts_length() {
    let base = serde_json::json!({
        "schema_version": 3,
        "sound_caption": "清晨胡同里的自行车铃",
        "summary": "",
        "keywords": ["自行车", "铃声"],
        "mood": "平静",
        "place_hint": "胡同",
        "event_type": "bicycle bell",
        "people_hints": []
    });
    decode_contextual_output(
        &base.to_string(),
        "您听，这个胡同口早上自行车铃一响，卖早点的吆喝声就跟着过来了。",
    )
    .expect("concise sound sketch is accepted");

    for (caption, expected) in [
        (
            "A bicycle bell in the morning",
            "sound_caption_language_mismatch",
        ),
        (
            "这段录音描述了清晨胡同里的自行车铃",
            "sound_caption_meta_language",
        ),
        ("胡同口早上自行车铃", "sound_caption_copies_transcript"),
    ] {
        let mut value = base.clone();
        value["sound_caption"] = serde_json::Value::String(caption.to_owned());
        let error = decode_contextual_output(
            &value.to_string(),
            "您听，这个胡同口早上自行车铃一响，卖早点的吆喝声就跟着过来了。",
        )
        .expect_err("invalid caption is rejected");
        assert_eq!(error.code, expected);
    }

    let mut long = base;
    long["sound_caption"] = serde_json::Value::String(
        "清晨胡同深处一辆自行车缓缓经过随后铃声与远处早点摊的吆喝持续交织在一起".into(),
    );
    let payload = decode_contextual_output(
        &long.to_string(),
        "您听，这个胡同口早上自行车铃一响，卖早点的吆喝声就跟着过来了。",
    )
    .expect("valid generated meaning is compacted to the card presentation bound");
    assert_eq!(payload.sound_caption.chars().count(), 14);
}

#[test]
fn outline_caption_may_equal_repeated_child_evidence() {
    let output = serde_json::json!({
        "schema_version": 3,
        "sound_caption": "Morning alley sounds",
        "summary": "",
        "keywords": ["alley"],
        "mood": null,
        "place_hint": null,
        "event_type": null,
        "people_hints": []
    });
    let payload = decode_contextual_outline_output(
        &output.to_string(),
        "Morning alley sounds\nMorning alley sounds\nMorning alley sounds",
    )
    .expect("an honest repeated outline can retain its shared child caption");
    assert_eq!(payload.sound_caption, "Morning alley sounds");

    let ordinary = decode_contextual_output(&output.to_string(), "Morning alley sounds")
        .expect_err("leaf captions still cannot copy a transcript");
    assert_eq!(ordinary.code, "sound_caption_copies_transcript");
}

#[test]
fn summary_is_optional_and_must_compress_long_source_text() {
    let short_source = "A bicycle bell rings while a breakfast seller calls from the alley.";
    let short = serde_json::json!({
        "schema_version": 3,
        "sound_caption": "Morning alley sounds",
        "summary": "A bell and a seller are heard.",
        "keywords": [],
        "mood": null,
        "place_hint": null,
        "event_type": null,
        "people_hints": []
    });
    let payload = decode_contextual_output(&short.to_string(), short_source)
        .expect("an unnecessary short-source summary does not invalidate other evidence");
    assert!(payload.summary.is_empty());

    let long_source = "A bicycle bell rings in the alley while a breakfast seller calls out. Children answer from a courtyard, rain falls on an awning, footsteps pass, and a distant bus starts before the street gradually becomes quiet again.";
    let mut long = short;
    long["summary"] = serde_json::Value::String("Rainy alley wakes with bells and voices.".into());
    let payload = decode_contextual_output(&long.to_string(), long_source)
        .expect("a substantially shorter summary is useful");
    assert_eq!(payload.summary, "Rainy alley wakes with bells and voices.");

    long["summary"] = serde_json::Value::String(
        "A bicycle bell rings while a seller calls and children answer as rain falls nearby."
            .into(),
    );
    let payload = decode_contextual_output(&long.to_string(), long_source)
        .expect("near-body-length summary is discarded without losing other evidence");
    assert!(payload.summary.is_empty());

    long["summary"] = serde_json::Value::String(
        "A genuinely concise summary that nevertheless contains far too many words to fit the detail presentation contract comfortably.".into(),
    );
    let doubled_source = format!("{long_source} {long_source}");
    let payload = decode_contextual_output(&long.to_string(), &doubled_source)
        .expect("useful long-source summary is hard-compacted for presentation");
    assert_eq!(payload.summary.split_whitespace().count(), 16);
}
