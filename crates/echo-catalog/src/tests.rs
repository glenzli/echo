//! Catalog facade contracts spanning asset paths and scan-root ownership.

use crate::{
    AssetLookup, AssetRegistrationInput, Catalog, CatalogError, add_scan_root,
    find_by_content_hash, list_scan_roots, mark_asset_missing, open_catalog, register_asset,
    relink_asset_by_hash, remove_scan_root,
};
use echo_domain::ContentHash;
use std::path::Path;
use std::path::PathBuf;

fn fixture(name: &str) -> (PathBuf, Catalog) {
    let root =
        std::env::temp_dir().join(format!("echo-catalog-jobs-{}-{name}", std::process::id()));
    let path = root.join("catalog.sqlite");
    let catalog = open_catalog(&path).expect("catalog opens");
    (root, catalog)
}

#[test]
fn scan_roots_round_trip_and_remove() {
    let (root, catalog) = fixture("roots");
    catalog
        .with_transaction(|transaction| add_scan_root(transaction, Path::new("/voices"), 1))
        .expect("add");
    catalog
        .with_transaction(|transaction| add_scan_root(transaction, Path::new("/voices"), 2))
        .expect("duplicate add is idempotent");
    let roots = catalog.with_transaction(list_scan_roots).expect("list");
    assert_eq!(roots.len(), 1);
    catalog
        .with_transaction(|transaction| remove_scan_root(transaction, roots[0].id))
        .expect("remove");
    let roots = catalog.with_transaction(list_scan_roots).expect("list");
    assert!(roots.is_empty());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn relink_restores_missing_assets_by_hash() {
    let (root, catalog) = fixture("relink");
    let hash = ContentHash::new([5; 32]);
    catalog
        .with_transaction(|transaction| {
            register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: hash,
                    path: Path::new("/old/lost.wav"),
                    size_bytes: 100,
                    codec: None,
                    duration_millis: None,
                    recorded_at_millis: None,
                    imported_at_millis: 0,
                },
            )
        })
        .expect("register");

    // Find the real id through the hash lookup.
    let id = catalog
        .with_transaction(|transaction| -> Result<String, CatalogError> {
            match find_by_content_hash(transaction, hash)? {
                AssetLookup::Found(asset) => Ok(asset.id.to_string()),
                AssetLookup::NotFound => panic!("must exist"),
            }
        })
        .expect("find id");
    catalog
        .with_transaction(|transaction| mark_asset_missing(transaction, &id))
        .expect("mark missing");

    let relinked = catalog
        .with_transaction(|transaction| {
            relink_asset_by_hash(transaction, &hash.to_string(), Path::new("/new/found.wav"))
        })
        .expect("relink");
    assert!(relinked, "missing asset relinks by hash");
    let asset = catalog
        .with_transaction(
            |transaction| -> Result<echo_domain::AudioAsset, CatalogError> {
                match find_by_content_hash(transaction, hash)? {
                    AssetLookup::Found(asset) => Ok(asset),
                    AssetLookup::NotFound => panic!("must exist"),
                }
            },
        )
        .expect("find");
    assert_eq!(
        asset.original.path,
        std::path::PathBuf::from("/new/found.wav")
    );
    let _ = std::fs::remove_dir_all(root);
}
