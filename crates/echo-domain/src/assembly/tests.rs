use super::*;

#[test]
fn markers_preserve_legacy_bytes_and_do_not_extend_audio() {
    let track = AssemblyTrack::new(
        AssemblyTrackId::new(),
        "Track".into(),
        0,
        0,
        false,
        false,
        vec![clip(AssemblyClipId::new(), 0)],
    )
    .unwrap();
    let legacy = SoundAssembly::new(
        SoundAssemblyId::new(),
        "Project".into(),
        AssemblyMaster::standard(),
        vec![track],
    )
    .unwrap();
    let old_bytes = serde_json::to_string(&legacy).unwrap();
    assert!(!old_bytes.contains("markers"));
    let loaded: SoundAssembly = serde_json::from_str(&old_bytes).unwrap();
    assert!(loaded.markers().is_empty());
    assert_eq!(serde_json::to_string(&loaded).unwrap(), old_bytes);
    let marker = AssemblyMarker::new(
        crate::AssemblyMarkerId::new(),
        "Later".into(),
        8000,
        Some(9000),
    )
    .unwrap();
    let annotated = legacy.clone().with_markers(vec![marker.clone()]).unwrap();
    assert_eq!(annotated.duration_millis(), 4000);
    annotated.validate().unwrap();
    assert_eq!(
        legacy
            .clone()
            .with_markers(vec![marker.clone(), marker.clone()]),
        Err(SoundAssemblyError::InvalidMarker)
    );
    assert_eq!(
        legacy.with_markers(vec![marker; 257]),
        Err(SoundAssemblyError::InvalidMarker)
    );
    let mut encoded = serde_json::to_value(&annotated).unwrap();
    encoded["markers"][0]["endMillis"] = 7000.into();
    assert_eq!(
        serde_json::from_value::<SoundAssembly>(encoded)
            .unwrap()
            .validate(),
        Err(SoundAssemblyError::InvalidMarker)
    );
}

fn clip(id: AssemblyClipId, timeline_start_millis: u64) -> AssemblyClip {
    AssemblyClip::new(
        id,
        AssetId::new(),
        0,
        0,
        4_000,
        timeline_start_millis,
        0,
        0,
        250,
        250,
        FadeCurve::EqualPower,
        FadeCurve::EqualPower,
        false,
    )
    .expect("clip")
}

#[test]
fn assembly_preserves_independent_track_and_clip_time() {
    let first = clip(AssemblyClipId::new(), 0);
    let second = clip(AssemblyClipId::new(), 1_500);
    let track = AssemblyTrack::new(
        AssemblyTrackId::new(),
        "Ambience".to_owned(),
        -300,
        -25,
        false,
        false,
        vec![first, second],
    )
    .expect("track");
    let assembly = SoundAssembly::new(
        SoundAssemblyId::new(),
        "Sound postcard".to_owned(),
        AssemblyMaster::standard(),
        vec![track],
    )
    .expect("assembly");

    assert_eq!(assembly.clip_count(), 2);
    assert_eq!(assembly.duration_millis(), 5_500);
    assert_eq!(
        assembly.tracks()[0].clips()[1].timeline_start_millis(),
        1_500
    );
}

#[test]
fn duplicate_clip_identity_is_rejected_across_tracks() {
    let id = AssemblyClipId::new();
    let first = AssemblyTrack::new(
        AssemblyTrackId::new(),
        "Voice".to_owned(),
        0,
        0,
        false,
        false,
        vec![clip(id, 0)],
    )
    .expect("first track");
    let second = AssemblyTrack::new(
        AssemblyTrackId::new(),
        "Room".to_owned(),
        0,
        0,
        false,
        false,
        vec![clip(id, 0)],
    )
    .expect("second track");

    assert_eq!(
        SoundAssembly::new(
            SoundAssemblyId::new(),
            "Duplicate".to_owned(),
            AssemblyMaster::standard(),
            vec![first, second],
        ),
        Err(SoundAssemblyError::DuplicateClipIdentity)
    );
}

#[test]
fn fades_and_timeline_are_bounded() {
    assert_eq!(
        AssemblyClip::new(
            AssemblyClipId::new(),
            AssetId::new(),
            0,
            0,
            1_000,
            0,
            0,
            0,
            700,
            400,
            FadeCurve::Linear,
            FadeCurve::Smooth,
            false,
        ),
        Err(SoundAssemblyError::OverlappingClipFades)
    );
    assert_eq!(
        AssemblyClip::new(
            AssemblyClipId::new(),
            AssetId::new(),
            0,
            0,
            1_000,
            MAX_ASSEMBLY_DURATION_MILLIS,
            0,
            0,
            0,
            0,
            FadeCurve::Linear,
            FadeCurve::Linear,
            false,
        ),
        Err(SoundAssemblyError::DurationOutOfRange)
    );
}

#[test]
fn serialized_master_cannot_bypass_the_output_contract() {
    let assembly = SoundAssembly::new(
        SoundAssemblyId::new(),
        "Master contract".to_owned(),
        AssemblyMaster::standard(),
        vec![
            AssemblyTrack::new(
                AssemblyTrackId::new(),
                "Track".to_owned(),
                0,
                0,
                false,
                false,
                vec![clip(AssemblyClipId::new(), 0)],
            )
            .expect("track"),
        ],
    )
    .expect("assembly");
    let mut value = serde_json::to_value(assembly).expect("assembly encodes");
    value["master"]["limiterCeilingCentibels"] = serde_json::json!(100);
    let decoded: SoundAssembly = serde_json::from_value(value).expect("shape decodes");

    assert_eq!(decoded.validate(), Err(SoundAssemblyError::InvalidLimiter));
}

#[test]
fn legacy_clips_round_trip_without_an_envelope_and_new_curves_validate() {
    let original = clip(AssemblyClipId::new(), 0);
    let json = serde_json::to_value(&original).expect("serialize");
    assert!(json.get("gainEnvelope").is_none());
    let reopened: AssemblyClip = serde_json::from_value(json.clone()).expect("legacy");
    assert_eq!(original, reopened);
    let mut authored = json;
    authored["gainEnvelope"] = serde_json::json!({"enabled": true, "points": [
        {"sourceMillis": 0, "gainCentibels": 0}, {"sourceMillis": 1500, "gainCentibels": -1800}
    ]});
    let reopened: AssemblyClip = serde_json::from_value(authored.clone()).expect("curve");
    assert!(reopened.validate().is_ok());
    assert_eq!(serde_json::to_value(reopened).expect("save"), authored);
    authored["gainEnvelope"]["points"][1]["sourceMillis"] = serde_json::json!(0);
    let malformed: AssemblyClip = serde_json::from_value(authored).expect("malformed");
    assert_eq!(
        malformed.validate(),
        Err(SoundAssemblyError::InvalidGainEnvelope)
    );
}
