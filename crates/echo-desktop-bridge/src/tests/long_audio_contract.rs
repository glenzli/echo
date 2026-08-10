//! Long-recording outline projection through the desktop session boundary.

use super::fixture_catalog;
use crate::session::open_session;

#[test]
fn outline_nodes_project_as_source_time_chapters() {
    let root = fixture_catalog();
    let session = open_session(
        root.join("catalog.sqlite").to_str().expect("utf8"),
        root.join("cache").to_str().expect("utf8"),
    )
    .expect("session opens");
    let asset = session
        .catalog()
        .with_transaction(|transaction| {
            echo_catalog::register_asset(
                transaction,
                &echo_catalog::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([79; 32]),
                    path: &root.join("long.wav"),
                    size_bytes: 32 * 1024 * 1024,
                    codec: Some("pcm"),
                    duration_millis: Some(960_000),
                    recorded_at_millis: None,
                    imported_at_millis: 0,
                },
            )
        })
        .expect("registration");
    let echo_catalog::RegisterAsset::Created(asset) = asset else {
        panic!("fixture must create")
    };

    session
        .catalog()
        .with_transaction(|transaction| {
            echo_catalog::ensure_long_audio_plan(
                transaction,
                asset.id,
                echo_core::LONG_AUDIO_PLAN_VERSION,
                &[
                    echo_catalog::LongAudioSegmentPlan {
                        index: 0,
                        start_millis: 0,
                        end_millis: 480_000,
                    },
                    echo_catalog::LongAudioSegmentPlan {
                        index: 1,
                        start_millis: 480_000,
                        end_millis: 960_000,
                    },
                ],
                1,
            )?;
            echo_catalog::upsert_long_audio_outline_node(
                transaction,
                &echo_catalog::LongAudioOutlineNode {
                    asset_id: asset.id,
                    plan_version: echo_core::LONG_AUDIO_PLAN_VERSION,
                    level: 0,
                    index: 1,
                    start_millis: 480_000,
                    end_millis: 960_000,
                    contextual: serde_json::json!({
                        "payload": {
                            "schema_version": 3,
                            "sound_caption": "Second chapter",
                            "summary": "A compact chapter summary.",
                            "keywords": [],
                            "mood": null,
                            "place_hint": null,
                            "event_type": null,
                            "people_hints": []
                        }
                    }),
                    updated_at_millis: 2,
                },
            )
        })
        .expect("outline records");

    let chapters = session
        .long_audio_chapters(&asset.id.to_string())
        .expect("chapters project");
    assert_eq!(chapters.len(), 1);
    assert_eq!(chapters[0].level, 0);
    assert_eq!(chapters[0].index, 1);
    assert_eq!(chapters[0].start_millis, 480_000);
    assert_eq!(chapters[0].end_millis, 960_000);
    assert_eq!(chapters[0].sound_caption, "Second chapter");
    assert_eq!(chapters[0].summary, "A compact chapter summary.");

    let _ = std::fs::remove_dir_all(root);
}
