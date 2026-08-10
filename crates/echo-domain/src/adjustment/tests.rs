use super::*;

#[test]
fn graph_preserves_authored_millisecond_and_centibel_units() {
    let graph = AdjustmentGraph::new(
        10_000,
        1_000,
        9_000,
        250,
        500,
        AdjustmentEffects::new(
            FadeCurves::new(FadeCurve::Smooth, FadeCurve::EqualPower),
            -350,
            80,
        ),
    )
    .expect("valid graph builds");
    assert_eq!(graph.trim_start_millis(), 1_000);
    assert_eq!(graph.trim_end_millis(), 9_000);
    assert_eq!(graph.fade_in_millis(), 250);
    assert_eq!(graph.fade_out_millis(), 500);
    assert_eq!(graph.fade_in_curve(), FadeCurve::Smooth);
    assert_eq!(graph.fade_out_curve(), FadeCurve::EqualPower);
    assert_eq!(graph.gain_centibels(), -350);
    assert_eq!(graph.low_cut_hertz(), 80);
}

#[test]
fn fade_curve_catalog_values_are_stable_and_closed() {
    for curve in [FadeCurve::Linear, FadeCurve::Smooth, FadeCurve::EqualPower] {
        assert_eq!(
            FadeCurve::from_catalog_value(curve.catalog_value()),
            Ok(curve)
        );
    }
    assert_eq!(FadeCurve::from_catalog_value(3), Err(FadeCurveValueError));
}

#[test]
fn graph_rejects_out_of_source_and_overlapping_envelopes() {
    assert_eq!(
        AdjustmentGraph::new(1_000, 900, 1_100, 0, 0, AdjustmentEffects::default()),
        Err(AdjustmentGraphError::InvalidTrimRange)
    );
    assert_eq!(
        AdjustmentGraph::new(1_000, 100, 900, 500, 400, AdjustmentEffects::default()),
        Err(AdjustmentGraphError::OverlappingFades)
    );
    assert_eq!(
        AdjustmentGraph::new(
            1_000,
            0,
            1_000,
            0,
            0,
            AdjustmentEffects::new(FadeCurves::linear(), MAX_GAIN_CENTIBELS + 1, 0),
        ),
        Err(AdjustmentGraphError::GainOutOfRange)
    );
    assert_eq!(
        AdjustmentGraph::new(
            1_000,
            0,
            1_000,
            0,
            0,
            AdjustmentEffects::new(FadeCurves::linear(), 0, MIN_LOW_CUT_HERTZ - 1),
        ),
        Err(AdjustmentGraphError::LowCutOutOfRange)
    );
}
