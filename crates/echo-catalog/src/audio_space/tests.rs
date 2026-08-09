use std::path::Path;

use echo_domain::{AnalysisKind, AnalysisRecord, ContentHash, ModelIdentity};

use super::*;
use crate::{
    AppendAnalysisRecord, AssetAffinity, AssetRegistrationInput, RegisterAsset, open_catalog,
    record_analysis, register_asset, set_asset_affinity,
};

#[test]
fn sound_wall_projection_keeps_text_and_user_affinity_distinct() {
    let root = std::env::temp_dir().join(format!("echo-sound-wall-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let asset_id = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let registered = register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([5; 32]),
                    path: Path::new("/voices/memory.wav"),
                    size_bytes: 100,
                    codec: Some("pcm"),
                    duration_millis: Some(1_000),
                    recorded_at_millis: None,
                    imported_at_millis: 10,
                },
            )?;
            let asset_id = match registered {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
            };
            record_analysis(
                transaction,
                &AppendAnalysisRecord {
                    asset_id,
                    record: AnalysisRecord::new(
                        AnalysisKind::Transcript,
                        serde_json::json!({ "text": "旧房子的窗户朝南", "segments": [] }),
                        ModelIdentity::new("test".into(), "1".into()),
                        None,
                        20,
                    ),
                },
            )?;
            set_asset_affinity(
                transaction,
                asset_id,
                AssetAffinity {
                    liked: true,
                    rating: 5,
                },
                30,
            )?;
            Ok(asset_id)
        })
        .expect("fixture writes");

    let assets = catalog
        .with_transaction(list_audio_space)
        .expect("projection reads");
    let projected = assets
        .into_iter()
        .find(|asset| asset.id == asset_id.to_string())
        .expect("asset projects");
    assert!(projected.liked);
    assert_eq!(projected.rating, 5);
    assert_eq!(
        projected.transcript.expect("text evidence")["text"],
        "旧房子的窗户朝南"
    );
    let _ = std::fs::remove_dir_all(root);
}
