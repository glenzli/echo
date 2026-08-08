//! Facade contracts for the domain crate: identity round-trips and the
//! progressive-level order that job planning depends on.

use crate::{
    ALL_ANALYSIS_LEVELS, AnalysisKind, AnalysisLevel, AnalysisRecord, AssetId, ContentHash,
    ModelIdentity, OriginalRef,
};
use std::str::FromStr;

#[test]
fn asset_id_round_trips_through_string() {
    let id = AssetId::new();
    let parsed = AssetId::from_str(&id.to_string()).expect("own serialized id parses");
    assert_eq!(parsed, id);
}

#[test]
fn content_hash_displays_lowercase_hex() {
    let hash = ContentHash::new([0xab; 32]);
    let text = hash.to_string();
    assert_eq!(text.len(), 64);
    assert!(text.chars().all(|character| character.is_ascii_hexdigit()));
}

#[test]
fn analysis_levels_ascend_metadata_to_contextual() {
    let levels = ALL_ANALYSIS_LEVELS;
    for pair in levels.windows(2) {
        assert!(pair[0] < pair[1], "levels must be strictly ordered");
    }
    assert_eq!(levels.first(), Some(&AnalysisLevel::Metadata));
    assert_eq!(levels.last(), Some(&AnalysisLevel::Contextual));
}

#[test]
fn analysis_record_carries_model_evidence() {
    let record = AnalysisRecord::new(
        AnalysisKind::Emotions,
        serde_json::json!({ "emotion": "happy" }),
        ModelIdentity::new("sensevoice".to_owned(), "small".to_owned()),
        Some(0.9),
        1_700_000_000_000,
    );
    assert_eq!(record.model.name, "sensevoice");
    assert_eq!(record.model.version, "small");
    assert_eq!(record.confidence, Some(0.9));
}

#[test]
fn original_keeps_content_hash_stable() {
    let original = OriginalRef {
        path: "/tmp/recording.m4a".into(),
        content_hash: ContentHash::new([1; 32]),
        path_status: crate::AssetPathStatus::Present,
        size_bytes: 4096,
        codec: None,
        duration_millis: None,
        recorded_at_millis: None,
        imported_at_millis: 1_700_000_000_000,
    };
    assert_eq!(original.content_hash.as_bytes(), &[1; 32]);
}
