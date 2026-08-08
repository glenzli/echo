//! Scan-root management: the configured directories Echo watches.

use rusqlite::Transaction;

use crate::error::CatalogError;

/// One configured scan root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanRoot {
    pub id: i64,
    pub root: std::path::PathBuf,
    pub enabled: bool,
    pub added_at_millis: i64,
}

/// Registers a scan root (idempotent by path).
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn add_scan_root(
    transaction: &Transaction<'_>,
    root: &std::path::Path,
    added_at_millis: i64,
) -> Result<ScanRoot, CatalogError> {
    transaction.execute(
        "INSERT INTO scan_roots (root, enabled, added_at_millis) VALUES (?1, 1, ?2) \
         ON CONFLICT(root) DO NOTHING",
        rusqlite::params![root.to_string_lossy(), added_at_millis],
    )?;
    let id: i64 = transaction.query_row(
        "SELECT id FROM scan_roots WHERE root = ?1",
        [root.to_string_lossy().as_ref()],
        |row| row.get(0),
    )?;
    Ok(ScanRoot {
        id,
        root: root.to_owned(),
        enabled: true,
        added_at_millis,
    })
}

/// Lists every scan root, most recently added first.
///
/// # Errors
///
/// Returns a catalog failure when the read cannot be applied.
pub fn list_scan_roots(transaction: &Transaction<'_>) -> Result<Vec<ScanRoot>, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT id, root, enabled, added_at_millis FROM scan_roots \
         ORDER BY added_at_millis DESC, id DESC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(ScanRoot {
            id: row.get(0)?,
            root: row.get::<_, String>(1)?.into(),
            enabled: row.get::<_, i64>(2)? != 0,
            added_at_millis: row.get(3)?,
        })
    })?;
    let mut roots = Vec::new();
    for row in rows {
        roots.push(row?);
    }
    Ok(roots)
}

/// Removes a scan root by id.
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn remove_scan_root(transaction: &Transaction<'_>, id: i64) -> Result<(), CatalogError> {
    transaction.execute("DELETE FROM scan_roots WHERE id = ?1", [id])?;
    Ok(())
}
