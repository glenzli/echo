use super::*;

fn fixture() -> std::path::PathBuf {
    let root =
        std::env::temp_dir().join(format!("echo-editor-project-test-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn portable_project_preserves_database_and_multichunk_resources() {
    let base = fixture();
    let source = base.join("source");
    drop(crate::editor_session::open(&source).unwrap());
    fs::create_dir_all(source.join("media/audio")).unwrap();
    let bytes = vec![91; CHUNK * 2 + 731];
    fs::write(source.join("media/audio/original.wav"), &bytes).unwrap();
    fs::create_dir_all(source.join("cache")).unwrap();
    fs::write(source.join("cache/edited.pcm"), b"processed working copy").unwrap();
    let project = base.join("saved.echo");
    save(&source, &project).unwrap();
    fs::remove_dir_all(source).unwrap();
    let moved = base.join("moved.echo");
    fs::rename(&project, &moved).unwrap();
    let reopened = base.join("another-session");
    open(&moved, &reopened).unwrap();
    assert_eq!(
        fs::read(reopened.join("media/audio/original.wav")).unwrap(),
        bytes
    );
    assert_eq!(
        fs::read(reopened.join("cache/edited.pcm")).unwrap(),
        b"processed working copy"
    );
    let session = crate::editor_session::open(&reopened).unwrap();
    assert!(session.independent);
    drop(session);
    save(&reopened, &moved).unwrap();
    open(&moved, &base.join("third-session")).unwrap();
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn foreign_destinations_and_resource_path_escape_are_rejected() {
    let base = fixture();
    let source = base.join("source");
    drop(crate::editor_session::open(&source).unwrap());
    let foreign = base.join("original.wav");
    fs::write(&foreign, b"immutable original").unwrap();
    assert!(save(&source, &foreign).is_err());
    assert_eq!(fs::read(&foreign).unwrap(), b"immutable original");
    let project = base.join("bad.echo");
    save(&source, &project).unwrap();
    let db = Connection::open(&project).unwrap();
    db.execute(
        "INSERT INTO editor_project_files VALUES('../original.wav',0,'fake')",
        [],
    )
    .unwrap();
    drop(db);
    assert!(open(&project, &base.join("rejected")).is_err());
    assert!(!base.join("rejected").exists());
    assert_eq!(fs::read(foreign).unwrap(), b"immutable original");
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn corrupt_resource_is_rejected_and_existing_work_is_never_replaced() {
    let base = fixture();
    let source = base.join("source");
    drop(crate::editor_session::open(&source).unwrap());
    fs::create_dir_all(source.join("media")).unwrap();
    fs::write(source.join("media/a"), b"abc").unwrap();
    let project = base.join("saved.echo");
    save(&source, &project).unwrap();
    assert!(open(&project, &source).is_err());
    let db = Connection::open(&project).unwrap();
    db.execute("UPDATE editor_project_chunks SET data=x'616264'", [])
        .unwrap();
    drop(db);
    assert!(
        open(&project, &base.join("rejected"))
            .unwrap_err()
            .contains("integrity")
    );
    assert_eq!(fs::read(source.join("media/a")).unwrap(), b"abc");
    fs::remove_dir_all(base).unwrap();
}

#[test]
fn source_identity_mismatch_never_replaces_a_valid_project() {
    let base = fixture();
    let root = base.join("session");
    let session = crate::editor_session::open(&root).unwrap();
    fs::create_dir_all(root.join("media")).unwrap();
    fs::write(root.join("media/a.wav"), b"original bytes").unwrap();
    let hash = echo_core::hash_file(&root.join("media/a.wav")).unwrap();
    session
        .catalog
        .with_transaction(|tx| {
            echo_catalog::register_asset(
                tx,
                &echo_catalog::AssetRegistrationInput {
                    content_hash: hash,
                    path: Path::new("media/a.wav"),
                    size_bytes: 14,
                    codec: None,
                    duration_millis: Some(1000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )
        })
        .unwrap();
    let saved = base.join("source.echo");
    save(&root, &saved).unwrap();
    let before = fs::read(&saved).unwrap();
    fs::write(root.join("media/a.wav"), b"changed bytes").unwrap();
    assert!(save(&root, &saved).unwrap_err().contains("identity"));
    assert_eq!(fs::read(saved).unwrap(), before);
    drop(session);
    fs::remove_dir_all(base).unwrap();
}

#[test]
#[ignore = "requires ECHO_CONTAINER_PROJECT_FIXTURE from a manually selected audio stream"]
fn selected_stream_project_round_trips_complete_container() {
    let source = std::path::PathBuf::from(std::env::var("ECHO_CONTAINER_PROJECT_FIXTURE").unwrap());
    let base = fixture();
    let project = base.join("selected.echo");
    save(&source, &project).unwrap();
    let db = readonly(&project).unwrap();
    let paths:Vec<String>=db.prepare("SELECT path FROM editor_project_files WHERE path LIKE 'media/container-audio/%' ORDER BY path").unwrap().query_map([],|r|r.get(0)).unwrap().collect::<Result<_,_>>().unwrap();
    assert!(paths.iter().any(|p| p.ends_with("/original")));
    assert!(paths.iter().any(|p| p.ends_with("/track-1.mka")));
    drop(db);
    let moved = base.join("moved.echo");
    fs::rename(project, &moved).unwrap();
    let reopened = base.join("reopened");
    open(&moved, &reopened).unwrap();
    for path in &paths {
        assert_eq!(
            echo_core::hash_file(&source.join(path)).unwrap(),
            echo_core::hash_file(&reopened.join(path)).unwrap()
        );
    }
    let db = readonly(&reopened.join("catalog.sqlite")).unwrap();
    let count: i64 = db
        .query_row("SELECT count(*) FROM jobs", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
    drop(db);
    println!(
        "Selected track and full container preserved through save, move and reopen; no library jobs"
    );
    fs::remove_dir_all(base).unwrap();
}
