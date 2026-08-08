//! Contextual understanding: summary, keywords, mood, and place hints from a
//! transcript via a local Ollama model.
//!
//! The worker is an HTTP call to the Ollama chat API with structured JSON
//! output; no Python subprocess is involved. Evidence is recorded with the
//! model identity, so a future model upgrade can re-run analysis.

use std::path::PathBuf;

use echo_catalog::{AppendAnalysisRecord, record_analysis};
use echo_domain::{AnalysisKind, AnalysisRecord, AssetId, ModelIdentity};
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreErrorKind};

/// The canonical contextual payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextualPayload {
    pub summary: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub mood: Option<String>,
    #[serde(default)]
    pub place_hint: Option<String>,
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub people_hints: Vec<String>,
}

/// The Ollama worker invocation contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextualWorker {
    /// Ollama endpoint, e.g. `http://127.0.0.1:11434`.
    pub endpoint: String,
    /// Model name as shown by `ollama list`, e.g. `qwen3.5:4b-mlx`.
    pub model: String,
}

const PROMPT_TEMPLATE: &str = r#"你是 Echo 声音记忆的上下文理解器。根据以下语音转写，
提取结构化的记忆信息。只用 JSON 回答，不要输出任何其他文字：

{
  "summary": "一句话中文摘要",
  "keywords": ["2-5 个中文关键词"],
  "mood": "情绪基调，如 开心/平静/难过，无则省略",
  "place_hint": "场景地点推断，如 家里/公园/车上，无则省略",
  "event_type": "事件类型，如 亲子出游/雨天日常/生日，无则省略",
  "people_hints": ["出现的人物线索"]
}

转写：
{transcript}"#;

/// Runs contextual analysis over a transcript and returns the payload.
///
/// # Errors
///
/// Returns [`CoreError`] when the Ollama server cannot be reached or the
/// response cannot be parsed.
pub fn run_contextual(
    transcript_text: &str,
    worker: &ContextualWorker,
) -> Result<ContextualPayload, CoreError> {
    let prompt = PROMPT_TEMPLATE.replace("{transcript}", transcript_text);
    let body = serde_json::json!({
        "model": worker.model,
        "messages": [{ "role": "user", "content": prompt }],
        "format": "json",
        "stream": false,
    });
    let url = format!("{}/api/chat", worker.endpoint.trim_end_matches('/'));
    let response = ureq::post(&url)
        .config()
        .timeout_global(Some(std::time::Duration::from_mins(10)))
        .build()
        .send_json(body)
        .map_err(|error| {
            CoreError::new(
                CoreErrorKind::Other,
                format!("cannot reach Ollama at {url}: {error}"),
            )
        })?;
    let body = response.into_body().read_to_string().map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot read Ollama response: {error}"),
        )
    })?;
    let chat: serde_json::Value = serde_json::from_str(&body).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot parse Ollama response: {error}"),
        )
    })?;
    let content = chat
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            CoreError::new(
                CoreErrorKind::Other,
                "Ollama response lacks a message content",
            )
        })?;
    serde_json::from_str(content).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("Ollama output is not valid contextual JSON: {error}"),
        )
    })
}

/// Records contextual evidence for an asset.
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn record_contextual(
    catalog: &echo_catalog::Catalog,
    asset_id: AssetId,
    payload: &ContextualPayload,
    model_name: &str,
) -> Result<(), CoreError> {
    let value = serde_json::to_value(payload).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot encode contextual payload: {error}"),
        )
    })?;
    let now = crate::import::now_millis();
    catalog
        .with_transaction(|transaction| {
            record_analysis(
                transaction,
                &AppendAnalysisRecord {
                    asset_id,
                    record: AnalysisRecord::new(
                        AnalysisKind::Contextual,
                        value,
                        ModelIdentity::new(model_name.to_owned(), "ollama".to_owned()),
                        None,
                        now,
                    ),
                },
            )
        })
        .map_err(CoreError::from)
}

/// Resolves a contextual model path for display/typing purposes.
#[allow(dead_code)]
pub(crate) fn _model_reference(endpoint: &str, model: &str) -> PathBuf {
    PathBuf::from(format!("{endpoint}::{model}"))
}
