//! Canonical sound-event evidence and deterministic browse presentation.
//!
//! Runtime results remain immutable and retain stable `AudioSet` class IDs,
//! policy, coverage and provenance. A compact Contextual projection is derived
//! only for existing sound-wall, filter and search consumers; labels never
//! replace ontology identity in the evidence record.

use std::collections::BTreeMap;

use echo_catalog::{
    AppendAnalysisRecord, AppendContextualAnalysis, record_analysis, record_contextual_analysis,
};
use echo_domain::{AnalysisKind, AnalysisRecord, AssetId, ModelIdentity};
use serde::{Deserialize, Serialize};

use crate::{
    AudioEventDetection, ContextualPayload,
    error::{CoreError, CoreErrorKind},
};

/// Current append-only `AudioEvents` evidence contract.
pub const AUDIO_EVENTS_SCHEMA_VERSION: u32 = 1;
const AUDIO_EVENT_PRESENTATION_REVISION: u32 = 1;

/// One bounded Runtime result positioned on the immutable Original timeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioEventChunk {
    pub source_start_seconds: f64,
    pub source_end_seconds: f64,
    pub detection: AudioEventDetection,
}

/// Complete accepted sound-event evidence for one asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioEventsEvidence {
    pub schema_version: u32,
    pub chunks: Vec<AudioEventChunk>,
}

impl AudioEventsEvidence {
    /// Wraps a single bounded Runtime result on the Original timeline.
    #[must_use]
    pub fn from_detection(detection: AudioEventDetection, source_start_seconds: f64) -> Self {
        let source_end_seconds = source_start_seconds + detection.coverage.input_duration_seconds;
        Self {
            schema_version: AUDIO_EVENTS_SCHEMA_VERSION,
            chunks: vec![AudioEventChunk {
                source_start_seconds,
                source_end_seconds,
                detection,
            }],
        }
    }
}

/// Appends `AudioEvents` and, when events exist, atomically publishes a compact
/// Contextual projection for the existing sound wall and browse facets.
///
/// Returns whether a presentation projection was published.
///
/// # Errors
///
/// Returns a validation, serialization, or catalog failure.
pub fn record_audio_events(
    catalog: &echo_catalog::Catalog,
    asset_id: AssetId,
    evidence: &AudioEventsEvidence,
) -> Result<bool, CoreError> {
    validate_evidence(evidence)?;
    let value = serde_json::to_value(evidence).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot encode sound-event evidence: {error}"),
        )
    })?;
    let presentation = presentation(evidence);
    let confidence = evidence
        .chunks
        .iter()
        .flat_map(|chunk| &chunk.detection.events)
        .map(|event| event.score)
        .max_by(f64::total_cmp);
    let first = &evidence.chunks[0].detection;
    let now = crate::util::now_millis();
    catalog
        .with_transaction(|transaction| {
            record_analysis(
                transaction,
                &AppendAnalysisRecord {
                    asset_id,
                    record: AnalysisRecord::new(
                        AnalysisKind::AudioEvents,
                        value,
                        ModelIdentity::new(
                            first.runtime.job.physical_model.clone(),
                            first.runtime.job.model_build.clone(),
                        ),
                        confidence,
                        now,
                    ),
                },
            )?;
            if let Some(payload) = &presentation {
                record_contextual_analysis(
                    transaction,
                    &AppendContextualAnalysis {
                        analysis: AppendAnalysisRecord {
                            asset_id,
                            record: AnalysisRecord::new(
                                AnalysisKind::Contextual,
                                serde_json::to_value(payload).map_err(|error| {
                                    echo_catalog::CatalogError::new(
                                        echo_catalog::CatalogErrorKind::Other,
                                        format!("cannot encode sound-event presentation: {error}"),
                                    )
                                })?,
                                ModelIdentity::new(
                                    "echo/audio-event-presentation".to_owned(),
                                    format!("revision-{AUDIO_EVENT_PRESENTATION_REVISION}"),
                                ),
                                confidence,
                                now,
                            ),
                        },
                    },
                )?;
            }
            Ok::<_, echo_catalog::CatalogError>(())
        })
        .map_err(CoreError::from)?;
    Ok(presentation.is_some())
}

fn validate_evidence(evidence: &AudioEventsEvidence) -> Result<(), CoreError> {
    if evidence.schema_version == AUDIO_EVENTS_SCHEMA_VERSION
        && let Some(reference) = evidence.chunks.first()
    {
        let mut previous_end = 0.0;
        let valid = evidence.chunks.iter().enumerate().all(|(index, chunk)| {
            let detection = &chunk.detection;
            let runtime = &detection.runtime;
            let timeline_valid = chunk.source_start_seconds.is_finite()
                && chunk.source_end_seconds.is_finite()
                && chunk.source_start_seconds >= 0.0
                && chunk.source_end_seconds > chunk.source_start_seconds
                && (index == 0 || chunk.source_start_seconds + 1e-6 >= previous_end)
                && approximately(
                    chunk.source_end_seconds - chunk.source_start_seconds,
                    detection.coverage.input_duration_seconds,
                );
            previous_end = chunk.source_end_seconds;
            timeline_valid
                && detection.model == crate::AUDIO_EVENT_DETECTION_INTENT
                && detection.object == "audio.event_detection"
                && crate::infer_runtime::supports_contract_version(&runtime.contract_version)
                && runtime.job.app_id == "echo"
                && runtime.job.intent == crate::AUDIO_EVENT_DETECTION_INTENT
                && runtime.job.state == "succeeded"
                && detection.ontology.id == reference.detection.ontology.id
                && detection.ontology.revision == reference.detection.ontology.revision
                && detection.ontology.class_id_namespace
                    == reference.detection.ontology.class_id_namespace
                && detection.policy.revision == reference.detection.policy.revision
                && detection.policy.score_kind == reference.detection.policy.score_kind
                && runtime.job.physical_model == reference.detection.runtime.job.physical_model
                && runtime.job.model_build == reference.detection.runtime.job.model_build
        });
        if valid {
            return Ok(());
        }
    }
    Err(CoreError::new(
        CoreErrorKind::InferenceRejected,
        "invalid sound-event evidence envelope",
    ))
}

fn presentation(evidence: &AudioEventsEvidence) -> Option<ContextualPayload> {
    let mut by_class = BTreeMap::<String, (String, f64)>::new();
    for event in evidence
        .chunks
        .iter()
        .flat_map(|chunk| &chunk.detection.events)
    {
        let label = compact_label(&event.label, 48);
        if label.is_empty() {
            continue;
        }
        let entry = by_class
            .entry(event.class_id.clone())
            .or_insert_with(|| (label.clone(), event.score));
        if event.score > entry.1 {
            *entry = (label, event.score);
        }
    }
    let mut ranked = by_class.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .1
            .1
            .total_cmp(&left.1.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    let labels = ranked
        .iter()
        .map(|(_, (label, _))| label.clone())
        .collect::<Vec<_>>();
    let sound_caption = compact_caption(&labels)?;
    Some(ContextualPayload {
        schema_version: crate::CONTEXTUAL_SCHEMA_VERSION,
        sound_caption,
        summary: String::new(),
        keywords: labels.iter().take(8).cloned().collect(),
        mood: None,
        place_hint: None,
        event_type: labels.first().cloned(),
        people_hints: Vec::new(),
    })
}

fn compact_caption(labels: &[String]) -> Option<String> {
    let mut selected = Vec::new();
    let mut words = 0_usize;
    for label in labels.iter().take(8) {
        let label_words = label.split_whitespace().count().max(1);
        let candidate_words = words + label_words;
        let candidate = selected
            .iter()
            .chain(std::iter::once(label))
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ");
        if !caption_fits(&candidate, candidate_words) {
            continue;
        }
        selected.push(label.clone());
        words = candidate_words;
        if selected.len() == 3 {
            break;
        }
    }
    if selected.is_empty() {
        return labels.first().map(|label| {
            if label.chars().any(is_cjk) {
                label.chars().take(14).collect()
            } else {
                compact_words(label, 7, 64)
            }
        });
    }
    Some(selected.join(" · "))
}

fn caption_fits(value: &str, words: usize) -> bool {
    if value.chars().any(is_cjk) {
        value.chars().count() <= 14
    } else {
        words <= 7 && value.chars().count() <= 64
    }
}

fn is_cjk(character: char) -> bool {
    matches!(
        u32::from(character),
        0x3040..=0x30ff | 0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff
    )
}

fn approximately(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-6
}

fn compact_label(value: &str, max_chars: usize) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_chars)
        .collect()
}

fn compact_words(value: &str, max_words: usize, max_chars: usize) -> String {
    value
        .split_whitespace()
        .take(max_words)
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_chars)
        .collect()
}

#[cfg(test)]
mod tests;
