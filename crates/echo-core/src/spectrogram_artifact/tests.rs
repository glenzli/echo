use echo_cache::{BlobRole, open_blob_store, put_blob};

use super::*;

#[test]
fn spectrogram_reader_rejects_incompatible_payload_shape() {
    let root = std::env::temp_dir().join(format!("echo-spectrogram-read-{}", std::process::id()));
    let store = open_blob_store(&root).expect("store opens");
    let malformed = SpectrogramArtifactPayload {
        schema: SPECTROGRAM_ARTIFACT_SCHEMA,
        canonical_sample_rate: 48_000,
        window_frames: 2_048,
        hop_frames: 512,
        time_columns: 2,
        frequency_bins: SPECTROGRAM_FREQUENCY_BINS,
        magnitudes: vec![0; 1],
    };
    let bytes = serde_json::to_vec(&malformed).expect("encodes");
    let (blob, _) = put_blob(&store, BlobRole::SpectrogramOverview, &bytes).expect("stores");
    let error = read_spectrogram_payload(&root, blob.content_hash, blob.size_bytes)
        .expect_err("shape rejected");
    assert_eq!(error.kind, CoreErrorKind::Other);
    let _ = std::fs::remove_dir_all(root);
}
