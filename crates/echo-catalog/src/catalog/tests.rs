use super::*;

#[test]
fn previous_catalog_revision_migrates_without_losing_assets() {
    let root = std::env::temp_dir().join(format!("echo-schema-migration-{}", std::process::id()));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            transaction.execute(
                "INSERT INTO assets (id, content_hash, path, size_bytes, imported_at_millis) \
                 VALUES ('asset', ?1, '/voice.wav', 1, 1)",
                ["00".repeat(32)],
            )?;
            transaction.execute("DROP TABLE asset_user_state", [])?;
            transaction.execute("DROP TABLE asset_source_metadata", [])?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260809.2' WHERE key = 'schema_version'",
                [],
            )?;
            Ok(())
        })
        .expect("previous fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("previous revision migrates");
    let (version, asset_count, user_state_table): (String, i64, i64) = migrated
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
                     AND name = 'asset_user_state'",
                    [],
                    |row| row.get(0),
                )?,
            ))
        })
        .expect("migration reads");
    assert_eq!(version, "20260809.3");
    assert_eq!(asset_count, 1);
    assert_eq!(user_state_table, 1);
    let _ = std::fs::remove_dir_all(root);
}
