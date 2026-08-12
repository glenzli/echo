use super::*;

fn model() -> MetadataFields {
    MetadataFields {
        sound_caption: "Rain at the window".into(),
        summary: "A quiet rainy morning".into(),
        event_type: "daily routine".into(),
        mood: "quiet".into(),
        keywords: vec!["rain".into(), "window".into()],
        transcript_text: "The rain is light.".into(),
        language: "en".into(),
    }
}

#[test]
fn sparse_overlay_preserves_future_model_fields() {
    let mut desired = model();
    desired.sound_caption = "Morning rain".into();
    desired.keywords = vec!["rain".into(), "kitchen".into()];
    let overlay = MetadataCalibration::between(model(), desired.clone()).expect("valid overlay");

    assert_eq!(
        overlay.calibrated_fields(),
        [MetadataField::SoundCaption, MetadataField::Keywords]
    );
    let mut new_model = model();
    new_model.summary = "A newer model summary".into();
    let effective = overlay.apply_to(new_model);
    assert_eq!(effective.sound_caption, desired.sound_caption);
    assert_eq!(effective.summary, "A newer model summary");
    assert_eq!(effective.keywords, desired.keywords);
}

#[test]
fn explicit_empty_value_is_a_real_user_correction() {
    let mut desired = model();
    desired.mood.clear();
    desired.keywords.clear();
    let overlay = MetadataCalibration::between(model(), desired).expect("valid overlay");

    assert_eq!(overlay.mood, Some(String::new()));
    assert_eq!(overlay.keywords, Some(Vec::new()));
}

#[test]
fn explicit_empty_value_survives_when_the_current_model_is_also_empty() {
    let mut model = model();
    model.summary.clear();
    let desired = model.clone();

    let overlay = MetadataCalibration::between_with_explicit_fields(
        model,
        desired,
        &[MetadataField::Summary],
    )
    .expect("valid explicit overlay");

    assert_eq!(overlay.summary, Some(String::new()));
    assert_eq!(
        MetadataField::from_wire_name("summary"),
        Some(MetadataField::Summary)
    );
    assert_eq!(MetadataField::from_wire_name("unknown"), None);
}

#[test]
fn normalization_deduplicates_keywords_and_bounds_payloads() {
    let normalized = MetadataFields {
        keywords: vec![" Rain ".into(), "rain".into(), " kitchen  sound ".into()],
        ..MetadataFields::default()
    }
    .normalized()
    .expect("metadata normalizes");
    assert_eq!(normalized.keywords, ["Rain", "kitchen sound"]);

    let oversized = MetadataFields {
        sound_caption: "x".repeat(MAX_METADATA_CAPTION_CHARACTERS + 1),
        ..MetadataFields::default()
    };
    assert!(oversized.normalized().is_err());
}
