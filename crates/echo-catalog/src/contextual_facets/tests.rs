use std::path::Path;

use echo_domain::{AnalysisKind, AnalysisRecord, ContentHash, ModelIdentity};

use super::*;
use crate::{AssetRegistrationInput, RegisterAsset, open_catalog, register_asset};

#[test]
fn facets_aggregate_normalized_keywords_from_only_latest_contextual_evidence() {
    let root = std::env::temp_dir().join(format!("echo-contextual-facets-{}", std::process::id()));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let (first, second) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let first = register(transaction, 1, Path::new("/voices/first.wav"));
            let second = register(transaction, 2, Path::new("/voices/second.wav"));
            append(
                transaction,
                first,
                10,
                &["Rain".into(), " Field Notes ".into()],
            )?;
            append(transaction, second, 11, &[" rain ".into()])?;
            Ok((first, second))
        })
        .expect("fixture writes");

    let facets = catalog
        .with_transaction(list_contextual_keyword_facets)
        .expect("facets read");
    assert_eq!(facets[0].key, "rain");
    assert_eq!(facets[0].count, 2);
    assert_eq!(facets[1].key, "field notes");

    catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            append(transaction, first, 12, &["Train".into()])
        })
        .expect("new evidence appends");
    let facets = catalog
        .with_transaction(list_contextual_keyword_facets)
        .expect("latest facets read");
    assert_eq!(
        facets
            .iter()
            .map(|facet| (facet.key.as_str(), facet.count))
            .collect::<Vec<_>>(),
        [("rain", 1), ("train", 1)]
    );
    assert!(facets.iter().all(|facet| facet.count <= 2));
    assert_ne!(first, second);
    let _ = std::fs::remove_dir_all(root);
}

fn register(transaction: &Transaction<'_>, byte: u8, path: &Path) -> echo_domain::AssetId {
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

fn append(
    transaction: &Transaction<'_>,
    asset_id: echo_domain::AssetId,
    recorded_at_millis: i64,
    keywords: &[String],
) -> Result<(), CatalogError> {
    record_contextual_analysis(
        transaction,
        &AppendContextualAnalysis {
            analysis: AppendAnalysisRecord {
                asset_id,
                record: AnalysisRecord::new(
                    AnalysisKind::Contextual,
                    serde_json::json!({
                        "summary": "fixture",
                        "keywords": keywords,
                    }),
                    ModelIdentity::new("test".into(), "1".into()),
                    None,
                    recorded_at_millis,
                ),
            },
            keywords,
        },
    )
}
