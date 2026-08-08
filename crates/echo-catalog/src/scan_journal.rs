//! Incremental scan journal: file fingerprints (size + mtime) that let scans
//! skip unchanged files without re-hashing.

use rusqlite::{OptionalExtension, Transaction};

use crate::error::CatalogError;

/// Looks up the journaled fingerprint for a path.
///
/// # Panics
///
/// Panics when the stored size exceeds `u64`.
///
/// # Errors
///
/// Returns a catalog failure when the read cannot be applied.
pub fn journal_fingerprint(
    transaction: &Transaction<'_>,
    path: &str,
) -> Result<Option<(u64, i64)>, CatalogError> {
    let fingerprint: Option<(i64, i64)> = transaction
        .query_row(
            "SELECT size_bytes, mtime_millis FROM scan_journal WHERE path = ?1",
            [path],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    Ok(fingerprint.map(|(size, mtime)| (u64::try_from(size).expect("size fits u64"), mtime)))
}

/// Records a file fingerprint after a successful import.
///
/// # Panics
///
/// Panics when the size exceeds `i64`.
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn upsert_journal(
    transaction: &Transaction<'_>,
    path: &str,
    size_bytes: u64,
    mtime_millis: i64,
    content_hash: &str,
) -> Result<(), CatalogError> {
    transaction.execute(
        "INSERT INTO scan_journal (path, size_bytes, mtime_millis, content_hash) \
         VALUES (?1, ?2, ?3, ?4) \
         ON CONFLICT(path) DO UPDATE SET size_bytes = ?2, mtime_millis = ?3, content_hash = ?4",
        rusqlite::params![
            path,
            i64::try_from(size_bytes).expect("size fits i64"),
            mtime_millis,
            content_hash
        ],
    )?;
    Ok(())
}
