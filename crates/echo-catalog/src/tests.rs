//! Facade contracts spanning catalog modules: idempotent registration,
//! level projection after analysis records, and asset listing.

use crate::{
    AppendAnalysisRecord, AssetLookup, AssetRegistrationInput, RegisterAsset, find_by_content_hash,
    find_by_id, list_assets, open_catalog, query_analysis, record_analysis, register_asset,
};
use echo_domain::{AnalysisKind, AnalysisLevel, AnalysisRecord, ContentHash, ModelIdentity};

#[test]
fn registration_is_idempotent_by_content() {
    let root = std::env::temp_dir().join(format!("echo-catalog-idempotent-{}", std::process::id()));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("catalog opens");
    let hash = ContentHash::new([7; 32]);

    let first = catalog
        .with_transaction(|transaction| {
            register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: hash,
                    path: &path,
                    size_bytes: 1024,
                    codec: None,
                    duration_millis: None,
                    recorded_at_millis: None,
                    imported_at_millis: 1_700_000_000_000,
                },
            )
        })
        .expect("first registration");
    let second = catalog
        .with_transaction(|transaction| {
            register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: hash,
                    path: &path,
                    size_bytes: 1024,
                    codec: None,
                    duration_millis: None,
                    recorded_at_millis: None,
                    imported_at_millis: 1_700_000_000_001,
                },
            )
        })
        .expect("second registration");

    match (first, second) {
        (RegisterAsset::Created(first), RegisterAsset::Existed(second)) => {
            assert_eq!(first.id, second.id, "same content must keep one identity");
        }
        other => panic!("expected created-then-existed, got {other:?}"),
    }

    let lookup = catalog
        .with_transaction(|transaction| find_by_content_hash(transaction, hash))
        .expect("lookup");
    assert!(matches!(lookup, AssetLookup::Found(_)));
    drop(catalog);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn analysis_records_lift_the_level_projection() {
    let root = std::env::temp_dir().join(format!("echo-catalog-level-{}", std::process::id()));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("catalog opens");
    let hash = ContentHash::new([9; 32]);

    let asset = catalog
        .with_transaction(|transaction| {
            register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: hash,
                    path: &path,
                    size_bytes: 1024,
                    codec: None,
                    duration_millis: None,
                    recorded_at_millis: None,
                    imported_at_millis: 0,
                },
            )
        })
        .expect("registration");
    let RegisterAsset::Created(asset) = asset else {
        panic!("fixture must create")
    };
    assert_eq!(asset.max_level, AnalysisLevel::Metadata);

    catalog
        .with_transaction(|transaction| {
            record_analysis(
                transaction,
                &AppendAnalysisRecord {
                    asset_id: asset.id,
                    record: AnalysisRecord::new(
                        AnalysisKind::Emotions,
                        serde_json::json!({ "emotion": "happy" }),
                        ModelIdentity::new("sensevoice".to_owned(), "small".to_owned()),
                        Some(0.9),
                        1_700_000_000_002,
                    ),
                },
            )
        })
        .expect("record");
    catalog
        .with_transaction(|transaction| {
            record_analysis(
                transaction,
                &AppendAnalysisRecord {
                    asset_id: asset.id,
                    record: AnalysisRecord::new(
                        AnalysisKind::Transcript,
                        serde_json::json!({ "text": "hello" }),
                        ModelIdentity::new("qwen-asr".to_owned(), "1.7b".to_owned()),
                        Some(0.95),
                        1_700_000_000_003,
                    ),
                },
            )
        })
        .expect("record");

    let projected = catalog
        .with_transaction(|transaction| {
            find_by_id(transaction, asset.id).map(|lookup| match lookup {
                AssetLookup::Found(asset) => asset,
                AssetLookup::NotFound => panic!("asset must exist"),
            })
        })
        .expect("read");
    assert_eq!(projected.max_level, AnalysisLevel::Understanding);

    let records = catalog
        .with_transaction(|transaction| query_analysis(transaction, asset.id))
        .expect("query");
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].kind, AnalysisKind::Transcript, "newest first");
    assert_eq!(records[0].model.name, "qwen-asr");
    drop(catalog);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn list_returns_newest_import_first() {
    let root = std::env::temp_dir().join(format!("echo-catalog-list-{}", std::process::id()));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("catalog opens");
    for (index, byte) in [1u8, 2u8, 3u8].into_iter().enumerate() {
        catalog
            .with_transaction(|transaction| {
                register_asset(
                    transaction,
                    &AssetRegistrationInput {
                        content_hash: ContentHash::new([byte; 32]),
                        path: &path,
                        size_bytes: 1024,
                        codec: None,
                        duration_millis: None,
                        recorded_at_millis: None,
                        imported_at_millis: i64::try_from(index).expect("index"),
                    },
                )
            })
            .expect("registration");
    }
    let assets = catalog.with_transaction(list_assets).expect("listing");
    assert_eq!(assets.len(), 3);
    assert_eq!(assets[0].original.imported_at_millis, 2);
    assert_eq!(assets[2].original.imported_at_millis, 0);
    drop(catalog);
    let _ = std::fs::remove_dir_all(root);
}
