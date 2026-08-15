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
            regions: vec![invalid]
        }
        .validate(1_000),
        Err(SpectralRepairError::InvalidTimeRange)
    );
    invalid = region();
    invalid.low_hertz = 10;
    assert_eq!(
        SpectralRepairSettings {
            regions: vec![invalid]
        }
        .validate(1_000),
        Err(SpectralRepairError::InvalidFrequencyRange)
    );
}

#[test]
fn json_is_camel_case_and_legacy_settings_default_to_identity() {
    let encoded = serde_json::to_value(SpectralRepairSettings {
        regions: vec![region()],
    })
    .expect("settings serialize");
    assert_eq!(encoded["regions"][0]["lowHertz"], 120);
    let decoded: SpectralRepairSettings = serde_json::from_str("{}").expect("legacy defaults");
    assert!(decoded.regions.is_empty());
}
