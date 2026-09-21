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
    let narration = receipt["schema_version"] == 1 && job["intent"] == "speech.synthesize";
    let request = &receipt["request"];
    // Missing choice is the original schema-2 Small-SFX receipt, never music.
    let choice = request
        .get("model_choice")
        .and_then(serde_json::Value::as_str);
    let music = choice == Some("stable_audio_3_small_music");
    let max_seconds = if choice == Some("stable_audio_open_small") {
        11
    } else {
        30
    };
    let sound_material = receipt["schema_version"] == 2
        && (request.get("model_choice").is_none()
            || matches!(
                choice,
                Some(
                    "stable_audio_3_small_sfx"
                        | "stable_audio_3_small_music"
                        | "stable_audio_open_small"
                )
            ))
        && receipt["generation_kind"] == if music { "music" } else { "sound_effect" }
        && job["intent"] == "audio.generate_sound"
        && request["model"] == "audio.generate_sound"
        && request["prompt"] == receipt["input_text"]
        && request["prompt"].as_str().is_some_and(|v| {
            !v.trim().is_empty() && v.chars().count() <= 500 && !v.chars().any(char::is_control)
        })
        && request["duration_seconds"]
            .as_i64()
            .is_some_and(|v| (1..=max_seconds).contains(&v) && duration.abs_diff(v * 1000) <= 100)
        && request["seed"]
            .as_u64()
            .is_some_and(|v| u32::try_from(v).is_ok())
        && if music {
            receipt["material_category"] == "music"
        } else {
            matches!(
                receipt["material_category"].as_str(),
                Some("ambience" | "effects")
            )
        };
    if !(narration || sound_material)
        || receipt["output_hash"].as_str() != Some(hash.as_str())
        || receipt["duration_millis"].as_i64() != Some(duration)
        || duration <= 0
        || job["app_id"] != "echo"
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
