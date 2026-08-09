use super::{ContextualPayload, decode_contextual_output};

#[test]
fn contextual_evidence_keeps_optional_product_fields() {
    let payload: ContextualPayload = serde_json::from_value(serde_json::json!({
        "summary": "雨声中的对话",
        "keywords": ["雨声", "对话"]
    }))
    .expect("payload parses");

    assert_eq!(payload.summary, "雨声中的对话");
    assert_eq!(payload.keywords.len(), 2);
    assert!(payload.mood.is_none());
}

#[test]
fn model_output_requires_the_complete_exact_schema() {
    let payload = decode_contextual_output(
        r#"{
            "summary":"雨声中的对话",
            "keywords":["雨声","对话"],
            "mood":"平静",
            "place_hint":null,
            "event_type":"conversation",
            "people_hints":[]
        }"#,
    )
    .expect("strict payload parses");
    assert_eq!(payload.summary, "雨声中的对话");
    assert_eq!(payload.keywords, ["雨声", "对话"]);

    let missing = decode_contextual_output(
        r#"{"summary":"x","keywords":["x"],"mood":null,"place_hint":null,"event_type":null}"#,
    )
    .expect_err("missing field is rejected");
    assert_eq!(missing.code, "unexpected_fields");

    let extra = decode_contextual_output(
        r#"{"summary":"x","keywords":["x"],"mood":null,"place_hint":null,"event_type":null,"people_hints":[],"confidence":1}"#,
    )
    .expect_err("extra field is rejected");
    assert_eq!(extra.code, "unexpected_fields");
}

#[test]
fn model_output_rejects_markdown_and_unbounded_fields() {
    let fenced = decode_contextual_output(
        "```json\n{\"summary\":\"x\",\"keywords\":[\"x\"],\"mood\":null,\"place_hint\":null,\"event_type\":null,\"people_hints\":[]}\n```",
    )
    .expect_err("markdown is not repaired");
    assert_eq!(fenced.code, "invalid_json");

    let too_many = serde_json::json!({
        "summary": "x",
        "keywords": ["1", "2", "3", "4", "5", "6", "7", "8", "9"],
        "mood": null,
        "place_hint": null,
        "event_type": null,
        "people_hints": []
    });
    let error =
        decode_contextual_output(&too_many.to_string()).expect_err("keyword bound is enforced");
    assert_eq!(error.code, "invalid_keyword_count");
}
