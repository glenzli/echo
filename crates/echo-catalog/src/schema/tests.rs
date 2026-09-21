use std::str::FromStr;

use super::{CatalogSchemaRevision, SCHEMA_VERSION, SOUND_ASSEMBLY_PREDECESSOR_SCHEMA_VERSION};
use crate::{CatalogError, CatalogErrorKind, open_catalog};

#[test]
fn revision_round_trips_in_date_dot_sequence_form() {
    assert_eq!(SCHEMA_VERSION.to_string(), "20260922.4");
    assert_eq!(
        CatalogSchemaRevision::from_str("20260815.5"),
        Ok(SOUND_ASSEMBLY_PREDECESSOR_SCHEMA_VERSION)
    );
    assert_eq!(SCHEMA_VERSION.date(), 20_260_922);
    assert_eq!(SCHEMA_VERSION.daily_sequence(), 4);
}

#[test]
fn sound_assembly_predecessor_migrates_atomically() {
    let root = std::env::temp_dir().join(format!(
        "echo-sound-assembly-migration-{}-{}",
        std::process::id(),
        uuid::Uuid::now_v7()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("catalog opens");
    catalog
        .with_transaction(|transaction| -> Result<(), CatalogError> {
            transaction.execute_batch(
                "DROP TABLE sound_assembly_exports;
                 DROP TABLE sound_assembly_clip_sources;
                 DROP TABLE sound_assembly_revisions;
                 DROP TABLE sound_assemblies;",
            )?;
            transaction.execute(
                "UPDATE catalog_meta SET value = ?1 WHERE key = 'schema_version'",
                [SOUND_ASSEMBLY_PREDECESSOR_SCHEMA_VERSION.to_string()],
            )?;
            Ok(())
        })
        .expect("predecessor fixture writes");
    drop(catalog);

    let migrated = open_catalog(&path).expect("predecessor migrates");
    migrated
        .with_transaction(|transaction| -> Result<(), CatalogError> {
            let version: String = transaction.query_row(
                "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )?;
            let table_count: i64 = transaction.query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name IN (
                    'sound_assemblies', 'sound_assembly_revisions',
                    'sound_assembly_clip_sources', 'sound_assembly_exports')",
                [],
                |row| row.get(0),
            )?;
            assert_eq!(version, SCHEMA_VERSION.to_string());
            assert_eq!(table_count, 4);
            Ok(())
        })
        .expect("migrated schema reads");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn revision_rejects_compact_or_noncanonical_encodings() {
    for invalid in [
        "2026080902",
        "20260809.02",
        "20260809.0",
        "2026089.2",
        "2026-08-09.2",
        "20260809.2.1",
        "20260230.1",
        "20261301.1",
    ] {
        assert!(
            CatalogSchemaRevision::from_str(invalid).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn catalog_persists_only_the_canonical_revision_text() {
    let root = std::env::temp_dir().join(format!("echo-schema-format-{}", std::process::id()));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("catalog opens");
    let stored: String = catalog
        .with_transaction(|transaction| -> Result<String, CatalogError> {
            transaction
                .query_row(
                    "SELECT value FROM catalog_meta WHERE key = 'schema_version'",
                    [],
                    |row| row.get(0),
                )
                .map_err(CatalogError::from)
        })
        .expect("revision reads");
    assert_eq!(stored, "20260922.4");

    catalog
        .with_transaction(|transaction| -> Result<(), CatalogError> {
            transaction.execute(
                "UPDATE catalog_meta SET value = '2026080902' WHERE key = 'schema_version'",
                [],
            )?;
            Ok(())
        })
        .expect("fixture becomes noncanonical");
    drop(catalog);
    let error = open_catalog(&path).expect_err("compact revision is rejected");
    assert_eq!(error.kind, CatalogErrorKind::SchemaMismatch);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn selection_transcript_vocabulary_upgrade_preserves_existing_records() {
    let root =
        std::env::temp_dir().join(format!("echo-selection-migration-{}", uuid::Uuid::now_v7()));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("catalog");
    catalog.with_transaction(|tx| -> Result<(),CatalogError> {
        tx.execute("UPDATE catalog_meta SET value = '20260920.1' WHERE key = 'schema_version'",[])?;
        tx.execute("UPDATE catalog_meta SET value = 'echo-catalog-20260920.1-memory-library' WHERE key = 'schema_identity'",[])?;
        tx.execute("INSERT INTO catalog_meta (key,value) VALUES ('keep','sentinel')",[])?;
        Ok(())
    }).expect("old vocabulary");
    drop(catalog);
    let migrated = open_catalog(&path).expect("upgrade");
    migrated
        .with_transaction(|tx| -> Result<(), CatalogError> {
            let kept: String =
                tx.query_row("SELECT value FROM catalog_meta WHERE key='keep'", [], |r| {
                    r.get(0)
                })?;
            let version: String = tx.query_row(
                "SELECT value FROM catalog_meta WHERE key='schema_version'",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(kept, "sentinel");
            assert_eq!(version, "20260922.4");
            Ok(())
        })
        .expect("preserved");
    drop(migrated);
    std::fs::remove_dir_all(root).expect("cleanup");
}
