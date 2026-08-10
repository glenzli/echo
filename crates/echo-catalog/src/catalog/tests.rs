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
                    duration_millis: Some(10_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )?;
            let asset_id = match registered {
                crate::RegisterAsset::Created(asset) | crate::RegisterAsset::Existed(asset) => {
                    asset.id
                }
            };
            crate::record_adjustment_graph(
                transaction,
                asset_id,
                echo_domain::AdjustmentGraph::new(
                    10_000,
                    1_000,
                    9_000,
                    250,
                    500,
                    echo_domain::AdjustmentEffects::new(
                        echo_domain::FadeCurves::new(
                            echo_domain::FadeCurve::Smooth,
                            echo_domain::FadeCurve::EqualPower,
                        ),
                        -300,
                        0,
                    ),
                )
                .expect("fixture adjustment validates"),
                2,
            )?;
            transaction.execute(
                "ALTER TABLE asset_adjustment_revisions DROP COLUMN low_cut_hertz",
                [],
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260810.2' WHERE key = 'schema_version'",
                [],
            )?;
            Ok(())
        })
        .expect("previous fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("previous revision migrates");
    let (version, asset_count, low_cut_column_count): (String, i64, i64) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get(0),
                )?,
                transaction.query_row("SELECT COUNT(*) FROM assets", [], |row| row.get(0))?,
                transaction.query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('asset_adjustment_revisions') \
                     WHERE name = 'low_cut_hertz'",
                    [],
                    |row| row.get(0),
                )?,
            ))
        })
        .expect("migration reads");
    assert_eq!(version, "20260810.3");
    assert_eq!(asset_count, 1);
    assert_eq!(low_cut_column_count, 1);
    let adjustment = migrated
        .with_transaction(|transaction| {
            let asset_id: String =
                transaction.query_row("SELECT id FROM assets LIMIT 1", [], |row| row.get(0))?;
            crate::latest_adjustment_graph(
                transaction,
                asset_id.parse().expect("stored asset id is valid"),
            )
        })
        .expect("migrated adjustment reads")
        .expect("adjustment survives");
    assert_eq!(
        adjustment.graph.fade_in_curve(),
        echo_domain::FadeCurve::Smooth
    );
    assert_eq!(
        adjustment.graph.fade_out_curve(),
        echo_domain::FadeCurve::EqualPower
    );
    assert_eq!(adjustment.graph.low_cut_hertz(), 0);
    let _ = std::fs::remove_dir_all(root);
}
