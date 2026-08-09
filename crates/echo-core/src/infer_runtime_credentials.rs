//! Owner-only storage for Echo's Infer Runtime consumer credential.
//!
//! The credential remains inside Rust: UI and C++ callers can observe only
//! whether it is available. Import validates the protected source and writes
//! an atomic owner-only copy into Echo's application data directory.

use std::{
    env, fmt, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use thiserror::Error;

const CREDENTIAL_DIRECTORY: &str = "credentials";
const CREDENTIAL_FILE: &str = "infer-runtime.token";
const MAX_CREDENTIAL_BYTES: u64 = 8 * 1024;
static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// A validated bearer credential whose debug representation is always redacted.
pub struct InferRuntimeCredential(String);

impl InferRuntimeCredential {
    /// Transfers the bearer value to the Runtime transport configuration.
    #[must_use]
    pub fn into_bearer_token(self) -> String {
        self.0
    }

    fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl fmt::Debug for InferRuntimeCredential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("InferRuntimeCredential([REDACTED])")
    }
}

/// Failures from the Echo-owned consumer credential store.
#[derive(Debug, Error)]
pub enum InferRuntimeCredentialError {
    /// The current platform has no usable per-user application data root.
    #[error("Echo's per-user data directory is unavailable")]
    DataDirectoryUnavailable,
    /// No credential has been imported yet.
    #[error("Infer Runtime consumer credential is missing at {path}")]
    Missing {
        /// Expected Echo-owned credential path.
        path: PathBuf,
    },
    /// A credential path is not a regular file or a store path is not a directory.
    #[error("Infer Runtime consumer credential store has an invalid file type at {path}")]
    InvalidFileType {
        /// Path that failed structural validation.
        path: PathBuf,
    },
    /// A credential or its private directory is accessible by group/other users.
    #[error("Infer Runtime consumer credential store is not owner-only at {path}")]
    InsecurePermissions {
        /// Path whose permissions are too broad.
        path: PathBuf,
    },
    /// The credential cannot be used as an HTTP bearer value.
    #[error("Infer Runtime consumer credential has invalid content")]
    InvalidCredential,
    /// A filesystem operation failed. The credential contents are never attached.
    #[error("Infer Runtime consumer credential store operation failed at {path}: {source}")]
    Io {
        /// Path involved in the failed operation.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: io::Error,
    },
}

/// Echo-owned secure storage for the Infer Runtime consumer credential.
#[derive(Debug, Clone)]
pub struct InferRuntimeCredentialStore {
    directory: PathBuf,
}

impl InferRuntimeCredentialStore {
    /// Uses an explicit private directory. This is primarily useful for
    /// focused tests and administrative tooling.
    #[must_use]
    pub fn at(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    /// Resolves Echo's platform-appropriate private credential directory.
    ///
    /// # Errors
    ///
    /// Returns [`InferRuntimeCredentialError::DataDirectoryUnavailable`] when
    /// the current user has no discoverable application data directory.
    pub fn for_current_user() -> Result<Self, InferRuntimeCredentialError> {
        Ok(Self::at(
            default_echo_data_root()?.join(CREDENTIAL_DIRECTORY),
        ))
    }

    /// Returns the stable destination for the Runtime consumer credential.
    #[must_use]
    pub fn credential_path(&self) -> PathBuf {
        self.directory.join(CREDENTIAL_FILE)
    }

    /// Reads and validates Echo's stored consumer credential.
    ///
    /// # Errors
    ///
    /// Returns a safe, content-free error when the credential is missing,
    /// structurally invalid, too permissive, unreadable, or unsuitable for an
    /// HTTP bearer header.
    pub fn load(&self) -> Result<InferRuntimeCredential, InferRuntimeCredentialError> {
        validate_private_directory(&self.directory)?;
        read_credential(&self.credential_path())
    }

    /// Reports whether a valid, owner-only credential is available.
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.load().is_ok()
    }

    /// Imports a protected source credential into Echo's private store.
    ///
    /// # Errors
    ///
    /// Returns a safe, content-free error if the source is absent, not a
    /// regular owner-only file, invalid, or cannot be written atomically.
    pub fn import_from(&self, source: &Path) -> Result<PathBuf, InferRuntimeCredentialError> {
        let credential = read_credential(source)?;
        ensure_private_directory(&self.directory)?;
        let destination = self.credential_path();
        write_credential_atomically(&destination, &credential)?;
        Ok(destination)
    }
}

/// Loads the current user's Echo-owned Infer Runtime consumer credential.
///
/// # Errors
///
/// Returns a content-free credential-store error when no safe credential is
/// available.
pub fn load_infer_runtime_credential() -> Result<InferRuntimeCredential, InferRuntimeCredentialError>
{
    InferRuntimeCredentialStore::for_current_user()?.load()
}

/// Reports whether the current user has a valid Echo-owned Runtime credential.
#[must_use]
pub fn infer_runtime_credential_available() -> bool {
    InferRuntimeCredentialStore::for_current_user().is_ok_and(|store| store.is_available())
}

fn read_credential(path: &Path) -> Result<InferRuntimeCredential, InferRuntimeCredentialError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            InferRuntimeCredentialError::Missing {
                path: path.to_owned(),
            }
        } else {
            io_error(path, source)
        }
    })?;
    if !metadata.file_type().is_file() {
        return Err(InferRuntimeCredentialError::InvalidFileType {
            path: path.to_owned(),
        });
    }
    validate_owner_only(path, &metadata)?;
    if metadata.len() == 0 || metadata.len() > MAX_CREDENTIAL_BYTES {
        return Err(InferRuntimeCredentialError::InvalidCredential);
    }

    let bytes = fs::read(path).map_err(|source| io_error(path, source))?;
    let value = std::str::from_utf8(&bytes)
        .map_err(|_| InferRuntimeCredentialError::InvalidCredential)?
        .trim();
    if value.is_empty()
        || value.len() > usize::try_from(MAX_CREDENTIAL_BYTES).unwrap_or(usize::MAX)
        || !value.is_ascii()
        || value.chars().any(char::is_whitespace)
        || value.chars().any(char::is_control)
    {
        return Err(InferRuntimeCredentialError::InvalidCredential);
    }
    Ok(InferRuntimeCredential(value.to_owned()))
}

fn ensure_private_directory(path: &Path) -> Result<(), InferRuntimeCredentialError> {
    fs::create_dir_all(path).map_err(|source| io_error(path, source))?;
    let metadata = fs::symlink_metadata(path).map_err(|source| io_error(path, source))?;
    if !metadata.file_type().is_dir() {
        return Err(InferRuntimeCredentialError::InvalidFileType {
            path: path.to_owned(),
        });
    }
    set_private_directory_permissions(path)?;
    validate_private_directory(path)
}

fn validate_private_directory(path: &Path) -> Result<(), InferRuntimeCredentialError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            InferRuntimeCredentialError::Missing {
                path: path.join(CREDENTIAL_FILE),
            }
        } else {
            io_error(path, source)
        }
    })?;
    if !metadata.file_type().is_dir() {
        return Err(InferRuntimeCredentialError::InvalidFileType {
            path: path.to_owned(),
        });
    }
    validate_owner_only(path, &metadata)
}

fn write_credential_atomically(
    destination: &Path,
    credential: &InferRuntimeCredential,
) -> Result<(), InferRuntimeCredentialError> {
    let directory =
        destination
            .parent()
            .ok_or_else(|| InferRuntimeCredentialError::InvalidFileType {
                path: destination.to_owned(),
            })?;
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = directory.join(format!(
        ".{CREDENTIAL_FILE}.{}.{}.tmp",
        std::process::id(),
        sequence
    ));

    let result = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        set_private_file_creation_mode(&mut options);
        let mut file = options
            .open(&temporary)
            .map_err(|source| io_error(&temporary, source))?;
        file.write_all(credential.as_bytes())
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
            .map_err(|source| io_error(&temporary, source))?;
        set_private_file_permissions(&temporary)?;
        replace_file(&temporary, destination)?;
        let metadata =
            fs::symlink_metadata(destination).map_err(|source| io_error(destination, source))?;
        validate_owner_only(destination, &metadata)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(unix)]
fn set_private_file_creation_mode(options: &mut fs::OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt;
    options.mode(0o600);
}

#[cfg(not(unix))]
fn set_private_file_creation_mode(_options: &mut fs::OpenOptions) {}

#[cfg(unix)]
fn set_private_directory_permissions(path: &Path) -> Result<(), InferRuntimeCredentialError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|source| io_error(path, source))
}

#[cfg(not(unix))]
fn set_private_directory_permissions(_path: &Path) -> Result<(), InferRuntimeCredentialError> {
    Ok(())
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> Result<(), InferRuntimeCredentialError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|source| io_error(path, source))
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> Result<(), InferRuntimeCredentialError> {
    Ok(())
}

#[cfg(unix)]
fn validate_owner_only(
    path: &Path,
    metadata: &fs::Metadata,
) -> Result<(), InferRuntimeCredentialError> {
    use std::os::unix::fs::PermissionsExt;
    if metadata.permissions().mode().trailing_zeros() >= 6 {
        Ok(())
    } else {
        Err(InferRuntimeCredentialError::InsecurePermissions {
            path: path.to_owned(),
        })
    }
}

#[cfg(not(unix))]
fn validate_owner_only(
    _path: &Path,
    _metadata: &fs::Metadata,
) -> Result<(), InferRuntimeCredentialError> {
    Ok(())
}

#[cfg(unix)]
fn replace_file(source: &Path, destination: &Path) -> Result<(), InferRuntimeCredentialError> {
    fs::rename(source, destination).map_err(|error| io_error(destination, error))
}

#[cfg(not(unix))]
fn replace_file(source: &Path, destination: &Path) -> Result<(), InferRuntimeCredentialError> {
    if destination.exists() {
        return Err(InferRuntimeCredentialError::Io {
            path: destination.to_owned(),
            source: io::Error::new(
                io::ErrorKind::AlreadyExists,
                "atomic credential replacement is unavailable on this platform",
            ),
        });
    }
    fs::rename(source, destination).map_err(|error| io_error(destination, error))
}

#[cfg(target_os = "macos")]
fn default_echo_data_root() -> Result<PathBuf, InferRuntimeCredentialError> {
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Library/Application Support/Echo"))
        .ok_or(InferRuntimeCredentialError::DataDirectoryUnavailable)
}

#[cfg(target_os = "windows")]
fn default_echo_data_root() -> Result<PathBuf, InferRuntimeCredentialError> {
    env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|root| root.join("Echo"))
        .ok_or(InferRuntimeCredentialError::DataDirectoryUnavailable)
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn default_echo_data_root() -> Result<PathBuf, InferRuntimeCredentialError> {
    if let Some(root) = env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(root).join("Echo"));
    }
    env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join(".local/share/Echo"))
        .ok_or(InferRuntimeCredentialError::DataDirectoryUnavailable)
}

fn io_error(path: &Path, source: io::Error) -> InferRuntimeCredentialError {
    InferRuntimeCredentialError::Io {
        path: path.to_owned(),
        source,
    }
}

#[cfg(test)]
mod tests;
