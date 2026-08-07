//! Facade contracts for the cache: identity round-trips, idempotent
//! publication, and corrupt-blob quarantine.

use crate::{
    BlobRole, CacheErrorKind, PutBlob, blob_path, open_blob_store, put_blob, quarantine_corrupt,
    read_verified,
};

fn fixture_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("echo-cache-{name}-{}", std::process::id()))
}

#[test]
fn put_read_round_trip_is_identity_stable() {
    let root = fixture_root("roundtrip");
    let store = open_blob_store(&root).expect("store opens");
    let payload = b"waveform pyramid bytes";
    let (blob, status) = put_blob(&store, BlobRole::WaveformPyramid, payload).expect("put");
    assert_eq!(status, PutBlob::Stored);
    assert_eq!(blob.size_bytes, payload.len() as u64);

    let again = put_blob(&store, BlobRole::WaveformPyramid, payload).expect("second put");
    assert_eq!(again.1, PutBlob::AlreadyPresent);

    let read = read_verified(&store, blob.content_hash, 1024).expect("verified read");
    assert_eq!(read, payload);
    drop(store);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn canonical_path_is_derived_from_digest() {
    let root = fixture_root("path");
    let store = open_blob_store(&root).expect("store opens");
    let (blob, _) = put_blob(&store, BlobRole::Embedding, b"x").expect("put");
    let text = blob.content_hash.to_string();
    let (first, rest) = text.split_at(2);
    let expected = root.join("blobs").join("b3").join(first).join(rest);
    assert_eq!(blob_path(&root, &blob.content_hash), expected);
    drop(store);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn corrupt_blob_quarantines_instead_of_serving() {
    let root = fixture_root("quarantine");
    let store = open_blob_store(&root).expect("store opens");
    let (blob, _) = put_blob(&store, BlobRole::RenderProxy, b"original").expect("put");
    let path = blob_path(&root, &blob.content_hash);
    std::fs::write(&path, b"tampered").expect("corrupt the blob");

    let read = read_verified(&store, blob.content_hash, 1024);
    assert_eq!(
        read.expect_err("tampered bytes must fail").kind,
        CacheErrorKind::Corrupt
    );

    quarantine_corrupt(&store, blob.content_hash, 1024).expect("quarantine");
    assert!(!path.exists(), "corrupt blob must leave the canonical tree");
    assert!(read_verified(&store, blob.content_hash, 1024).is_err());
    drop(store);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn bounded_read_rejects_oversized_payload() {
    let root = fixture_root("bound");
    let store = open_blob_store(&root).expect("store opens");
    let (blob, _) = put_blob(&store, BlobRole::Transcript, b"long payload").expect("put");
    let read = read_verified(&store, blob.content_hash, 4);
    assert_eq!(
        read.expect_err("oversized read must fail").kind,
        CacheErrorKind::TooLarge
    );
    drop(store);
    let _ = std::fs::remove_dir_all(root);
}
