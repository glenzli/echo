use super::*;

#[test]
fn convolution_requires_an_immutable_selection() {
    let settings = SpaceSettings {
        mode: SpaceMode::Convolution,
        ..SpaceSettings::default()
    };
    assert!(!settings.is_valid());
    assert!(SpaceSettings::default().is_valid());
}

#[test]
fn legacy_json_restores_algorithmic_identity() {
    let settings: SpaceSettings = serde_json::from_str("{}").expect("legacy space");
    assert_eq!(settings, SpaceSettings::default());
}
