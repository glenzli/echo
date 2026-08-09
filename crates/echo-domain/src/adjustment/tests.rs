use super::*;

#[test]
fn graph_preserves_authored_millisecond_and_centibel_units() {
    let graph =
        AdjustmentGraph::new(10_000, 1_000, 9_000, 250, 500, -350).expect("valid graph builds");
    assert_eq!(graph.trim_start_millis(), 1_000);
    assert_eq!(graph.trim_end_millis(), 9_000);
    assert_eq!(graph.fade_in_millis(), 250);
    assert_eq!(graph.fade_out_millis(), 500);
    assert_eq!(graph.gain_centibels(), -350);
}

#[test]
fn graph_rejects_out_of_source_and_overlapping_envelopes() {
    assert_eq!(
        AdjustmentGraph::new(1_000, 900, 1_100, 0, 0, 0),
        Err(AdjustmentGraphError::InvalidTrimRange)
    );
    assert_eq!(
        AdjustmentGraph::new(1_000, 100, 900, 500, 400, 0),
        Err(AdjustmentGraphError::OverlappingFades)
    );
    assert_eq!(
        AdjustmentGraph::new(1_000, 0, 1_000, 0, 0, MAX_GAIN_CENTIBELS + 1),
        Err(AdjustmentGraphError::GainOutOfRange)
    );
}
