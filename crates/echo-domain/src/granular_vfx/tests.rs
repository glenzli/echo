use super::*;

#[test]
fn default_matches_the_canonical_audio_processor_contract() {
    let settings = GranularVfxSettings::default();
    assert!(!settings.enabled);
    assert_eq!(settings.mix_percent, 45);
    assert_eq!(settings.grain_millis, 80);
    assert_eq!(settings.density_tenths_hertz, 120);
    assert_eq!(settings.lookback_millis, 250);
    assert_eq!(settings.scatter_millis, 120);
    assert_eq!(settings.pitch_cents, 0);
    assert_eq!(settings.stereo_spread_percent, 50);
    assert_eq!(settings.random_seed, 0x4543_484F);
    assert!(settings.is_valid());
}

#[test]
fn ranges_and_two_second_history_budget_fail_closed() {
    let mut settings = GranularVfxSettings {
        grain_millis: 19,
        ..GranularVfxSettings::default()
    };
    assert!(!settings.is_valid());

    settings = GranularVfxSettings {
        lookback_millis: 1_400,
        scatter_millis: 200,
        grain_millis: 250,
        pitch_cents: 1_200,
        ..GranularVfxSettings::default()
    };
    assert!(!settings.is_valid());

    settings.pitch_cents = -1_200;
    assert!(settings.is_valid());
}
