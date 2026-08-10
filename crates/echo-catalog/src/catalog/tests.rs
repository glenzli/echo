use super::*;

#[test]
fn immediately_previous_revision_adds_processing_recipes_without_rewriting_assets() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-processing-recipes-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    let (asset_id, revision_id) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let registered = crate::register_asset(
                transaction,
                &crate::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([64; 32]),
                    path: std::path::Path::new("/voice/recipe-migration.wav"),
                    size_bytes: 1,
                    codec: Some("pcm"),
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
            let revision = crate::record_adjustment_graph(
                transaction,
                asset_id,
                echo_domain::AdjustmentGraph::identity(10_000).expect("identity validates"),
                2,
            )?;
            transaction.execute_batch(
                "DROP TABLE processing_recipe_application_targets;
                 DROP TABLE processing_recipe_application_batches;
                 DROP TABLE processing_recipe_revisions;
                 DROP TABLE processing_recipes;",
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260811.11' WHERE key = 'schema_version'",
                [],
            )?;
            Ok((asset_id, revision.revision_id))
        })
        .expect("previous fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("previous revision migrates");
    let (version, table_count, asset_count, stored_revision_id): (String, i64, i64, i64) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get(0),
                )?,
                transaction.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN (
                     'processing_recipes', 'processing_recipe_revisions',
                     'processing_recipe_application_batches',
                     'processing_recipe_application_targets')",
                    [],
                    |row| row.get(0),
                )?,
                transaction.query_row("SELECT COUNT(*) FROM assets", [], |row| row.get(0))?,
                transaction.query_row(
                    "SELECT id FROM asset_adjustment_revisions WHERE asset_id = ?1",
                    [asset_id.to_string()],
                    |row| row.get(0),
                )?,
            ))
        })
        .expect("migration reads");
    assert_eq!(version, "20260811.12");
    assert_eq!(table_count, 4);
    assert_eq!(asset_count, 1);
    assert_eq!(stored_revision_id, revision_id);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[allow(clippy::too_many_lines)] // Migration fixture verifies history and normalized re-save.
fn restorative_effects_revision_adds_settings_without_rewriting_history() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-restorative-effects-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    let (asset_id, revision_id) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let registered = crate::register_asset(
                transaction,
                &crate::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([42; 32]),
                    path: std::path::Path::new("/voice/restorative.wav"),
                    size_bytes: 1,
                    codec: Some("pcm"),
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
            let saved = crate::record_adjustment_graph(
                transaction,
                asset_id,
                echo_domain::AdjustmentGraph::identity(10_000).expect("identity graph validates"),
                2,
            )?;
            transaction.execute(
                "UPDATE asset_adjustment_revisions SET effect_chain_json = \
                 '{\"nodes\":[\"restoration\",\"equalizer\",\"dynamics\",\"space\",\"master\"]}' \
                 WHERE id = ?1",
                [saved.revision_id],
            )?;
            transaction.execute(
                "ALTER TABLE asset_adjustment_revisions DROP COLUMN de_hum_json",
                [],
            )?;
            transaction.execute(
                "ALTER TABLE asset_adjustment_revisions DROP COLUMN de_click_json",
                [],
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260811.10' WHERE key = 'schema_version'",
                [],
            )?;
            Ok((asset_id, saved.revision_id))
        })
        .expect("previous fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("previous revision migrates");
    let (version, column_count, legacy_json): (String, i64, String) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get(0),
                )?,
                transaction.query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('asset_adjustment_revisions') \
                     WHERE name IN ('de_hum_json', 'de_click_json')",
                    [],
                    |row| row.get(0),
                )?,
                transaction.query_row(
                    "SELECT effect_chain_json FROM asset_adjustment_revisions WHERE id = ?1",
                    [revision_id],
                    |row| row.get(0),
                )?,
            ))
        })
        .expect("migration reads");
    assert_eq!(version, "20260811.12");
    assert_eq!(column_count, 2);
    assert!(!legacy_json.contains("active_count"));
    assert!(!legacy_json.contains("de_hum"));

    let restored = migrated
        .with_transaction(|transaction| crate::latest_adjustment_graph(transaction, asset_id))
        .expect("legacy revision restores")
        .expect("legacy revision exists");
    assert_eq!(restored.revision_id, revision_id);
    assert_eq!(
        restored.graph.de_hum(),
        echo_domain::DeHumSettings::default()
    );
    assert_eq!(
        restored.graph.de_click(),
        echo_domain::DeClickSettings::default()
    );
    assert_eq!(
        restored.graph.effect_chain().nodes(),
        [
            echo_domain::EffectNodeKind::Restoration,
            echo_domain::EffectNodeKind::Equalizer,
            echo_domain::EffectNodeKind::Dynamics,
            echo_domain::EffectNodeKind::Space,
            echo_domain::EffectNodeKind::Master,
        ]
    );

    let replacement = echo_domain::AdjustmentGraph::new(
        10_000,
        0,
        10_000,
        0,
        0,
        echo_domain::AdjustmentEffects::new(echo_domain::FadeCurves::linear(), 100, 0),
    )
    .expect("replacement graph validates");
    let new_revision = migrated
        .with_transaction(|transaction| {
            crate::record_adjustment_graph(transaction, asset_id, replacement, 3)
        })
        .expect("replacement revision writes");
    assert!(new_revision.revision_id > revision_id);
    let (historical_json, current_json): (String, String) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT effect_chain_json FROM asset_adjustment_revisions WHERE id = ?1",
                    [revision_id],
                    |row| row.get(0),
                )?,
                transaction.query_row(
                    "SELECT effect_chain_json FROM asset_adjustment_revisions WHERE id = ?1",
                    [new_revision.revision_id],
                    |row| row.get(0),
                )?,
            ))
        })
        .expect("revision encodings read");
    assert_eq!(historical_json, legacy_json);
    assert!(current_json.contains("active_count"));
    assert!(current_json.contains("de_hum"));
    assert!(current_json.contains("de_click"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn fixed_chain_revision_adds_authored_effect_chain_column() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-effect-chain-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            transaction.execute(
                "ALTER TABLE asset_adjustment_revisions DROP COLUMN effect_chain_json",
                [],
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260811.8' WHERE key = 'schema_version'",
                [],
            )?;
            Ok(())
        })
        .expect("previous fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("previous revision migrates");
    let (version, default_expression): (String, String) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get(0),
                )?,
                transaction.query_row(
                    "SELECT dflt_value FROM pragma_table_info('asset_adjustment_revisions') \
                     WHERE name = 'effect_chain_json'",
                    [],
                    |row| row.get(0),
                )?,
            ))
        })
        .expect("migration reads");
    assert_eq!(version, "20260811.12");
    assert!(default_expression.contains("restoration"));
    assert!(default_expression.contains("master"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn immediately_previous_revision_adds_delivery_formats() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-delivery-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            transaction.execute("DROP TABLE render_exports", [])?;
            transaction.execute_batch(crate::schema::RENDER_EXPORTS_MIGRATION_SQL)?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260811.7' WHERE key = 'schema_version'",
                [],
            )?;
            Ok(())
        })
        .expect("previous fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("previous revision migrates");
    let (version, table_sql): (String, String) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get(0),
                )?,
                transaction.query_row(
                    "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'render_exports'",
                    [],
                    |row| row.get(0),
                )?,
            ))
        })
        .expect("migration reads");
    assert_eq!(version, "20260811.12");
    assert!(table_sql.contains("wav_pcm16"));
    assert!(table_sql.contains("flac24"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn immediately_previous_revision_adds_restoration_chain() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-restoration-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            transaction.execute(
                "ALTER TABLE asset_adjustment_revisions DROP COLUMN restoration_json",
                [],
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260811.6' WHERE key = 'schema_version'",
                [],
            )?;
            Ok(())
        })
        .expect("previous fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("previous revision migrates");
    let (version, restoration_column_count): (String, i64) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get(0),
                )?,
                transaction.query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('asset_adjustment_revisions') \
                     WHERE name = 'restoration_json'",
                    [],
                    |row| row.get(0),
                )?,
            ))
        })
        .expect("migration reads");
    assert_eq!(version, "20260811.12");
    assert_eq!(restoration_column_count, 1);
    let _ = std::fs::remove_dir_all(root);
}

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
    assert_eq!(version, "20260811.12");
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
    assert_eq!(version, "20260811.12");
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
    assert_eq!(version, "20260811.12");
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
    assert_eq!(version, "20260811.12");
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
    assert_eq!(version, "20260811.12");
    assert_eq!(document_count, 1);
    assert_eq!(fts_count, 1);
    let _ = std::fs::remove_dir_all(root);
}
