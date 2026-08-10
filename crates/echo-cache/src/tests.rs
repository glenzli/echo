//! Facade contracts for the cache: identity round-trips, idempotent
//! publication, and corrupt-blob quarantine.

use crate::{
    BlobRole, CacheErrorKind, PutBlob, blob_path, open_blob_store, put_blob, put_file,
    quarantine_corrupt, read_verified,
};

fn fixture_root(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("echo-cache-{name}-{}", std::process::id()))
}

#[test]
fn file_publication_streams_to_the_same_identity() {
    let root = fixture_root("file-publication");
    let source = root.with_extension("source");
    std::fs::write(&source, vec![0x5a; 512 * 1024]).expect("fixture writes");
    let store = open_blob_store(&root).expect("store opens");
    let (from_file, status) = put_file(&store, BlobRole::RenderProxy, &source).expect("file put");
    assert_eq!(status, PutBlob::Stored);
    assert_eq!(from_file.size_bytes, 512 * 1024);
    let from_bytes =
        put_blob(&store, BlobRole::RenderProxy, &vec![0x5a; 512 * 1024]).expect("byte put");
    assert_eq!(from_bytes.0.content_hash, from_file.content_hash);
    assert_eq!(from_bytes.1, PutBlob::AlreadyPresent);
    drop(store);
    let _ = std::fs::remove_file(source);
    let _ = std::fs::remove_dir_all(root);
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
