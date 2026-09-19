use super::*;

#[test]
fn opening_audio_is_isolated_and_never_enqueues_library_analysis() {
    let base =
        std::env::temp_dir().join(format!("echo-editor-session-test-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&base).unwrap();
    let original = base.join("tone.wav");
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
    fs::write(&original, &wave).unwrap();
    let root = base.join("session");
    let id = import(&root, &original).unwrap();
    assert_eq!(import(&root, &original).unwrap(), id);
    let session = open(&root).unwrap();
    assert!(session.start_workers("").is_err());
    assert!(session.queue_scans().is_err());
    assert!(session.add_root("/").is_err());
    session
        .catalog
        .with_transaction(|tx| {
            for table in ["jobs", "scan_roots", "analysis_records"] {
                assert_eq!(
                    tx.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
                        .get::<_, i64>(0))?,
                    0
                );
            }
            let path: String = tx.query_row("SELECT path FROM assets", [], |r| r.get(0))?;
            assert!(!Path::new(&path).is_absolute());
            assert_eq!(fs::read(root.join(path)).unwrap(), wave);
            Ok::<_, echo_catalog::CatalogError>(())
        })
        .unwrap();
    assert_eq!(fs::read(original).unwrap(), wave);
    drop(session);
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn independent_entry_rejects_a_library_catalog_before_mutating_it() {
    let root = std::env::temp_dir().join(format!(
        "echo-library-isolation-test-{}",
        uuid::Uuid::new_v4()
    ));
    drop(echo_catalog::open_catalog(&root.join("catalog.sqlite")).unwrap());
    let before = fs::read(root.join("catalog.sqlite")).unwrap();
    assert!(open(&root).is_err());
    assert_eq!(before, fs::read(root.join("catalog.sqlite")).unwrap());
    fs::remove_dir_all(root).unwrap();
}
