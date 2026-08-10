use std::str::FromStr;

use super::{CatalogSchemaRevision, SCHEMA_VERSION};
use crate::{CatalogError, CatalogErrorKind, open_catalog};

#[test]
fn revision_round_trips_in_date_dot_sequence_form() {
    assert_eq!(SCHEMA_VERSION.to_string(), "20260811.8");
    assert_eq!(
        CatalogSchemaRevision::from_str("20260811.8"),
        Ok(SCHEMA_VERSION)
    );
    assert_eq!(SCHEMA_VERSION.date(), 20_260_811);
    assert_eq!(SCHEMA_VERSION.daily_sequence(), 8);
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
    assert_eq!(stored, "20260811.8");

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
