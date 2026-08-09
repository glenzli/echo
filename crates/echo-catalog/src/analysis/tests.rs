use std::path::Path;

use echo_domain::{AnalysisKind, AnalysisRecord, ContentHash, ModelIdentity};

use super::*;
use crate::{
    AssetRegistrationInput, RegisterAsset, mark_asset_missing, open_catalog, register_asset,
};

fn register(
    transaction: &rusqlite::Transaction<'_>,
    byte: u8,
    path: &Path,
) -> echo_domain::AssetId {
    match register_asset(
        transaction,
        &AssetRegistrationInput {
            content_hash: ContentHash::new([byte; 32]),
            path,
            size_bytes: 100,
            codec: Some("pcm"),
            duration_millis: Some(1_000),
            recorded_at_millis: None,
            imported_at_millis: i64::from(byte),
        },
    )
    .expect("asset registers")
    {
        RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
    }
}

#[test]
fn missing_analysis_projection_excludes_evidence_and_offline_originals() {
    let root =
        std::env::temp_dir().join(format!("echo-analysis-projection-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let (ready, analyzed, offline) = catalog
        .with_transaction(|transaction| -> Result<_, crate::CatalogError> {
            let ready = register(transaction, 1, Path::new("/voices/ready.wav"));
            let analyzed = register(transaction, 2, Path::new("/voices/analyzed.wav"));
            let offline = register(transaction, 3, Path::new("/voices/offline.wav"));
            record_analysis(
                transaction,
                &AppendAnalysisRecord {
                    asset_id: analyzed,
                    record: AnalysisRecord::new(
                        AnalysisKind::Transcript,
                        serde_json::json!({ "text": "done" }),
                        ModelIdentity::new("test".into(), "1".into()),
                        None,
                        10,
                    ),
                },
            )?;
            mark_asset_missing(transaction, &offline.to_string())?;
            Ok((ready, analyzed, offline))
        })
        .expect("fixture writes");

    let missing = catalog
        .with_transaction(|transaction| {
            list_assets_missing_analysis(transaction, AnalysisKind::Transcript)
        })
        .expect("projection reads");
    assert_eq!(missing, [ready]);
    assert!(!missing.contains(&analyzed));
    assert!(!missing.contains(&offline));
    let _ = std::fs::remove_dir_all(root);
}
