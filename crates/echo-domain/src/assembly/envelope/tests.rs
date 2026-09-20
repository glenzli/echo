use super::*;

#[test]
fn bypass_preserves_points_but_does_not_allow_malformed_coordinates() {
    let mut curve = GainEnvelope {
        enabled: false,
        points: vec![
            GainEnvelopePoint {
                source_millis: 1200,
                gain_centibels: 0,
            },
            GainEnvelopePoint {
                source_millis: 1600,
                gain_centibels: -1800,
            },
        ],
    };
    assert!(curve.validate().is_ok());
    assert!(!curve.is_empty());
    let saved = serde_json::to_string(&curve).unwrap();
    assert_eq!(serde_json::from_str::<GainEnvelope>(&saved).unwrap(), curve);
    curve.points[1].source_millis = 1200;
    assert_eq!(
        curve.validate(),
        Err(SoundAssemblyError::InvalidGainEnvelope)
    );
    curve.points[1].source_millis = 1600;
    curve.points[1].gain_centibels = 1201;
    assert!(curve.validate().is_err());
}
