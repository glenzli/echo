use crate::{editor_project, editor_session};
#[test]
fn memory_info_survives_moved_private_project_without_source_changes() {
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
    let json = r#"{"notes":"私人备注，不参与 AI 分析","place":"外婆家阳台","timeDescription":"大约 2020 年夏天","moments":[{"startMillis":10,"endMillis":70,"note":"第一次叫爸爸"}]}"#;
    session.set_memory_info(&id, false, 0, json).unwrap();
    let saved = session.memory_info_json(&id, false).unwrap();
    assert!(session.set_memory_info(&id, false, 0, json).is_err());
    let project = root.join("memory.echo-project");
    editor_project::save(&session_root, &project).unwrap();
    drop(session);
    std::fs::remove_dir_all(&session_root).unwrap();
    let moved = root.join("moved.echo-project");
    std::fs::rename(project, &moved).unwrap();
    let reopened_root = root.join("reopened");
    editor_project::open(&moved, &reopened_root).unwrap();
    let reopened = editor_session::open(&reopened_root).unwrap();
    assert_eq!(reopened.memory_info_json(&id, false).unwrap(), saved);
    assert!(!reopened.list_assets().unwrap()[0].in_memory);
    assert_eq!(std::fs::read(&source).unwrap(), wave);
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}
