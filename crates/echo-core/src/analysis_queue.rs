//! Default background-analysis admission.
//!
//! Import and startup call this owner to persist product-level inference
//! intent. Model execution, routing, retry, and resource lifecycle remain with
//! the worker adapter today and Infer Build later.

use echo_catalog::{Catalog, JobKind, enqueue_job, list_assets_missing_analysis};
use echo_domain::{AnalysisKind, AssetId};
use rusqlite::Transaction;

use crate::error::CoreError;

/// Persists the default transcription intent for one imported asset.
pub(crate) fn enqueue_transcription(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    now_millis: i64,
) -> Result<(), echo_catalog::CatalogError> {
    enqueue_job(
        transaction,
        &transcription_job_id(asset_id),
        JobKind::Transcribe,
        &serde_json::json!({ "asset_id": asset_id.to_string() }),
        now_millis,
    )
}

/// Ensures every present asset without transcript evidence has one durable
/// transcription intent. Existing pending, running, completed, or failed jobs
/// keep their terminal policy because ordinary enqueue is idempotent.
pub(crate) fn enqueue_missing_transcriptions(
    catalog: &Catalog,
    now_millis: i64,
) -> Result<u64, CoreError> {
    catalog
        .with_transaction(|transaction| -> Result<_, echo_catalog::CatalogError> {
            let assets = list_assets_missing_analysis(transaction, AnalysisKind::Transcript)?;
            for asset_id in &assets {
                enqueue_transcription(transaction, *asset_id, now_millis)?;
            }
            Ok(u64::try_from(assets.len()).expect("asset count fits u64"))
        })
        .map_err(CoreError::from)
}

fn transcription_job_id(asset_id: AssetId) -> String {
    format!("transcribe-{asset_id}")
}

#[cfg(test)]
mod tests;
