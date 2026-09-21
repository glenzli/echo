//! Append-only personal context for an original recording or a whole assembly.
//! Stored in the Catalog (including standalone projects), never in source bytes.
use crate::{CatalogError, CatalogErrorKind};
use echo_domain::MemoryInfo;
use rusqlite::{OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};

pub(crate) const SCHEMA_SQL: &str = r"
CREATE TABLE IF NOT EXISTS memory_info_revisions (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 asset_id TEXT REFERENCES assets(id),
 assembly_id TEXT REFERENCES sound_assemblies(id),
 info_json TEXT NOT NULL CHECK(json_valid(info_json)),
 created_at_millis INTEGER NOT NULL CHECK(created_at_millis >= 0),
 CHECK((asset_id IS NULL) != (assembly_id IS NULL))
);
CREATE INDEX IF NOT EXISTS memory_info_asset ON memory_info_revisions(asset_id,id DESC);
CREATE INDEX IF NOT EXISTS memory_info_assembly ON memory_info_revisions(assembly_id,id DESC);
";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfoRevision {
    pub revision: i64,
    pub info: MemoryInfo,
}

fn column(assembly: bool) -> &'static str {
    if assembly { "assembly_id" } else { "asset_id" }
}
fn error(message: impl Into<String>) -> CatalogError {
    CatalogError::new(CatalogErrorKind::Constraint, message)
}

fn duration(tx: &Transaction<'_>, id: &str, assembly: bool) -> Result<Option<u64>, CatalogError> {
    if assembly {
        tx.query_row("SELECT id FROM sound_assemblies WHERE id=?1", [id], |_| {
            Ok(())
        })?;
        Ok(None)
    } else {
        let value: Option<i64> = tx.query_row(
            "SELECT duration_millis FROM assets WHERE id=?1",
            [id],
            |r| r.get(0),
        )?;
        Ok(Some(
            u64::try_from(value.unwrap_or(0)).map_err(|_| error("invalid source duration"))?,
        ))
    }
}

/// Read personal context without falling back to AI or source tags.
/// # Errors
/// Rejects missing identities and invalid stored data.
pub fn memory_info(
    tx: &Transaction<'_>,
    id: &str,
    assembly: bool,
) -> Result<MemoryInfoRevision, CatalogError> {
    duration(tx, id, assembly)?;
    let query = format!(
        "SELECT id,info_json FROM memory_info_revisions WHERE {}=?1 ORDER BY id DESC LIMIT 1",
        column(assembly)
    );
    let row: Option<(i64, String)> = tx
        .query_row(&query, [id], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?;
    row.map_or_else(
        || Ok(MemoryInfoRevision::default()),
        |(revision, json)| {
            Ok(MemoryInfoRevision {
                revision,
                info: serde_json::from_str(&json).map_err(|e| error(e.to_string()))?,
            })
        },
    )
}

/// Save a replacement revision; stale editors cannot overwrite newer notes.
/// # Errors
/// Rejects stale revisions, unknown sounds and invalid text or source-time ranges.
pub fn record_memory_info(
    tx: &Transaction<'_>,
    id: &str,
    assembly: bool,
    expected_revision: i64,
    info: MemoryInfo,
    now: i64,
) -> Result<i64, CatalogError> {
    let info = info
        .normalized(duration(tx, id, assembly)?)
        .map_err(error)?;
    let current = memory_info(tx, id, assembly)?;
    if current.revision != expected_revision || now < 0 {
        return Err(error("memory information changed; reopen before saving"));
    }
    if info == current.info {
        return Ok(current.revision);
    }
    let query = format!(
        "INSERT INTO memory_info_revisions({},info_json,created_at_millis) VALUES(?1,?2,?3)",
        column(assembly)
    );
    tx.execute(
        &query,
        params![
            id,
            serde_json::to_string(&info).map_err(|e| error(e.to_string()))?,
            now
        ],
    )?;
    Ok(tx.last_insert_rowid())
}

#[cfg(test)]
mod tests;
