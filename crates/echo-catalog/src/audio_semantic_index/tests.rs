use super::*;
use crate::{AssetRegistrationInput, RegisterAsset, open_catalog, register_asset};
use echo_domain::ContentHash;
use std::path::Path;

#[test]
fn only_present_current_sources_in_the_exact_space_are_searchable() {
    let root = std::env::temp_dir().join(format!("echo-clap-index-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog");
    catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let mut query = vec![0.0; CLAP_DIMENSIONS];
            query[0] = 1.0;
            let mut ids = Vec::new();
            for index in 0..4_u8 {
                let hash = ContentHash::new([index + 17; 32]);
                let asset = match register_asset(
                    transaction,
                    &AssetRegistrationInput {
                        content_hash: hash.clone(),
                        path: Path::new(&format!("/clap/{index}.wav")),
                        size_bytes: 100,
                        codec: Some("pcm"),
                        duration_millis: Some(8000),
                        recorded_at_millis: None,
                        imported_at_millis: i64::from(index),
                    },
                )? {
                    RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset,
                };
                let source = AudioSemanticSource {
                    asset_id: asset.id,
                    source_revision: format!("echo:clap-audio:v1:{hash}"),
                    path: format!("/clap/{index}.wav"),
                    duration_millis: 8000,
                };
                upsert_audio_semantic_segment(
                    transaction,
                    &source,
                    if index == 3 {
                        "other-space"
                    } else {
                        "fixture-space"
                    },
                    &query,
                    &serde_json::json!({}),
                    1,
                )?;
                ids.push(asset.id.to_string());
            }
            transaction.execute(
                "UPDATE assets SET path_status='missing' WHERE id=?1",
                [&ids[1]],
            )?;
            transaction.execute(
                "UPDATE audio_semantic_segments SET source_revision='obsolete' WHERE asset_id=?1",
                [&ids[2]],
            )?;
            let hits = search_audio_semantic_segments(transaction, &query, "fixture-space", 20)?;
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].asset_id, ids[0]);
            assert!((hits[0].score - 1.0).abs() < 1e-6);
            assert!(
                search_audio_semantic_segments(transaction, &query[..511], "fixture-space", 20)
                    .is_err()
            );
            query[0] = f32::NAN;
            assert!(
                search_audio_semantic_segments(transaction, &query, "fixture-space", 20).is_err()
            );
            Ok(())
        })
        .expect("index lifecycle");
    drop(catalog);
    let _ = std::fs::remove_dir_all(root);
}
