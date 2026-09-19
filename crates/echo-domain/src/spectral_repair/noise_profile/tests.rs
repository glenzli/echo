use super::*;

fn profile() -> NoiseProfileSettings {
    NoiseProfileSettings {
        algorithm_version: 1,
        enabled: false,
        capture_start_millis: 100,
        capture_end_millis: 900,
        power_centibels: vec![-6000; 1025],
        reduction_centibels: 1200,
        sensitivity_centibels: 600,
        smoothing_bins: 3,
    }
}

#[test]
fn captured_evidence_round_trips_while_bypassed() {
    let settings = profile();
    assert!(settings.validate(1000).is_ok());
    let encoded = serde_json::to_string(&settings).unwrap();
    assert_eq!(
        serde_json::from_str::<NoiseProfileSettings>(&encoded).unwrap(),
        settings
    );
}

#[test]
fn rejects_unsupported_or_incomplete_noise_evidence() {
    let mut settings = profile();
    settings.power_centibels.pop();
    assert!(settings.validate(1000).is_err());
    settings = profile();
    settings.algorithm_version = 2;
    assert!(settings.validate(1000).is_err());
    settings = profile();
    settings.capture_end_millis = 1100;
    assert!(settings.validate(1000).is_err());
    settings = profile();
    settings.capture_end_millis = 199;
    assert!(settings.validate(1000).is_err());
    settings = profile();
    settings.power_centibels[1024] = -14401;
    assert!(settings.validate(1000).is_err());
    settings = profile();
    settings.smoothing_bins = 9;
    assert!(settings.validate(1000).is_err());
}
