use std::{fs, path::PathBuf};

use super::*;

fn root(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "echo-ir-{name}-{}-{}",
        std::process::id(),
        Uuid::now_v7()
    ))
}

fn user_owned(name: &str) -> IrImportProvenance {
    IrImportProvenance {
        display_name: name.to_owned(),
        creator: Some("Test Creator".to_owned()),
        source_url: None,
        attribution: None,
        rights: IrRightsDeclaration::UserOwnedNoRedistribution,
    }
}

#[test]
fn import_owns_exact_source_after_original_disappears() {
    let root = root("owns-source");
    let import_path = root.join("incoming.wav");
    fs::create_dir_all(&root).expect("create root");
    let payload = b"RIFF deterministic test IR bytes";
    fs::write(&import_path, payload).expect("write import");
    let store = IrSourceStore::open(&root.join("store")).expect("open store");

    let (record, outcome) = store
        .import(&import_path, user_owned("Small room"))
        .expect("import source");
    assert_eq!(outcome, ImportOutcome::Stored);
    assert_eq!(record.source_hash, ContentHash::from(blake3::hash(payload)));
    assert_eq!(record.source_size_bytes, payload.len() as u64);
    fs::remove_file(&import_path).expect("remove original");

    let owned = store
        .verify_source(record.source_hash, record.source_size_bytes)
        .expect("verify owned source");
    assert_eq!(fs::read(owned).expect("read owned source"), payload);
    let event: IrImportRecord = serde_json::from_slice(
        &fs::read(store.provenance_path(record.import_id)).expect("read event"),
    )
    .expect("parse event");
    assert_eq!(event, record);
}

#[test]
fn duplicate_bytes_keep_distinct_provenance_events() {
    let root = root("dedup");
    fs::create_dir_all(&root).expect("create root");
    let first_path = root.join("first.wav");
    let second_path = root.join("second.wav");
    fs::write(&first_path, b"same IR").expect("write first");
    fs::write(&second_path, b"same IR").expect("write second");
    let store = IrSourceStore::open(&root.join("store")).expect("open store");

    let (first, first_outcome) = store
        .import(&first_path, user_owned("First title"))
        .expect("first import");
    let mut second_provenance = user_owned("Corrected title");
    second_provenance.attribution = Some("Recorded by A".to_owned());
    let (second, second_outcome) = store
        .import(&second_path, second_provenance)
        .expect("second import");

    assert_eq!(first_outcome, ImportOutcome::Stored);
    assert_eq!(second_outcome, ImportOutcome::AlreadyPresent);
    assert_eq!(first.source_hash, second.source_hash);
    assert_ne!(first.import_id, second.import_id);
    assert!(store.provenance_path(first.import_id).exists());
    assert!(store.provenance_path(second.import_id).exists());
}

#[test]
fn invalid_rights_and_corrupt_owned_source_fail_closed() {
    let root = root("fail-closed");
    fs::create_dir_all(&root).expect("create root");
    let import_path = root.join("source.wav");
    fs::write(&import_path, b"valid source").expect("write source");
    let store = IrSourceStore::open(&root.join("store")).expect("open store");

    let invalid = IrImportProvenance {
        display_name: "Source".to_owned(),
        creator: None,
        source_url: None,
        attribution: None,
        rights: IrRightsDeclaration::Spdx {
            expression: " ".to_owned(),
            license_url: None,
        },
    };
    let error = store
        .import(&import_path, invalid)
        .expect_err("reject rights");
    assert_eq!(error.kind, IrStoreErrorKind::InvalidInput);

    let (record, _) = store
        .import(&import_path, user_owned("Source"))
        .expect("import source");
    fs::write(store.source_path(record.source_hash), b"corrupt").expect("corrupt object");
    let error = store
        .verify_source(record.source_hash, record.source_size_bytes)
        .expect_err("reject corruption");
    assert_eq!(error.kind, IrStoreErrorKind::Corrupt);
}

#[test]
fn empty_or_non_file_sources_fail_before_provenance_publication() {
    let root = root("invalid-source");
    fs::create_dir_all(&root).expect("create root");
    let empty_path = root.join("empty.wav");
    fs::write(&empty_path, []).expect("write empty source");
    let store = IrSourceStore::open(&root.join("store")).expect("open store");

    let empty_error = store
        .import(&empty_path, user_owned("Empty"))
        .expect_err("reject empty source");
    assert_eq!(empty_error.kind, IrStoreErrorKind::InvalidInput);
    let directory_error = store
        .import(&root, user_owned("Directory"))
        .expect_err("reject directory source");
    assert_eq!(directory_error.kind, IrStoreErrorKind::InvalidInput);
    assert_eq!(
        fs::read_dir(root.join("store/provenance/events"))
            .expect("read events")
            .count(),
        0
    );
}
