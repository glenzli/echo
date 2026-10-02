use super::*;
use std::{fs, path::Path};

use echo_catalog::{
    AssetRegistrationInput, RegisterAsset, record_adjustment_graph, register_asset,
};
use echo_domain::{
    AdjustmentEffects, AdjustmentGraph, AssetId, ContentHash, FadeCurve, FadeCurves,
};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        Self {
            root: std::env::temp_dir()
                .join(format!("echo-editor-inspect-{}", uuid::Uuid::now_v7())),
        }
    }

    fn request(&self) -> InspectRequest {
        InspectRequest {
            session_root: self.root.clone(),
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if self.root.exists() {
            fs::remove_dir_all(&self.root).unwrap();
        }
    }
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    let mut entries = BTreeMap::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            entries.insert(path.clone(), None);
            entries.extend(snapshot(&path));
        } else {
            entries.insert(path.clone(), Some(fs::read(path).unwrap()));
        }
    }
    entries
}

fn register_source(catalog: &echo_catalog::Catalog, root: &Path) -> AssetId {
    let bytes = b"deterministic synthetic source; no decoding needed by inspect";
    fs::create_dir_all(root.join("media")).unwrap();
    fs::write(root.join("media/source.wav"), bytes).unwrap();
    catalog
        .with_transaction(|tx| {
            let registered = register_asset(
                tx,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new(*blake3::hash(bytes).as_bytes()),
                    path: Path::new("media/source.wav"),
                    size_bytes: u64::try_from(bytes.len()).unwrap(),
                    codec: Some("pcm"),
                    duration_millis: Some(1500),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )?;
            let (RegisterAsset::Created(asset) | RegisterAsset::Existed(asset)) = registered;
            Ok::<_, CatalogError>(asset.id)
        })
        .unwrap()
}

fn graph(start: u64, end: u64) -> AdjustmentGraph {
    AdjustmentGraph::new(
        1500,
        start,
        end,
        50,
        50,
        AdjustmentEffects::new(
            FadeCurves::new(FadeCurve::Linear, FadeCurve::Smooth),
            -300,
            0,
        ),
    )
    .unwrap()
}

#[test]
fn inspect_reports_latest_asset_and_exact_saved_assembly_source_without_writes() {
    let fixture = Fixture::new();
    let session = crate::editor_session::open(&fixture.root).unwrap();
    let asset = register_source(&session.catalog, &fixture.root);
    let first = session
        .catalog
        .with_transaction(|tx| record_adjustment_graph(tx, asset, graph(0, 1000), 2))
        .unwrap();
    let assembly = session
        .create_sound_assembly("Saved mix", &[asset.to_string()], 0)
        .unwrap();
    let latest = session
        .catalog
        .with_transaction(|tx| record_adjustment_graph(tx, asset, graph(100, 900), 3))
        .unwrap();
    assert_ne!(first.revision_id, latest.revision_id);
    drop(session);
    let before = snapshot(&fixture.root);
    let input_hash = echo_core::hash_file(&fixture.root.join("media/source.wav")).unwrap();

    let result = inspect(&fixture.request()).unwrap();
    assert_eq!(result.assets.len(), 1);
    assert_eq!(result.assets[0].asset_id, asset.to_string());
    assert_eq!(result.assets[0].content_hash_blake3, input_hash.to_string());
    assert_eq!(result.assets[0].source_duration_millis, Some(1500));
    assert_eq!(result.assets[0].output_duration_millis, Some(800));
    assert_eq!(result.assets[0].adjustment_revision_id, latest.revision_id);
    assert_eq!(result.assemblies.len(), 1);
    assert_eq!(
        result.assemblies[0].assembly_revision_id,
        assembly.revision_id
    );
    assert_eq!(result.assemblies[0].duration_millis, 1000);
    assert_eq!(
        result.assemblies[0].sources,
        vec![AssemblySourceIdentity {
            asset_id: asset.to_string(),
            adjustment_revision_id: first.revision_id,
            content_hash_blake3: input_hash.to_string(),
        }]
    );
    let json = result.to_json();
    assert_eq!(json["schema_version"], 1);
    assert_eq!(
        json["capabilities"]["state_scope"],
        "saved_independent_session"
    );
    assert_eq!(json["capabilities"]["operations"], json!(["inspect"]));
    assert_eq!(json["capabilities"]["source_bytes_verified"], false);
    assert_eq!(json["assets"][0]["output_duration_ms"], 800);
    assert_eq!(snapshot(&fixture.root), before);
    assert_eq!(
        echo_core::hash_file(&fixture.root.join("media/source.wav")).unwrap(),
        input_hash
    );
}

#[test]
fn inspect_does_not_open_or_verify_source_bytes() {
    let fixture = Fixture::new();
    let session = crate::editor_session::open(&fixture.root).unwrap();
    register_source(&session.catalog, &fixture.root);
    drop(session);
    fs::remove_file(fixture.root.join("media/source.wav")).unwrap();
    let before = snapshot(&fixture.root);
    let result = inspect(&fixture.request()).unwrap();
    assert_eq!(result.assets[0].adjustment_revision_id, 0);
    assert_eq!(result.assets[0].output_duration_millis, Some(1500));
    assert_eq!(snapshot(&fixture.root), before);
}

#[test]
fn inspect_refuses_missing_paths_without_creating_anything() {
    let fixture = Fixture::new();
    assert_eq!(
        inspect(&fixture.request()).unwrap_err().code,
        EditorCommandErrorCode::MissingSession
    );
    assert!(!fixture.root.exists());
    fs::create_dir_all(&fixture.root).unwrap();
    assert_eq!(
        inspect(&fixture.request()).unwrap_err().code,
        EditorCommandErrorCode::MissingSession
    );
    assert!(snapshot(&fixture.root).is_empty());
}

#[test]
fn inspect_refuses_non_private_catalog_without_writing_a_marker() {
    let fixture = Fixture::new();
    drop(echo_catalog::open_catalog(&fixture.root.join("catalog.sqlite")).unwrap());
    let before = snapshot(&fixture.root);
    let error = inspect(&fixture.request()).unwrap_err();
    assert_eq!(error.code, EditorCommandErrorCode::NotIndependentSession);
    assert_eq!(error.to_json()["error"]["code"], "not_independent_session");
    assert_eq!(snapshot(&fixture.root), before);
}

#[test]
fn inspect_refuses_migration_and_unknown_schema_without_repair() {
    for version in ["20260922.3", "20990101.1"] {
        let fixture = Fixture::new();
        let session = crate::editor_session::open(&fixture.root).unwrap();
        session
            .catalog
            .with_transaction(|tx| {
                tx.execute(
                    "UPDATE catalog_meta SET value=?1 WHERE key='schema_version'",
                    [version],
                )?;
                Ok::<_, CatalogError>(())
            })
            .unwrap();
        drop(session);
        let before = snapshot(&fixture.root);
        let error = inspect(&fixture.request()).unwrap_err();
        assert_eq!(error.code, EditorCommandErrorCode::UnsupportedCatalogSchema);
        assert_eq!(snapshot(&fixture.root), before);
    }
}

#[test]
fn inspect_refuses_wal_and_preserves_the_full_directory() {
    let fixture = Fixture::new();
    drop(crate::editor_session::open(&fixture.root).unwrap());
    let connection = rusqlite::Connection::open(fixture.root.join("catalog.sqlite")).unwrap();
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    drop(connection);
    let before = snapshot(&fixture.root);
    let error = inspect(&fixture.request()).unwrap_err();
    assert_eq!(
        error.code,
        EditorCommandErrorCode::UnsupportedReadOnlyStorage
    );
    assert_eq!(
        error.to_json()["error"]["code"],
        "unsupported_read_only_storage"
    );
    assert_eq!(snapshot(&fixture.root), before);
}
