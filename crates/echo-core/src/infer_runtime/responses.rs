//! Infer Runtime Responses-family consumer for contextual understanding.
//!
//! Echo owns the product prompt and output ingestion policy. Runtime owns
//! admission, routing and physical model execution. This module bridges those
//! contracts without exposing a generic naked-model API to the rest of Echo.

use std::{collections::BTreeMap, time::Duration};

use serde::Serialize;
use serde_json::Value;

use super::{
    InferRuntimeClient, InferRuntimeError, InferRuntimeErrorKind, RuntimeProvenance,
    RuntimeSession, default_background_constraints, validate_succeeded_job,
};

/// Stable Runtime Intent used for Echo's contextual metadata.
pub const CONTEXTUAL_INTENT: &str = "text.summarize";
/// Contextual input is bounded explicitly; long-recording segmentation is a
/// separate product slice and must never become silent truncation here.
pub const MAX_CONTEXTUAL_INPUT_BYTES: usize = 64 * 1024;

const MAX_CONTEXTUAL_OUTPUT_BYTES: usize = 16 * 1024;
const CONTEXTUAL_INSTRUCTIONS: &str = r#"Analyze the supplied transcript as evidence about a real recording. Return exactly one compact JSON object and no markdown, using this exact shape and JSON types: {"schema_version":3,"sound_caption":"...","summary":"","keywords":[],"mood":null,"place_hint":null,"event_type":null,"people_hints":[]}. Keep every key even when evidence is absent. schema_version must be the JSON integer 3, never a string. sound_caption is a one-line sound sketch for a visual card: describe the audible scene or event in the transcript's primary language, use at most 14 CJK characters or 7 words, do not quote or excerpt the transcript, and do not use meta wording such as 'this recording', 'the audio', 'a person', or 'the speaker'. summary is optional compression, not a paraphrase: for a transcript shorter than 60 CJK characters or 30 words, summary must be the empty string. For longer transcripts, summary may be empty; when present it must use the transcript's primary language, use at most 40 CJK characters or 16 words, and contain no more than about one third as many characters or words as the transcript. keywords must be a JSON array of 0 to 8 short strings. mood, place_hint, and event_type must each be one short string or JSON null; use null instead of an explanation such as 'unknown' or 'not mentioned'. A non-null place_hint must be at most 20 CJK characters or 8 words. people_hints must be a JSON array of 0 to 8 short strings. Do not invent identities, locations, or events that are not supported by the transcript."#;

/// Product-level contextual request mapped onto Runtime Responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextualIntent {
    pub model: String,
    pub instructions: String,
    pub max_output_tokens: u32,
    pub metadata: BTreeMap<String, String>,
}

impl Default for ContextualIntent {
    fn default() -> Self {
        Self {
            model: CONTEXTUAL_INTENT.to_owned(),
            instructions: CONTEXTUAL_INSTRUCTIONS.to_owned(),
            max_output_tokens: 384,
            metadata: default_background_constraints(),
        }
    }
}

/// Model output text plus the Runtime evidence that produced it.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextualResponse {
    pub output_text: String,
    pub runtime: RuntimeProvenance,
}

#[derive(Serialize)]
struct ResponsesRequest<'a> {
    model: &'a str,
    input: &'a str,
    instructions: &'a str,
    stream: bool,
    background: bool,
    metadata: &'a BTreeMap<String, String>,
    max_output_tokens: u32,
}

impl InferRuntimeClient {
    /// Submits Echo's bounded contextual request and requires local-only,
    /// App-scoped Runtime provenance before returning any model text.
    ///
    /// # Errors
    ///
    /// Returns a classified admission, transport, response, or provenance
    /// failure. Product JSON validation happens in the contextual owner.
    pub fn contextualize(
        &self,
        input: &str,
        intent: &ContextualIntent,
    ) -> Result<ContextualResponse, InferRuntimeError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(InferRuntimeError::new(
                InferRuntimeErrorKind::Rejected,
                "contextual_input_required",
                None,
            ));
        }
        if input.len() > MAX_CONTEXTUAL_INPUT_BYTES {
            return Err(InferRuntimeError::new(
                InferRuntimeErrorKind::Rejected,
                "contextual_input_too_large",
                None,
            ));
        }
        self.validate_token()?;
        let session = self.begin_session()?;
        let request = ResponsesRequest {
            model: &intent.model,
            input,
            instructions: &intent.instructions,
            stream: false,
            background: false,
            metadata: &intent.metadata,
            max_output_tokens: intent.max_output_tokens,
        };
        let url = session.url("/v1/responses");
        let response = ureq::post(&url)
            .header("Authorization", self.authorization())
            .config()
            .timeout_global(Some(Duration::from_mins(30)))
            .proxy(None)
            .max_redirects(0)
            .http_status_as_error(false)
            .build()
            .send_json(&request)
            .map_err(|error| self.transport_error(error))?;
        let (_, body) = super::checked_json_response(response)?;
        let response: Value = serde_json::from_str(&body).map_err(|_| {
            InferRuntimeError::new(
                InferRuntimeErrorKind::Protocol,
                "invalid_contextual_response",
                Some(200),
            )
        })?;
        let response_id = response
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let model = response
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if response_id.is_empty() || model != CONTEXTUAL_INTENT {
            return Err(InferRuntimeError::new(
                InferRuntimeErrorKind::Protocol,
                "invalid_contextual_identity",
                Some(200),
            ));
        }
        let output_text = extract_output_text(&response).ok_or_else(|| {
            InferRuntimeError::new(
                InferRuntimeErrorKind::Protocol,
                "contextual_output_text_missing",
                Some(200),
            )
        })?;
        if output_text.is_empty() || output_text.len() > MAX_CONTEXTUAL_OUTPUT_BYTES {
            return Err(InferRuntimeError::new(
                InferRuntimeErrorKind::Protocol,
                "invalid_contextual_output_size",
                Some(200),
            ));
        }
        let job = self.job_snapshot(&session, response_id)?;
        validate_succeeded_job(&job, CONTEXTUAL_INTENT)?;
        validate_contextual_constraints(&job)?;
        Ok(ContextualResponse {
            output_text,
            runtime: RuntimeProvenance {
                contract_version: RuntimeSession::contract_version().to_owned(),
                job,
            },
        })
    }
}

fn extract_output_text(response: &Value) -> Option<String> {
    if let Some(text) = response.get("output_text").and_then(Value::as_str) {
        return Some(text.trim().to_owned());
    }
    let mut chunks = Vec::new();
    for output in response.get("output").and_then(Value::as_array)? {
        let Some(content) = output.get("content").and_then(Value::as_array) else {
            continue;
        };
        for item in content {
            if item.get("type").and_then(Value::as_str) == Some("output_text")
                && let Some(text) = item.get("text").and_then(Value::as_str)
            {
                chunks.push(text);
            }
        }
    }
    (!chunks.is_empty()).then(|| chunks.concat().trim().to_owned())
}

fn validate_contextual_constraints(
    job: &super::RuntimeJobSnapshot,
) -> Result<(), InferRuntimeError> {
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
        && constraints.max_cost_usd == Some(0.0)
        && constraints.fallback.as_deref() == Some("none")
        && job
            .attempts
            .iter()
            .all(|attempt| attempt.trigger != "fallback");
    if !valid {
        return Err(InferRuntimeError::new(
            InferRuntimeErrorKind::Protocol,
            "inconsistent_contextual_constraints",
            Some(200),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
