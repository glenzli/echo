//! Immutable Runtime generation receipts. Unlike source declarations, these
//! cannot be cleared by correcting a user label and are not analysis evidence.
use crate::{CatalogError, source_disclosure::error};
use echo_domain::AssetId;
use rusqlite::{Transaction, params};

pub(crate) const SCHEMA_SQL: &str = r"
CREATE TABLE IF NOT EXISTS generated_audio_receipts (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 asset_id TEXT NOT NULL REFERENCES assets(id),
 runtime_job_id TEXT NOT NULL UNIQUE,
 receipt_json TEXT NOT NULL CHECK(json_valid(receipt_json))
);
CREATE INDEX IF NOT EXISTS generated_audio_asset ON generated_audio_receipts(asset_id,id DESC);
";

/// Preserve execution evidence in the same transaction that admits the source.
/// # Errors
/// Rejects receipts detached from the registered audio or conflicting Job ids.
pub fn record_generated_audio(
    tx: &Transaction<'_>,
    id: AssetId,
    receipt: &serde_json::Value,
) -> Result<(), CatalogError> {
    let (hash, duration): (String, i64) = tx.query_row(
        "SELECT content_hash,duration_millis FROM assets WHERE id=?1",
        [id.to_string()],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let job = &receipt["runtime"]["job"];
    let job_id = job["id"]
        .as_str()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| error("missing generation Job"))?;
    if receipt["schema_version"] != 1
        || receipt["output_hash"].as_str() != Some(hash.as_str())
        || receipt["duration_millis"].as_i64() != Some(duration)
        || duration <= 0
        || job["app_id"] != "echo"
        || job["intent"] != "speech.synthesize"
        || job["state"] != "succeeded"
        || job["physical_model"].as_str().is_none_or(str::is_empty)
        || job["model_build"].as_str().is_none_or(str::is_empty)
    {
        return Err(error("invalid generation receipt"));
    }
    let json = serde_json::to_string(receipt).map_err(|e| error(e.to_string()))?;
    tx.execute("INSERT INTO generated_audio_receipts(asset_id,runtime_job_id,receipt_json) VALUES(?1,?2,?3) ON CONFLICT(runtime_job_id) DO NOTHING", params![id.to_string(),job_id,json])?;
    let existing: (String, String) = tx.query_row(
        "SELECT asset_id,receipt_json FROM generated_audio_receipts WHERE runtime_job_id=?1",
        [job_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if existing != (id.to_string(), json) {
        return Err(error("generation Job already belongs to another result"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
