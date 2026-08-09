use super::ContextualPayload;

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
