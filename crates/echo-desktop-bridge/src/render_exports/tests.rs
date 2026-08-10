use std::{fs, path::Path};

use echo_catalog::{AssetRegistrationInput, RegisterAsset, register_asset};
use echo_domain::ContentHash;

#[test]
fn session_records_verified_original_render() {
    let root = std::env::temp_dir().join(format!("echo-session-render-{}", std::process::id()));
    fs::create_dir_all(&root).expect("fixture root creates");
    let source = root.join("source.wav");
    let output = root.join("output.wav");
    fs::write(&source, b"immutable").expect("source writes");
    fs::write(&output, b"rendered").expect("output writes");
    let session = crate::session::open_session(
        root.join("catalog.sqlite").to_str().expect("utf8 path"),
        root.join("cache").to_str().expect("utf8 path"),
    )
    .expect("session opens");
    let asset = session
        .catalog()
        .with_transaction(|transaction| {
            let registered = register_asset(
                transaction,
                &AssetRegistrationInput {
                    content_hash: ContentHash::new([3; 32]),
                    path: Path::new(&source),
                    size_bytes: 9,
                    codec: Some("pcm"),
                    duration_millis: Some(1_000),
                    recorded_at_millis: None,
                    imported_at_millis: 1,
                },
            )?;
            Ok::<_, echo_catalog::CatalogError>(match registered {
                RegisterAsset::Created(asset) | RegisterAsset::Existed(asset) => asset,
            })
        })
        .expect("asset registers");
    let id = session
        .record_render_export(
            &asset.id.to_string(),
            0,
            output.to_str().expect("utf8 path"),
            48_000,
            2,
            24,
            1,
            8,
            -18.0,
            -1.0,
        )
        .expect("render records");
    assert!(id > 0);
    let _ = fs::remove_dir_all(root);
}
