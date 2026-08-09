//! Default source-metadata admission. Import records metadata immediately;
//! startup uses this owner to backfill older present assets in the background.

use echo_catalog::{Catalog, JobKind, enqueue_job, list_assets_missing_source_metadata};
use echo_domain::AssetId;
use rusqlite::Transaction;

use crate::error::CoreError;

pub(crate) fn enqueue_source_metadata(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    now_millis: i64,
) -> Result<(), echo_catalog::CatalogError> {
    enqueue_job(
        transaction,
        &format!("metadata-{asset_id}"),
        JobKind::ExtractMetadata,
        &serde_json::json!({ "asset_id": asset_id.to_string() }),
        now_millis,
    )
}

pub(crate) fn enqueue_missing_source_metadata(
    catalog: &Catalog,
    now_millis: i64,
) -> Result<u64, CoreError> {
    catalog
        .with_transaction(|transaction| -> Result<_, echo_catalog::CatalogError> {
            let assets = list_assets_missing_source_metadata(transaction)?;
            for asset_id in &assets {
                enqueue_source_metadata(transaction, *asset_id, now_millis)?;
            }
            Ok(u64::try_from(assets.len()).expect("asset count fits u64"))
        })
        .map_err(CoreError::from)
}

#[cfg(test)]
mod tests;
