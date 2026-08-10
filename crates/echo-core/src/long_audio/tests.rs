use super::*;

#[test]
fn leaf_plan_is_contiguous_and_runtime_bounded() {
    let plan = plan_segments(1_100_000);
    assert_eq!(plan.len(), 3);
    assert_eq!(plan[0].start_millis, 0);
    assert_eq!(plan[0].end_millis, 480_000);
    assert_eq!(plan[1].start_millis, plan[0].end_millis);
    assert_eq!(plan[2].end_millis, 1_100_000);
    for segment in plan {
        let pcm_bytes = (segment.end_millis - segment.start_millis) * 16_000 * 2 / 1000 + 44;
        assert!(pcm_bytes < crate::MAX_AUDIO_UPLOAD_BYTES);
    }
}

#[test]
fn direct_audio_duration_matches_the_event_runtime_ceiling() {
    let asset = |duration_millis| {
        AudioAsset::new(
            AssetId::new(),
            echo_domain::OriginalRef {
                path: "fixture.wav".into(),
                content_hash: echo_domain::ContentHash::new([9; 32]),
                path_status: echo_domain::AssetPathStatus::Present,
                size_bytes: 1_024,
                codec: Some("pcm".to_owned()),
                duration_millis: Some(duration_millis),
                recorded_at_millis: None,
                imported_at_millis: 1,
            },
            echo_domain::AnalysisLevel::Asr,
        )
    };

    assert!(!requires_segmentation(&asset(600_000)));
    assert!(requires_segmentation(&asset(600_001)));
}

#[test]
fn aggregate_offsets_leaf_timestamps_to_original_time() {
    let transcript = TranscriptPayload {
        model: "audio.transcribe".to_owned(),
        language: Some("zh".to_owned()),
        text: "second".to_owned(),
        segments: vec![crate::TranscriptSegment {
            text: "second".to_owned(),
            start: 1.0,
            end: 2.0,
            words: None,
        }],
        runtime: None,
    };
    let segment = LongAudioSegment {
        asset_id: AssetId::new(),
        plan_version: 1,
        index: 1,
        start_millis: 480_000,
        end_millis: 960_000,
        proxy: None,
        transcript: Some(serde_json::to_value(transcript).unwrap()),
        alignment: None,
        contextual: None,
        updated_at_millis: 0,
    };
    let aggregate = aggregate_transcript(&[segment]).expect("aggregate");
    assert_eq!(aggregate.text, "second");
    assert!((aggregate.segments[0].start - 481.0).abs() < f64::EPSILON);
    assert!((aggregate.segments[0].end - 482.0).abs() < f64::EPSILON);
}
