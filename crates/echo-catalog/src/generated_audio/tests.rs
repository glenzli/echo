use crate::{CatalogError, open_catalog};

#[test]
fn persisted_prompt_revisions_keep_exact_original_and_effective_text() {
    let mut receipt = serde_json::json!({
        "schema_version": 3,
        "input_text": "Rain, no music",
        "request": {"prompt": "Rain, no music"},
        "prompt_preparation": {
            "original_prompt": "Rain, no music", "effective_prompt": "Rain, no music",
            "rules_revision": "infer.sound-prompt-preparation@20260926.1",
            "text_job": null, "preparation_elapsed_ms": 0
        }
    });
    assert!(super::prompt_matches(&receipt));
    receipt["prompt_preparation"]["rules_revision"] =
        "infer.sound-prompt-preparation@20260926.2".into();
    assert!(super::prompt_matches(&receipt));
    receipt["request"]["prompt"] = "Rain and music".into();
    assert!(!super::prompt_matches(&receipt));
    receipt["request"]["prompt"] = "Rain, no music".into();
    receipt["prompt_preparation"]["rules_revision"] = "unknown".into();
    assert!(!super::prompt_matches(&receipt));
}

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
            assert_eq!(version, "20260922.4");
            Ok(())
        })
        .unwrap();
    drop(catalog);
    std::fs::remove_dir_all(root).unwrap();
}
