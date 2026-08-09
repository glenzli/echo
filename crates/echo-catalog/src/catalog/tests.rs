use super::*;

#[test]
fn previous_catalog_revision_migrates_without_losing_assets() {
    let root = std::env::temp_dir().join(format!("echo-schema-migration-{}", std::process::id()));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let registered = crate::register_asset(
                transaction,
                &crate::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([0; 32]),
                    path: std::path::Path::new("/voice.wav"),
                    size_bytes: 1,
                    codec: None,
                    duration_millis: None,
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )?;
            let _asset_id = match registered {
                crate::RegisterAsset::Created(asset) | crate::RegisterAsset::Existed(asset) => {
                    asset.id
                }
            };
            transaction.execute("DROP TABLE asset_adjustment_revisions", [])?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260809.6' WHERE key = 'schema_version'",
                [],
            )?;
            Ok(())
        })
        .expect("previous fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("previous revision migrates");
    let (version, asset_count, adjustment_table_count): (String, i64, i64) = migrated
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
                     AND name = 'asset_adjustment_revisions'",
                    [],
                    |row| row.get(0),
                )?,
            ))
        })
        .expect("migration reads");
    assert_eq!(version, "20260810.1");
    assert_eq!(asset_count, 1);
    assert_eq!(adjustment_table_count, 1);
    let _ = std::fs::remove_dir_all(root);
}
