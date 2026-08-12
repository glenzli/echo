use crate::session::open_session;

const NOW_MILLIS: i64 = 1_786_622_400_000;
const PRIOR_SAME_DAY_MILLIS: i64 = 1_755_075_600_000;
const PRIOR_OTHER_DAY_MILLIS: i64 = 1_754_989_200_000;

#[test]
fn revisit_sections_cross_the_live_desktop_session() {
    let root = std::env::temp_dir().join(format!(
        "echo-desktop-revisit-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("t")
    ));
    let session = open_session(
        root.join("catalog.sqlite").to_str().expect("utf8"),
        root.join("cache").to_str().expect("utf8"),
    )
    .expect("session opens");
    let (resumable, completed, anniversary) = session
        .catalog()
        .with_transaction(|transaction| {
            let register = |seed: u8, recorded_at_millis: i64, imported_at_millis: i64| {
                let path = root.join(format!("revisit-{seed}.wav"));
                let registered = echo_catalog::register_asset(
                    transaction,
                    &echo_catalog::AssetRegistrationInput {
                        content_hash: echo_domain::ContentHash::new([seed; 32]),
                        path: &path,
                        size_bytes: 1_024,
                        codec: Some("pcm"),
                        duration_millis: Some(120_000),
                        recorded_at_millis: Some(recorded_at_millis),
                        imported_at_millis,
                    },
                )?;
                Ok::<_, echo_catalog::CatalogError>(match registered {
                    echo_catalog::RegisterAsset::Created(asset)
                    | echo_catalog::RegisterAsset::Existed(asset) => asset.id,
                })
            };
            let resumable = register(91, PRIOR_OTHER_DAY_MILLIS, 1)?;
            let completed = register(92, PRIOR_OTHER_DAY_MILLIS, 2)?;
            let anniversary = register(93, PRIOR_SAME_DAY_MILLIS, 3)?;
            echo_catalog::record_asset_listening_progress(
                transaction,
                resumable,
                60_000,
                0,
                120_000,
                NOW_MILLIS - 2,
            )?;
            echo_catalog::record_asset_listening_progress(
                transaction,
                completed,
                115_000,
                0,
                120_000,
                NOW_MILLIS - 1,
            )?;
            Ok::<_, echo_catalog::CatalogError>((resumable, completed, anniversary))
        })
        .expect("fixtures write");

    let snapshot = session
        .revisit_snapshot(NOW_MILLIS)
        .expect("Revisit projects");
    assert_eq!(snapshot.continue_listening, vec![resumable.to_string()]);
    assert_eq!(snapshot.recently_listened, vec![completed.to_string()]);
    assert_eq!(snapshot.on_this_day, vec![anniversary.to_string()]);
    assert_eq!(snapshot.recently_added.len(), 3);
    let _ = std::fs::remove_dir_all(root);
}
