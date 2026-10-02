//! Public CLI contracts: safe intake across working directories and machine inspection.
use std::{fs, path::Path, process::Command};

fn cli(directory: &Path, arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_echo-cli"))
        .current_dir(directory)
        .args(arguments)
        .output()
        .expect("CLI executes")
}

#[test]
fn operator_intake_and_probe_are_cwd_independent_and_structured() {
    let root = std::env::temp_dir().join(format!("echo-cli-contract-{}", std::process::id()));
    fs::create_dir_all(root.join("input")).unwrap();
    fs::create_dir_all(root.join("elsewhere")).unwrap();
    let source = root.join("input/tone.wav");
    // A deterministic 10 ms mono PCM16 WAV; no downloads or inference.
    let data = vec![0u8; 960];
    let mut wav = b"RIFF".to_vec();
    wav.extend(996u32.to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16u32.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(48000u32.to_le_bytes());
    wav.extend(96000u32.to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(960u32.to_le_bytes());
    wav.extend(&data);
    fs::write(&source, &wav).unwrap();
    let catalog = root.join("catalog.sqlite");
    let catalog_text = catalog.to_str().unwrap();
    for cwd in [root.join("input"), root.join("elsewhere")] {
        let relative = if cwd.ends_with("input") {
            "tone.wav"
        } else {
            "../input/tone.wav"
        };
        let output = cli(&cwd, &["import", catalog_text, relative]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let store = echo_catalog::open_catalog(&catalog).unwrap();
    let assets = store.with_transaction(echo_catalog::list_assets).unwrap();
    assert_eq!(assets.len(), 1);
    assert_eq!(assets[0].original.path, source.canonicalize().unwrap());
    assert_eq!(fs::read(&source).unwrap(), wav);
    let probe = cli(
        &root.join("elsewhere"),
        &["probe", assets[0].original.path.to_str().unwrap(), "--json"],
    );
    assert!(probe.status.success());
    let value: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["has_audio"], true);
    assert_eq!(value["sample_rate_hz"], 48000);
    assert_eq!(value["channels"], 1);
    assert_eq!(value["duration_ms"], 10);
    assert_eq!(value["codec_name"], "pcm_s16le");
    assert!(value.get("metadata").is_none());
    assert!(value.get("path").is_none());
    let missing = cli(&root, &["probe", "absent.wav", "--json"]);
    assert!(!missing.status.success());
    assert!(missing.stdout.is_empty());
    let rejected = cli(&root, &["scan", "never-created.sqlite", "--workers", "0"]);
    assert!(!rejected.status.success());
    assert!(!root.join("never-created.sqlite").exists());
    drop(store);
    fs::remove_dir_all(root).unwrap();
}
