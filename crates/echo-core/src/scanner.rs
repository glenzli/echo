//! Folder scanning: walk a configured root, journal known files, queue
//! imports for new or changed files, and detect missing assets.
//!
//! The scan itself never hashes payloads; it fingerprints by size + mtime.
//! Hashing happens in the import job, which also relinks a `missing` asset
//! when the hash matches.

use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use echo_catalog::{
    JobKind, ScanRootJobPayload, add_scan_root, enqueue_job, journal_fingerprint, list_assets,
    list_scan_roots, mark_asset_missing,
};

use crate::error::{CoreError, CoreErrorKind};

/// Audio extensions Echo scans by default.
const AUDIO_EXTENSIONS: &[&str] = &[
    "mp3", "m4a", "aac", "flac", "wav", "aiff", "aif", "ogg", "opus", "wma", "m4b", "mov", "m4v",
    "mp4",
];

/// The directory scanner.
#[derive(Debug, Clone)]
pub struct FolderScanner {
    pub catalog_path: PathBuf,
    pub now_millis: i64,
}

/// Outcome of one scan pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanOutcome {
    pub files_walked: u64,
    pub import_jobs_queued: u64,
    pub assets_marked_missing: u64,
}

/// Runs one scan pass over `root`: journal-aware import queueing plus missing
/// detection for registered assets under the root.
///
/// # Errors
///
/// Returns [`CoreError`] when the root cannot be walked.
pub fn scan_root(
    catalog: &echo_catalog::Catalog,
    root: &Path,
    now_millis: i64,
) -> Result<ScanOutcome, CoreError> {
    let mut outcome = ScanOutcome::default();

    // Phase 1: walk the tree, fingerprint files, queue imports.
    let mut queue_import = |path: &Path| -> Result<(), CoreError> {
        let metadata = fs::metadata(path).map_err(|error| {
            CoreError::new(
                CoreErrorKind::SourceUnavailable,
                format!("cannot stat {}: {error}", path.display()),
            )
        })?;
        let size = metadata.len();
        let mtime = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |duration| {
                i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
            });
        let path_text = path.to_string_lossy().into_owned();
        let unchanged =
            catalog.with_transaction(|transaction| journal_fingerprint(transaction, &path_text))?;
        if unchanged == Some((size, mtime)) {
            return Ok(());
        }
        outcome.files_walked += 1;
        catalog.with_transaction(|transaction| {
            enqueue_job(
                transaction,
                &format!("import-{}", path_text.replace(['/', '\\'], "_")),
                JobKind::ImportFile,
                &echo_catalog::FileJobPayload {
                    path: path.to_owned(),
                }
                .encode(),
                now_millis,
            )
        })?;
        outcome.import_jobs_queued += 1;
        Ok(())
    };

    walk_audio_files(root, &mut |path| {
        queue_import(path).map_err(|error| error.to_string())
    })
    .map_err(|message| CoreError::new(CoreErrorKind::SourceUnavailable, message))?;

    // Phase 2: detect assets registered under the root that vanished.
    let registered = catalog.with_transaction(list_assets)?;
    for asset in registered {
        if !asset.original.path.starts_with(root) {
            continue;
        }
        let path = &asset.original.path;
        if !path.exists() {
            let already_missing = catalog.with_transaction(|transaction| {
                asset_is_missing(transaction, &asset.id.to_string())
            })?;
            if !already_missing {
                catalog.with_transaction(|transaction| {
                    mark_asset_missing(transaction, &asset.id.to_string())
                })?;
                outcome.assets_marked_missing += 1;
            }
        }
    }
    Ok(outcome)
}

/// Marks an asset missing (for scan-time detection) and returns whether it
/// already was.
fn asset_is_missing(
    transaction: &rusqlite::Transaction<'_>,
    asset_id: &str,
) -> Result<bool, CoreError> {
    let status: String = transaction
        .query_row(
            "SELECT path_status FROM assets WHERE id = ?1",
            [asset_id],
            |row| row.get(0),
        )
        .map_err(|error| CoreError::new(CoreErrorKind::Catalog, error.to_string()))?;
    Ok(status == "missing")
}

/// Recursively walks `root`, calling `visit` for every audio file.
fn walk_audio_files(
    root: &Path,
    visit: &mut dyn FnMut(&Path) -> Result<(), String>,
) -> Result<(), String> {
    let entries =
        fs::read_dir(root).map_err(|error| format!("cannot read {}: {error}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("cannot stat {}: {error}", path.display()))?;
        if file_type.is_dir() {
            walk_audio_files(&path, visit)?;
            continue;
        }
        if file_type.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    AUDIO_EXTENSIONS
                        .iter()
                        .any(|candidate| candidate.eq_ignore_ascii_case(extension))
                })
        {
            visit(&path)?;
        }
    }
    Ok(())
}

/// Enqueues a scan job for every enabled root (idempotent within this pass).
///
/// # Errors
///
/// Returns [`CoreError`] when queueing fails.
pub fn queue_scans_for_enabled_roots(
    catalog: &echo_catalog::Catalog,
    now_millis: i64,
) -> Result<u64, CoreError> {
    let roots = catalog.with_transaction(list_scan_roots)?;
    let mut queued = 0;
    for root in roots.iter().filter(|root| root.enabled) {
        catalog.with_transaction(|transaction| {
            enqueue_job(
                transaction,
                &format!(
                    "scan-{}",
                    root.root.to_string_lossy().replace(['/', '\\'], "_")
                ),
                JobKind::ScanRoot,
                &ScanRootJobPayload {
                    root: root.root.clone(),
                }
                .encode(),
                now_millis,
            )
        })?;
        queued += 1;
    }
    Ok(queued)
}

/// Registers a scan root and queues its scan.
///
/// # Errors
///
/// Returns [`CoreError`] when registration or queueing fails.
pub fn add_root_and_scan(
    catalog: &echo_catalog::Catalog,
    root: &Path,
    now_millis: i64,
) -> Result<(), CoreError> {
    catalog.with_transaction(|transaction| add_scan_root(transaction, root, now_millis))?;
    catalog.with_transaction(|transaction| {
        enqueue_job(
            transaction,
            &format!("scan-{}", root.to_string_lossy().replace(['/', '\\'], "_")),
            JobKind::ScanRoot,
            &ScanRootJobPayload {
                root: root.to_owned(),
            }
            .encode(),
            now_millis,
        )
    })?;
    Ok(())
}
