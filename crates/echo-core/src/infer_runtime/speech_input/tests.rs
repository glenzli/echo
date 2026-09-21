use super::*;
#[test]
fn temporary_proxy_preserves_original_and_cleans_up_after_use() {
    let root = std::env::temp_dir().join(format!(
        "echo-speech-fixture-{}",
        echo_domain::AssetId::new()
    ));
    std::fs::create_dir(&root).unwrap();
    let frames = 8000u32;
    let mut wav = Vec::new();
    wav.extend(b"RIFF");
    wav.extend((36 + frames * 2).to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(8000u32.to_le_bytes());
    wav.extend(16000u32.to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend((frames * 2).to_le_bytes());
    wav.resize(44 + frames as usize * 2, 0);
    let original = root.join("source.wave");
    std::fs::write(&original, &wav).unwrap();
    let proxy = SpeechInput::prepare(&original).unwrap();
    let prepared = proxy.path().to_owned();
    assert_ne!(prepared, original);
    let decoded = echo_bridge::probe(&prepared).unwrap();
    assert_eq!(
        (
            decoded.sample_rate,
            decoded.channel_count,
            decoded.duration_millis
        ),
        (16000, 1, 1000)
    );
    assert_eq!(std::fs::read(&original).unwrap(), wav);
    drop(proxy);
    assert!(!prepared.exists());
    let direct = root.join("source.wav");
    std::fs::write(&direct, &wav).unwrap();
    let passthrough = SpeechInput::prepare(&direct).unwrap();
    assert_eq!(passthrough.path(), direct);
    drop(passthrough);
    assert!(direct.exists());
    let invalid = root.join("broken.mka");
    std::fs::write(&invalid, b"not audio").unwrap();
    assert!(SpeechInput::prepare(&invalid).is_err());
    std::fs::remove_dir_all(root).unwrap();
}
