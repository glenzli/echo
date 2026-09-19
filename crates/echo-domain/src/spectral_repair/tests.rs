use super::*;

fn region() -> SpectralAttenuationRegion {
    SpectralAttenuationRegion {
        start_millis: 120,
        end_millis: 480,
        low_hertz: 120,
        high_hertz: 4_000,
        attenuation_centibels: 2_400,
        time_feather_millis: 40,
        frequency_feather_hertz: 120,
    }
}

#[test]
fn identity_and_bounded_region_are_valid() {
    assert!(SpectralRepairSettings::identity().validate(1_000).is_ok());
    assert!(
        SpectralRepairSettings {
            noise_profile: None,
            enabled: true,
            regions: vec![region()]
        }
        .validate(1_000)
        .is_ok()
    );
}

#[test]
fn regions_fail_closed_outside_the_source_or_frequency_contract() {
    let mut invalid = region();
    invalid.end_millis = 1_001;
    assert_eq!(
        SpectralRepairSettings {
            noise_profile: None,
            enabled: true,
            regions: vec![invalid]
        }
        .validate(1_000),
        Err(SpectralRepairError::InvalidTimeRange)
    );
    invalid = region();
    invalid.low_hertz = 10;
    assert_eq!(
        SpectralRepairSettings {
            noise_profile: None,
            enabled: true,
            regions: vec![invalid]
        }
        .validate(1_000),
        Err(SpectralRepairError::InvalidFrequencyRange)
    );
}

#[test]
fn json_is_camel_case_and_legacy_settings_default_to_identity() {
    let encoded = serde_json::to_value(SpectralRepairSettings {
        noise_profile: None,
        enabled: false,
        regions: vec![region()],
    })
    .expect("settings serialize");
    assert_eq!(encoded["regions"][0]["lowHertz"], 120);
    assert_eq!(encoded["enabled"], false);
    let decoded: SpectralRepairSettings = serde_json::from_str("{}").expect("legacy defaults");
    assert!(decoded.enabled);
    assert!(decoded.regions.is_empty());
}

#[test]
fn disabled_layer_retains_regions_without_contributing() {
    let settings = SpectralRepairSettings {
        noise_profile: None,
        enabled: false,
        regions: vec![region()],
    };
    assert!(settings.validate(1_000).is_ok());
    assert!(!settings.is_enabled());
}

#[test]
fn absent_noise_profile_keeps_legacy_serialization_bytes() {
    let legacy = r#"{"enabled":true,"regions":[]}"#;
    let decoded: SpectralRepairSettings = serde_json::from_str(legacy).unwrap();
    assert!(decoded.noise_profile.is_none());
    assert_eq!(serde_json::to_string(&decoded).unwrap(), legacy);
}
