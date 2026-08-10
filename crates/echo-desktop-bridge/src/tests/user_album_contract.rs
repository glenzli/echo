use crate::session::open_session;

#[test]
fn user_album_lifecycle_crosses_the_live_session_projection() {
    let root = std::env::temp_dir().join(format!(
        "echo-desktop-user-album-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("t")
    ));
    let session = open_session(
        root.join("catalog.sqlite").to_str().expect("utf8"),
        root.join("cache").to_str().expect("utf8"),
    )
    .expect("session opens");
    let asset_ids = session
        .catalog()
        .with_transaction(
            |transaction| -> Result<Vec<String>, echo_catalog::CatalogError> {
                [71, 72]
                    .into_iter()
                    .map(|byte| {
                        let registered = echo_catalog::register_asset(
                            transaction,
                            &echo_catalog::AssetRegistrationInput {
                                content_hash: echo_domain::ContentHash::new([byte; 32]),
                                path: &root.join(format!("voice-{byte}.wav")),
                                size_bytes: 1_024,
                                codec: Some("pcm"),
                                duration_millis: Some(2_100),
                                recorded_at_millis: None,
                                imported_at_millis: i64::from(byte),
                            },
                        )?;
                        Ok(match registered {
                            echo_catalog::RegisterAsset::Created(asset)
                            | echo_catalog::RegisterAsset::Existed(asset) => asset.id.to_string(),
                        })
                    })
                    .collect()
            },
        )
        .expect("fixtures write");

    let album_id = session
        .create_user_album("Morning voices", &asset_ids)
        .expect("suggestion snapshot saves");
    let albums = session.user_albums().expect("albums project");
    assert_eq!(albums.len(), 1);
    assert_eq!(albums[0].id, album_id);
    assert_eq!(albums[0].name, "Morning voices");
    assert_eq!(albums[0].count, 2);
    assert_eq!(albums[0].cover_asset_id, asset_ids[0]);

    session
        .set_user_album_membership(album_id, &asset_ids[0], false)
        .expect("member removes");
    session
        .rename_user_album(album_id, "Quiet mornings")
        .expect("album renames");
    let albums = session.user_albums().expect("albums reproject");
    assert_eq!(albums[0].name, "Quiet mornings");
    assert_eq!(albums[0].count, 1);
    assert_eq!(albums[0].cover_asset_id, asset_ids[1]);

    session.delete_user_album(album_id).expect("album deletes");
    assert!(session.user_albums().expect("albums list").is_empty());
    let _ = std::fs::remove_dir_all(root);
}
