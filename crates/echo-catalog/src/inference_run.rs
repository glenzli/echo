//! Durable linkage between one Echo background job and its Infer Runtime run.
//!
//! Echo's job queue remains the product workflow owner. This table records the
//! external contract identity, stable error semantics and the final App-scoped
//! Job snapshot needed to audit which model attempt produced analysis evidence.

use echo_domain::AssetId;
use rusqlite::{OptionalExtension, Transaction};

use crate::{CatalogError, CatalogErrorKind};

type StoredInferenceRun = (
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    Option<u16>,
    Option<String>,
    Option<String>,
    i64,
);

/// Echo's projection of the external Runtime lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceRunState {
    /// The local job is submitting its request; no Runtime id is available yet.
    Submitting,
    /// Runtime completed and Echo accepted the result and provenance.
    Succeeded,
    /// Runtime or transport failed.
    Failed,
    /// Runtime accepted cancellation.
    Cancelled,
    /// Runtime exceeded the request deadline.
    Expired,
}

/// One persisted Runtime linkage.
#[derive(Debug, Clone, PartialEq)]
pub struct InferenceRun {
    pub local_job_id: String,
    pub asset_id: AssetId,
    pub intent: String,
    pub runtime_job_id: Option<String>,
    pub contract_version: String,
    pub state: InferenceRunState,
    pub http_status: Option<u16>,
    pub error_code: Option<String>,
    pub snapshot: Option<serde_json::Value>,
    pub updated_at_millis: i64,
}

/// Atomic replacement of the latest projection for one local job.
#[derive(Debug, Clone)]
pub struct UpsertInferenceRun<'a> {
    pub local_job_id: &'a str,
    pub asset_id: AssetId,
    pub intent: &'a str,
    pub runtime_job_id: Option<&'a str>,
    pub contract_version: &'a str,
    pub state: InferenceRunState,
    pub http_status: Option<u16>,
    pub error_code: Option<&'a str>,
    pub snapshot: Option<&'a serde_json::Value>,
    pub updated_at_millis: i64,
}

/// Inserts or replaces the Runtime projection for one Echo job.
///
/// # Errors
///
/// Returns a catalog failure when identities are empty or the write fails.
pub fn upsert_inference_run(
    transaction: &Transaction<'_>,
    update: &UpsertInferenceRun<'_>,
) -> Result<(), CatalogError> {
    if update.local_job_id.is_empty()
        || update.intent.is_empty()
        || update.contract_version.is_empty()
    {
        return Err(CatalogError::new(
            CatalogErrorKind::Other,
            "inference run requires local job, intent, and contract identities",
        ));
    }
    transaction.execute(
        "INSERT INTO inference_runs (
             local_job_id, asset_id, intent, runtime_job_id, contract_version, state,
             http_status, error_code, snapshot_json, updated_at_millis
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(local_job_id) DO UPDATE SET
             asset_id = excluded.asset_id,
             intent = excluded.intent,
             runtime_job_id = excluded.runtime_job_id,
             contract_version = excluded.contract_version,
             state = excluded.state,
             http_status = excluded.http_status,
             error_code = excluded.error_code,
             snapshot_json = excluded.snapshot_json,
             updated_at_millis = excluded.updated_at_millis",
        rusqlite::params![
            update.local_job_id,
            update.asset_id.to_string(),
            update.intent,
            update.runtime_job_id,
            update.contract_version,
            state_text(update.state),
            update.http_status,
            update.error_code,
            update.snapshot.map(serde_json::Value::to_string),
            update.updated_at_millis,
        ],
    )?;
    Ok(())
}

/// Reads the Runtime projection for one Echo job.
///
/// # Errors
///
/// Returns a catalog failure when stored identity, state, or JSON is invalid.
pub fn inference_run(
    transaction: &Transaction<'_>,
    local_job_id: &str,
) -> Result<Option<InferenceRun>, CatalogError> {
    let row: Option<StoredInferenceRun> = transaction
        .query_row(
            "SELECT local_job_id, asset_id, intent, runtime_job_id, contract_version, state,
                    http_status, error_code, snapshot_json, updated_at_millis
             FROM inference_runs WHERE local_job_id = ?1",
            [local_job_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                ))
            },
        )
        .optional()?;
    let Some((
        local_job_id,
        asset_id,
        intent,
        runtime_job_id,
        contract_version,
        state,
        http_status,
        error_code,
        snapshot_json,
        updated_at_millis,
    )) = row
    else {
        return Ok(None);
    };
    let asset_id = asset_id.parse().map_err(|error| {
        CatalogError::new(
            CatalogErrorKind::Other,
            format!("invalid inference asset id {asset_id}: {error}"),
        )
    })?;
    let state = parse_state(&state)?;
    let snapshot = snapshot_json
        .map(|json| {
            serde_json::from_str(&json).map_err(|error| {
                CatalogError::new(
                    CatalogErrorKind::Other,
                    format!("invalid inference snapshot JSON: {error}"),
                )
            })
        })
        .transpose()?;
    Ok(Some(InferenceRun {
        local_job_id,
        asset_id,
        intent,
        runtime_job_id,
        contract_version,
        state,
        http_status,
        error_code,
        snapshot,
        updated_at_millis,
    }))
}

const fn state_text(state: InferenceRunState) -> &'static str {
    match state {
        InferenceRunState::Submitting => "submitting",
        InferenceRunState::Succeeded => "succeeded",
        InferenceRunState::Failed => "failed",
        InferenceRunState::Cancelled => "cancelled",
        InferenceRunState::Expired => "expired",
    }
}

fn parse_state(text: &str) -> Result<InferenceRunState, CatalogError> {
    match text {
        "submitting" => Ok(InferenceRunState::Submitting),
        "succeeded" => Ok(InferenceRunState::Succeeded),
        "failed" => Ok(InferenceRunState::Failed),
        "cancelled" => Ok(InferenceRunState::Cancelled),
        "expired" => Ok(InferenceRunState::Expired),
        _ => Err(CatalogError::new(
            CatalogErrorKind::Other,
            format!("unknown inference run state {text}"),
        )),
    }
}

#[cfg(test)]
mod tests;
