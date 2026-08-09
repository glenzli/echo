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
            let asset_id = match registered {
                crate::RegisterAsset::Created(asset) | crate::RegisterAsset::Existed(asset) => {
                    asset.id
                }
            };
            transaction.execute(
                "INSERT INTO analysis_records (asset_id, kind, value, model, model_version, \
                 recorded_at_millis) VALUES (?1, 'contextual', \
                 '{\"summary\":\"Rain\",\"keywords\":[\"Rain\"]}', 'test', '1', 2)",
                [asset_id.to_string()],
            )?;
            transaction.execute("DROP TABLE contextual_browse_facets", [])?;
            transaction.execute_batch(
                "CREATE TABLE contextual_keyword_facets (
                    analysis_record_id INTEGER NOT NULL,
                    asset_id TEXT NOT NULL,
                    normalized_keyword TEXT NOT NULL,
                    display_keyword TEXT NOT NULL,
                    PRIMARY KEY (analysis_record_id, normalized_keyword)
                );",
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260809.5' WHERE key = 'schema_version'",
                [],
            )?;
            Ok(())
        })
        .expect("previous fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("previous revision migrates");
    let (version, asset_count, facet_count): (String, i64, i64) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get(0),
                )?,
                transaction.query_row("SELECT COUNT(*) FROM assets", [], |row| row.get(0))?,
                transaction.query_row(
                    "SELECT COUNT(*) FROM contextual_browse_facets",
                    [],
                    |row| row.get(0),
                )?,
            ))
        })
        .expect("migration reads");
    assert_eq!(version, "20260809.6");
    assert_eq!(asset_count, 1);
    assert_eq!(facet_count, 1);
    let _ = std::fs::remove_dir_all(root);
}
