use echo_cache::{BlobRole, blob_path, open_blob_store, put_blob};

use super::*;

#[test]
fn waveform_reader_rejects_incompatible_payload_shape() {
    let root = std::env::temp_dir().join(format!("echo-waveform-read-{}", std::process::id()));
    let store = open_blob_store(&root).expect("store opens");
    let malformed = WaveformArtifactPayload {
        schema: WAVEFORM_ARTIFACT_SCHEMA,
        canonical_sample_rate: 48_000,
        levels: vec![WaveformArtifactLevel {
            samples_per_bucket: 256,
            mins: vec![-0.5],
            maxs: Vec::new(),
        }],
    };
    let bytes = serde_json::to_vec(&malformed).expect("encodes");
    let (blob, _) = put_blob(&store, BlobRole::WaveformPyramid, &bytes).expect("stores");
    let error = read_waveform_payload(&root, blob.content_hash, blob.size_bytes)
        .expect_err("shape rejected");
    assert_eq!(error.kind, CoreErrorKind::Other);
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn waveform_reader_quarantines_corrupt_content_before_rebuild() {
    let root = std::env::temp_dir().join(format!("echo-waveform-corrupt-{}", std::process::id()));
    let store = open_blob_store(&root).expect("store opens");
    let payload = WaveformArtifactPayload {
        schema: WAVEFORM_ARTIFACT_SCHEMA,
        canonical_sample_rate: 48_000,
        levels: Vec::new(),
    };
    let bytes = serde_json::to_vec(&payload).expect("encodes");
    let (blob, _) = put_blob(&store, BlobRole::WaveformPyramid, &bytes).expect("stores");
    let canonical = blob_path(&root, &blob.content_hash);
    std::fs::write(&canonical, b"corrupt").expect("corrupts fixture");
    let _ = read_waveform_payload(&root, blob.content_hash, blob.size_bytes)
        .expect_err("corruption rejected");
    assert!(
        !canonical.exists(),
        "corrupt canonical blob was quarantined"
    );
    let _ = std::fs::remove_dir_all(root);
}
