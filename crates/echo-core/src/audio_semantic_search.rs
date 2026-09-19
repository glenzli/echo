//! CLAP audio evidence: short, ASR-empty originals and ephemeral text queries.

use echo_catalog::{
    Catalog, ClaimedJob, JobState, enqueue_job, job_by_id, list_audio_sources_needing_embedding,
    retry_job, search_audio_semantic_segments, upsert_audio_semantic_segment,
};
use echo_domain::AssetId;

use crate::{
    AudioTextQueryEmbeddingIntent, CoreError, CoreErrorKind, InferRuntimeClient, WorkerConfig,
};

const INDEX_REVISION: u32 = 1;
const LEGACY_SCHEMA_ERROR: &str =
    "InferenceRejected: Infer Runtime consumer_core_unsupported (ContractMismatch)";

pub(crate) fn enqueue_missing_documents(
    catalog: &Catalog,
    now_millis: i64,
) -> Result<u64, CoreError> {
    catalog
        .with_transaction(|transaction| {
            let sources = list_audio_sources_needing_embedding(transaction)?;
            for source in &sources {
                retry_legacy_schema_failure(transaction, source.asset_id, now_millis)?;
                enqueue_job(
                    transaction,
                    &job_id(source.asset_id),
                    echo_catalog::JobKind::EmbedAudio,
                    &serde_json::json!({ "asset_id": source.asset_id.to_string() }),
                    now_millis,
                )?;
            }
            Ok::<_, echo_catalog::CatalogError>(
                u64::try_from(sources.len()).expect("source count fits u64"),
            )
        })
        .map_err(CoreError::from)
}

// The old SDK rejected CLAP before submission. Preserve the existing job and
// attempt history, and persist the guard atomically so another contract failure
// cannot become a retry loop on every startup or scan.
fn retry_legacy_schema_failure(
    transaction: &rusqlite::Transaction<'_>,
    asset_id: AssetId,
    now_millis: i64,
) -> Result<(), echo_catalog::CatalogError> {
    let Some(mut job) = job_by_id(transaction, &job_id(asset_id))? else {
        return Ok(());
    };
    if job.state != JobState::Failed
        || job.error.as_deref() != Some(LEGACY_SCHEMA_ERROR)
        || job.payload.get("clap_schema_retry").is_some()
    {
        return Ok(());
    }
    job.payload["clap_schema_retry"] = serde_json::json!(true);
    transaction.execute(
        "UPDATE jobs SET payload = ?2 WHERE id = ?1",
        rusqlite::params![job.id, job.payload.to_string()],
    )?;
    retry_job(transaction, &job.id, now_millis)?;
    Ok(())
}

pub(crate) fn dispatch_document(
    catalog: &Catalog,
    config: &WorkerConfig,
    job: &ClaimedJob,
) -> Result<(), CoreError> {
    let asset_id = job
        .payload
        .get("asset_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| CoreError::new(CoreErrorKind::Other, "audio embedding job lacks asset_id"))?
        .parse::<AssetId>()
        .map_err(|e| CoreError::new(CoreErrorKind::Other, e.to_string()))?;
    let source = catalog
        .with_transaction(|transaction| {
            Ok::<_, echo_catalog::CatalogError>(
                list_audio_sources_needing_embedding(transaction)?
                    .into_iter()
                    .find(|s| s.asset_id == asset_id),
            )
        })?
        .ok_or_else(|| {
            CoreError::new(
                CoreErrorKind::Other,
                "audio embedding source is no longer eligible",
            )
        })?;
    super::worker::record_inference_submitting(
        catalog,
        job,
        asset_id,
        crate::AUDIO_EMBEDDING_INTENT,
    )?;
    let client = InferRuntimeClient::new(config.infer_runtime.clone());
    let payload =
        match client.embed_audio(std::path::Path::new(&source.path), &source.source_revision) {
            Ok(value) => value,
            Err(error) => {
                super::worker::record_inference_failure(
                    catalog,
                    job,
                    asset_id,
                    crate::AUDIO_EMBEDDING_INTENT,
                    &error,
                )?;
                return Err(error.into());
            }
        };
    let runtime = serde_json::json!({ "provider": payload.provider, "runtime": payload.runtime });
    catalog.with_transaction(|transaction| {
        upsert_audio_semantic_segment(
            transaction,
            &source,
            &payload.space,
            &payload.values,
            &runtime,
            crate::util::now_millis(),
        )
    })?;
    super::worker::record_inference_success(catalog, job, asset_id, &payload.runtime)
}

pub(crate) fn search(
    catalog: &Catalog,
    config: &crate::InferRuntimeConfig,
    query: &str,
    limit: u64,
) -> Result<Vec<echo_catalog::AudioSemanticSearchHit>, CoreError> {
    let text = query.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return Ok(Vec::new());
    }
    let language = if text.is_ascii() {
        "en"
    } else if text.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)) {
        "zh"
    } else {
        return Ok(Vec::new());
    };
    let digest = blake3::hash(text.as_bytes());
    let revision = format!(
        "echo:clap-query:v{INDEX_REVISION}:{}",
        &digest.to_hex()[..24]
    );
    let client = InferRuntimeClient::new(config.clone());
    let payload = client
        .embed_audio_text_query(
            &text,
            &AudioTextQueryEmbeddingIntent::new(revision, language),
        )
        .map_err(CoreError::from)?;
    catalog
        .with_transaction(|transaction| {
            search_audio_semantic_segments(transaction, &payload.values, &payload.space, limit)
        })
        .map_err(CoreError::from)
}

fn job_id(asset_id: AssetId) -> String {
    format!("embed-audio-clap-v{INDEX_REVISION}-{asset_id}")
}

#[cfg(test)]
mod tests;
