use echo_domain::ContentHash;

use super::*;
use crate::{AssetRegistrationInput, RegisterAsset, open_catalog, register_asset};

#[test]
fn segment_progress_resumes_and_plan_replacement_is_bounded() {
    let root = std::env::temp_dir().join(format!("echo-long-audio-{}", std::process::id()));
    let catalog = open_catalog(&root).expect("catalog opens");
    let asset = catalog
        .with_transaction(|transaction| {
            register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([0x11; 32]),
                    path: std::path::Path::new("/tmp/long.wav"),
                    size_bytes: 32,
                    codec: Some("pcm_s16le"),
                    duration_millis: Some(1_100_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )
        })
        .expect("asset registers");
    let asset = match asset {
        RegisterAsset::Created(asset) => asset,
        RegisterAsset::Existed(_) => unreachable!(),
    };
    let plan = [
        LongAudioSegmentPlan {
            index: 0,
            start_millis: 0,
            end_millis: 480_000,
        },
        LongAudioSegmentPlan {
            index: 1,
            start_millis: 480_000,
            end_millis: 960_000,
        },
        LongAudioSegmentPlan {
            index: 2,
            start_millis: 960_000,
            end_millis: 1_100_000,
        },
    ];
    catalog
        .with_transaction(|transaction| {
            ensure_long_audio_plan(transaction, asset.id, 1, &plan, 10)?;
            record_long_audio_proxy(
                transaction,
                asset.id,
                1,
                0,
                &LongAudioProxyRef {
                    content_hash: ContentHash::new([0x22; 32]),
                    size_bytes: 99,
                },
                11,
            )?;
            record_long_audio_stage(
                transaction,
                asset.id,
                1,
                0,
                LongAudioStage::Transcript,
                &serde_json::json!({"text":"first"}),
                12,
            )
        })
        .expect("progress records");
    catalog
        .with_transaction(|transaction| ensure_long_audio_plan(transaction, asset.id, 1, &plan, 20))
        .expect("same plan resumes");
    let segments = catalog
        .with_transaction(|transaction| list_long_audio_segments(transaction, asset.id, 1))
        .expect("segments read");
    assert_eq!(segments.len(), 3);
    assert!(segments[0].proxy.is_some());
    assert_eq!(segments[0].transcript.as_ref().unwrap()["text"], "first");

    let replacement = [LongAudioSegmentPlan {
        index: 0,
        start_millis: 0,
        end_millis: 1_100_000,
    }];
    catalog
        .with_transaction(|transaction| {
            ensure_long_audio_plan(transaction, asset.id, 2, &replacement, 30)
        })
        .expect("new plan replaces projection");
    let old = catalog
        .with_transaction(|transaction| list_long_audio_segments(transaction, asset.id, 1))
        .expect("old plan query");
    assert!(old.is_empty());
    let _ = std::fs::remove_file(root);
}
