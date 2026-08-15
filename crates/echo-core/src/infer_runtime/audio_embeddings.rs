//! Typed CLAP audio and paired text-query SDK boundary.

use std::{collections::BTreeMap, path::Path};

use infer_runtime_client::AudioTextEmbeddingRequest;
use serde::{Deserialize, Serialize};

use super::{
    AUDIO_EMBEDDING_CAPABILITY, InferRuntimeClient, InferRuntimeError, RuntimeProvenance,
    default_background_constraints, protocol, rejected, validate_local_only_job,
    validate_succeeded_job,
};

pub const AUDIO_EMBEDDING_INTENT: &str = "audio.embed";
pub const AUDIO_TEXT_QUERY_EMBEDDING_INTENT: &str = "audio.embed_text_query";
const DIMENSIONS: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioTextQueryEmbeddingIntent {
    pub revision: String,
    pub language: String,
    pub metadata: BTreeMap<String, String>,
}

impl AudioTextQueryEmbeddingIntent {
    #[must_use]
    pub fn new(revision: impl Into<String>, language: impl Into<String>) -> Self {
        Self {
            revision: revision.into(),
            language: language.into(),
            metadata: default_background_constraints(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioEmbeddingPayload {
    pub values: Vec<f32>,
    pub space: String,
    pub provider: serde_json::Value,
    pub runtime: RuntimeProvenance,
}

impl InferRuntimeClient {
    pub fn embed_audio(
        &self,
        source: &Path,
        source_revision: &str,
    ) -> Result<AudioEmbeddingPayload, InferRuntimeError> {
        if source_revision.is_empty() || source_revision.len() > 256 {
            return Err(rejected("invalid_audio_embedding_identity"));
        }
        let metadata = default_background_constraints();
        let (response, job) = self
            .transport()?
            .embed_audio(source, source_revision, &metadata)?;
        validate(&response, AUDIO_EMBEDDING_INTENT, source_revision)?;
        let job = validate_succeeded_job(job, AUDIO_EMBEDDING_INTENT, AUDIO_EMBEDDING_CAPABILITY)?;
        validate_local_only_job(&job, "inconsistent_audio_embedding_constraints")?;
        Ok(payload(response, job))
    }

    pub fn embed_audio_text_query(
        &self,
        text: &str,
        intent: &AudioTextQueryEmbeddingIntent,
    ) -> Result<AudioEmbeddingPayload, InferRuntimeError> {
        let text = text.trim();
        if text.is_empty()
            || text.len() > 16 * 1024
            || intent.revision.is_empty()
            || intent.language.is_empty()
            || !matches!(intent.language.as_str(), "en" | "zh")
        {
            return Err(rejected("invalid_audio_text_query"));
        }
        let request = AudioTextEmbeddingRequest {
            model: AUDIO_TEXT_QUERY_EMBEDDING_INTENT.to_owned(),
            text: text.to_owned(),
            query_revision: intent.revision.clone(),
            language: intent.language.clone(),
            metadata: intent.metadata.clone(),
        };
        let (response, job) = self.transport()?.embed_audio_text(&request)?;
        validate(
            &response,
            AUDIO_TEXT_QUERY_EMBEDDING_INTENT,
            &intent.revision,
        )?;
        let job = validate_succeeded_job(
            job,
            AUDIO_TEXT_QUERY_EMBEDDING_INTENT,
            AUDIO_EMBEDDING_CAPABILITY,
        )?;
        validate_local_only_job(&job, "inconsistent_audio_text_embedding_constraints")?;
        Ok(payload(response, job))
    }
}

fn validate(
    response: &infer_runtime_client::AudioEmbeddingResponse,
    intent: &str,
    revision: &str,
) -> Result<(), InferRuntimeError> {
    let revision_matches = if intent == AUDIO_EMBEDDING_INTENT {
        response.source_revision.as_deref() == Some(revision)
    } else {
        response.query_revision.as_deref() == Some(revision)
    };
    if response.model != intent
        || response.embedding.len() != DIMENSIONS
        || response.embedding_space.identity.is_empty()
        || !response.embedding_space.normalized
        || response.embedding_space.distance_metric != "cosine"
        || !revision_matches
        || response.embedding.iter().any(|v| !v.is_finite())
    {
        return Err(protocol("invalid_audio_embedding_payload"));
    }
    Ok(())
}

fn payload(
    response: infer_runtime_client::AudioEmbeddingResponse,
    job: super::RuntimeJobSnapshot,
) -> AudioEmbeddingPayload {
    let provider = serde_json::json!({ "build": response.provenance.build, "artifact_set_sha256": response.provenance.artifact_set_sha256, "runtime": response.provenance.runtime, "precision": response.provenance.precision, "requested_execution_provider": response.provenance.requested_execution_provider, "actual_execution_provider": response.provenance.actual_execution_provider, "preprocessing_identity": response.provenance.preprocessing_identity, "tokenizer_identity": response.provenance.tokenizer_identity, "query_normalizer": response.query_normalizer });
    AudioEmbeddingPayload {
        values: response.embedding,
        space: response.embedding_space.identity,
        provider,
        runtime: RuntimeProvenance {
            contract_version: super::EXPECTED_CONTRACT_VERSION.to_owned(),
            job,
        },
    }
}
