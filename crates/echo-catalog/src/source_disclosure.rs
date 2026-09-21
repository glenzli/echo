//! Append-only user source disclosures and conservative reference propagation.
//! Empty declarations mean unknown, never certified live recording.
use crate::{CatalogError, CatalogErrorKind};
use echo_domain::{AssetId, SoundAssembly, SourceDisclosureSpan, validate_source_disclosures};
use rusqlite::{OptionalExtension, Transaction, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const SCHEMA_SQL: &str = r"
CREATE TABLE IF NOT EXISTS asset_source_disclosures (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 asset_id TEXT NOT NULL REFERENCES assets(id),
 original_content_hash TEXT NOT NULL,
 spans_json TEXT NOT NULL CHECK(json_valid(spans_json)),
 created_at_millis INTEGER NOT NULL CHECK(created_at_millis >= 0)
);
CREATE INDEX IF NOT EXISTS source_disclosure_latest ON asset_source_disclosures(asset_id,id DESC);
CREATE TABLE IF NOT EXISTS render_export_disclosures (
 render_export_id INTEGER PRIMARY KEY REFERENCES render_exports(id),
 disclosure_json TEXT NOT NULL CHECK(json_valid(disclosure_json))
);
";

/// User declaration revision, deliberately distinct from inference evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDisclosureRevision {
    pub revision_id: i64,
    pub asset_id: String,
    pub original_content_hash: String,
    pub spans: Vec<SourceDisclosureSpan>,
    pub created_at_millis: i64,
}
/// Sources referenced by one recording or a fixed assembly revision.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceDisclosureSummary {
    pub sources: Vec<SourceDisclosureRevision>,
}
impl SourceDisclosureSummary {
    #[must_use]
    pub fn has_generated_source(&self) -> bool {
        self.sources
            .iter()
            .any(|s| s.spans.iter().any(|p| p.kind.is_generated()))
    }
    #[must_use]
    pub fn has_ai_processed_source(&self) -> bool {
        self.sources
            .iter()
            .any(|s| s.spans.iter().any(|p| !p.kind.is_generated()))
    }
}

/// Latest declarations in one indexed Catalog read.
/// # Errors
/// Returns a failure for unreadable or malformed history.
pub fn source_disclosures(
    tx: &Transaction<'_>,
) -> Result<BTreeMap<String, SourceDisclosureRevision>, CatalogError> {
    read_disclosures(tx, None)
}

pub(crate) fn asset_source_disclosure(
    tx: &Transaction<'_>,
    id: AssetId,
) -> Result<SourceDisclosureSummary, CatalogError> {
    Ok(SourceDisclosureSummary {
        sources: read_disclosures(tx, Some(id))?.into_values().collect(),
    })
}

fn read_disclosures(
    tx: &Transaction<'_>,
    id: Option<AssetId>,
) -> Result<BTreeMap<String, SourceDisclosureRevision>, CatalogError> {
    let selection = if id.is_some() { " WHERE a.id=?1" } else { "" };
    let sql = format!(
        "SELECT d.id,d.asset_id,d.original_content_hash,d.spans_json,d.created_at_millis FROM assets a JOIN asset_source_disclosures d ON d.id=(SELECT latest.id FROM asset_source_disclosures latest WHERE latest.asset_id=a.id ORDER BY latest.id DESC LIMIT 1){selection}"
    );
    let mut query = tx.prepare(&sql)?;
    let ids: Vec<String> = id.into_iter().map(|value| value.to_string()).collect();
    let rows = query.query_map(rusqlite::params_from_iter(ids), |r| {
        Ok((
            r.get::<_, i64>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, i64>(4)?,
        ))
    })?;
    let mut result = BTreeMap::new();
    for row in rows {
        let (revision_id, asset_id, original_content_hash, json, created_at_millis) = row?;
        let spans = serde_json::from_str(&json).map_err(|e| error(e.to_string()))?;
        result.insert(
            asset_id.clone(),
            SourceDisclosureRevision {
                revision_id,
                asset_id,
                original_content_hash,
                spans,
                created_at_millis,
            },
        );
    }
    Ok(result)
}

/// Append one whole replacement, retaining corrections and clear operations.
/// # Errors
/// Rejects stale revisions, invalid source coordinates and unknown assets atomically.
pub fn record_source_disclosure(
    tx: &Transaction<'_>,
    asset_id: AssetId,
    expected_revision: i64,
    spans: &[SourceDisclosureSpan],
    now: i64,
) -> Result<i64, CatalogError> {
    let id = asset_id.to_string();
    let (hash, duration): (String, Option<i64>) = tx.query_row(
        "SELECT content_hash,duration_millis FROM assets WHERE id=?1",
        [&id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let duration = u64::try_from(duration.unwrap_or(0)).map_err(|e| error(e.to_string()))?;
    validate_source_disclosures(spans, duration).map_err(error)?;
    if now < 0 {
        return Err(error("source disclosure time is invalid"));
    }
    let previous:Option<(i64,String)>=tx.query_row("SELECT id,spans_json FROM asset_source_disclosures WHERE asset_id=?1 ORDER BY id DESC LIMIT 1",[&id],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    if previous.as_ref().map_or(0, |p| p.0) != expected_revision {
        return Err(error("source disclosure changed; reopen before saving"));
    }
    let json = serde_json::to_string(spans).map_err(|e| error(e.to_string()))?;
    if previous.as_ref().is_some_and(|p| p.1 == json) || (previous.is_none() && spans.is_empty()) {
        return Ok(expected_revision);
    }
    tx.execute("INSERT INTO asset_source_disclosures(asset_id,original_content_hash,spans_json,created_at_millis) VALUES(?1,?2,?3,?4)",params![id,hash,json,now])?;
    Ok(tx.last_insert_rowid())
}

/// Conservative source-level label: do not claim effect tails or cropped
/// processing history are clean simply because a marked source interval moved.
#[must_use]
pub fn assembly_source_disclosure(
    assembly: &SoundAssembly,
    all: &BTreeMap<String, SourceDisclosureRevision>,
) -> SourceDisclosureSummary {
    let solo = assembly
        .tracks()
        .iter()
        .any(echo_domain::AssemblyTrack::solo);
    let mut ids = BTreeSet::new();
    for track in assembly.tracks() {
        if track.muted() || (solo && !track.solo()) {
            continue;
        }
        for clip in track.clips() {
            if !clip.muted() {
                ids.insert(clip.asset_id().to_string());
            }
        }
    }
    SourceDisclosureSummary {
        sources: ids.iter().filter_map(|id| all.get(id).cloned()).collect(),
    }
}

pub(crate) fn error(message: impl Into<String>) -> CatalogError {
    CatalogError::new(CatalogErrorKind::Other, message)
}

#[cfg(test)]
mod tests;
