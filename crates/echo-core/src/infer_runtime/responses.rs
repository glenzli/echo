//! Echo's contextual product contract over the official Responses SDK.

use std::collections::BTreeMap;

use infer_runtime_client::ResponsesRequest;
use serde_json::Value;

use super::{
    InferRuntimeClient, InferRuntimeError, RuntimeProvenance, default_background_constraints,
    protocol, rejected, validate_local_only_job, validate_succeeded_job,
};

pub const CONTEXTUAL_INTENT: &str = "text.summarize";
pub const MAX_CONTEXTUAL_INPUT_BYTES: usize = 64 * 1024;

const RESPONSES_CAPABILITY: &str = "infer.responses@20260812.1";
const MAX_CONTEXTUAL_OUTPUT_BYTES: usize = 16 * 1024;
const CONTEXTUAL_INSTRUCTIONS: &str = r#"Analyze the supplied transcript as evidence about a real recording. Return exactly one compact JSON object and no markdown, using this exact shape and JSON types: {"schema_version":3,"sound_caption":"...","summary":"","keywords":[],"mood":null,"place_hint":null,"event_type":null,"people_hints":[]}. Keep every key even when evidence is absent. schema_version must be the JSON integer 3, never a string. sound_caption is a one-line sound sketch for a visual card: describe the audible scene or event in the transcript's primary language, use at most 14 CJK characters or 7 words, do not quote or excerpt the transcript, and do not use meta wording such as 'this recording', 'the audio', 'a person', or 'the speaker'. summary is optional compression, not a paraphrase: for a transcript shorter than 60 CJK characters or 30 words, summary must be the empty string. For longer transcripts, summary may be empty; when present it must use the transcript's primary language, use at most 40 CJK characters or 16 words, and contain no more than about one third as many characters or words as the transcript. keywords must be a JSON array of 0 to 8 short strings. mood, place_hint, and event_type must each be one short string or JSON null; use null instead of an explanation such as 'unknown' or 'not mentioned'. A non-null place_hint must be at most 20 CJK characters or 8 words. people_hints must be a JSON array of 0 to 8 short strings. Do not invent identities, locations, or events that are not supported by the transcript."#;

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

#[derive(Debug, Clone, PartialEq)]
pub struct ContextualResponse {
    pub output_text: String,
    pub runtime: RuntimeProvenance,
}

impl InferRuntimeClient {
    /// Runs Echo's bounded `text.summarize` Intent through the official SDK.
    ///
    /// # Errors
    ///
    /// Returns an input, SDK, Runtime, output or provenance validation failure.
    pub fn contextualize(
        &self,
        input: &str,
        intent: &ContextualIntent,
    ) -> Result<ContextualResponse, InferRuntimeError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(rejected("contextual_input_required"));
        }
        if input.len() > MAX_CONTEXTUAL_INPUT_BYTES {
            return Err(rejected("contextual_input_too_large"));
        }
        if intent.model != CONTEXTUAL_INTENT {
            return Err(rejected("invalid_contextual_intent"));
        }
        let request = ResponsesRequest {
            model: intent.model.clone(),
            input: Value::String(input.to_owned()),
            instructions: Some(Value::String(intent.instructions.clone())),
            stream: false,
            background: false,
            metadata: intent.metadata.clone(),
            tools: Vec::new(),
            reasoning: None,
            max_output_tokens: Some(intent.max_output_tokens),
        };
        let (response, job) = self.transport()?.contextualize(&request)?;
        if response.model != CONTEXTUAL_INTENT || response.status != "completed" {
            return Err(protocol("invalid_contextual_identity"));
        }
        let output_text = extract_output_text(&response.extra, &response.output)
            .ok_or_else(|| protocol("contextual_output_text_missing"))?;
        if output_text.is_empty() || output_text.len() > MAX_CONTEXTUAL_OUTPUT_BYTES {
            return Err(protocol("invalid_contextual_output_size"));
        }
        let job = validate_succeeded_job(job, CONTEXTUAL_INTENT, RESPONSES_CAPABILITY)?;
        validate_local_only_job(&job, "inconsistent_contextual_constraints")?;
        Ok(ContextualResponse {
            output_text,
            runtime: RuntimeProvenance {
                contract_version: super::EXPECTED_CONTRACT_VERSION.to_owned(),
                job,
            },
        })
    }
}

fn extract_output_text(extra: &BTreeMap<String, Value>, output: &[Value]) -> Option<String> {
    if let Some(text) = extra.get("output_text").and_then(Value::as_str) {
        return Some(text.trim().to_owned());
    }
    let mut chunks = Vec::new();
    for item in output {
        let Some(content) = item.get("content").and_then(Value::as_array) else {
            continue;
        };
        for part in content {
            if part.get("type").and_then(Value::as_str) == Some("output_text")
                && let Some(text) = part.get("text").and_then(Value::as_str)
            {
                chunks.push(text);
            }
        }
    }
    (!chunks.is_empty()).then(|| chunks.concat().trim().to_owned())
}

#[cfg(test)]
mod tests;
