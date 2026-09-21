use super::*;
#[test]
fn kinds_ranges_and_notes_are_bounded_without_inventing_authenticity() {
    let span = SourceDisclosureSpan {
        kind: SourceDisclosureKind::AiGenerated,
        start_millis: 100,
        end_millis: 500,
        note: "用户声明的补充雨声".into(),
    };
    assert!(validate_source_disclosures(std::slice::from_ref(&span), 1000).is_ok());
    assert!(validate_source_disclosures(&[], 0).is_ok());
    assert!(validate_source_disclosures(std::slice::from_ref(&span), 499).is_err());
    assert!(validate_source_disclosures(&vec![span.clone(); 65], 1000).is_err());
    assert!(SourceDisclosureKind::ReconstructedSpeech.is_generated());
    assert!(!SourceDisclosureKind::AiProcessed.is_generated());
    let json = serde_json::to_string(&span).unwrap();
    assert_eq!(
        serde_json::from_str::<SourceDisclosureSpan>(&json).unwrap(),
        span
    );
    assert!(
        serde_json::from_str::<SourceDisclosureSpan>(
            &json.replace("ai_generated", "verified_recording")
        )
        .is_err()
    );
    let mut invalid = span;
    invalid.note = "x".repeat(241);
    assert!(validate_source_disclosures(&[invalid], 1000).is_err());
}
