//! Typed local text embeddings for Echo-owned semantic retrieval.
//!
//! Runtime owns physical execution. Echo validates the exact shared space,
//! App-scoped Job, local-only constraints, and stale query/source revision.

use std::{collections::BTreeMap, time::Duration};

use serde::{Deserialize, Serialize};

use super::{
    InferRuntimeClient, InferRuntimeError, InferRuntimeErrorKind, RuntimeJobSnapshot,
    RuntimeProvenance, RuntimeSession, checked_json_response, default_background_constraints,
    validate_succeeded_job,
};

pub const TEXT_EMBEDDING_INTENT: &str = "semantic.embed_text";
pub const TEXT_EMBEDDING_DIMENSIONS: usize = 768;
const MAX_TEXT_BYTES: usize = 4_096;
const MAX_REVISION_BYTES: usize = 256;

/// Product request for a bounded text document or natural-language query.
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

/// Stable typed-provider provenance returned with a semantic vector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextEmbeddingProviderProvenance {
    pub job_id: String,
    pub provider: String,
    pub deployment: String,
    pub model_build: String,
    pub artifact_sha256: String,
    pub preprocessing_identity: String,
    pub postprocessing_identity: String,
    pub tokenizer: Option<serde_json::Value>,
    pub runtime: String,
    pub requested_execution_provider: String,
    pub actual_execution_provider: String,
    pub execution_provider_fallback_reason: Option<String>,
    pub precision: String,
}

/// Validated normalized vector plus Runtime and provider evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextEmbeddingPayload {
    pub values: Vec<f32>,
    pub space: String,
    pub provider: TextEmbeddingProviderProvenance,
    pub runtime: RuntimeProvenance,
}

#[derive(Serialize)]
struct TextEmbeddingRequest<'a> {
    model: &'a str,
    text: &'a str,
    query_revision: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<&'a str>,
    metadata: &'a BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct TextEmbeddingResponse {
    id: String,
    object: String,
    status: String,
    query_revision: String,
    embedding: EmbeddingVector,
    provenance: TextEmbeddingProviderProvenance,
}

#[derive(Deserialize)]
struct EmbeddingVector {
    values: Vec<f32>,
    dimensions: usize,
    normalized: bool,
    distance_metric: String,
    space: String,
}

impl InferRuntimeClient {
    /// Embeds one bounded text value and validates complete local provenance.
    ///
    /// # Errors
    ///
    /// Returns a classified validation, transport, Runtime, or provenance failure.
    pub fn embed_text(
        &self,
        text: &str,
        intent: &TextEmbeddingIntent,
    ) -> Result<TextEmbeddingPayload, InferRuntimeError> {
        let text = text.trim();
        validate_request(text, intent)?;
        self.validate_token()?;
        let session = self.begin_session()?;
        let request = TextEmbeddingRequest {
            model: &intent.model,
            text,
            query_revision: &intent.revision,
            language: intent.language.as_deref(),
            metadata: &intent.metadata,
        };
        let url = session.url("/infer/v1/vision/text-embeddings");
        let response = ureq::post(&url)
            .header("Authorization", self.authorization())
            .config()
            .timeout_global(Some(Duration::from_mins(5)))
            .proxy(None)
            .max_redirects(0)
            .http_status_as_error(false)
            .build()
            .send_json(&request)
            .map_err(|error| self.transport_error(error))?;
        let (_, body) = checked_json_response(response)?;
        let response: TextEmbeddingResponse = serde_json::from_str(&body).map_err(|_| {
            InferRuntimeError::new(
                InferRuntimeErrorKind::Protocol,
                "invalid_text_embedding_response",
                Some(200),
            )
        })?;
        validate_response(&response, intent)?;
        let job = self.job_snapshot(&session, &response.id)?;
        validate_succeeded_job(&job, TEXT_EMBEDDING_INTENT)?;
        validate_constraints(&job)?;
        if response.provenance.job_id != response.id
            || response.provenance.provider != job.provider
            || response.provenance.deployment != job.deployment
            || response.provenance.model_build != job.model_build
        {
            return Err(protocol("inconsistent_text_embedding_provenance"));
        }
        Ok(TextEmbeddingPayload {
            values: response.embedding.values,
            space: response.embedding.space,
            provider: response.provenance,
            runtime: RuntimeProvenance {
                contract_version: RuntimeSession::contract_version().to_owned(),
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
    response: &TextEmbeddingResponse,
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

fn validate_constraints(job: &RuntimeJobSnapshot) -> Result<(), InferRuntimeError> {
    let constraints = &job.constraints;
    let valid = job.policy == "local-first"
        && job.priority == "background"
        && job.placement == "local"
        && job.capability_level == "foundational"
        && constraints.policy.as_deref() == Some("local-first")
        && constraints.priority.as_deref() == Some("background")
        && constraints.placement.as_deref() == Some("local_only")
        && constraints.prefer.as_deref() == Some("local")
        && constraints.offline_required == Some(true)
        && constraints.capability_floor.as_deref() == Some("foundational")
        && constraints.latency.as_deref() == Some("throughput")
        && constraints.fallback.as_deref() == Some("none")
        && constraints.max_cost_usd == Some(0.0)
        && job
            .attempts
            .iter()
            .all(|attempt| attempt.trigger != "fallback");
    if valid {
        Ok(())
    } else {
        Err(protocol("inconsistent_text_embedding_constraints"))
    }
}

fn rejected(code: &'static str) -> InferRuntimeError {
    InferRuntimeError::new(InferRuntimeErrorKind::Rejected, code, None)
}

fn protocol(code: &'static str) -> InferRuntimeError {
    InferRuntimeError::new(InferRuntimeErrorKind::Protocol, code, Some(200))
}

#[cfg(test)]
mod tests;
