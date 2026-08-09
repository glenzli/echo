use std::path::Path;

use echo_domain::{AnalysisKind, AnalysisRecord, ContentHash, ModelIdentity};

use super::*;
use crate::{
    AppendAnalysisRecord, AppendContextualAnalysis, AssetRegistrationInput, RegisterAsset,
    SourceMetadata, SourceMetadataEntry, open_catalog, record_contextual_analysis,
    record_source_metadata, register_asset,
};

#[test]
fn candidates_require_two_members_and_follow_newest_evidence() {
    let root = std::env::temp_dir().join(format!(
        "echo-smart-albums-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("t")
    ));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let (first, second) = seed_candidates(&catalog);

    let candidates = catalog
        .with_transaction(list_smart_album_candidates)
        .expect("candidates read");
    assert!(
        candidates
            .iter()
            .all(|candidate| candidate.member_asset_ids.len() >= 2)
    );
    assert!(
        candidates
            .iter()
            .all(|candidate| !candidate.key.starts_with("ai:mood:"))
    );
    assert_candidate(
        &candidates,
        "ai:event:family breakfast",
        SmartAlbumEvidence::Ai,
        SmartAlbumFacet::Event,
        &[first, second],
    );
    assert_candidate(
        &candidates,
        "ai:place:kitchen",
        SmartAlbumEvidence::Ai,
        SmartAlbumFacet::Place,
        &[first, second],
    );
    assert!(candidates.iter().any(|candidate| {
        candidate.evidence == SmartAlbumEvidence::Original
            && candidate.facet == SmartAlbumFacet::Time
            && members_match(&candidate.member_asset_ids, &[first, second])
    }));
    assert_candidate(
        &candidates,
        "original:place:+31.2304+121.4737/",
        SmartAlbumEvidence::Original,
        SmartAlbumFacet::Place,
        &[first, second],
    );

    catalog
        .with_transaction(|transaction| {
            append_contextual(
                transaction,
                first,
                20,
                "quiet",
                "garden",
                "birdsong",
                "neighbor",
            )
        })
        .expect("new evidence appends");
    let candidates = catalog
        .with_transaction(list_smart_album_candidates)
        .expect("latest candidates read");
    assert!(
        candidates
            .iter()
            .all(|candidate| candidate.key != "ai:event:family breakfast")
    );
    assert!(candidates.iter().any(|candidate| {
        candidate.evidence == SmartAlbumEvidence::Original
            && candidate.facet == SmartAlbumFacet::Time
            && members_match(&candidate.member_asset_ids, &[first, second])
    }));
    let _ = std::fs::remove_dir_all(root);
}

fn seed_candidates(catalog: &crate::Catalog) -> (echo_domain::AssetId, echo_domain::AssetId) {
    catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            let first = register(
                transaction,
                1,
                Path::new("/voices/first.wav"),
                1_786_233_600_000,
            );
            let second = register(
                transaction,
                2,
                Path::new("/voices/second.wav"),
                1_786_237_200_000,
            );
            let third = register(
                transaction,
                3,
                Path::new("/voices/third.wav"),
                1_786_320_000_000,
            );
            append_contextual(
                transaction,
                first,
                10,
                "calm",
                "Kitchen",
                "family breakfast",
                "parent",
            )?;
            append_contextual(
                transaction,
                second,
                11,
                " Calm ",
                " kitchen ",
                "Family breakfast",
                "Parent",
            )?;
            append_contextual(
                transaction,
                third,
                12,
                "busy",
                "station",
                "commute",
                "announcer",
            )?;
            for asset_id in [first, second] {
                record_source_metadata(
                    transaction,
                    asset_id,
                    &SourceMetadata {
                        container_format: "wav".to_owned(),
                        sample_rate: 48_000,
                        channel_count: 1,
                        entries: vec![SourceMetadataEntry {
                            key: "com.apple.quicktime.location.ISO6709".to_owned(),
                            value: "+31.2304+121.4737/".to_owned(),
                        }],
                    },
                    None,
                )?;
            }
            Ok((first, second))
        })
        .expect("fixtures write")
}

fn assert_candidate(
    candidates: &[SmartAlbumCandidate],
    key: &str,
    evidence: SmartAlbumEvidence,
    facet: SmartAlbumFacet,
    members: &[echo_domain::AssetId],
) {
    let candidate = candidates
        .iter()
        .find(|candidate| candidate.key == key)
        .expect("candidate exists");
    assert_eq!(candidate.evidence, evidence);
    assert_eq!(candidate.facet, facet);
    assert!(members_match(&candidate.member_asset_ids, members));
}

fn members_match(left: &[echo_domain::AssetId], right: &[echo_domain::AssetId]) -> bool {
    left.iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        == right
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
}

fn register(
    transaction: &Transaction<'_>,
    byte: u8,
    path: &Path,
    recorded_at_millis: i64,
) -> echo_domain::AssetId {
    match register_asset(
        transaction,
        &AssetRegistrationInput {
            content_hash: ContentHash::new([byte; 32]),
            path,
            size_bytes: 100,
            codec: Some("pcm"),
            duration_millis: Some(1_000),
            recorded_at_millis: Some(recorded_at_millis),
            imported_at_millis: i64::from(byte),
        },
    )
    .expect("asset registers")
    {
        RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset.id,
    }
}

fn append_contextual(
    transaction: &Transaction<'_>,
    asset_id: echo_domain::AssetId,
    recorded_at_millis: i64,
    mood: &str,
    place: &str,
    event: &str,
    person: &str,
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
                        "keywords": ["field note"],
                        "mood": mood,
                        "place_hint": place,
                        "event_type": event,
                        "people_hints": [person],
                    }),
                    ModelIdentity::new("test".into(), "1".into()),
                    None,
                    recorded_at_millis,
                ),
            },
        },
    )
}
