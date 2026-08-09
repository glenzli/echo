use std::path::Path;

use super::{import_job_id, scan_job_id};

#[test]
fn import_identity_changes_with_file_fingerprint() {
    let original = import_job_id(Path::new("/audio/voice.wav"), 100, 10);
    assert_eq!(
        original,
        import_job_id(Path::new("/audio/voice.wav"), 100, 10)
    );
    assert_ne!(
        original,
        import_job_id(Path::new("/audio/voice.wav"), 101, 10)
    );
    assert_ne!(
        original,
        import_job_id(Path::new("/audio/voice.wav"), 100, 11)
    );
}

#[test]
fn import_identity_does_not_collapse_path_separators_to_one_name() {
    let nested = import_job_id(Path::new("/a/b_voice.wav"), 100, 10);
    let flat = import_job_id(Path::new("/a_b/voice.wav"), 100, 10);
    assert_ne!(nested, flat);
}

#[test]
fn scan_identity_preserves_the_full_root_path() {
    assert_ne!(
        scan_job_id(Path::new("/a/b_voice")),
        scan_job_id(Path::new("/a_b/voice"))
    );
}
