use crate::{CatalogError, open_catalog};
#[test]
fn previous_disclosure_catalog_upgrades_without_losing_its_ledger() {
    let root = std::env::temp_dir().join(format!(
        "echo-generation-migration-{}",
        uuid::Uuid::now_v7()
    ));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).unwrap();
    catalog.with_transaction(|tx| -> Result<(),CatalogError> {
        tx.execute_batch("DROP TABLE generated_audio_receipts; UPDATE catalog_meta SET value='20260922.1' WHERE key='schema_version';")?;
        Ok(())
    }).unwrap();
    drop(catalog);
    let catalog = open_catalog(&path).unwrap();
    catalog
        .with_transaction(|tx| -> Result<(), CatalogError> {
            let count: i64 =
                tx.query_row("SELECT count(*) FROM generated_audio_receipts", [], |r| {
                    r.get(0)
                })?;
            assert_eq!(count, 0);
            let version: String = tx.query_row(
                "SELECT value FROM catalog_meta WHERE key='schema_version'",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(version, "20260922.2");
            Ok(())
        })
        .unwrap();
    drop(catalog);
    std::fs::remove_dir_all(root).unwrap();
}
