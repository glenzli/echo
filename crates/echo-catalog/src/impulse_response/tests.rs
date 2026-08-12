use super::*;

fn content_hash(value: char) -> ContentHash {
    ContentHash::from_str(&value.to_string().repeat(64)).expect("content hash")
}

fn record() -> ImpulseResponseRecord {
    ImpulseResponseRecord {
        import_id: Uuid::parse_str("018f5f1a-ff90-7c71-9ec4-66d36516664c").expect("uuid"),
        source_hash: content_hash('a'),
        source_size_bytes: 4_096,
        prepared_hash: content_hash('b'),
        prepared_size_bytes: 8_192,
        preparation_version: 1,
        source_sample_rate: 44_100,
        channel_count: 2,
        source_frame_count: 22_050,
        prepared_frame_count: 24_000,
        avcodec_version: 1,
        swresample_version: 2,
        imported_at_millis: 123_456,
        original_path: "/tmp/room.wav".to_owned(),
        display_name: "Room IR".to_owned(),
        creator: Some("Field recordist".to_owned()),
        source_url: Some("https://example.invalid/room".to_owned()),
        attribution: Some("Room IR by Field recordist".to_owned()),
        rights: ImpulseResponseRights::Spdx {
            expression: "CC0-1.0".to_owned(),
            license_url: None,
        },
    }
}

#[test]
fn rights_shape_is_explicit() {
    assert_eq!(
        ImpulseResponseRights::UserOwnedNoRedistribution,
        ImpulseResponseRights::UserOwnedNoRedistribution
    );
}

#[test]
fn immutable_import_round_trips_and_rejects_conflicting_identity() {
    let path = std::env::temp_dir().join(format!("echo-ir-catalog-{}.sqlite", Uuid::now_v7()));
    let catalog = crate::open_catalog(&path).expect("catalog");
    let expected = record();
    catalog
        .with_transaction(|transaction| record_impulse_response(transaction, &expected))
        .expect("record");
    catalog
        .with_transaction(|transaction| record_impulse_response(transaction, &expected))
        .expect("idempotent replay");
    let restored = catalog
        .with_transaction(list_impulse_responses)
        .expect("list records");
    assert_eq!(restored, vec![expected.clone()]);

    let mut conflict = expected;
    conflict.display_name = "Different evidence".to_owned();
    assert!(
        catalog
            .with_transaction(|transaction| record_impulse_response(transaction, &conflict))
            .is_err()
    );
    drop(catalog);
    let _ = std::fs::remove_file(path);
}
