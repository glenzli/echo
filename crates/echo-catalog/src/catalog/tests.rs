use super::*;

fn previous_render_schema_fixture() -> (std::path::PathBuf, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("echo-schema-migration-{}", std::process::id()));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let _registered = crate::register_asset(
                transaction,
                &crate::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([0; 32]),
                    path: std::path::Path::new("/voice.wav"),
                    size_bytes: 1,
                    codec: None,
                    duration_millis: Some(10_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )?;
            transaction.execute("DROP TABLE user_album_members", [])?;
            transaction.execute("DROP TABLE user_albums", [])?;
            transaction.execute("DROP TABLE render_exports", [])?;
            transaction.execute("DROP TABLE long_audio_outline_nodes", [])?;
            transaction.execute("DROP TABLE long_audio_segments", [])?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260811.2' WHERE key = 'schema_version'",
                [],
            )?;
            Ok(())
        })
        .expect("previous fixture writes");
    drop(catalog);
    (root, path)
}

#[test]
fn previous_catalog_revision_adds_render_exports_without_losing_assets() {
    let (root, path) = previous_render_schema_fixture();
    let migrated = open_catalog(&path).expect("previous revision migrates");
    let (version, asset_count, render_table_count, album_table_count): (String, i64, i64, i64) =
        migrated
            .with_transaction(|transaction| -> Result<_, CatalogError> {
                Ok((
                    transaction.query_row(
                        "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                        [],
                        |row| row.get(0),
                    )?,
                    transaction.query_row("SELECT COUNT(*) FROM assets", [], |row| row.get(0))?,
                    transaction.query_row(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' \
                     AND name = 'render_exports'",
                        [],
                        |row| row.get(0),
                    )?,
                    transaction.query_row(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' \
                     AND name = 'user_albums'",
                        [],
                        |row| row.get(0),
                    )?,
                ))
            })
            .expect("migration reads");
    assert_eq!(version, "20260811.6");
    assert_eq!(asset_count, 1);
    assert_eq!(render_table_count, 1);
    assert_eq!(album_table_count, 1);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn immediately_previous_catalog_revision_adds_user_albums() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-user-album-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            transaction.execute("DROP TABLE user_album_members", [])?;
            transaction.execute("DROP TABLE user_albums", [])?;
            transaction.execute("DROP TABLE long_audio_outline_nodes", [])?;
            transaction.execute("DROP TABLE long_audio_segments", [])?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260811.3' WHERE key = 'schema_version'",
                [],
            )?;
            Ok(())
        })
        .expect("previous fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("previous revision migrates");
    let (version, album_table_count): (String, i64) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get(0),
                )?,
                transaction.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' \
                     AND name = 'user_albums'",
                    [],
                    |row| row.get(0),
                )?,
            ))
        })
        .expect("migration reads");
    assert_eq!(version, "20260811.6");
    assert_eq!(album_table_count, 1);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn legacy_catalog_revision_migrates_both_compatible_steps() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-legacy-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            transaction.execute("DROP TABLE user_album_members", [])?;
            transaction.execute("DROP TABLE user_albums", [])?;
            transaction.execute("DROP TABLE render_exports", [])?;
            transaction.execute("DROP TABLE long_audio_outline_nodes", [])?;
            transaction.execute("DROP TABLE long_audio_segments", [])?;
            transaction.execute(
                "ALTER TABLE asset_adjustment_revisions DROP COLUMN reverb_json",
                [],
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260811.1' WHERE key = 'schema_version'",
                [],
            )?;
            Ok(())
        })
        .expect("legacy fixture writes");
    drop(catalog);
    let migrated = open_catalog(&path).expect("legacy revision migrates");
    let (version, render_table_count, reverb_column_count, album_table_count): (
        String,
        i64,
        i64,
        i64,
    ) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get(0),
                )?,
                transaction.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' \
                     AND name = 'render_exports'",
                    [],
                    |row| row.get(0),
                )?,
                transaction.query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('asset_adjustment_revisions') \
                     WHERE name = 'reverb_json'",
                    [],
                    |row| row.get(0),
                )?,
                transaction.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' \
                     AND name = 'user_albums'",
                    [],
                    |row| row.get(0),
                )?,
            ))
        })
        .expect("migration reads");
    assert_eq!(version, "20260811.6");
    assert_eq!(render_table_count, 1);
    assert_eq!(reverb_column_count, 1);
    assert_eq!(album_table_count, 1);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn immediately_previous_revision_adds_long_audio_projection() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-long-audio-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            transaction.execute("DROP TABLE long_audio_outline_nodes", [])?;
            transaction.execute("DROP TABLE long_audio_segments", [])?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260811.4' WHERE key = 'schema_version'",
                [],
            )?;
            Ok(())
        })
        .expect("previous fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("previous revision migrates");
    let (version, segment_table_count): (String, i64) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get(0),
                )?,
                transaction.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' \
                     AND name = 'long_audio_segments'",
                    [],
                    |row| row.get(0),
                )?,
            ))
        })
        .expect("migration reads");
    assert_eq!(version, "20260811.6");
    assert_eq!(segment_table_count, 1);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn immediately_previous_revision_adds_semantic_search_projection() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-semantic-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            transaction.execute("DROP TABLE semantic_document_fts", [])?;
            transaction.execute("DROP TABLE semantic_documents", [])?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260811.5' WHERE key = 'schema_version'",
                [],
            )?;
            Ok(())
        })
        .expect("previous fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("previous revision migrates");
    let (version, document_count, fts_count): (String, i64, i64) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get(0),
                )?,
                transaction.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' \
                     AND name = 'semantic_documents'",
                    [],
                    |row| row.get(0),
                )?,
                transaction.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' \
                     AND name = 'semantic_document_fts'",
                    [],
                    |row| row.get(0),
                )?,
            ))
        })
        .expect("migration reads");
    assert_eq!(version, "20260811.6");
    assert_eq!(document_count, 1);
    assert_eq!(fts_count, 1);
    let _ = std::fs::remove_dir_all(root);
}
