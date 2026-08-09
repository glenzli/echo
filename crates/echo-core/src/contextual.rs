//! Contextual understanding evidence and strict model-output ingestion.
//!
//! Runtime executes a stable text Intent but does not guarantee structured
//! output. Echo validates the complete product schema here before publishing
//! any summary or facet.

use echo_catalog::{AppendAnalysisRecord, AppendContextualAnalysis, record_contextual_analysis};
use echo_domain::{AnalysisKind, AnalysisRecord, AssetId, ModelIdentity};
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreErrorKind};

const CONTEXTUAL_KEYS: [&str; 6] = [
    "summary",
    "keywords",
    "mood",
    "place_hint",
    "event_type",
    "people_hints",
];

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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StrictContextualPayload {
    summary: String,
    keywords: Vec<String>,
    mood: Option<String>,
    place_hint: Option<String>,
    event_type: Option<String>,
    people_hints: Vec<String>,
}

/// A content-free validation failure safe for job diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid contextual model output: {code}")]
pub(crate) struct ContextualOutputError {
    pub code: &'static str,
}

pub(crate) fn decode_contextual_output(
    output: &str,
) -> Result<ContextualPayload, ContextualOutputError> {
    let value: serde_json::Value =
        serde_json::from_str(output.trim()).map_err(|_| invalid_output("invalid_json"))?;
    let object = value
        .as_object()
        .ok_or_else(|| invalid_output("object_required"))?;
    if object.len() != CONTEXTUAL_KEYS.len()
        || CONTEXTUAL_KEYS.iter().any(|key| !object.contains_key(*key))
    {
        return Err(invalid_output("unexpected_fields"));
    }
    let decoded: StrictContextualPayload =
        serde_json::from_value(value).map_err(|_| invalid_output("invalid_field_type"))?;
    let summary = bounded_required(&decoded.summary, 400, "invalid_summary")?;
    if !(1..=8).contains(&decoded.keywords.len()) {
        return Err(invalid_output("invalid_keyword_count"));
    }
    let keywords = decoded
        .keywords
        .into_iter()
        .map(|keyword| bounded_required(&keyword, 48, "invalid_keyword"))
        .collect::<Result<Vec<_>, _>>()?;
    let mood = bounded_optional(decoded.mood, 80, "invalid_mood")?;
    let place_hint = bounded_optional(decoded.place_hint, 120, "invalid_place_hint")?;
    let event_type = bounded_optional(decoded.event_type, 80, "invalid_event_type")?;
    if decoded.people_hints.len() > 8 {
        return Err(invalid_output("invalid_people_count"));
    }
    let people_hints = decoded
        .people_hints
        .into_iter()
        .map(|person| bounded_required(&person, 80, "invalid_people_hint"))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ContextualPayload {
        summary,
        keywords,
        mood,
        place_hint,
        event_type,
        people_hints,
    })
}

fn bounded_required(
    value: &str,
    max_chars: usize,
    code: &'static str,
) -> Result<String, ContextualOutputError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max_chars {
        return Err(invalid_output(code));
    }
    Ok(value.to_owned())
}

fn bounded_optional(
    value: Option<String>,
    max_chars: usize,
    code: &'static str,
) -> Result<Option<String>, ContextualOutputError> {
    value
        .map(|value| bounded_required(&value, max_chars, code))
        .transpose()
}

const fn invalid_output(code: &'static str) -> ContextualOutputError {
    ContextualOutputError { code }
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
    provenance: &crate::RuntimeProvenance,
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
            record_contextual_analysis(
                transaction,
                &AppendContextualAnalysis {
                    analysis: AppendAnalysisRecord {
                        asset_id,
                        record: AnalysisRecord::new(
                            AnalysisKind::Contextual,
                            value,
                            ModelIdentity::new(
                                provenance.job.physical_model.clone(),
                                provenance.job.model_build.clone(),
                            ),
                            None,
                            now,
                        ),
                    },
                    keywords: &payload.keywords,
                },
            )
        })
        .map_err(CoreError::from)
}

#[cfg(test)]
mod tests;
