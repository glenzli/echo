use super::*;
#[test]
fn compact_declaration_is_deterministic_and_bounded() {
    use SourceDisclosureKind::{AiGenerated, AiProcessed, ReconstructedSpeech};
    let text =
        encode_portable_disclosure([ReconstructedSpeech, AiGenerated, AiGenerated, AiProcessed]);
    assert!(text.len() < MAX_BYTES);
    assert_eq!(
        decode_portable_disclosure(&text),
        Some(vec![AiProcessed, AiGenerated, ReconstructedSpeech])
    );
    assert_eq!(
        text,
        encode_portable_disclosure([AiGenerated, AiProcessed, ReconstructedSpeech])
    );
    assert_eq!(
        decode_portable_disclosure(&encode_portable_disclosure([])),
        Some(vec![])
    );
}
#[test]
fn foreign_future_duplicate_and_oversized_comments_are_ignored() {
    let text = encode_portable_disclosure([SourceDisclosureKind::AiGenerated]);
    for invalid in [
        "ai_generated".to_owned(),
        text.replace(".v1", ".v2"),
        text.replace("referenced_sources", "verified_recording"),
        text.replace("[\"ai_generated\"]", "[\"ai_generated\",\"ai_generated\"]"),
        format!("{text}{}", " ".repeat(MAX_BYTES)),
        text.replace("\"scope\"", "\"unknown\""),
    ] {
        assert_eq!(decode_portable_disclosure(&invalid), None, "{invalid}");
    }
}

#[test]
fn memory_comments_preserve_bounded_first_line_disclosure() {
    let text = encode_portable_disclosure([SourceDisclosureKind::AiGenerated]);
    let annotated = format!(
        "{text}\nNotes: {}\nPlace: 外婆家\nTime: Summer",
        "私人备注".repeat(500)
    );
    assert_eq!(
        decode_portable_disclosure(&annotated),
        Some(vec![SourceDisclosureKind::AiGenerated])
    );
    assert_eq!(
        decode_portable_disclosure(&format!("Notes: user text\n{text}")),
        None
    );
}
