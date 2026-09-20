use super::*;
use crate::{EditSegment, FadeCurves};

fn cut(hidden: bool, gap: u64) -> EditTimeline {
    let segments = [
        (0, 1000, EditSegmentState::Audible, 0),
        (
            1000,
            2000,
            if hidden {
                EditSegmentState::Hidden
            } else {
                EditSegmentState::Audible
            },
            gap,
        ),
        (2000, 4000, EditSegmentState::Audible, 0),
    ]
    .into_iter()
    .map(|(a, b, state, gap)| {
        EditSegment::new(a, b, state, 0, 0, 0, FadeCurves::linear(), gap).unwrap()
    })
    .collect();
    EditTimeline::new(0, 4000, segments).unwrap()
}

fn curve() -> GainEnvelope {
    GainEnvelope {
        enabled: false,
        points: vec![
            GainEnvelopePoint {
                source_millis: 0,
                gain_centibels: 0,
            },
            GainEnvelopePoint {
                source_millis: 4000,
                gain_centibels: -4000,
            },
        ],
    }
}

#[test]
fn hidden_speech_and_inserted_gap_keep_later_gain_on_the_same_original() {
    let old = EditTimeline::identity(0, 4000).unwrap();
    let mapped = curve().remap_source_edits(&old, &cut(true, 500)).unwrap();
    assert!(!mapped.enabled);
    assert_eq!(mapped.gain_at(500), -500);
    assert_eq!(mapped.gain_at(1500), -2000);
    assert_eq!(mapped.gain_at(2500), -3000);
    let restored = mapped.remap_source_edits(&cut(true, 500), &old).unwrap();
    assert_eq!(restored.gain_at(1500), 0);
    assert_eq!(restored.gain_at(3000), -3000);
}

#[test]
fn trims_rebase_points_and_unchanged_timelines_preserve_authored_bytes() {
    let old = EditTimeline::identity(0, 4000).unwrap();
    assert_eq!(curve().remap_source_edits(&old, &old).unwrap(), curve());
    assert_eq!(
        curve().remap_source_edits(&old, &cut(false, 0)).unwrap(),
        curve()
    );
    let new = EditTimeline::identity(1500, 3500).unwrap();
    let mapped = curve().remap_source_edits(&old, &new).unwrap();
    assert_eq!(mapped.gain_at(0), -1500);
    assert_eq!(mapped.gain_at(1000), -2500);
    assert!(mapped.points.iter().all(|p| p.source_millis <= 2000));
}
