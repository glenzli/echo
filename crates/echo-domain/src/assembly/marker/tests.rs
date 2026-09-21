use super::*;

#[test]
fn points_and_ranges_keep_composition_coordinates() {
    let point = AssemblyMarker::new(AssemblyMarkerId::new(), "门打开".into(), 750, None).unwrap();
    let encoded = serde_json::to_value(&point).unwrap();
    assert_eq!(encoded["startMillis"], 750);
    assert!(encoded.get("endMillis").is_none());
    let range =
        AssemblyMarker::new(AssemblyMarkerId::new(), "雨声".into(), 1000, Some(4000)).unwrap();
    let decoded: AssemblyMarker =
        serde_json::from_value(serde_json::to_value(&range).unwrap()).unwrap();
    assert_eq!(range, decoded);
    assert_eq!(decoded.end_millis(), Some(4000));
}

#[test]
fn malformed_serialized_markers_are_rejected() {
    let marker =
        AssemblyMarker::new(AssemblyMarkerId::new(), "Range".into(), 10, Some(20)).unwrap();
    for (key, value) in [
        ("name", serde_json::json!("   ")),
        ("name", serde_json::json!("界".repeat(121))),
        ("endMillis", serde_json::json!(10)),
        ("endMillis", serde_json::json!(9)),
        ("endMillis", serde_json::json!(14_400_001)),
        ("startMillis", serde_json::json!(14_400_001)),
    ] {
        let mut encoded = serde_json::to_value(&marker).unwrap();
        encoded[key] = value;
        let decoded: AssemblyMarker = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.validate(), Err(SoundAssemblyError::InvalidMarker));
    }
}
