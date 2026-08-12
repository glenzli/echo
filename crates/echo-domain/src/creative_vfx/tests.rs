use super::*;

#[test]
fn defaults_are_disabled_and_valid() {
    let settings = CreativeVfxSettings::default();
    settings.validate().expect("default VFX settings validate");
    assert!(!settings.scene.enabled);
    assert!(!settings.delay.enabled);
    assert!(!settings.modulation.enabled);
    assert!(!settings.transform.enabled);
    assert!(!settings.digital_degrade.enabled);
    assert!(!settings.drive.enabled);
    assert!(!settings.rotary.enabled);
}

#[test]
fn stable_character_wire_values_fail_closed() {
    assert_eq!(SceneVfxCharacter::Underwater.wire_value(), 4);
    assert_eq!(DelayVfxCharacter::Echo.wire_value(), 1);
    assert_eq!(ModulationVfxCharacter::Tremolo.wire_value(), 3);
    assert_eq!(TransformVfxCharacter::Ghost.wire_value(), 4);
    assert_eq!(DigitalDegradeVfxCharacter::LoFi.wire_value(), 2);
    assert_eq!(DriveVfxCharacter::Fuzz.wire_value(), 2);
    assert_eq!(RotaryVfxSpeed::Brake.wire_value(), 2);
    assert!(SceneVfxCharacter::from_wire_value(5).is_err());
    assert!(DelayVfxCharacter::from_wire_value(2).is_err());
    assert!(ModulationVfxCharacter::from_wire_value(4).is_err());
    assert!(TransformVfxCharacter::from_wire_value(5).is_err());
    assert!(DigitalDegradeVfxCharacter::from_wire_value(3).is_err());
    assert!(DriveVfxCharacter::from_wire_value(3).is_err());
    assert!(RotaryVfxSpeed::from_wire_value(3).is_err());
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

    let mut settings = CreativeVfxSettings::default();
    settings.digital_degrade.bitcrusher.bit_depth = 1;
    assert_eq!(
        settings.validate(),
        Err(CreativeVfxSettingsError::DigitalDegrade)
    );

    let mut settings = CreativeVfxSettings::default();
    settings.drive.tone_hertz = 499;
    assert_eq!(settings.validate(), Err(CreativeVfxSettingsError::Drive));

    let mut settings = CreativeVfxSettings::default();
    settings.rotary.motion_percent = 101;
    assert_eq!(settings.validate(), Err(CreativeVfxSettingsError::Rotary));
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
        digital_degrade: DigitalDegradeVfxSettings {
            character: DigitalDegradeVfxCharacter::LoFi,
            enabled: true,
            mix_percent: 74,
            bitcrusher: BitcrusherSettings { bit_depth: 7 },
            sample_rate_reduction: SampleRateReductionSettings {
                target_rate_hertz: 11_025,
            },
        },
        drive: DriveVfxSettings {
            character: DriveVfxCharacter::Fuzz,
            enabled: true,
            mix_percent: 61,
            drive_centibels: 2_700,
            tone_hertz: 5_200,
            output_gain_centibels: -750,
        },
        rotary: RotaryVfxSettings {
            speed: RotaryVfxSpeed::Fast,
            enabled: true,
            mix_percent: 43,
            motion_percent: 88,
            stereo_width_percent: 72,
        },
    };
    let encoded = serde_json::to_string(&settings).expect("settings encode");
    let decoded: CreativeVfxSettings = serde_json::from_str(&encoded).expect("settings decode");
    assert_eq!(decoded, settings);
}

#[test]
fn legacy_json_defaults_digital_degrade_to_disabled_identity() {
    let mut value = serde_json::to_value(CreativeVfxSettings::default()).expect("settings encode");
    value
        .as_object_mut()
        .expect("settings object")
        .remove("digital_degrade");
    let restored: CreativeVfxSettings =
        serde_json::from_value(value).expect("legacy settings decode");
    assert_eq!(
        restored.digital_degrade,
        DigitalDegradeVfxSettings::default()
    );
    assert!(!restored.digital_degrade.enabled);
}

#[test]
fn legacy_json_defaults_drive_and_rotary_to_disabled_identity() {
    let mut value = serde_json::to_value(CreativeVfxSettings::default()).expect("settings encode");
    let object = value.as_object_mut().expect("settings object");
    object.remove("drive");
    object.remove("rotary");
    let restored: CreativeVfxSettings =
        serde_json::from_value(value).expect("legacy settings decode");
    assert_eq!(restored.drive, DriveVfxSettings::default());
    assert_eq!(restored.rotary, RotaryVfxSettings::default());
}
