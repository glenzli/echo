use std::path::Path;

use echo_domain::{AssetId, ContentHash};

use super::*;
use crate::{AssetRegistrationInput, RegisterAsset, open_catalog, register_asset};

#[test]
fn album_lifecycle_preserves_explicit_members_and_cover() {
    let root = std::env::temp_dir().join(format!(
        "echo-user-albums-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("t")
    ));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let (first, second, third) = catalog
        .with_transaction(|transaction| -> Result<_, CatalogError> {
            Ok((
                register(transaction, 1, "/voices/first.wav"),
                register(transaction, 2, "/voices/second.wav"),
                register(transaction, 3, "/voices/third.wav"),
            ))
        })
        .expect("assets register");

    let album_id = catalog
        .with_transaction(|transaction| {
            create_user_album(
                transaction,
                CreateUserAlbum {
                    name: "Family mornings",
                    member_asset_ids: &[first, second, first],
                },
                10,
            )
        })
        .expect("album creates");
    let albums = catalog
        .with_transaction(list_user_albums)
        .expect("album lists");
    assert_eq!(albums.len(), 1);
    assert_eq!(albums[0].cover_asset_id, Some(first));
    assert_eq!(albums[0].member_asset_ids.len(), 2);

    catalog
        .with_transaction(|transaction| {
            set_user_album_membership(transaction, album_id, third, true, 20)
        })
        .expect("member adds");
    catalog
        .with_transaction(|transaction| {
            set_user_album_membership(transaction, album_id, first, false, 30)
        })
        .expect("cover removes");
    catalog
        .with_transaction(|transaction| {
            rename_user_album(transaction, album_id, "Morning voices", 40)
        })
        .expect("album renames");

    let album = catalog
        .with_transaction(list_user_albums)
        .expect("album lists")
        .pop()
        .expect("album remains");
    assert_eq!(album.name, "Morning voices");
    assert_eq!(album.cover_asset_id, Some(second));
    assert_eq!(album.member_asset_ids, vec![second, third]);
    assert_eq!(album.updated_at_millis, 40);

    catalog
        .with_transaction(|transaction| delete_user_album(transaction, album_id))
        .expect("album deletes");
    assert!(
        catalog
            .with_transaction(list_user_albums)
            .expect("albums list")
            .is_empty()
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn names_are_bounded_and_unique_without_rewriting_user_text() {
    let root = std::env::temp_dir().join(format!(
        "echo-user-album-names-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("t")
    ));
    let catalog = open_catalog(&root.join("catalog.sqlite")).expect("catalog opens");
    let album_id = catalog
        .with_transaction(|transaction| {
            create_user_album(
                transaction,
                CreateUserAlbum {
                    name: "Field Notes",
                    member_asset_ids: &[],
                },
                1,
            )
        })
        .expect("album creates");
    for invalid in ["", " padded", "padded ", "line\nbreak"] {
        assert!(
            catalog
                .with_transaction(|transaction| {
                    rename_user_album(transaction, album_id, invalid, 2)
                })
                .is_err(),
            "{invalid:?} must be rejected"
        );
    }
    assert!(
        catalog
            .with_transaction(|transaction| {
                create_user_album(
                    transaction,
                    CreateUserAlbum {
                        name: "field notes",
                        member_asset_ids: &[],
                    },
                    3,
                )
            })
            .is_err()
    );
    assert_eq!(
        catalog
            .with_transaction(list_user_albums)
            .expect("albums list")[0]
            .name,
        "Field Notes"
    );
    let _ = std::fs::remove_dir_all(root);
}

fn register(transaction: &Transaction<'_>, byte: u8, path: &str) -> AssetId {
    match register_asset(
        transaction,
        &AssetRegistrationInput {
            content_hash: ContentHash::new([byte; 32]),
            path: Path::new(path),
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
