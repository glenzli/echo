use std::collections::BTreeSet;

use crate::session::open_session;

#[test]
fn smart_album_membership_crosses_the_live_cxx_projection() {
    let root = std::env::temp_dir().join(format!(
        "echo-desktop-smart-album-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("t")
    ));
    let session = open_session(
        root.join("catalog.sqlite").to_str().expect("utf8"),
        root.join("cache").to_str().expect("utf8"),
    )
    .expect("session opens");
    let ids = session
        .catalog()
        .with_transaction(
            |transaction| -> Result<Vec<_>, echo_catalog::CatalogError> {
                let mut ids = Vec::new();
                for byte in [41, 42] {
                    let registered = echo_catalog::register_asset(
                        transaction,
                        &echo_catalog::AssetRegistrationInput {
                            content_hash: echo_domain::ContentHash::new([byte; 32]),
                            path: &root.join(format!("voice-{byte}.wav")),
                            size_bytes: 1024,
                            codec: Some("pcm"),
                            duration_millis: Some(2_100),
                            recorded_at_millis: Some(1_786_233_600_000 + i64::from(byte)),
                            imported_at_millis: i64::from(byte),
                        },
                    )?;
                    let asset = match registered {
                        echo_catalog::RegisterAsset::Created(asset)
                        | echo_catalog::RegisterAsset::Existed(asset) => asset,
                    };
                    echo_catalog::record_contextual_analysis(
                        transaction,
                        &echo_catalog::AppendContextualAnalysis {
                            analysis: echo_catalog::AppendAnalysisRecord {
                                asset_id: asset.id,
                                record: echo_domain::AnalysisRecord::new(
                                    echo_domain::AnalysisKind::Contextual,
                                    serde_json::json!({
                                        "schema_version": 2,
                                        "sound_caption": "Breakfast voices in a kitchen",
                                        "summary": "fixture",
                                        "keywords": ["kitchen"],
                                        "mood": "calm",
                                        "place_hint": "home",
                                        "event_type": "family breakfast",
                                        "people_hints": []
                                    }),
                                    echo_domain::ModelIdentity::new("qwen".into(), "build".into()),
                                    None,
                                    i64::from(byte),
                                ),
                            },
                        },
                    )?;
                    ids.push(asset.id.to_string());
                }
                Ok(ids)
            },
        )
        .expect("fixtures write");

    let albums = session.smart_albums().expect("albums project");
    let event = albums
        .iter()
        .find(|album| album.key == "ai:event:family breakfast")
        .expect("event album exists");
    assert_eq!(event.evidence, "ai");
    assert_eq!(event.facet, "event");
    assert_eq!(event.count, 2);
    assert_eq!(
        event
            .member_asset_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>(),
        ids.into_iter().collect::<BTreeSet<_>>()
    );
    let _ = std::fs::remove_dir_all(root);
}
