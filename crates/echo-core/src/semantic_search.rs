//! Durable semantic-document embedding and on-demand natural-language query.
//!
//! The background queue owns document indexing; query embeddings are ephemeral
//! and never enter Catalog, metadata, or logs. Both paths use the same exact
//! Runtime embedding space and Echo's local-only Consumer policy.

use echo_catalog::{
    Catalog, ClaimedJob, JobKind, SemanticSearchHit, SemanticSource, UpsertSemanticDocument,
    enqueue_job, index_semantic_source_text, list_semantic_sources_needing_embedding,
    remove_semantic_documents_outside_contract, search_semantic_documents, semantic_source,
    upsert_semantic_document,
};
use echo_domain::AssetId;

use crate::{CoreError, CoreErrorKind, InferRuntimeClient, TextEmbeddingIntent, WorkerConfig};

const SEMANTIC_DOCUMENT_REVISION: u32 = 2;
const SEMANTIC_QUERY_REVISION: u32 = 1;

pub(crate) fn enqueue_missing_documents(
    catalog: &Catalog,
    now_millis: i64,
) -> Result<u64, CoreError> {
    catalog
        .with_transaction(|transaction| -> Result<_, echo_catalog::CatalogError> {
            remove_semantic_documents_outside_contract(
                transaction,
                crate::EXPECTED_CONTRACT_VERSION,
            )?;
            let sources = list_semantic_sources_needing_embedding(transaction)?;
            for source in &sources {
                enqueue_document(transaction, source, now_millis)?;
            }
            Ok(u64::try_from(sources.len()).expect("source count fits u64"))
        })
        .map_err(CoreError::from)
}

pub(crate) fn enqueue_current_document(
    catalog: &Catalog,
    asset_id: AssetId,
    now_millis: i64,
) -> Result<(), CoreError> {
    catalog.with_transaction(|transaction| {
        if let Some(source) = semantic_source(transaction, asset_id)? {
            enqueue_document(transaction, &source, now_millis)?;
        }
        Ok::<_, echo_catalog::CatalogError>(())
    })?;
    Ok(())
}

fn enqueue_document(
    transaction: &rusqlite::Transaction<'_>,
    source: &SemanticSource,
    now_millis: i64,
) -> Result<(), echo_catalog::CatalogError> {
    index_semantic_source_text(transaction, source)?;
    enqueue_job(
        transaction,
        &document_job_id(source),
        JobKind::EmbedText,
        &serde_json::json!({
            "asset_id": source.asset_id.to_string(),
            "source_revision": source.revision,
        }),
        now_millis,
    )
}

pub(crate) fn dispatch_document(
    catalog: &Catalog,
    config: &WorkerConfig,
    job: &ClaimedJob,
) -> Result<(), CoreError> {
    let (asset_id, expected_revision) = job_identity(&job.payload)?;
    let Some(source) = catalog.with_transaction(|transaction| {
        semantic_source(transaction, asset_id).map_err(CoreError::from)
    })?
    else {
        return Ok(());
    };
    if source.revision != expected_revision {
        enqueue_current_document(catalog, asset_id, crate::util::now_millis())?;
        return Ok(());
    }
    super::worker::record_inference_submitting(
        catalog,
        job,
        asset_id,
        crate::TEXT_EMBEDDING_INTENT,
    )?;
    let client = InferRuntimeClient::new(config.infer_runtime.clone());
    let payload = match client.embed_text(&source.text, &TextEmbeddingIntent::new(&source.revision))
    {
        Ok(payload) => payload,
        Err(error) => {
            super::worker::record_inference_failure(
                catalog,
                job,
                asset_id,
                crate::TEXT_EMBEDDING_INTENT,
                &error,
            )?;
            return Err(error.into());
        }
    };
    let runtime = serde_json::json!({
        "provider": &payload.provider,
        "runtime": &payload.runtime,
    });
    catalog.with_transaction(|transaction| {
        let current = semantic_source(transaction, asset_id)?.ok_or_else(|| {
            echo_catalog::CatalogError::new(
                echo_catalog::CatalogErrorKind::Other,
                "semantic source disappeared before publication",
            )
        })?;
        if current.revision != source.revision {
            enqueue_document(transaction, &current, crate::util::now_millis())?;
            return Ok(());
        }
        upsert_semantic_document(
            transaction,
            &UpsertSemanticDocument {
                source: &source,
                embedding_space: &payload.space,
                values: &payload.values,
                runtime: &runtime,
                updated_at_millis: crate::util::now_millis(),
            },
        )
    })?;
    super::worker::record_inference_success(catalog, job, asset_id, &payload.runtime)
}

/// Runs an ephemeral query embedding and searches only its exact vector space.
/// Query text is never persisted.
///
/// # Errors
///
/// Returns a credential, Runtime, contract, or Catalog failure without
/// persisting the query text.
pub fn search(
    catalog: &Catalog,
    config: &crate::InferRuntimeConfig,
    query: &str,
    limit: u64,
) -> Result<Vec<SemanticSearchHit>, CoreError> {
    let normalized = query.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return Ok(Vec::new());
    }
    let digest = blake3::hash(normalized.as_bytes());
    let revision = format!(
        "echo:semantic-query:v{SEMANTIC_QUERY_REVISION}:{}",
        &digest.to_hex()[..24]
    );
    let client = InferRuntimeClient::new(config.clone());
    let payload = client
        .embed_text(&normalized, &TextEmbeddingIntent::new(revision))
        .map_err(CoreError::from)?;
    catalog.with_transaction(|transaction| {
        search_semantic_documents(transaction, &payload.values, &payload.space, limit)
            .map_err(CoreError::from)
    })
}

fn document_job_id(source: &SemanticSource) -> String {
    let digest = blake3::hash(source.revision.as_bytes());
    format!(
        "embed-text-v{SEMANTIC_DOCUMENT_REVISION}-{}-{}",
        source.asset_id,
        &digest.to_hex()[..16]
    )
}

fn job_identity(payload: &serde_json::Value) -> Result<(AssetId, String), CoreError> {
    let asset_id = payload
        .get("asset_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| CoreError::new(CoreErrorKind::Other, "semantic job lacks asset_id"))?
        .parse::<AssetId>()
        .map_err(|error| CoreError::new(CoreErrorKind::Other, format!("bad asset id: {error}")))?;
    let source_revision = payload
        .get("source_revision")
        .and_then(serde_json::Value::as_str)
        .filter(|revision| !revision.is_empty())
        .ok_or_else(|| CoreError::new(CoreErrorKind::Other, "semantic job lacks revision"))?;
    Ok((asset_id, source_revision.to_owned()))
}

#[cfg(test)]
mod tests;
