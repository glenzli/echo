use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use super::{InferRuntimeCredentialError, InferRuntimeCredentialStore, read_credential};

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn imports_and_loads_an_owner_only_credential() {
    let fixture = TestDirectory::new();
    let source = fixture.path().join("source.token");
    write_protected(&source, b"test-consumer-token\n");
    let store = InferRuntimeCredentialStore::at(fixture.path().join("echo-credentials"));

    let destination = store.import_from(&source).expect("import credential");
    let loaded = store.load().expect("load imported credential");

    assert_eq!(destination, store.credential_path());
    assert_eq!(format!("{loaded:?}"), "InferRuntimeCredential([REDACTED])");
    assert_eq!(loaded.as_bytes(), b"test-consumer-token");
    assert_owner_only(&destination);
    assert_owner_only(destination.parent().expect("credential directory"));
}

#[test]
fn rejects_a_group_readable_source() {
    let fixture = TestDirectory::new();
    let source = fixture.path().join("source.token");
    fs::write(&source, b"test-consumer-token\n").expect("write fixture");
    set_mode(&source, 0o640);

    let error = read_credential(&source).expect_err("reject permissive source");

    assert!(matches!(
        error,
        InferRuntimeCredentialError::InsecurePermissions { .. }
    ));
}

#[test]
fn rejects_whitespace_inside_a_bearer_value() {
    let fixture = TestDirectory::new();
    let source = fixture.path().join("source.token");
    write_protected(&source, b"not a bearer token\n");

    let error = read_credential(&source).expect_err("reject invalid bearer");

    assert!(matches!(
        error,
        InferRuntimeCredentialError::InvalidCredential
    ));
}

#[cfg(unix)]
#[test]
fn rejects_a_symlink_source() {
    use std::os::unix::fs::symlink;

    let fixture = TestDirectory::new();
    let target = fixture.path().join("target.token");
    let source = fixture.path().join("source.token");
    write_protected(&target, b"test-consumer-token\n");
    symlink(&target, &source).expect("create source symlink");

    let error = read_credential(&source).expect_err("reject symlink");

    assert!(matches!(
        error,
        InferRuntimeCredentialError::InvalidFileType { .. }
    ));
}

fn write_protected(path: &Path, contents: &[u8]) {
    fs::write(path, contents).expect("write fixture");
    set_mode(path, 0o600);
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).expect("set fixture mode");
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) {}

#[cfg(unix)]
fn assert_owner_only(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::metadata(path)
        .expect("read metadata")
        .permissions()
        .mode();
    assert!(mode.trailing_zeros() >= 6);
}

#[cfg(not(unix))]
fn assert_owner_only(_path: &Path) {}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let sequence = TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "echo-credential-tests-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create test directory");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
