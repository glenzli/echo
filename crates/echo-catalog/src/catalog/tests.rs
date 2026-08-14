use super::*;
use crate::{ImpulseResponseLayout, list_impulse_responses};

fn replace_current_ir_schema_with_v4(
    transaction: &rusqlite::Transaction<'_>,
) -> Result<(), CatalogError> {
    transaction.execute_batch(
        "DROP TABLE impulse_response_imports;
         DROP TABLE impulse_response_preparations;
         CREATE TABLE impulse_response_preparations (
            prepared_hash TEXT PRIMARY KEY,
            source_hash TEXT NOT NULL REFERENCES impulse_response_sources(source_hash),
            size_bytes INTEGER NOT NULL CHECK (size_bytes > 0),
            preparation_version INTEGER NOT NULL CHECK (preparation_version > 0),
            source_sample_rate INTEGER NOT NULL CHECK (source_sample_rate > 0),
            channel_count INTEGER NOT NULL CHECK (channel_count IN (1, 2)),
            source_frame_count INTEGER NOT NULL CHECK (source_frame_count > 0),
            prepared_frame_count INTEGER NOT NULL CHECK (prepared_frame_count > 0),
            avcodec_version INTEGER NOT NULL CHECK (avcodec_version > 0),
            swresample_version INTEGER NOT NULL CHECK (swresample_version > 0),
            created_at_millis INTEGER NOT NULL CHECK (created_at_millis >= 0)
         );
         CREATE TABLE impulse_response_imports (
            import_id TEXT PRIMARY KEY,
            source_hash TEXT NOT NULL REFERENCES impulse_response_sources(source_hash),
            prepared_hash TEXT NOT NULL REFERENCES impulse_response_preparations(prepared_hash),
            imported_at_millis INTEGER NOT NULL CHECK (imported_at_millis >= 0),
            original_path TEXT NOT NULL,
            display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
            creator TEXT, source_url TEXT, attribution TEXT,
            rights_kind TEXT NOT NULL CHECK (
                rights_kind IN ('spdx', 'user_owned_no_redistribution')),
            spdx_expression TEXT, license_url TEXT,
            CHECK ((rights_kind = 'spdx' AND length(trim(spdx_expression)) > 0) OR
                   (rights_kind = 'user_owned_no_redistribution' AND
                    spdx_expression IS NULL AND license_url IS NULL))
         );
         CREATE INDEX impulse_response_imports_newest
            ON impulse_response_imports (imported_at_millis DESC, import_id DESC);",
    )?;
    Ok(())
}

#[test]
#[allow(clippy::too_many_lines)] // One physical v4 fixture audits every preserved value domain.
fn true_stereo_revision_backfills_v4_layouts_and_preserves_import_rows() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-true-stereo-migration-{}",
        uuid::Uuid::now_v7()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    let (
        asset_id,
        revision_id,
        recipe_id,
        authored_space,
        authored_creative,
        authored_chain,
        recipe_patch,
    ) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let asset_id = match crate::register_asset(
                transaction,
                &crate::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([0x95; 32]),
                    path: std::path::Path::new("/voice/true-stereo-v4.wav"),
                    size_bytes: 100,
                    codec: Some("pcm"),
                    duration_millis: Some(120_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )? {
                crate::RegisterAsset::Created(asset) | crate::RegisterAsset::Existed(asset) => {
                    asset.id
                }
            };
            crate::record_asset_listening_progress(transaction, asset_id, 40_000, 0, 120_000, 20)?;
            let seeded_listening: (i64, i64) = transaction.query_row(
                "SELECT last_listened_at_millis, resume_position_millis
                 FROM asset_user_state WHERE asset_id = ?1",
                [asset_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?;
            assert_eq!(seeded_listening, (20, 40_000));
            transaction.execute(
                "INSERT INTO metadata_calibration_revisions
                 (asset_id, sound_caption, summary, event_type, mood, keywords_json,
                  transcript_text, language, created_at_millis)
                 VALUES (?1, 'V4 caption', NULL, NULL, NULL, NULL, NULL, 'en', 21)",
                [asset_id.to_string()],
            )?;
            let revision = crate::record_adjustment_graph(
                transaction,
                asset_id,
                echo_domain::AdjustmentGraph::identity(120_000)
                    .expect("identity adjustment validates"),
                22,
            )?;
            let (authored_space, authored_creative, authored_chain): (String, String, String) =
                transaction.query_row(
                    "SELECT space_json, creative_vfx_json, effect_chain_json
                     FROM asset_adjustment_revisions WHERE id = ?1",
                    [revision.revision_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )?;
            let recipe_adjustment = crate::processing_recipe_patch_from_asset(
                transaction,
                asset_id,
                &[echo_domain::ProcessingComponent::Space],
            )?;
            let recipe = crate::create_processing_recipe(
                transaction,
                crate::CreateProcessingRecipe {
                    name: "V4 room",
                    patch: &recipe_adjustment,
                },
                23,
            )?;
            let recipe_patch: String = transaction.query_row(
                "SELECT patch_json FROM processing_recipe_revisions WHERE recipe_id = ?1",
                [recipe.id.to_string()],
                |row| row.get(0),
            )?;
            replace_current_ir_schema_with_v4(transaction)?;
            for (suffix, channels) in [('a', 1), ('b', 2)] {
                let source_hash = suffix.to_string().repeat(64);
                let prepared_hash = char::from_u32(u32::from(suffix) + 2)
                    .expect("fixture character")
                    .to_string()
                    .repeat(64);
                transaction.execute(
                    "INSERT INTO impulse_response_sources
                     (source_hash, size_bytes, created_at_millis)
                     VALUES (?1, 4096, 10)",
                    [&source_hash],
                )?;
                transaction.execute(
                    "INSERT INTO impulse_response_preparations
                     (prepared_hash, source_hash, size_bytes, preparation_version,
                      source_sample_rate, channel_count, source_frame_count,
                      prepared_frame_count, avcodec_version, swresample_version,
                      created_at_millis)
                     VALUES (?1, ?2, 8192, 1, 48000, ?3, 100, 100, 1, 1, 11)",
                    rusqlite::params![prepared_hash, source_hash, channels],
                )?;
                transaction.execute(
                    "INSERT INTO impulse_response_imports
                     (import_id, source_hash, prepared_hash, imported_at_millis,
                      original_path, display_name, creator, source_url, attribution,
                      rights_kind, spdx_expression, license_url)
                     VALUES (?1, ?2, ?3, 12, '/irs/fixture.wav', ?4, NULL, NULL,
                             NULL, 'spdx', 'CC0-1.0', NULL)",
                    rusqlite::params![
                        uuid::Uuid::now_v7().to_string(),
                        source_hash,
                        prepared_hash,
                        format!("{channels} channel")
                    ],
                )?;
            }
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260813.4' WHERE key = 'schema_version'",
                [],
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = 'echo-catalog-20260813.4-freeze-granular-vfx'
                 WHERE key = 'schema_identity'",
                [],
            )?;
            Ok((
                asset_id,
                revision.revision_id,
                recipe.id,
                authored_space,
                authored_creative,
                authored_chain,
                recipe_patch,
            ))
        })
        .expect("v4 fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("v4 migrates");
    migrated
        .with_transaction(|transaction| -> Result<(), CatalogError> {
            let rows = list_impulse_responses(transaction)?;
            assert_eq!(rows.len(), 2);
            assert!(
                rows.iter()
                    .any(|row| row.layout == ImpulseResponseLayout::Mono)
            );
            assert!(
                rows.iter()
                    .any(|row| row.layout == ImpulseResponseLayout::StereoParallel)
            );
            let foreign_key_failures: i64 = transaction.query_row(
                "SELECT COUNT(*) FROM pragma_foreign_key_check",
                [],
                |row| row.get(0),
            )?;
            assert_eq!(foreign_key_failures, 0);
            let listening = crate::asset_listening_state(transaction, asset_id)?;
            assert_eq!(listening.last_listened_at_millis, 20);
            assert_eq!(listening.resume_position_millis, 40_000);
            assert_eq!(
                crate::latest_metadata_calibration(transaction, asset_id)?
                    .expect("metadata calibration remains")
                    .calibration
                    .sound_caption
                    .as_deref(),
                Some("V4 caption")
            );
            let stored_authored: (String, String, String) = transaction.query_row(
                "SELECT space_json, creative_vfx_json, effect_chain_json
                 FROM asset_adjustment_revisions WHERE id = ?1",
                [revision_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
            assert_eq!(
                stored_authored,
                (authored_space, authored_creative, authored_chain)
            );
            assert_eq!(
                transaction.query_row(
                    "SELECT patch_json FROM processing_recipe_revisions WHERE recipe_id = ?1",
                    [recipe_id.to_string()],
                    |row| row.get::<_, String>(0),
                )?,
                recipe_patch
            );
            Ok(())
        })
        .expect("migrated rows read");
    drop(migrated);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[allow(clippy::too_many_lines)] // One predecessor fixture protects every .3-owned value domain.
fn freeze_granular_revision_preserves_metadata_listening_ir_and_authored_bytes() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-freeze-granular-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    let (asset_id, revision_id, legacy_creative, legacy_chain) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let asset_id = match crate::register_asset(
                transaction,
                &crate::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([0x84; 32]),
                    path: std::path::Path::new("/voice/freeze-granular-migration.wav"),
                    size_bytes: 1,
                    codec: Some("pcm"),
                    duration_millis: Some(120_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )? {
                crate::RegisterAsset::Created(asset) | crate::RegisterAsset::Existed(asset) => {
                    asset.id
                }
            };
            crate::record_asset_listening_progress(
                transaction,
                asset_id,
                35_000,
                0,
                120_000,
                2_000,
            )?;
            transaction.execute(
                "INSERT INTO metadata_calibration_revisions
                 (asset_id, sound_caption, summary, event_type, mood, keywords_json,
                  transcript_text, language, created_at_millis)
                 VALUES (?1, 'Preserved caption', NULL, 'dialogue', NULL,
                         '[\"spoken\",\"close\"]', 'Preserved transcript', 'en', 2001)",
                [asset_id.to_string()],
            )?;
            let mut creative = echo_domain::CreativeVfxSettings::default();
            creative.drive.enabled = true;
            creative.drive.drive_centibels = 1_900;
            let graph = echo_domain::AdjustmentGraph::new(
                120_000,
                500,
                119_500,
                120,
                240,
                echo_domain::AdjustmentEffects::new(
                    echo_domain::FadeCurves::new(
                        echo_domain::FadeCurve::Smooth,
                        echo_domain::FadeCurve::EqualPower,
                    ),
                    -325,
                    80,
                )
                .with_creative_vfx(creative),
            )
            .expect("authored graph validates");
            let revision = crate::record_adjustment_graph(transaction, asset_id, graph, 2_002)?;

            let mut legacy_creative =
                serde_json::to_value(creative).expect("creative settings encode");
            let creative_object = legacy_creative
                .as_object_mut()
                .expect("creative settings object");
            creative_object.remove("freeze");
            creative_object.remove("granular");
            let legacy_creative =
                serde_json::to_string(&legacy_creative).expect("legacy creative encodes");
            let mut legacy_chain =
                serde_json::to_value(echo_domain::EffectChain::standard()).expect("chain encodes");
            legacy_chain["nodes"]
                .as_array_mut()
                .expect("chain nodes")
                .truncate(15);
            let legacy_chain = serde_json::to_string(&legacy_chain).expect("legacy chain encodes");
            transaction.execute(
                "UPDATE asset_adjustment_revisions SET creative_vfx_json = ?1,
                 effect_chain_json = ?2 WHERE id = ?3",
                rusqlite::params![legacy_creative, legacy_chain, revision.revision_id],
            )?;

            replace_current_ir_schema_with_v4(transaction)?;
            let source_hash = "a".repeat(64);
            let prepared_hash = "b".repeat(64);
            transaction.execute(
                "INSERT INTO impulse_response_sources
                 (source_hash, size_bytes, created_at_millis) VALUES (?1, 4096, 2003)",
                [&source_hash],
            )?;
            transaction.execute(
                "INSERT INTO impulse_response_preparations
                 (prepared_hash, source_hash, size_bytes, preparation_version,
                  source_sample_rate, channel_count, source_frame_count,
                  prepared_frame_count, avcodec_version, swresample_version,
                  created_at_millis)
                 VALUES (?1, ?2, 8192, 1, 48000, 2, 24000, 24000, 1, 1, 2004)",
                rusqlite::params![prepared_hash, source_hash],
            )?;
            transaction.execute(
                "INSERT INTO impulse_response_imports
                 (import_id, source_hash, prepared_hash, imported_at_millis,
                  original_path, display_name, creator, source_url, attribution,
                  rights_kind, spdx_expression, license_url)
                 VALUES ('018f5f1a-ff90-7c71-9ec4-66d36516664c', ?1, ?2, 2005,
                         '/irs/preserved.wav', 'Preserved IR', 'Recorder', NULL,
                         'Preserved attribution', 'spdx', 'CC0-1.0', NULL)",
                rusqlite::params![source_hash, prepared_hash],
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260813.3' WHERE key = 'schema_version'",
                [],
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value =
                 'echo-catalog-20260813.3-metadata-calibration'
                 WHERE key = 'schema_identity'",
                [],
            )?;
            Ok((
                asset_id,
                revision.revision_id,
                legacy_creative,
                legacy_chain,
            ))
        })
        .expect("real .3 fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("Freeze and Granular revision migrates");
    migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let version: String = transaction.query_row(
                "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )?;
            let identity: String = transaction.query_row(
                "SELECT value FROM catalog_meta WHERE key = 'schema_identity'",
                [],
                |row| row.get(0),
            )?;
            assert_eq!(version, "20260813.5");
            assert_eq!(identity, "echo-catalog-20260813.5-true-stereo-ir");
            assert_eq!(
                transaction.query_row(
                    "SELECT creative_vfx_json FROM asset_adjustment_revisions WHERE id = ?1",
                    [revision_id],
                    |row| row.get::<_, String>(0),
                )?,
                legacy_creative
            );
            assert_eq!(
                transaction.query_row(
                    "SELECT effect_chain_json FROM asset_adjustment_revisions WHERE id = ?1",
                    [revision_id],
                    |row| row.get::<_, String>(0),
                )?,
                legacy_chain
            );
            let listening = crate::asset_listening_state(transaction, asset_id)?;
            assert_eq!(listening.last_listened_at_millis, 2_000);
            assert_eq!(listening.resume_position_millis, 35_000);
            let calibration = crate::latest_metadata_calibration(transaction, asset_id)?
                .expect("metadata calibration remains");
            assert_eq!(
                calibration.calibration.sound_caption.as_deref(),
                Some("Preserved caption")
            );
            assert_eq!(
                transaction.query_row(
                    "SELECT display_name FROM impulse_response_imports",
                    [],
                    |row| row.get::<_, String>(0),
                )?,
                "Preserved IR"
            );
            let restored =
                crate::latest_adjustment_graph(transaction, asset_id)?.expect("adjustment remains");
            assert_eq!(restored.revision_id, revision_id);
            assert_eq!(restored.graph.gain_centibels(), -325);
            assert!(restored.graph.creative_vfx().drive.enabled);
            assert_eq!(
                restored.graph.creative_vfx().freeze,
                echo_domain::FreezeVfxSettings::default()
            );
            assert_eq!(
                restored.graph.creative_vfx().granular,
                echo_domain::GranularVfxSettings::default()
            );
            let normalized = serde_json::to_value(restored.graph.effect_chain())
                .expect("normalized chain encodes");
            assert_eq!(normalized["nodes"].as_array().map(Vec::len), Some(17));
            assert_eq!(restored.graph.effect_chain().nodes().len(), 5);
            assert!(
                !restored
                    .graph
                    .effect_chain()
                    .nodes()
                    .contains(&echo_domain::EffectNodeKind::FreezeVfx)
            );
            assert!(
                !restored
                    .graph
                    .effect_chain()
                    .nodes()
                    .contains(&echo_domain::EffectNodeKind::GranularVfx)
            );
            Ok(())
        })
        .expect("migrated .3 values read");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn metadata_calibration_revision_preserves_analysis_and_convolution_schema() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-metadata-calibration-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    let (asset_id, analysis_count, space_json) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let asset_id = match crate::register_asset(
                transaction,
                &crate::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([0x83; 32]),
                    path: std::path::Path::new("/voice/calibration-migration.wav"),
                    size_bytes: 1,
                    codec: Some("pcm"),
                    duration_millis: Some(2_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )? {
                crate::RegisterAsset::Created(asset) | crate::RegisterAsset::Existed(asset) => {
                    asset.id
                }
            };
            crate::record_analysis(
                transaction,
                &crate::AppendAnalysisRecord {
                    asset_id,
                    record: echo_domain::AnalysisRecord::new(
                        echo_domain::AnalysisKind::Transcript,
                        serde_json::json!({"text":"preserved evidence"}),
                        echo_domain::ModelIdentity::new("asr".into(), "1".into()),
                        None,
                        2,
                    ),
                },
            )?;
            let revision = crate::record_adjustment_graph(
                transaction,
                asset_id,
                echo_domain::AdjustmentGraph::identity(2_000).expect("identity validates"),
                3,
            )?;
            let space_json = transaction.query_row(
                "SELECT space_json FROM asset_adjustment_revisions WHERE id = ?1",
                [revision.revision_id],
                |row| row.get::<_, String>(0),
            )?;
            let analysis_count = transaction.query_row(
                "SELECT COUNT(*) FROM analysis_records WHERE asset_id = ?1",
                [asset_id.to_string()],
                |row| row.get::<_, i64>(0),
            )?;
            transaction.execute_batch(
                "DROP TABLE metadata_calibration_revisions; \
                 UPDATE catalog_meta SET value = '20260813.2' WHERE key = 'schema_version'; \
                 UPDATE catalog_meta SET value = 'echo-catalog-20260813.2-convolution-space' \
                 WHERE key = 'schema_identity';",
            )?;
            Ok((asset_id, analysis_count, space_json))
        })
        .expect("candidate 2 fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("metadata calibration revision migrates");
    migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let version: String = transaction.query_row(
                "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )?;
            assert_eq!(version, "20260813.5");
            assert_eq!(
                transaction.query_row(
                    "SELECT COUNT(*) FROM analysis_records WHERE asset_id = ?1",
                    [asset_id.to_string()],
                    |row| row.get::<_, i64>(0),
                )?,
                analysis_count
            );
            assert_eq!(
                transaction.query_row(
                    "SELECT space_json FROM asset_adjustment_revisions WHERE asset_id = ?1",
                    [asset_id.to_string()],
                    |row| row.get::<_, String>(0),
                )?,
                space_json
            );
            assert!(crate::latest_metadata_calibration(transaction, asset_id)?.is_none());
            Ok(())
        })
        .expect("migrated evidence reads");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn listening_continuity_revision_preserves_user_and_adjustment_state() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-listening-continuity-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    let (asset_id, revision_id, creative_json) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let registered = crate::register_asset(
                transaction,
                &crate::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([0x72; 32]),
                    path: std::path::Path::new("/voice/listening-migration.wav"),
                    size_bytes: 1,
                    codec: Some("pcm"),
                    duration_millis: Some(120_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )?;
            let asset_id = match registered {
                crate::RegisterAsset::Created(asset) | crate::RegisterAsset::Existed(asset) => {
                    asset.id
                }
            };
            crate::set_asset_affinity(
                transaction,
                asset_id,
                crate::AssetAffinity {
                    liked: true,
                    rating: 4,
                },
                2,
            )?;
            let revision = crate::record_adjustment_graph(
                transaction,
                asset_id,
                echo_domain::AdjustmentGraph::identity(120_000)
                    .expect("identity adjustment validates"),
                3,
            )?;
            let creative_json = transaction.query_row(
                "SELECT creative_vfx_json FROM asset_adjustment_revisions WHERE id = ?1",
                [revision.revision_id],
                |row| row.get::<_, String>(0),
            )?;
            transaction.execute(
                "ALTER TABLE asset_user_state DROP COLUMN resume_position_millis",
                [],
            )?;
            transaction.execute(
                "ALTER TABLE asset_user_state DROP COLUMN last_listened_at_millis",
                [],
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260812.2' WHERE key = 'schema_version'",
                [],
            )?;
            Ok((asset_id, revision.revision_id, creative_json))
        })
        .expect("candidate 2 fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("listening continuity revision migrates");
    let (version, affinity, listening, restored_json, restored_revision) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get::<_, String>(0),
                )?,
                crate::asset_affinity(transaction, asset_id)?,
                crate::asset_listening_state(transaction, asset_id)?,
                transaction.query_row(
                    "SELECT creative_vfx_json FROM asset_adjustment_revisions WHERE id = ?1",
                    [revision_id],
                    |row| row.get::<_, String>(0),
                )?,
                crate::latest_adjustment_graph(transaction, asset_id)?
                    .expect("adjustment remains")
                    .revision_id,
            ))
        })
        .expect("migrated listening state reads");
    assert_eq!(version, "20260813.5");
    assert_eq!(
        affinity,
        crate::AssetAffinity {
            liked: true,
            rating: 4
        }
    );
    assert_eq!(listening, crate::AssetListeningState::default());
    assert_eq!(restored_json, creative_json);
    assert_eq!(restored_revision, revision_id);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[allow(clippy::too_many_lines)] // One migration contract protects both predecessor domains.
fn deterministic_vfx_revision_preserves_listening_state_and_legacy_json() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-deterministic-vfx-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    let identity =
        echo_domain::AdjustmentGraph::identity(1_000).expect("identity adjustment validates");
    let mut legacy_creative =
        serde_json::to_value(identity.creative_vfx()).expect("creative VFX encodes");
    legacy_creative
        .as_object_mut()
        .expect("creative VFX object")
        .remove("digital_degrade");
    legacy_creative
        .as_object_mut()
        .expect("creative VFX object")
        .remove("drive");
    legacy_creative
        .as_object_mut()
        .expect("creative VFX object")
        .remove("rotary");
    let legacy_creative =
        serde_json::to_string(&legacy_creative).expect("legacy creative VFX encodes");
    let mut legacy_chain =
        serde_json::to_value(identity.effect_chain()).expect("effect chain encodes");
    legacy_chain["nodes"]
        .as_array_mut()
        .expect("effect nodes array")
        .truncate(12);
    let legacy_chain = serde_json::to_string(&legacy_chain).expect("legacy chain encodes");
    let (asset_id, revision_id) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let registered = crate::register_asset(
                transaction,
                &crate::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([71; 32]),
                    path: std::path::Path::new("/voice/legacy-deterministic-vfx.wav"),
                    size_bytes: 1,
                    codec: Some("pcm"),
                    duration_millis: Some(1_000),
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
                "INSERT INTO asset_user_state
                     (asset_id, liked, rating, last_listened_at_millis,
                      resume_position_millis, updated_at_millis)
                 VALUES (?1, 1, 4, 987654321, 640, 987654322)",
                [asset_id.to_string()],
            )?;
            let revision = crate::record_adjustment_graph(transaction, asset_id, identity, 2)?;
            transaction.execute(
                "UPDATE asset_adjustment_revisions SET creative_vfx_json = ?1, \
                 effect_chain_json = ?2 WHERE id = ?3",
                rusqlite::params![legacy_creative, legacy_chain, revision.revision_id],
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260812.3' WHERE key = 'schema_version'",
                [],
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = 'echo-catalog-20260812.3-listening-continuity'
                 WHERE key = 'schema_identity'",
                [],
            )?;
            Ok((asset_id, revision.revision_id))
        })
        .expect("Listening revision fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("deterministic VFX revision migrates");
    let (
        version,
        identity,
        listening_columns,
        listening_values,
        stored_creative,
        stored_chain,
        restored,
    ) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get::<_, String>(0),
                )?,
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_identity'",
                    [],
                    |row| row.get::<_, String>(0),
                )?,
                transaction.query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('asset_user_state')
                     WHERE name IN ('last_listened_at_millis', 'resume_position_millis')",
                    [],
                    |row| row.get::<_, i64>(0),
                )?,
                transaction.query_row(
                    "SELECT last_listened_at_millis, resume_position_millis
                     FROM asset_user_state WHERE asset_id = ?1",
                    [asset_id.to_string()],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
                )?,
                transaction.query_row(
                    "SELECT creative_vfx_json FROM asset_adjustment_revisions WHERE id = ?1",
                    [revision_id],
                    |row| row.get::<_, String>(0),
                )?,
                transaction.query_row(
                    "SELECT effect_chain_json FROM asset_adjustment_revisions WHERE id = ?1",
                    [revision_id],
                    |row| row.get::<_, String>(0),
                )?,
                crate::latest_adjustment_graph(transaction, asset_id)?
                    .expect("legacy adjustment remains"),
            ))
        })
        .expect("migrated deterministic VFX reads");
    assert_eq!(version, "20260813.5");
    assert_eq!(identity, "echo-catalog-20260813.5-true-stereo-ir");
    assert_eq!(listening_columns, 2);
    assert_eq!(listening_values, (987_654_321, 640));
    assert_eq!(stored_creative, legacy_creative);
    assert_eq!(stored_chain, legacy_chain);
    assert!(!restored.graph.creative_vfx().digital_degrade.enabled);
    let normalized =
        serde_json::to_value(restored.graph.effect_chain()).expect("normalized chain encodes");
    let nodes = normalized["nodes"].as_array().expect("effect nodes array");
    assert_eq!(nodes.len(), 17);
    assert_eq!(
        nodes.last().and_then(serde_json::Value::as_str),
        Some("granular_vfx")
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[allow(clippy::too_many_lines)] // One migration contract preserves VFX and listening evidence.
fn drive_rotary_revision_preserves_deterministic_vfx_bytes_and_listening_state() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-drive-rotary-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    let graph = echo_domain::AdjustmentGraph::identity(2_000).expect("identity graph validates");
    let mut legacy_creative =
        serde_json::to_value(graph.creative_vfx()).expect("creative VFX encodes");
    let creative_object = legacy_creative
        .as_object_mut()
        .expect("creative VFX object");
    creative_object.remove("drive");
    creative_object.remove("rotary");
    let legacy_creative =
        serde_json::to_string(&legacy_creative).expect("legacy creative VFX encodes");
    let mut legacy_chain = serde_json::to_value(graph.effect_chain()).expect("chain encodes");
    legacy_chain["nodes"]
        .as_array_mut()
        .expect("effect nodes array")
        .truncate(13);
    let legacy_chain = serde_json::to_string(&legacy_chain).expect("legacy chain encodes");
    let (asset_id, revision_id) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let registered = crate::register_asset(
                transaction,
                &crate::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([73; 32]),
                    path: std::path::Path::new("/voice/drive-rotary-migration.wav"),
                    size_bytes: 1,
                    codec: Some("pcm"),
                    duration_millis: Some(2_000),
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
                "INSERT INTO asset_user_state
                     (asset_id, liked, rating, last_listened_at_millis,
                      resume_position_millis, updated_at_millis)
                 VALUES (?1, 0, 0, 1234567, 875, 1234568)",
                [asset_id.to_string()],
            )?;
            let revision = crate::record_adjustment_graph(transaction, asset_id, graph, 2)?;
            transaction.execute(
                "UPDATE asset_adjustment_revisions SET creative_vfx_json = ?1, \
                 effect_chain_json = ?2 WHERE id = ?3",
                rusqlite::params![legacy_creative, legacy_chain, revision.revision_id],
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260812.4' WHERE key = 'schema_version'",
                [],
            )?;
            Ok((asset_id, revision.revision_id))
        })
        .expect("deterministic VFX fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("Drive and Rotary revision migrates");
    let (version, stored_creative, stored_chain, listening, restored) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get::<_, String>(0),
                )?,
                transaction.query_row(
                    "SELECT creative_vfx_json FROM asset_adjustment_revisions WHERE id = ?1",
                    [revision_id],
                    |row| row.get::<_, String>(0),
                )?,
                transaction.query_row(
                    "SELECT effect_chain_json FROM asset_adjustment_revisions WHERE id = ?1",
                    [revision_id],
                    |row| row.get::<_, String>(0),
                )?,
                crate::asset_listening_state(transaction, asset_id)?,
                crate::latest_adjustment_graph(transaction, asset_id)?.expect("adjustment remains"),
            ))
        })
        .expect("migrated state reads");
    assert_eq!(version, "20260813.5");
    assert_eq!(stored_creative, legacy_creative);
    assert_eq!(stored_chain, legacy_chain);
    assert_eq!(listening.last_listened_at_millis, 1_234_567);
    assert_eq!(listening.resume_position_millis, 875);
    assert_eq!(restored.revision_id, revision_id);
    assert_eq!(
        restored.graph.creative_vfx().drive,
        echo_domain::DriveVfxSettings::default()
    );
    assert_eq!(
        restored.graph.creative_vfx().rotary,
        echo_domain::RotaryVfxSettings::default()
    );
    assert_eq!(restored.graph.effect_chain().nodes().len(), 5);
    let normalized = serde_json::to_value(restored.graph.effect_chain()).expect("chain normalizes");
    assert_eq!(normalized["nodes"].as_array().map(Vec::len), Some(17));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[allow(clippy::too_many_lines)] // One migration contract protects authored and listening evidence.
fn convolution_space_revision_preserves_drive_rotary_and_listening_state() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-convolution-space-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    let graph = echo_domain::AdjustmentGraph::identity(2_000).expect("identity graph validates");
    let (asset_id, revision_id, creative_json) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let registered = crate::register_asset(
                transaction,
                &crate::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([74; 32]),
                    path: std::path::Path::new("/voice/convolution-space-migration.wav"),
                    size_bytes: 1,
                    codec: Some("pcm"),
                    duration_millis: Some(2_000),
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
                "INSERT INTO asset_user_state
                     (asset_id, liked, rating, last_listened_at_millis,
                      resume_position_millis, updated_at_millis)
                 VALUES (?1, 1, 5, 7654321, 1250, 7654322)",
                [asset_id.to_string()],
            )?;
            let revision = crate::record_adjustment_graph(transaction, asset_id, graph, 2)?;
            let creative_json = transaction.query_row(
                "SELECT creative_vfx_json FROM asset_adjustment_revisions WHERE id = ?1",
                [revision.revision_id],
                |row| row.get::<_, String>(0),
            )?;
            transaction.execute_batch(
                "DROP TABLE impulse_response_imports;
                 DROP TABLE impulse_response_preparations;
                 DROP TABLE impulse_response_sources;
                 ALTER TABLE asset_adjustment_revisions DROP COLUMN space_json;",
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260813.1' WHERE key = 'schema_version'",
                [],
            )?;
            Ok((asset_id, revision.revision_id, creative_json))
        })
        .expect("Drive and Rotary fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("Convolution Space revision migrates");
    let (version, stored_creative, stored_space, listening, restored, ir_table_count) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get::<_, String>(0),
                )?,
                transaction.query_row(
                    "SELECT creative_vfx_json FROM asset_adjustment_revisions WHERE id = ?1",
                    [revision_id],
                    |row| row.get::<_, String>(0),
                )?,
                transaction.query_row(
                    "SELECT space_json FROM asset_adjustment_revisions WHERE id = ?1",
                    [revision_id],
                    |row| row.get::<_, String>(0),
                )?,
                crate::asset_listening_state(transaction, asset_id)?,
                crate::latest_adjustment_graph(transaction, asset_id)?.expect("adjustment remains"),
                transaction.query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN
                     ('impulse_response_sources', 'impulse_response_preparations',
                      'impulse_response_imports')",
                    [],
                    |row| row.get::<_, i64>(0),
                )?,
            ))
        })
        .expect("migrated Convolution Space state reads");
    assert_eq!(version, "20260813.5");
    assert_eq!(stored_creative, creative_json);
    assert_eq!(
        stored_space,
        serde_json::to_string(&echo_domain::SpaceSettings::default()).expect("space encodes")
    );
    assert_eq!(listening.last_listened_at_millis, 7_654_321);
    assert_eq!(listening.resume_position_millis, 1_250);
    assert_eq!(restored.revision_id, revision_id);
    assert_eq!(
        restored.graph.space(),
        echo_domain::SpaceSettings::default()
    );
    assert_eq!(ir_table_count, 3);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn space_character_revision_preserves_legacy_reverb_json_as_room() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-space-character-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    let legacy_json = r#"{"enabled":true,"mix_percent":24,"pre_delay_millis":28,"decay_millis":2400,"size_percent":68,"damping_percent":52,"low_cut_hertz":150,"high_cut_hertz":9000}"#;
    let (asset_id, revision_id) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let registered = crate::register_asset(
                transaction,
                &crate::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([69; 32]),
                    path: std::path::Path::new("/voice/legacy-room.wav"),
                    size_bytes: 1,
                    codec: Some("pcm"),
                    duration_millis: Some(1_000),
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
                echo_domain::AdjustmentGraph::identity(1_000)
                    .expect("identity adjustment validates"),
                2,
            )?;
            transaction.execute(
                "UPDATE asset_adjustment_revisions SET reverb_json = ?1 WHERE id = ?2",
                rusqlite::params![legacy_json, revision.revision_id],
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260811.16' WHERE key = 'schema_version'",
                [],
            )?;
            Ok((asset_id, revision.revision_id))
        })
        .expect("legacy room fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("space character revision migrates");
    let (version, stored_json, restored) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get::<_, String>(0),
                )?,
                transaction.query_row(
                    "SELECT reverb_json FROM asset_adjustment_revisions WHERE id = ?1",
                    [revision_id],
                    |row| row.get::<_, String>(0),
                )?,
                crate::latest_adjustment_graph(transaction, asset_id)?
                    .expect("legacy adjustment remains"),
            ))
        })
        .expect("migrated room reads");
    assert_eq!(version, "20260813.5");
    assert_eq!(stored_json, legacy_json);
    assert_eq!(restored.revision_id, revision_id);
    assert_eq!(
        restored.graph.reverb().character,
        echo_domain::ReverbCharacter::Room
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn creative_vfx_revision_defaults_legacy_history_to_disabled_without_rewriting_it() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-creative-vfx-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    let (asset_id, revision_id, legacy_reverb, legacy_chain) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let registered = crate::register_asset(
                transaction,
                &crate::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([70; 32]),
                    path: std::path::Path::new("/voice/legacy-creative-vfx.wav"),
                    size_bytes: 1,
                    codec: Some("pcm"),
                    duration_millis: Some(1_000),
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
                echo_domain::AdjustmentGraph::identity(1_000)
                    .expect("identity adjustment validates"),
                2,
            )?;
            let legacy_reverb = transaction.query_row(
                "SELECT reverb_json FROM asset_adjustment_revisions WHERE id = ?1",
                [revision.revision_id],
                |row| row.get::<_, String>(0),
            )?;
            let legacy_chain = transaction.query_row(
                "SELECT effect_chain_json FROM asset_adjustment_revisions WHERE id = ?1",
                [revision.revision_id],
                |row| row.get::<_, String>(0),
            )?;
            transaction.execute(
                "ALTER TABLE asset_adjustment_revisions DROP COLUMN creative_vfx_json",
                [],
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260812.1' WHERE key = 'schema_version'",
                [],
            )?;
            Ok((asset_id, revision.revision_id, legacy_reverb, legacy_chain))
        })
        .expect("legacy creative VFX fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("creative VFX revision migrates");
    let (version, stored_reverb, stored_chain, default_expression, restored) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get::<_, String>(0),
                )?,
                transaction.query_row(
                    "SELECT reverb_json FROM asset_adjustment_revisions WHERE id = ?1",
                    [revision_id],
                    |row| row.get::<_, String>(0),
                )?,
                transaction.query_row(
                    "SELECT effect_chain_json FROM asset_adjustment_revisions WHERE id = ?1",
                    [revision_id],
                    |row| row.get::<_, String>(0),
                )?,
                transaction.query_row(
                    "SELECT dflt_value FROM pragma_table_info('asset_adjustment_revisions') \
                     WHERE name = 'creative_vfx_json'",
                    [],
                    |row| row.get::<_, String>(0),
                )?,
                crate::latest_adjustment_graph(transaction, asset_id)?
                    .expect("legacy adjustment remains"),
            ))
        })
        .expect("migrated creative VFX reads");
    assert_eq!(version, "20260813.5");
    assert_eq!(stored_reverb, legacy_reverb);
    assert_eq!(stored_chain, legacy_chain);
    assert!(default_expression.contains("telephone"));
    assert_eq!(restored.revision_id, revision_id);
    assert_eq!(
        restored.graph.creative_vfx(),
        echo_domain::CreativeVfxSettings::default()
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[allow(clippy::too_many_lines)] // One migration contract protects graph and recipe history bytes.
fn channel_repair_revision_adds_identity_without_rewriting_existing_history() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-channel-repair-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    let (asset_id, revision_id, legacy_chain, legacy_patch) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let registered = crate::register_asset(
                transaction,
                &crate::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([68; 32]),
                    path: std::path::Path::new("/voice/legacy-channels.wav"),
                    size_bytes: 1,
                    codec: Some("pcm"),
                    duration_millis: Some(1_000),
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
                echo_domain::AdjustmentGraph::identity(1_000)
                    .expect("identity adjustment validates"),
                2,
            )?;
            let legacy_chain = r#"{"nodes":["restoration","equalizer","dynamics","space","master","de_hum","de_click"],"active_count":5}"#;
            transaction.execute(
                "UPDATE asset_adjustment_revisions SET effect_chain_json = ?1 WHERE id = ?2",
                rusqlite::params![legacy_chain, revision.revision_id],
            )?;
            let legacy_patch = r#"{"legacy":"recipe bytes stay immutable"}"#;
            transaction.execute(
                "INSERT INTO processing_recipes \
                 (id, name, created_at_millis, updated_at_millis, archived_at_millis) \
                 VALUES ('recipe-channel-legacy', 'Legacy channel recipe', 2, 2, NULL)",
                [],
            )?;
            transaction.execute(
                "INSERT INTO processing_recipe_revisions \
                 (id, recipe_id, revision_number, patch_json, created_at_millis) \
                 VALUES ('recipe-channel-legacy-r1', 'recipe-channel-legacy', 1, ?1, 2)",
                [legacy_patch],
            )?;
            transaction.execute(
                "ALTER TABLE asset_adjustment_revisions DROP COLUMN channel_repair_json",
                [],
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260811.15' WHERE key = 'schema_version'",
                [],
            )?;
            Ok((
                asset_id,
                revision.revision_id,
                legacy_chain.to_owned(),
                legacy_patch.to_owned(),
            ))
        })
        .expect("legacy channel fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("channel repair revision migrates");
    let (version, stored_chain, stored_patch, channel_column_count) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get::<_, String>(0),
                )?,
                transaction.query_row(
                    "SELECT effect_chain_json FROM asset_adjustment_revisions WHERE id = ?1",
                    [revision_id],
                    |row| row.get::<_, String>(0),
                )?,
                transaction.query_row(
                    "SELECT patch_json FROM processing_recipe_revisions \
                     WHERE id = 'recipe-channel-legacy-r1'",
                    [],
                    |row| row.get::<_, String>(0),
                )?,
                transaction.query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('asset_adjustment_revisions') \
                     WHERE name = 'channel_repair_json'",
                    [],
                    |row| row.get::<_, i64>(0),
                )?,
            ))
        })
        .expect("migrated channel history reads");
    assert_eq!(version, "20260813.5");
    assert_eq!(channel_column_count, 1);
    assert_eq!(stored_chain, legacy_chain);
    assert_eq!(stored_patch, legacy_patch);
    let restored = migrated
        .with_transaction(|transaction| crate::latest_adjustment_graph(transaction, asset_id))
        .expect("legacy adjustment restores")
        .expect("legacy adjustment exists");
    assert_eq!(restored.revision_id, revision_id);
    assert_eq!(
        restored.graph.channel_repair(),
        echo_domain::ChannelRepairSettings::identity()
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn de_plosive_revision_preserves_legacy_restoration_json_and_defaults_disabled() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-de-plosive-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    let (asset_id, revision_id, legacy_json) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let registered = crate::register_asset(
                transaction,
                &crate::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([67; 32]),
                    path: std::path::Path::new("/voice/legacy-plosives.wav"),
                    size_bytes: 1,
                    codec: Some("pcm"),
                    duration_millis: Some(1_000),
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
                echo_domain::AdjustmentGraph::identity(1_000)
                    .expect("identity adjustment validates"),
                2,
            )?;
            let legacy_json = r#"{"enabled":true,"noise_reduction":{"enabled":false,"reduction_centibels":900,"sensitivity_percent":50,"smoothing_millis":240},"de_esser":{"enabled":false,"frequency_hertz":6500,"threshold_centibels":-2400,"reduction_centibels":600}}"#;
            transaction.execute(
                "UPDATE asset_adjustment_revisions SET restoration_json = ?1 WHERE id = ?2",
                rusqlite::params![legacy_json, revision.revision_id],
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260811.14' WHERE key = 'schema_version'",
                [],
            )?;
            Ok((asset_id, revision.revision_id, legacy_json.to_owned()))
        })
        .expect("legacy fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("de-plosive revision migrates");
    let (version, stored_json, restored) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get::<_, String>(0),
                )?,
                transaction.query_row(
                    "SELECT restoration_json FROM asset_adjustment_revisions WHERE id = ?1",
                    [revision_id],
                    |row| row.get::<_, String>(0),
                )?,
                crate::latest_adjustment_graph(transaction, asset_id)?
                    .expect("legacy adjustment remains"),
            ))
        })
        .expect("migrated restoration reads");
    assert_eq!(version, "20260813.5");
    assert_eq!(stored_json, legacy_json);
    assert_eq!(restored.revision_id, revision_id);
    assert_eq!(
        restored.graph.restoration().de_plosive,
        echo_domain::DePlosiveSettings::default()
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[allow(clippy::too_many_lines)] // One migration fixture protects all append-only history layers.
fn source_edit_revision_adds_columns_without_rewriting_adjustments_recipes_or_receipts() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-source-edit-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    let (asset_id, adjustment_revision_id, recipe, application, patch_json) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let registered = crate::register_asset(
                transaction,
                &crate::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([66; 32]),
                    path: std::path::Path::new("/voice/source-edit-migration.wav"),
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
            let graph = echo_domain::AdjustmentGraph::identity(10_000)
                .expect("identity adjustment validates");
            let adjustment =
                crate::record_adjustment_graph(transaction, asset_id, graph.clone(), 2)?;
            let recipe_patch = echo_domain::AdjustmentPatch::from_graph(
                graph,
                &[echo_domain::ProcessingComponent::Master],
            )
            .expect("recipe patch validates");
            let recipe = crate::create_processing_recipe(
                transaction,
                crate::CreateProcessingRecipe {
                    name: "Historic source edit cleanup",
                    patch: &recipe_patch,
                },
                3,
            )?;
            let application = crate::apply_processing_recipe(
                transaction,
                recipe.id,
                &[asset_id],
                echo_domain::ProcessingMergeMode::Merge,
                4,
            )?;
            let patch_json: String = transaction.query_row(
                "SELECT patch_json FROM processing_recipe_revisions WHERE id = ?1",
                [recipe.current_revision.revision_id().to_string()],
                |row| row.get(0),
            )?;
            transaction.execute(
                "ALTER TABLE asset_adjustment_revisions DROP COLUMN edit_timeline_json",
                [],
            )?;
            transaction.execute(
                "ALTER TABLE asset_adjustment_revisions DROP COLUMN effect_masks_json",
                [],
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260811.13' WHERE key = 'schema_version'",
                [],
            )?;
            Ok((
                asset_id,
                adjustment.revision_id,
                recipe,
                application,
                patch_json,
            ))
        })
        .expect("previous fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("source edit revision migrates");
    let (version, source_edit_columns, adjustment_ids, stored_patch_json): (
        String,
        i64,
        Vec<i64>,
        String,
    ) = migrated
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let mut statement = transaction.prepare(
                "SELECT id FROM asset_adjustment_revisions WHERE asset_id = ?1 ORDER BY id",
            )?;
            let adjustment_ids = statement
                .query_map([asset_id.to_string()], |row| row.get::<_, i64>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok((
                transaction.query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get(0),
                )?,
                transaction.query_row(
                    "SELECT COUNT(*) FROM pragma_table_info('asset_adjustment_revisions') \
                     WHERE name IN ('edit_timeline_json', 'effect_masks_json')",
                    [],
                    |row| row.get(0),
                )?,
                adjustment_ids,
                transaction.query_row(
                    "SELECT patch_json FROM processing_recipe_revisions WHERE id = ?1",
                    [recipe.current_revision.revision_id().to_string()],
                    |row| row.get(0),
                )?,
            ))
        })
        .expect("migrated evidence reads");
    assert_eq!(version, "20260813.5");
    assert_eq!(source_edit_columns, 2);
    assert_eq!(adjustment_ids, vec![adjustment_revision_id]);
    assert_eq!(stored_patch_json, patch_json);
    assert_eq!(
        migrated
            .with_transaction(|transaction| {
                crate::processing_recipe_application_receipt(transaction, application.batch_id)
            })
            .expect("application receipt reads")
            .expect("application receipt remains"),
        application
    );
    let restored = migrated
        .with_transaction(|transaction| crate::latest_adjustment_graph(transaction, asset_id))
        .expect("legacy adjustment reads")
        .expect("legacy adjustment remains");
    assert_eq!(restored.revision_id, adjustment_revision_id);
    assert_eq!(restored.graph.edit_timeline().segments().len(), 1);
    assert_eq!(restored.graph.effect_masks(), &[]);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
#[allow(clippy::too_many_lines)] // Migration fixture preserves every recipe evidence layer.
fn immediately_previous_revision_adds_recipe_management_without_rewriting_history() {
    let root = std::env::temp_dir().join(format!(
        "echo-schema-recipe-management-migration-{}",
        std::process::id()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("current catalog opens");
    let (recipe, application) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let registered = crate::register_asset(
                transaction,
                &crate::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([65; 32]),
                    path: std::path::Path::new("/voice/recipe-management.wav"),
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
            let graph = echo_domain::AdjustmentGraph::identity(10_000)
                .expect("identity adjustment validates");
            let recipe_patch = echo_domain::AdjustmentPatch::from_graph(
                graph,
                &[echo_domain::ProcessingComponent::Master],
            )
            .expect("recipe patch validates");
            let recipe = crate::create_processing_recipe(
                transaction,
                crate::CreateProcessingRecipe {
                    name: "Historic cleanup",
                    patch: &recipe_patch,
                },
                2,
            )?;
            let application = crate::apply_processing_recipe(
                transaction,
                recipe.id,
                &[asset_id],
                echo_domain::ProcessingMergeMode::Merge,
                3,
            )?;
            transaction.execute_batch(
                "DROP TABLE processing_recipe_application_revert_targets;
                 DROP TABLE processing_recipe_application_reverts;
                 ALTER TABLE processing_recipes DROP COLUMN archived_at_millis;",
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = '20260811.12' WHERE key = 'schema_version'",
                [],
            )?;
            Ok((recipe, application))
        })
        .expect("previous fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("previous revision migrates");
    let (version, archived_column_count, revert_table_count, revision_count) = migrated
        .with_transaction(
            |transaction| -> Result<(String, i64, i64, i64), CatalogError> {
                Ok((
                    transaction.query_row(
                        "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                        [],
                        |row| row.get(0),
                    )?,
                    transaction.query_row(
                        "SELECT COUNT(*) FROM pragma_table_info('processing_recipes') \
                     WHERE name = 'archived_at_millis'",
                        [],
                        |row| row.get(0),
                    )?,
                    transaction.query_row(
                        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN (\
                     'processing_recipe_application_reverts', \
                     'processing_recipe_application_revert_targets')",
                        [],
                        |row| row.get(0),
                    )?,
                    transaction.query_row(
                        "SELECT COUNT(*) FROM processing_recipe_revisions WHERE recipe_id = ?1",
                        [recipe.id.to_string()],
                        |row| row.get(0),
                    )?,
                ))
            },
        )
        .expect("migration schema reads");
    assert_eq!(version, "20260813.5");
    assert_eq!(archived_column_count, 1);
    assert_eq!(revert_table_count, 2);
    assert_eq!(revision_count, 1);
    assert_eq!(
        migrated
            .with_transaction(|transaction| crate::processing_recipe(transaction, recipe.id))
            .expect("recipe reads")
            .expect("recipe remains")
            .current_revision,
        recipe.current_revision
    );
    assert_eq!(
        migrated
            .with_transaction(|transaction| {
                crate::processing_recipe_application_receipt(transaction, application.batch_id)
            })
            .expect("application reads")
            .expect("application remains"),
        application
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn processing_recipe_revision_adds_recipes_without_rewriting_assets() {
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
                "DROP TABLE processing_recipe_application_revert_targets;
                 DROP TABLE processing_recipe_application_reverts;
                 DROP TABLE processing_recipe_application_targets;
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
    assert_eq!(version, "20260813.5");
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
    assert_eq!(version, "20260813.5");
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
    assert_eq!(version, "20260813.5");
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
    assert_eq!(version, "20260813.5");
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
    assert_eq!(version, "20260813.5");
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
    assert_eq!(version, "20260813.5");
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
    assert_eq!(version, "20260813.5");
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
    assert_eq!(version, "20260813.5");
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
    assert_eq!(version, "20260813.5");
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
    assert_eq!(version, "20260813.5");
    assert_eq!(document_count, 1);
    assert_eq!(fts_count, 1);
    let _ = std::fs::remove_dir_all(root);
}
