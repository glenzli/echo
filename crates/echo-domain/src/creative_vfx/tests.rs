use super::*;

#[test]
fn defaults_are_disabled_and_valid() {
    let settings = CreativeVfxSettings::default();
    settings.validate().expect("default VFX settings validate");
    assert!(!settings.scene.enabled);
    assert!(!settings.delay.enabled);
    assert!(!settings.modulation.enabled);
    assert!(!settings.transform.enabled);
}

#[test]
fn stable_character_wire_values_fail_closed() {
    assert_eq!(SceneVfxCharacter::Underwater.wire_value(), 4);
    assert_eq!(DelayVfxCharacter::Echo.wire_value(), 1);
    assert_eq!(ModulationVfxCharacter::Tremolo.wire_value(), 3);
    assert_eq!(TransformVfxCharacter::Ghost.wire_value(), 4);
    assert!(SceneVfxCharacter::from_wire_value(5).is_err());
    assert!(DelayVfxCharacter::from_wire_value(2).is_err());
    assert!(ModulationVfxCharacter::from_wire_value(4).is_err());
    assert!(TransformVfxCharacter::from_wire_value(5).is_err());
}

#[test]
fn family_ranges_are_validated_independently() {
    let mut settings = CreativeVfxSettings::default();
    settings.scene.intensity_percent = 101;
    assert_eq!(settings.validate(), Err(CreativeVfxSettingsError::Scene));

    let mut settings = CreativeVfxSettings::default();
    settings.delay.echo.feedback_percent = 91;
    assert_eq!(settings.validate(), Err(CreativeVfxSettingsError::Delay));

    let mut settings = CreativeVfxSettings::default();
    settings.modulation.phaser.sweep_low_hertz = 3_000;
    assert_eq!(
        settings.validate(),
        Err(CreativeVfxSettingsError::Modulation)
    );

    let mut settings = CreativeVfxSettings::default();
    settings.transform.amount_percent = 101;
    assert_eq!(
        settings.validate(),
        Err(CreativeVfxSettingsError::Transform)
    );
}

#[test]
fn aggregate_json_preserves_typed_family_settings() {
    let settings = CreativeVfxSettings {
        scene: SceneVfxSettings {
            character: SceneVfxCharacter::Radio,
            enabled: true,
            ..SceneVfxSettings::default()
        },
        delay: DelayVfxSettings {
            character: DelayVfxCharacter::Echo,
            enabled: true,
            ..DelayVfxSettings::default()
        },
        modulation: ModulationVfxSettings {
            character: ModulationVfxCharacter::Phaser,
            enabled: true,
            ..ModulationVfxSettings::default()
        },
        transform: TransformVfxSettings {
            character: TransformVfxCharacter::Giant,
            enabled: true,
            ..TransformVfxSettings::default()
        },
    };
    let encoded = serde_json::to_string(&settings).expect("settings encode");
    let decoded: CreativeVfxSettings = serde_json::from_str(&encoded).expect("settings decode");
    assert_eq!(decoded, settings);
}
