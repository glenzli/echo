use crate::session::open_session;

#[test]
fn listening_continuity_crosses_the_live_session_projection() {
    let root = std::env::temp_dir().join(format!(
        "echo-desktop-listening-state-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("t")
    ));
    let session = open_session(
        root.join("catalog.sqlite").to_str().expect("utf8"),
        root.join("cache").to_str().expect("utf8"),
    )
    .expect("session opens");
    let asset_id = session
        .catalog()
        .with_transaction(|transaction| {
            let registered = echo_catalog::register_asset(
                transaction,
                &echo_catalog::AssetRegistrationInput {
                    content_hash: echo_domain::ContentHash::new([83; 32]),
                    path: &root.join("long-listening.wav"),
                    size_bytes: 1_024,
                    codec: Some("pcm"),
                    duration_millis: Some(120_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )?;
            Ok::<_, echo_catalog::CatalogError>(match registered {
                echo_catalog::RegisterAsset::Created(asset)
                | echo_catalog::RegisterAsset::Existed(asset) => asset.id,
            })
        })
        .expect("fixture writes");
    session
        .set_asset_affinity(&asset_id.to_string(), true, 4)
        .expect("affinity saves");

    let checkpoint = session
        .record_listening_progress(&asset_id.to_string(), 72_000, 0, 120_000)
        .expect("checkpoint saves");
    assert_eq!(checkpoint.resume_position_millis, 72_000);
    let assets = session.list_assets().expect("assets project");
    assert_eq!(assets[0].resume_position_millis, 72_000);
    assert!(assets[0].last_listened_at_millis > 0);
    assert!(assets[0].liked);
    assert_eq!(assets[0].rating, 4);

    let completed = session
        .record_listening_progress(&asset_id.to_string(), 115_000, 0, 120_000)
        .expect("near-end checkpoint saves");
    assert_eq!(completed.resume_position_millis, 0);
    let assets = session.list_assets().expect("assets reproject");
    assert_eq!(assets[0].resume_position_millis, 0);
    assert!(assets[0].liked);
    assert_eq!(assets[0].rating, 4);
    let _ = std::fs::remove_dir_all(root);
}
