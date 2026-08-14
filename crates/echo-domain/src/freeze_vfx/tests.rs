use super::*;

#[test]
fn default_is_disabled_and_carries_canonical_capture_contract() {
    let settings = FreezeVfxSettings::default();
    assert!(!settings.enabled);
    assert_eq!(settings.mix_percent, 70);
    assert_eq!(settings.capture_source_millis, 100);
    assert!(settings.is_valid());
}

#[test]
fn enabled_capture_requires_complete_canonical_pre_roll() {
    let mut settings = FreezeVfxSettings {
        enabled: true,
        capture_source_millis: FREEZE_CAPTURE_PRE_ROLL_MILLIS - 1,
        ..FreezeVfxSettings::default()
    };
    assert!(!settings.is_valid());
    settings.capture_source_millis = FREEZE_CAPTURE_PRE_ROLL_MILLIS;
    assert!(settings.is_valid());
    settings.mix_percent = 101;
    assert!(!settings.is_valid());
}
