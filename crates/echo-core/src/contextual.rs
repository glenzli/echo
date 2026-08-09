//! Contextual understanding evidence and strict model-output ingestion.
//!
//! Runtime executes a stable text Intent but does not guarantee structured
//! output. Echo validates the complete product schema here, hard-compacts
//! presentation text, and only then publishes a summary or facet.

use echo_catalog::{AppendAnalysisRecord, AppendContextualAnalysis, record_contextual_analysis};
use echo_domain::{AnalysisKind, AnalysisRecord, AssetId, ModelIdentity};
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreErrorKind};

/// Current append-only contextual presentation contract.
pub const CONTEXTUAL_SCHEMA_VERSION: u32 = 3;
/// Prompt/validation revision within the current persisted schema.
pub const CONTEXTUAL_JOB_REVISION: u32 = 1;

const CONTEXTUAL_KEYS: [&str; 8] = [
    "schema_version",
    "sound_caption",
    "summary",
    "keywords",
    "mood",
    "place_hint",
    "event_type",
    "people_hints",
];
const SOUND_CAPTION_META_PREFIXES: [&str; 13] = [
    "this recording",
    "the recording",
    "this audio",
    "the audio",
    "a person",
    "someone",
    "the speaker",
    "这段录音",
    "该录音",
    "录音中",
    "这段音频",
    "有人描述",
    "说话人",
];

/// The canonical contextual payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextualPayload {
    #[serde(default = "legacy_contextual_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub sound_caption: String,
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
    schema_version: u32,
    sound_caption: String,
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
    transcript: &str,
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
    if decoded.schema_version != CONTEXTUAL_SCHEMA_VERSION {
        return Err(invalid_output("unsupported_schema_version"));
    }
    let sound_caption = validate_sound_caption(&decoded.sound_caption, transcript)?;
    let summary = validate_summary(&decoded.summary, transcript);
    if decoded.keywords.len() > 8 {
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
        schema_version: decoded.schema_version,
        sound_caption,
        summary,
        keywords,
        mood,
        place_hint,
        event_type,
        people_hints,
    })
}

impl ContextualPayload {
    /// Whether this evidence can drive the current sound-wall presentation.
    #[must_use]
    pub fn is_current(&self) -> bool {
        self.schema_version == CONTEXTUAL_SCHEMA_VERSION && !self.sound_caption.trim().is_empty()
    }
}

const fn legacy_contextual_schema_version() -> u32 {
    1
}

fn validate_sound_caption(value: &str, transcript: &str) -> Result<String, ContextualOutputError> {
    let caption = bounded_required(value, 64, "invalid_sound_caption")?;
    if caption.contains(['\n', '\r']) {
        return Err(invalid_output("invalid_sound_caption"));
    }

    let lowercase = caption.to_lowercase();
    if SOUND_CAPTION_META_PREFIXES
        .iter()
        .any(|prefix| lowercase.starts_with(prefix))
    {
        return Err(invalid_output("sound_caption_meta_language"));
    }

    let script = dominant_script(&caption);
    let unit_count = text_units(&caption, script);
    let maximum_units = if uses_character_units(script) { 14 } else { 7 };
    if unit_count == 0 {
        return Err(invalid_output("sound_caption_too_long"));
    }

    validate_primary_script(&caption, transcript, "sound_caption_language_mismatch")?;
    let normalized_caption = normalize_comparison_text(&caption);
    let normalized_transcript = normalize_comparison_text(transcript);
    if normalized_caption == normalized_transcript
        || (normalized_caption.chars().count() >= 8
            && normalized_transcript.contains(&normalized_caption))
    {
        return Err(invalid_output("sound_caption_copies_transcript"));
    }
    Ok(compact_text(&caption, script, maximum_units))
}

fn validate_summary(value: &str, transcript: &str) -> String {
    let summary = value.trim();
    if summary.is_empty() {
        return String::new();
    }
    if summary.contains(['\n', '\r']) {
        return String::new();
    }
    if validate_primary_script(summary, transcript, "summary_language_mismatch").is_err() {
        return String::new();
    }

    let script = dominant_script(transcript);
    let summary_units = text_units(summary, script);
    let transcript_units = text_units(transcript, script);
    let maximum_summary_units = if uses_character_units(script) { 40 } else { 16 };
    let minimum_source_units = if uses_character_units(script) { 60 } else { 30 };
    if summary_units == 0
        || transcript_units < minimum_source_units
        || summary_units.saturating_mul(3) > transcript_units
    {
        return String::new();
    }
    compact_text(summary, script, maximum_summary_units)
}

fn uses_character_units(script: Option<Script>) -> bool {
    matches!(script, Some(Script::Cjk | Script::Hangul))
}

fn text_units(value: &str, script: Option<Script>) -> usize {
    if uses_character_units(script) {
        value
            .chars()
            .filter(|character| !character.is_whitespace())
            .count()
    } else {
        value.split_whitespace().count()
    }
}

fn compact_text(value: &str, script: Option<Script>, maximum_units: usize) -> String {
    if uses_character_units(script) {
        let mut units = 0;
        value
            .trim()
            .chars()
            .take_while(|character| {
                if character.is_whitespace() {
                    true
                } else if units < maximum_units {
                    units += 1;
                    true
                } else {
                    false
                }
            })
            .collect::<String>()
            .trim()
            .to_owned()
    } else {
        value
            .split_whitespace()
            .take(maximum_units)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn normalize_comparison_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn validate_primary_script(
    value: &str,
    transcript: &str,
    code: &'static str,
) -> Result<(), ContextualOutputError> {
    if let (Some(value_script), Some(transcript_script)) =
        (dominant_script(value), dominant_script(transcript))
        && value_script != transcript_script
    {
        return Err(invalid_output(code));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Script {
    Cjk,
    Hangul,
    Cyrillic,
    Arabic,
    Devanagari,
    Latin,
}

fn dominant_script(value: &str) -> Option<Script> {
    let mut counts = [0_u32; 6];
    for character in value.chars() {
        let codepoint = u32::from(character);
        let script = if matches!(
            codepoint,
            0x3040..=0x309f
                | 0x30a0..=0x30ff
                | 0x31f0..=0x31ff
                | 0x3400..=0x4dbf
                | 0x4e00..=0x9fff
                | 0xf900..=0xfaff
        ) {
            Some(Script::Cjk)
        } else if matches!(codepoint, 0xac00..=0xd7af | 0x1100..=0x11ff) {
            Some(Script::Hangul)
        } else if matches!(codepoint, 0x0400..=0x052f) {
            Some(Script::Cyrillic)
        } else if matches!(codepoint, 0x0600..=0x06ff | 0x0750..=0x077f) {
            Some(Script::Arabic)
        } else if matches!(codepoint, 0x0900..=0x097f) {
            Some(Script::Devanagari)
        } else if character.is_ascii_alphabetic()
            || matches!(codepoint, 0x00c0..=0x024f | 0x1e00..=0x1eff)
        {
            Some(Script::Latin)
        } else {
            None
        };
        if let Some(script) = script {
            counts[script as usize] += 1;
        }
    }
    counts
        .iter()
        .enumerate()
        .max_by_key(|(_, count)| **count)
        .filter(|(_, count)| **count > 0)
        .map(|(index, _)| match index {
            0 => Script::Cjk,
            1 => Script::Hangul,
            2 => Script::Cyrillic,
            3 => Script::Arabic,
            4 => Script::Devanagari,
            _ => Script::Latin,
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
                },
            )
        })
        .map_err(CoreError::from)
}

#[cfg(test)]
mod tests;
