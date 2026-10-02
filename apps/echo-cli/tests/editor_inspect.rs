//! Black-box consumption of the production saved-session facade by echo-cli.
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn cli(directory: &Path, arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_echo-cli"))
        .current_dir(directory)
        .args(arguments)
        .output()
        .expect("CLI executes")
}

fn fixture() -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "echo-cli-editor-inspect-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    fs::create_dir(root.join("elsewhere")).unwrap();
    let mut wave = b"RIFF".to_vec();
    wave.extend(996_u32.to_le_bytes());
    wave.extend(b"WAVEfmt ");
    wave.extend(16_u32.to_le_bytes());
    wave.extend(1_u16.to_le_bytes());
    wave.extend(1_u16.to_le_bytes());
    wave.extend(48_000_u32.to_le_bytes());
    wave.extend(96_000_u32.to_le_bytes());
    wave.extend(2_u16.to_le_bytes());
    wave.extend(16_u16.to_le_bytes());
    wave.extend(b"data");
    wave.extend(960_u32.to_le_bytes());
    wave.resize(1004, 0);
    fs::write(root.join("tone.wav"), wave).unwrap();
    root
}

#[test]
fn real_cli_inspects_only_explicit_saved_session_and_returns_structured_refusals() {
    let root = fixture();
    let source = root.join("tone.wav");
    let session = root.join("private-session");
    let asset_id = echo_desktop_bridge::editor_import_audio(
        session.to_str().unwrap(),
        source.to_str().unwrap(),
    )
    .unwrap();
    let before = fs::read(session.join("catalog.sqlite")).unwrap();
    let input_hash = echo_core::hash_file(&source).unwrap();
    let cwd = root.join("elsewhere");
    let discovery = cli(&cwd, &["edit", "capabilities"]);
    assert!(discovery.status.success());
    let capabilities: serde_json::Value = serde_json::from_slice(&discovery.stdout).unwrap();
    assert_eq!(capabilities["operations"], serde_json::json!(["inspect"]));
    assert_eq!(capabilities["state_scope"], "saved_independent_session");

    let output = cli(
        &cwd,
        &["edit", "inspect", "--session", "../private-session"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["ok"], true);
    assert_eq!(value["assets"][0]["asset_id"], asset_id);
    assert_eq!(value["assets"][0]["adjustment_revision_id"], 0);
    assert_eq!(value["assets"][0]["source_duration_ms"], 10);
    assert_eq!(
        value["assets"][0]["content_hash_blake3"],
        input_hash.to_string()
    );
    assert_eq!(value["capabilities"]["source_bytes_verified"], false);
    assert!(value["assets"][0].get("path").is_none());
    assert!(value["assets"][0].get("transcript").is_none());
    assert_eq!(fs::read(session.join("catalog.sqlite")).unwrap(), before);
    assert_eq!(echo_core::hash_file(&source).unwrap(), input_hash);
    let assets = echo_catalog::open_catalog_read_only(&session.join("catalog.sqlite"))
        .unwrap()
        .with_transaction(echo_catalog::list_assets)
        .unwrap();
    assert_eq!(
        echo_core::hash_file(&session.join(&assets[0].original.path)).unwrap(),
        input_hash
    );

    let missing = cli(&cwd, &["edit", "inspect", "--session", "../never-created"]);
    assert!(!missing.status.success());
    let refusal: serde_json::Value = serde_json::from_slice(&missing.stdout).unwrap();
    assert_eq!(refusal["ok"], false);
    assert_eq!(refusal["error"]["code"], "missing_session");
    assert!(!root.join("never-created").exists());
    for arguments in [vec!["edit", "inspect"], vec!["edit", "preview"]] {
        let rejected = cli(&cwd, &arguments);
        assert!(!rejected.status.success());
        assert!(rejected.stdout.is_empty());
    }
    assert_eq!(fs::read_dir(&cwd).unwrap().count(), 0);
    assert!(!cwd.join("catalog.sqlite").exists());
    fs::remove_dir_all(root).unwrap();
}
