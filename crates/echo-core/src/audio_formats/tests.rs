use super::*;

#[test]
fn discovery_and_picker_have_one_unambiguous_registry() {
    let unique: std::collections::BTreeSet<_> = AUDIO_EXTENSIONS.iter().collect();
    assert_eq!(unique.len(), AUDIO_EXTENSIONS.len());
    assert!(AUDIO_EXTENSIONS.iter().all(|value| {
        !value.is_empty()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    }));
    let patterns = audio_file_patterns();
    for extension in AUDIO_EXTENSIONS {
        assert!(
            patterns
                .split_whitespace()
                .any(|pattern| pattern == format!("*.{extension}"))
        );
    }
}

#[test]
fn uploads_use_case_insensitive_container_hints() {
    use std::path::Path;
    assert_eq!(
        audio_content_type(Path::new("VOICE.MKA")),
        "audio/x-matroska"
    );
    assert_eq!(audio_content_type(Path::new("voice.caf")), "audio/x-caf");
    assert_eq!(audio_content_type(Path::new("voice.AIFC")), "audio/aiff");
    assert_eq!(
        audio_content_type(Path::new("unknown")),
        "application/octet-stream"
    );
}
