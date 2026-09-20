//! Explicit, bounded original-time transcription. This never starts a worker
//! pool or marks a whole recording as analyzed; the caller accepts the result.
use crate::{
    CoreError, CoreErrorKind, InferRuntimeClient, InferRuntimeConfig, TranscriptPayload,
    TranscriptionIntent, hash_file,
};
use echo_catalog::{AppendAnalysisRecord, AssetLookup, Catalog, find_by_id, record_analysis};
use echo_domain::{AnalysisKind, AnalysisRecord, AssetId, ContentHash, ModelIdentity};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MAX_SELECTION_TRANSCRIPTION_MILLIS: u64 = 300_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectionTranscript {
    pub schema_version: u32,
    pub source_hash: ContentHash,
    pub start_millis: u64,
    pub end_millis: u64,
    /// Segment and word times are absolute original seconds.
    pub transcript: TranscriptPayload,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alignment: Option<crate::AlignmentPayload>,
}

fn rejected(message: &str) -> CoreError {
    CoreError::new(CoreErrorKind::InferenceRejected, message)
}

impl SelectionTranscript {
    /// # Errors
    /// Rejects malformed or unprovenanced evidence before storage or editing.
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.schema_version != 1
            || self.end_millis <= self.start_millis
            || self.end_millis - self.start_millis > MAX_SELECTION_TRANSCRIPTION_MILLIS
            || self.transcript.runtime.as_ref().is_none_or(|r| {
                r.job.app_id != "echo"
                    || r.job.intent != "audio.transcribe"
                    || r.job.state != "succeeded"
                    || r.job.model_build.is_empty()
                    || r.job.physical_model.is_empty()
            })
            || self.transcript.segments.len() > 10_000
        {
            return Err(rejected("invalid selection transcript"));
        }
        let start = std::time::Duration::from_millis(self.start_millis).as_secs_f64();
        let end = std::time::Duration::from_millis(self.end_millis).as_secs_f64();
        let valid =
            |a: f64, b: f64| a.is_finite() && b.is_finite() && a >= start && b > a && b <= end;
        let valid_unit =
            |a: f64, b: f64| a.is_finite() && b.is_finite() && a >= start && b >= a && b <= end;
        for segment in &self.transcript.segments {
            if !valid(segment.start, segment.end)
                || segment.words.as_ref().is_some_and(|words| {
                    words.iter().any(|word| {
                        !valid_unit(word.start, word.end)
                            || word.start < segment.start
                            || word.end > segment.end
                    })
                })
            {
                return Err(rejected("invalid selection transcript timing"));
            }
        }
        if let Some(alignment) = &self.alignment
            && (alignment.text.split_whitespace().collect::<String>()
                != self.transcript.text.split_whitespace().collect::<String>()
                || alignment.runtime.job.intent != "audio.align"
                || alignment.runtime.job.app_id != "echo"
                || alignment.runtime.job.state != "succeeded"
                || alignment.runtime.job.physical_model.is_empty()
                || alignment.runtime.job.model_build.is_empty()
                || alignment.items.is_empty()
                || alignment.items.len() > 20000
                || alignment
                    .items
                    .iter()
                    .any(|item| !valid_unit(item.start, item.end))
                || alignment
                    .items
                    .windows(2)
                    .any(|pair| pair[1].start < pair[0].start))
        {
            return Err(rejected("invalid selection alignment"));
        }
        Ok(())
    }
}

struct TemporaryProxy(PathBuf);
impl Drop for TemporaryProxy {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Runs exactly one source range through the existing local-only Infer consumer.
/// # Errors
/// Rejects invalid ranges, changed sources, unavailable Runtime or malformed timing.
pub fn transcribe_selection(
    catalog: &Catalog,
    cache_root: &Path,
    id: AssetId,
    start: u64,
    end: u64,
    config: InferRuntimeConfig,
) -> Result<SelectionTranscript, CoreError> {
    let AssetLookup::Found(asset) = catalog.with_transaction(|tx| find_by_id(tx, id))? else {
        return Err(rejected("source not found"));
    };
    if end <= start
        || end - start > MAX_SELECTION_TRANSCRIPTION_MILLIS
        || end > asset.original.duration_millis.unwrap_or(0)
    {
        return Err(rejected(
            "select between one millisecond and five minutes of original audio",
        ));
    }
    if hash_file(&asset.original.path)? != asset.original.content_hash {
        return Err(rejected("original source changed"));
    }
    let root = cache_root.join("work").join("selection-transcription");
    std::fs::create_dir_all(&root)
        .map_err(|e| CoreError::new(CoreErrorKind::SourceUnavailable, e.to_string()))?;
    let temporary = TemporaryProxy(root.join(format!("{}.wav", AssetId::new())));
    echo_bridge::build_analysis_proxy(&asset.original.path, &temporary.0, start, end)
        .map_err(|e| CoreError::new(CoreErrorKind::AudioEngineRejected, e.message))?;
    if hash_file(&asset.original.path)? != asset.original.content_hash {
        return Err(rejected("original source changed"));
    }
    let client = InferRuntimeClient::new(config);
    let mut transcript = client
        .transcribe(&temporary.0, &TranscriptionIntent::default())
        .map_err(|e| CoreError::new(CoreErrorKind::InferenceRejected, e.to_string()))?;
    if hash_file(&asset.original.path)? != asset.original.content_hash {
        return Err(rejected("original source changed"));
    }
    let offset = std::time::Duration::from_millis(start).as_secs_f64();
    for segment in &mut transcript.segments {
        segment.start += offset;
        segment.end += offset;
        if let Some(words) = &mut segment.words {
            for word in words {
                word.start += offset;
                word.end += offset;
            }
        }
    }
    let mut alignment = if transcript.text.trim().is_empty() {
        None
    } else {
        client
            .align(
                &temporary.0,
                &transcript.text,
                &crate::AlignmentIntent {
                    language: transcript.language.clone(),
                    ..Default::default()
                },
            )
            .ok()
    };
    if let Some(evidence) = &mut alignment {
        for item in &mut evidence.items {
            item.start += offset;
            item.end += offset;
        }
    }
    let mut result = SelectionTranscript {
        schema_version: 1,
        source_hash: asset.original.content_hash,
        start_millis: start,
        end_millis: end,
        transcript,
        alignment: None,
    };
    result.validate()?;
    result.alignment = alignment;
    // ASR evidence remains usable if optional alignment is missing or invalid.
    if result.validate().is_err() {
        result.alignment = None;
    }
    if hash_file(&asset.original.path)? != asset.original.content_hash {
        return Err(rejected("original source changed"));
    }
    Ok(result)
}

/// Accepts completed evidence after the presentation has checked its current request.
/// # Errors
/// Rejects a changed asset identity or an invalid result; does not enqueue any jobs.
pub fn record_selection_transcript(
    catalog: &Catalog,
    id: AssetId,
    result: &SelectionTranscript,
) -> Result<(), CoreError> {
    result.validate()?;
    let runtime = result
        .transcript
        .runtime
        .as_ref()
        .ok_or_else(|| rejected("missing provenance"))?;
    catalog.with_transaction(|tx| -> Result<(), CoreError> {
        let AssetLookup::Found(asset) = find_by_id(tx, id)? else {
            return Err(rejected("source not found"));
        };
        if asset.original.content_hash != result.source_hash
            || result.end_millis > asset.original.duration_millis.unwrap_or(0)
        {
            return Err(rejected("selection source no longer matches"));
        }
        record_analysis(
            tx,
            &AppendAnalysisRecord {
                asset_id: id,
                record: AnalysisRecord::new(
                    AnalysisKind::SelectionTranscript,
                    serde_json::to_value(result).map_err(|e| rejected(&e.to_string()))?,
                    ModelIdentity::new(
                        runtime.job.physical_model.clone(),
                        runtime.job.model_build.clone(),
                    ),
                    None,
                    crate::util::now_millis(),
                ),
            },
        )?;
        Ok(())
    })
}

#[cfg(test)]
mod tests;
