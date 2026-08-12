//! Echo semantic-search validation over the official text-embedding SDK.

use std::collections::BTreeMap;

use infer_runtime_client::TextEmbeddingRequest;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    InferRuntimeClient, InferRuntimeError, RuntimeProvenance, default_background_constraints,
    protocol, rejected, validate_local_only_job, validate_succeeded_job,
};

pub const TEXT_EMBEDDING_INTENT: &str = "semantic.embed_text";
pub const TEXT_EMBEDDING_DIMENSIONS: usize = 768;

const TEXT_EMBEDDING_CAPABILITY: &str = "infer.vision.text-embedding@20260811.1";
const MAX_TEXT_BYTES: usize = 4_096;
const MAX_REVISION_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEmbeddingIntent {
    pub model: String,
    pub revision: String,
    pub language: Option<String>,
    pub metadata: BTreeMap<String, String>,
}

impl TextEmbeddingIntent {
    #[must_use]
    pub fn new(revision: impl Into<String>) -> Self {
        Self {
            model: TEXT_EMBEDDING_INTENT.to_owned(),
            revision: revision.into(),
            language: None,
            metadata: default_background_constraints(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextEmbeddingProviderProvenance {
    pub job_id: String,
    pub provider: String,
    pub deployment: String,
    pub model_build: String,
    pub artifact_sha256: String,
    pub preprocessing_identity: String,
    pub postprocessing_identity: String,
    pub tokenizer: Option<Value>,
    pub runtime: String,
    pub requested_execution_provider: String,
    pub actual_execution_provider: String,
    pub execution_provider_fallback_reason: Option<String>,
    pub precision: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextEmbeddingPayload {
    pub values: Vec<f32>,
    pub space: String,
    pub provider: TextEmbeddingProviderProvenance,
    pub runtime: RuntimeProvenance,
}

impl InferRuntimeClient {
    /// Embeds one bounded text value and validates Echo's local-only policy.
    ///
    /// # Errors
    ///
    /// Returns an input, SDK, Runtime, vector or provenance validation failure.
    pub fn embed_text(
        &self,
        text: &str,
        intent: &TextEmbeddingIntent,
    ) -> Result<TextEmbeddingPayload, InferRuntimeError> {
        let text = text.trim();
        validate_request(text, intent)?;
        let request = TextEmbeddingRequest {
            model: intent.model.clone(),
            text: text.to_owned(),
            query_revision: intent.revision.clone(),
            language: intent.language.clone(),
            metadata: intent.metadata.clone(),
        };
        let (response, job) = self.transport()?.embed_text(&request)?;
        validate_response(&response, intent)?;
        let job = validate_succeeded_job(job, TEXT_EMBEDDING_INTENT, TEXT_EMBEDDING_CAPABILITY)?;
        validate_local_only_job(&job, "inconsistent_text_embedding_constraints")?;
        if response.provenance.job_id != response.id
            || response.provenance.provider != job.provider
            || response.provenance.deployment != job.deployment
            || response.provenance.model_build != job.model_build
        {
            return Err(protocol("inconsistent_text_embedding_provenance"));
        }
        let extra = &response.provenance.extra;
        let provider = TextEmbeddingProviderProvenance {
            job_id: response.provenance.job_id,
            provider: response.provenance.provider,
            deployment: response.provenance.deployment,
            model_build: response.provenance.model_build,
            artifact_sha256: response.provenance.artifact_sha256,
            preprocessing_identity: response.provenance.preprocessing_identity,
            postprocessing_identity: response.provenance.postprocessing_identity,
            tokenizer: extra.get("tokenizer").cloned(),
            runtime: response.provenance.runtime,
            requested_execution_provider: response.provenance.requested_execution_provider,
            actual_execution_provider: response.provenance.actual_execution_provider,
            execution_provider_fallback_reason: response
                .provenance
                .execution_provider_fallback_reason,
            precision: response.provenance.precision,
        };
        Ok(TextEmbeddingPayload {
            values: response.embedding.values,
            space: response.embedding.space,
            provider,
            runtime: RuntimeProvenance {
                contract_version: super::EXPECTED_CONTRACT_VERSION.to_owned(),
                job,
            },
        })
    }
}

fn validate_request(text: &str, intent: &TextEmbeddingIntent) -> Result<(), InferRuntimeError> {
    if text.is_empty() || text.len() > MAX_TEXT_BYTES {
        return Err(rejected("text_embedding_input_size"));
    }
    if intent.model != TEXT_EMBEDDING_INTENT
        || intent.revision.is_empty()
        || intent.revision.len() > MAX_REVISION_BYTES
    {
        return Err(rejected("invalid_text_embedding_identity"));
    }
    Ok(())
}

fn validate_response(
    response: &infer_runtime_client::TextEmbeddingResponse,
    intent: &TextEmbeddingIntent,
) -> Result<(), InferRuntimeError> {
    let vector = &response.embedding;
    if response.id.is_empty()
        || response.object != "vision.text_embedding"
        || response.status != "completed"
        || response.query_revision != intent.revision
        || vector.dimensions != TEXT_EMBEDDING_DIMENSIONS
        || vector.values.len() != TEXT_EMBEDDING_DIMENSIONS
        || !vector.normalized
        || vector.distance_metric != "cosine"
        || vector.space.trim().is_empty()
        || vector.values.iter().any(|value| !value.is_finite())
    {
        return Err(protocol("invalid_text_embedding_payload"));
    }
    let norm_sq = vector
        .values
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>();
    if !(0.98..=1.02).contains(&norm_sq) {
        return Err(protocol("unnormalized_text_embedding"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
