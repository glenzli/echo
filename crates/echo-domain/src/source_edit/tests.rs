use super::*;

#[test]
fn timeline_wire_is_camel_case_with_numeric_recoverable_states() {
    let timeline = EditTimeline::new(
        1_000,
        3_000,
        vec![
            EditSegment::new(
                1_000,
                2_000,
                EditSegmentState::Audible,
                -125,
                20,
                30,
                FadeCurves::new(FadeCurve::Smooth, FadeCurve::EqualPower),
                250,
            )
            .expect("audible segment validates"),
            EditSegment::new(
                2_000,
                3_000,
                EditSegmentState::Hidden,
                0,
                0,
                0,
                FadeCurves::linear(),
                0,
            )
            .expect("hidden segment validates"),
        ],
    )
    .expect("continuous timeline validates");

    assert_eq!(timeline.output_duration_millis(), 1_250);
    assert_eq!(EditSegmentState::Muted.wire_value(), 1);
    assert_eq!(
        EditSegmentState::from_wire_value(2),
        Ok(EditSegmentState::Hidden)
    );
    assert_eq!(
        EditSegmentState::from_wire_value(3),
        Err(EditSegmentStateValueError)
    );

    let encoded = serde_json::to_value(&timeline).expect("timeline encodes");
    assert_eq!(encoded["trimStartMillis"], 1_000);
    assert_eq!(encoded["segments"][0]["sourceStartMillis"], 1_000);
    assert_eq!(encoded["segments"][1]["state"], 2);
    assert_eq!(encoded["segments"][0]["fadeInCurve"], "smooth");
    assert_eq!(
        serde_json::from_value::<EditTimeline>(encoded).expect("timeline decodes"),
        timeline
    );
}

#[test]
fn timeline_enforces_continuity_segment_count_and_non_empty_output() {
    let segment = |start, end, state, gap| {
        EditSegment::new(start, end, state, 0, 0, 0, FadeCurves::linear(), gap)
            .expect("fixture segment validates")
    };
    assert_eq!(
        EditTimeline::new(
            0,
            2_000,
            vec![
                segment(0, 900, EditSegmentState::Audible, 0),
                segment(1_000, 2_000, EditSegmentState::Audible, 0),
            ],
        ),
        Err(EditTimelineError::DiscontinuousSegments)
    );
    assert_eq!(
        EditTimeline::new(
            0,
            1_000,
            vec![segment(0, 1_000, EditSegmentState::Hidden, 0)],
        ),
        Err(EditTimelineError::EmptyOutput)
    );
    assert_eq!(
        EditSegment::new(
            0,
            1_000,
            EditSegmentState::Audible,
            0,
            0,
            0,
            FadeCurves::linear(),
            MAX_EDIT_GAP_MILLIS + 1,
        ),
        Err(EditTimelineError::GapOutOfRange)
    );

    let too_many = (0..=MAX_EDIT_SEGMENTS)
        .map(|index| segment(index as u64, index as u64 + 1, EditSegmentState::Audible, 0))
        .collect();
    assert_eq!(
        EditTimeline::new(0, MAX_EDIT_SEGMENTS as u64 + 1, too_many),
        Err(EditTimelineError::InvalidSegmentCount)
    );
}

#[test]
fn effect_mask_wire_and_structural_bounds_are_stable() {
    let mask = EffectMask::new(
        2_000,
        6_000,
        DEFAULT_EFFECT_MASK_FEATHER_MILLIS,
        vec![EffectNodeKind::Equalizer, EffectNodeKind::Dynamics],
    )
    .expect("mask validates");
    let encoded = serde_json::to_value(&mask).expect("mask encodes");
    assert_eq!(encoded["startMillis"], 2_000);
    assert_eq!(encoded["effectNodes"][0], "equalizer");
    assert_eq!(
        serde_json::from_value::<EffectMask>(encoded).expect("mask decodes"),
        mask
    );
    let restored_default: EffectMask = serde_json::from_value(serde_json::json!({
        "startMillis": 2_000,
        "endMillis": 6_000,
        "effectNodes": ["equalizer"]
    }))
    .expect("omitted feather restores the authored default");
    assert_eq!(
        restored_default.feather_millis(),
        DEFAULT_EFFECT_MASK_FEATHER_MILLIS
    );

    assert_eq!(
        EffectMask::new(
            2_000,
            3_000,
            MAX_EFFECT_MASK_FEATHER_MILLIS + 1,
            vec![EffectNodeKind::Equalizer],
        ),
        Err(EffectMaskError::FeatherOutOfRange)
    );
    assert_eq!(
        EffectMask::new(
            2_000,
            3_000,
            10,
            vec![EffectNodeKind::Equalizer, EffectNodeKind::Equalizer],
        ),
        Err(EffectMaskError::DuplicateEffectNode)
    );
    assert_eq!(
        EffectMask::new(2_000, 3_000, 10, vec![EffectNodeKind::Master]),
        Err(EffectMaskError::UnsupportedEffectNode)
    );
    assert_eq!(
        EffectMask::new(2_000, 3_000, 10, vec![EffectNodeKind::DeClick]),
        Err(EffectMaskError::UnsupportedEffectNode)
    );
    assert_eq!(
        EffectMask::new(2_000, 3_000, 10, vec![EffectNodeKind::TransformVfx]),
        Err(EffectMaskError::UnsupportedEffectNode)
    );
    assert_eq!(
        EffectMask::new(2_000, 3_000, 10, vec![EffectNodeKind::FreezeVfx]),
        Err(EffectMaskError::UnsupportedEffectNode)
    );
    assert_eq!(
        EffectMask::new(2_000, 3_000, 10, vec![EffectNodeKind::GranularVfx]),
        Err(EffectMaskError::UnsupportedEffectNode)
    );
    assert_eq!(
        EffectMask::new(2_000, 3_000, 10, vec![EffectNodeKind::PitchVfx]),
        Err(EffectMaskError::UnsupportedEffectNode)
    );
}
