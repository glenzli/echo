use crate::{editor_project, editor_session};
#[test]
fn private_project_retains_source_labels_without_touching_pcm() {
    let root =
        std::env::temp_dir().join(format!("echo-disclosure-portable-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("tone.wav");
    let mut wave = Vec::new();
    let length = 4800_u32 * 2;
    wave.extend(b"RIFF");
    wave.extend((36 + length).to_le_bytes());
    wave.extend(b"WAVEfmt ");
    wave.extend(16_u32.to_le_bytes());
    wave.extend(1_u16.to_le_bytes());
    wave.extend(1_u16.to_le_bytes());
    wave.extend(48000_u32.to_le_bytes());
    wave.extend(96000_u32.to_le_bytes());
    wave.extend(2_u16.to_le_bytes());
    wave.extend(16_u16.to_le_bytes());
    wave.extend(b"data");
    wave.extend(length.to_le_bytes());
    wave.resize(44 + length as usize, 0);
    std::fs::write(&source, &wave).unwrap();
    let session_root = root.join("session");
    let id = editor_session::import(&session_root, &source).unwrap();
    let session = editor_session::open(&session_root).unwrap();
    let json = r#"[{"kind":"ai_generated","startMillis":0,"endMillis":100,"note":"Declared synthetic fixture"}]"#;
    session.set_source_disclosure(&id, 0, json).unwrap();
    assert!(session.set_source_disclosure(&id, 0, "[]").is_err());
    let assets = session.list_assets().unwrap();
    assert!(assets[0].has_generated_source);
    assert!(!assets[0].in_memory);
    let project = root.join("memory.echo-project");
    editor_project::save(&session_root, &project).unwrap();
    drop(session);
    let reopened_root = root.join("reopened");
    editor_project::open(&project, &reopened_root).unwrap();
    let reopened = editor_session::open(&reopened_root).unwrap();
    let assets = reopened.list_assets().unwrap();
    assert!(assets[0].has_generated_source);
    assert!(
        assets[0]
            .source_disclosure_json
            .contains("Declared synthetic fixture")
    );
    assert_eq!(std::fs::read(&source).unwrap(), wave);
    assert_eq!(
        std::fs::read(reopened_root.join(&assets[0].path)).unwrap(),
        wave
    );
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}
