use super::*;

use std::path::Path;

use echo_domain::ContentHash;

use crate::{AssetRegistrationInput, RegisterAsset, register_asset};

#[test]
fn cjk_queries_and_literal_quotes_remain_valid_phrases() {
    assert_eq!(segment_cjk("小火车 rain"), "小 火 车 rain");
    assert_eq!(fts_phrase("rain \"station\""), "\"rain \"\"station\"\"\"");

    let root = std::env::temp_dir().join(format!(
        "echo-search-phrase-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("t")
    ));
    let catalog = crate::open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    catalog
        .with_transaction(|transaction| index_transcript(transaction, "fixture", "雨中的小火车"))
        .expect("transcript indexes");
    let hits = catalog
        .with_transaction(|transaction| search_transcripts(transaction, "小火车", 10))
        .expect("CJK phrase searches");
    assert_eq!(hits.len(), 1);
    catalog
        .with_transaction(|transaction| search_transcripts(transaction, "小\"火车", 10))
        .expect("embedded quote cannot break FTS syntax");

    drop(catalog);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn model_reanalysis_cannot_replace_user_calibrated_transcript_search() {
    let root = std::env::temp_dir().join(format!(
        "echo-search-calibration-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("t")
    ));
    let catalog = crate::open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let asset = match register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([229; 32]),
                    path: Path::new("/sounds/calibrated.wav"),
                    size_bytes: 100,
                    codec: Some("pcm"),
                    duration_millis: Some(1_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )? {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset,
            };
            transaction.execute(
                "INSERT INTO metadata_calibration_revisions (asset_id, transcript_text, \
                 created_at_millis) VALUES (?1, ?2, ?3)",
                rusqlite::params![asset.id.to_string(), "user corrected phrase", 2],
            )?;
            index_transcript(transaction, &asset.id.to_string(), "new model phrase")?;
            Ok(asset)
        })
        .expect("fixture writes");

    catalog
        .with_transaction(|transaction| {
            assert!(
                search_transcripts(transaction, "user corrected", 10)?
                    .iter()
                    .any(|hit| hit.asset_id == asset.id.to_string())
            );
            assert!(search_transcripts(transaction, "new model", 10)?.is_empty());
            Ok::<_, crate::CatalogError>(())
        })
        .expect("effective transcript searches");

    drop(catalog);
    let _ = std::fs::remove_dir_all(root);
}
