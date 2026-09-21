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

#[test]
fn exported_wav_comment_restores_labels_on_private_import() {
    let root =
        std::env::temp_dir().join(format!("echo-embedded-disclosure-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&root).unwrap();
    let comment =
        echo_domain::encode_portable_disclosure([echo_domain::SourceDisclosureKind::AiGenerated]);
    let payload = comment.len() + 1;
    let padded = payload + payload % 2;
    let data_length = 4800_u32 * 2;
    let mut wave = Vec::new();
    wave.extend(b"RIFF");
    wave.extend((36 + data_length + 20 + u32::try_from(padded).unwrap()).to_le_bytes());
    wave.extend(b"WAVEfmt ");
    wave.extend(16_u32.to_le_bytes());
    wave.extend(1_u16.to_le_bytes());
    wave.extend(1_u16.to_le_bytes());
    wave.extend(48000_u32.to_le_bytes());
    wave.extend(96000_u32.to_le_bytes());
    wave.extend(2_u16.to_le_bytes());
    wave.extend(16_u16.to_le_bytes());
    wave.extend(b"data");
    wave.extend(data_length.to_le_bytes());
    wave.resize(44 + data_length as usize, 0);
    wave.extend(b"LIST");
    wave.extend(u32::try_from(12 + padded).unwrap().to_le_bytes());
    wave.extend(b"INFOICMT");
    wave.extend(u32::try_from(payload).unwrap().to_le_bytes());
    wave.extend(comment.as_bytes());
    wave.resize(wave.len() + padded - comment.len(), 0);
    let source = root.join("exported.wav");
    std::fs::write(&source, &wave).unwrap();
    let session_root = root.join("private");
    let id = editor_session::import(&session_root, &source).unwrap();
    let session = editor_session::open(&session_root).unwrap();
    let asset = &session.list_assets().unwrap()[0];
    assert!(asset.has_generated_source);
    assert!(asset.source_disclosure_json.contains("embedded_export"));
    assert!(!asset.in_memory);
    assert_eq!(session.export_source_disclosure(&id, 0).unwrap(), comment);
    session.set_source_disclosure(&id, 0, "[]").unwrap();
    assert!(!session.list_assets().unwrap()[0].has_generated_source);
    assert_eq!(std::fs::read(source).unwrap(), wave);
    drop(session);
    std::fs::remove_dir_all(root).unwrap();
}
