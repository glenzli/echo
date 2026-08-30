use super::*;

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
