use super::*;

#[test]
fn event_job_requires_a_valid_asset_identity() {
    let missing = asset_id_of(&serde_json::json!({})).expect_err("missing identity is rejected");
    assert!(missing.to_string().contains("event job lacks asset_id"));

    let malformed = asset_id_of(&serde_json::json!({ "asset_id": "not-an-asset" }))
        .expect_err("malformed identity is rejected");
    assert!(malformed.to_string().contains("bad asset id"));
}
