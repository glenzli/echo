//! Resumable long-recording analysis orchestration.
//!
//! This owner plans bounded source ranges, builds one streaming proxy at a
//! time, serially submits leaf inference, and recursively reduces leaf
//! contextual evidence. Runtime routing and proxy DSP remain in their own
//! owners; this module only coordinates durable product stages.

use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use echo_catalog::{
    Catalog, ClaimedJob, LongAudioOutlineNode, LongAudioProxyRef, LongAudioSegment,
    LongAudioSegmentPlan, LongAudioStage, ensure_long_audio_plan, list_long_audio_outline_nodes,
    list_long_audio_segments, record_long_audio_proxy, record_long_audio_stage,
    update_job_progress, upsert_long_audio_outline_node,
};
use echo_domain::{AssetId, AudioAsset};
use serde::{Deserialize, Serialize};

use crate::{
    AlignmentPayload, ContextualIntent, ContextualPayload, CoreError, CoreErrorKind,
    InferRuntimeClient, RuntimeProvenance, TranscriptPayload, WorkerConfig,
};

pub const LONG_AUDIO_PLAN_VERSION: u32 = 1;
const LONG_AUDIO_THRESHOLD_MILLIS: u64 = 15 * 60 * 1000;
const LEAF_DURATION_MILLIS: u64 = 8 * 60 * 1000;
const OUTLINE_FAN_OUT: usize = 6;
const CONTEXTUAL_VALIDATION_ATTEMPTS: usize = 3;
const CONTEXTUAL_RETRY_INSTRUCTION: &str = "\nThis is a strict JSON retry. Return only the one JSON object described above: no markdown fence, preface, explanation, or trailing text.";
const CONTEXTUAL_OUTPUT_TOKEN_BUDGETS: [u32; CONTEXTUAL_VALIDATION_ATTEMPTS] = [384, 768, 1_024];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ContextualEvidence {
    payload: ContextualPayload,
    runtime: RuntimeProvenance,
}

#[derive(Debug, Clone)]
struct OutlineValue {
    start_millis: u64,
    end_millis: u64,
    evidence: ContextualEvidence,
}

pub(crate) fn requires_segmentation(asset: &AudioAsset) -> bool {
    asset.original.size_bytes > crate::MAX_AUDIO_UPLOAD_BYTES
        || asset
            .original
            .duration_millis
            .is_some_and(|duration| duration > LONG_AUDIO_THRESHOLD_MILLIS)
}

pub(crate) fn dispatch_long_audio(
    catalog: &Catalog,
    config: &WorkerConfig,
    job: &ClaimedJob,
    asset: &AudioAsset,
) -> Result<(), CoreError> {
    let duration = asset.original.duration_millis.ok_or_else(|| {
        CoreError::new(
            CoreErrorKind::AudioEngineRejected,
            "long recording has no probed duration",
        )
    })?;
    let plan = plan_segments(duration);
    catalog.with_transaction(|transaction| {
        ensure_long_audio_plan(
            transaction,
            asset.id,
            LONG_AUDIO_PLAN_VERSION,
            &plan,
            crate::util::now_millis(),
        )
        .map_err(CoreError::from)
    })?;
    let store =
        echo_cache::open_blob_store(&config.cache_root).map_err(|error| cache_error(&error))?;
    let client = InferRuntimeClient::new(config.infer_runtime.clone());
    let mut segments = catalog.with_transaction(|transaction| {
        list_long_audio_segments(transaction, asset.id, LONG_AUDIO_PLAN_VERSION)
            .map_err(CoreError::from)
    })?;
    let mut last_transcription_runtime = None;
    let segment_count = segments.len();
    for segment in &mut segments {
        let transcript = process_leaf(
            catalog,
            config,
            job,
            asset,
            &store,
            &client,
            segment,
            segment_count,
        )?;
        last_transcription_runtime = transcript.runtime.clone().or(last_transcription_runtime);
    }

    let aggregate = aggregate_transcript(&segments)?;
    crate::analysis::record_aggregate_transcript(
        catalog,
        asset.id,
        &aggregate,
        LONG_AUDIO_PLAN_VERSION,
    )?;
    if !aggregate.text.trim().is_empty() {
        crate::analysis::record_aggregate_alignment(
            catalog,
            asset.id,
            &aggregate_alignment(&segments, &aggregate)?,
            LONG_AUDIO_PLAN_VERSION,
        )?;
        if let Some(root) = aggregate_outline(catalog, &client, asset.id, &segments)? {
            crate::record_contextual(catalog, asset.id, &root.payload, &root.runtime)?;
        }
    }
    if let Some(runtime) = last_transcription_runtime {
        super::worker::record_inference_success(catalog, job, asset.id, &runtime)?;
    }
    catalog.with_transaction(|transaction| {
        update_job_progress(transaction, &job.id, 99, crate::util::now_millis())
            .map_err(CoreError::from)
    })
}

#[allow(clippy::too_many_arguments)]
fn process_leaf(
    catalog: &Catalog,
    config: &WorkerConfig,
    job: &ClaimedJob,
    asset: &AudioAsset,
    store: &echo_cache::BlobStore,
    client: &InferRuntimeClient,
    segment: &mut LongAudioSegment,
    segment_count: usize,
) -> Result<TranscriptPayload, CoreError> {
    let proxy_path = ensure_proxy(catalog, store, asset, segment, &config.cache_root)?;
    update_progress(catalog, job, segment.index, 0, segment_count)?;
    let transcript = load_or_transcribe(catalog, client, asset.id, segment, &proxy_path)?;
    segment.transcript =
        Some(serde_json::to_value(&transcript).map_err(|error| json_error(&error))?);
    update_progress(catalog, job, segment.index, 1, segment_count)?;
    if transcript.text.trim().is_empty() {
        update_progress(catalog, job, segment.index, 3, segment_count)?;
        return Ok(transcript);
    }
    let alignment = load_or_align(catalog, client, asset.id, segment, &proxy_path, &transcript)?;
    segment.alignment = Some(serde_json::to_value(&alignment).map_err(|error| json_error(&error))?);
    update_progress(catalog, job, segment.index, 2, segment_count)?;
    let contextual = load_or_contextual(catalog, client, asset.id, segment, &transcript)?;
    segment.contextual =
        Some(serde_json::to_value(contextual).map_err(|error| json_error(&error))?);
    update_progress(catalog, job, segment.index, 3, segment_count)?;
    Ok(transcript)
}

fn load_or_transcribe(
    catalog: &Catalog,
    client: &InferRuntimeClient,
    asset_id: AssetId,
    segment: &LongAudioSegment,
    proxy_path: &std::path::Path,
) -> Result<TranscriptPayload, CoreError> {
    if let Some(value) = &segment.transcript {
        return decode_stage(value, "transcript");
    }
    let payload = client
        .transcribe(proxy_path, &crate::TranscriptionIntent::default())
        .map_err(CoreError::from)?;
    persist_stage(
        catalog,
        asset_id,
        segment.index,
        LongAudioStage::Transcript,
        &payload,
    )?;
    Ok(payload)
}

fn load_or_align(
    catalog: &Catalog,
    client: &InferRuntimeClient,
    asset_id: AssetId,
    segment: &LongAudioSegment,
    proxy_path: &std::path::Path,
    transcript: &TranscriptPayload,
) -> Result<AlignmentPayload, CoreError> {
    if let Some(value) = &segment.alignment {
        return decode_stage(value, "alignment");
    }
    let payload = client
        .align(
            proxy_path,
            &transcript.text,
            &crate::AlignmentIntent {
                language: transcript.language.clone(),
                ..crate::AlignmentIntent::default()
            },
        )
        .map_err(CoreError::from)?;
    persist_stage(
        catalog,
        asset_id,
        segment.index,
        LongAudioStage::Alignment,
        &payload,
    )?;
    Ok(payload)
}

fn load_or_contextual(
    catalog: &Catalog,
    client: &InferRuntimeClient,
    asset_id: AssetId,
    segment: &LongAudioSegment,
    transcript: &TranscriptPayload,
) -> Result<ContextualEvidence, CoreError> {
    if let Some(value) = &segment.contextual {
        return decode_stage(value, "contextual");
    }
    let evidence = contextualize_validated(client, &transcript.text, false)?;
    persist_stage(
        catalog,
        asset_id,
        segment.index,
        LongAudioStage::Contextual,
        &evidence,
    )?;
    Ok(evidence)
}

fn plan_segments(duration_millis: u64) -> Vec<LongAudioSegmentPlan> {
    let mut segments = Vec::new();
    let mut start = 0u64;
    while start < duration_millis {
        let end = start
            .saturating_add(LEAF_DURATION_MILLIS)
            .min(duration_millis);
        segments.push(LongAudioSegmentPlan {
            index: u32::try_from(segments.len()).expect("segment count fits u32"),
            start_millis: start,
            end_millis: end,
        });
        start = end;
    }
    segments
}

fn ensure_proxy(
    catalog: &Catalog,
    store: &echo_cache::BlobStore,
    asset: &AudioAsset,
    segment: &mut LongAudioSegment,
    cache_root: &std::path::Path,
) -> Result<PathBuf, CoreError> {
    if let Some(proxy) = &segment.proxy {
        if let Ok(path) = echo_cache::verify_blob(store, proxy.content_hash, proxy.size_bytes) {
            return Ok(path);
        }
        let _ = echo_cache::quarantine_corrupt(store, proxy.content_hash, proxy.size_bytes);
    }
    let work_root = cache_root.join("work").join("analysis-proxy");
    std::fs::create_dir_all(&work_root).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot create proxy work directory: {error}"),
        )
    })?;
    let temporary = work_root.join(format!(
        "{}-{}-{}.wav",
        asset.id,
        segment.index,
        std::process::id()
    ));
    let build = echo_bridge::build_analysis_proxy(
        &asset.original.path,
        &temporary,
        segment.start_millis,
        segment.end_millis,
    )
    .map_err(|error| CoreError::new(CoreErrorKind::AudioEngineRejected, error.message));
    let metadata = match build {
        Ok(metadata) => metadata,
        Err(error) => {
            let _ = std::fs::remove_file(&temporary);
            return Err(error);
        }
    };
    if metadata.size_bytes > crate::MAX_AUDIO_UPLOAD_BYTES {
        let _ = std::fs::remove_file(&temporary);
        return Err(CoreError::new(
            CoreErrorKind::InferenceRejected,
            "analysis proxy exceeds Runtime upload limit",
        ));
    }
    let (cached, _) = echo_cache::put_file(store, echo_cache::BlobRole::RenderProxy, &temporary)
        .map_err(|error| cache_error(&error))?;
    let _ = std::fs::remove_file(&temporary);
    let proxy = LongAudioProxyRef {
        content_hash: cached.content_hash,
        size_bytes: cached.size_bytes,
    };
    catalog.with_transaction(|transaction| {
        record_long_audio_proxy(
            transaction,
            asset.id,
            LONG_AUDIO_PLAN_VERSION,
            segment.index,
            &proxy,
            crate::util::now_millis(),
        )
        .map_err(CoreError::from)
    })?;
    segment.proxy = Some(proxy.clone());
    Ok(echo_cache::blob_path(cache_root, &proxy.content_hash))
}

fn persist_stage<T: Serialize>(
    catalog: &Catalog,
    asset_id: AssetId,
    segment_index: u32,
    stage: LongAudioStage,
    payload: &T,
) -> Result<(), CoreError> {
    let value = serde_json::to_value(payload).map_err(|error| json_error(&error))?;
    catalog.with_transaction(|transaction| {
        record_long_audio_stage(
            transaction,
            asset_id,
            LONG_AUDIO_PLAN_VERSION,
            segment_index,
            stage,
            &value,
            crate::util::now_millis(),
        )
        .map_err(CoreError::from)
    })
}

fn aggregate_transcript(segments: &[LongAudioSegment]) -> Result<TranscriptPayload, CoreError> {
    let mut text_parts = Vec::new();
    let mut aggregate_segments = Vec::new();
    let mut language = None;
    for segment in segments {
        let Some(value) = &segment.transcript else {
            return Err(CoreError::new(
                CoreErrorKind::InferenceRejected,
                "long-audio segment lacks transcript",
            ));
        };
        let transcript = decode_stage::<TranscriptPayload>(value, "transcript")?;
        if !transcript.text.trim().is_empty() {
            text_parts.push(transcript.text.trim().to_owned());
        }
        if language.is_none() {
            language.clone_from(&transcript.language);
        }
        let offset = Duration::from_millis(segment.start_millis).as_secs_f64();
        for mut item in transcript.segments {
            item.start += offset;
            item.end += offset;
            if let Some(words) = &mut item.words {
                for word in words {
                    word.start += offset;
                    word.end += offset;
                }
            }
            aggregate_segments.push(item);
        }
    }
    Ok(TranscriptPayload {
        model: "echo.long-audio.aggregate".to_owned(),
        language,
        text: text_parts.join("\n"),
        segments: aggregate_segments,
        runtime: None,
    })
}

fn aggregate_alignment(
    segments: &[LongAudioSegment],
    transcript: &TranscriptPayload,
) -> Result<serde_json::Value, CoreError> {
    let mut items = Vec::new();
    let mut runtime_jobs = Vec::new();
    for segment in segments {
        let Some(value) = &segment.alignment else {
            continue;
        };
        let alignment = decode_stage::<AlignmentPayload>(value, "alignment")?;
        let offset = Duration::from_millis(segment.start_millis).as_secs_f64();
        items.extend(alignment.items.into_iter().map(|mut item| {
            item.start += offset;
            item.end += offset;
            item
        }));
        runtime_jobs.push(alignment.runtime.job.id);
    }
    Ok(serde_json::json!({
        "model": "echo.long-audio.aggregate",
        "text": transcript.text,
        "language": transcript.language,
        "items": items,
        "runtime_job_ids": runtime_jobs,
    }))
}

fn aggregate_outline(
    catalog: &Catalog,
    client: &InferRuntimeClient,
    asset_id: AssetId,
    segments: &[LongAudioSegment],
) -> Result<Option<ContextualEvidence>, CoreError> {
    let existing = catalog.with_transaction(|transaction| {
        list_long_audio_outline_nodes(transaction, asset_id, LONG_AUDIO_PLAN_VERSION)
            .map_err(CoreError::from)
    })?;
    let existing = existing
        .into_iter()
        .map(|node| ((node.level, node.index), node))
        .collect::<BTreeMap<_, _>>();
    let mut current = Vec::new();
    for segment in segments {
        let Some(value) = &segment.contextual else {
            continue;
        };
        let evidence = decode_stage::<ContextualEvidence>(value, "contextual")?;
        let outline = OutlineValue {
            start_millis: segment.start_millis,
            end_millis: segment.end_millis,
            evidence,
        };
        persist_outline(catalog, asset_id, 0, segment.index, &outline)?;
        current.push(outline);
    }
    if current.is_empty() {
        return Ok(None);
    }
    let mut level = 1u32;
    while current.len() > 1 {
        let mut next = Vec::new();
        for (index, group) in current.chunks(OUTLINE_FAN_OUT).enumerate() {
            let index = u32::try_from(index).expect("outline index fits u32");
            let start_millis = group.first().expect("group is nonempty").start_millis;
            let end_millis = group.last().expect("group is nonempty").end_millis;
            let value = if group.len() == 1 {
                group[0].clone()
            } else if let Some(node) = existing
                .get(&(level, index))
                .filter(|node| node.start_millis == start_millis && node.end_millis == end_millis)
            {
                OutlineValue {
                    start_millis,
                    end_millis,
                    evidence: decode_stage::<ContextualEvidence>(
                        &node.contextual,
                        "outline contextual",
                    )?,
                }
            } else {
                let input = group
                    .iter()
                    .map(|child| {
                        let payload = &child.evidence.payload;
                        if payload.summary.trim().is_empty() {
                            payload.sound_caption.clone()
                        } else {
                            format!("{} — {}", payload.sound_caption, payload.summary)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                OutlineValue {
                    start_millis,
                    end_millis,
                    evidence: contextualize_validated(client, &input, true)?,
                }
            };
            persist_outline(catalog, asset_id, level, index, &value)?;
            next.push(value);
        }
        current = next;
        level += 1;
    }
    Ok(current.pop().map(|value| value.evidence))
}

fn contextualize_validated(
    client: &InferRuntimeClient,
    input: &str,
    allow_child_caption: bool,
) -> Result<ContextualEvidence, CoreError> {
    let mut validation_error = None;
    for (attempt, max_output_tokens) in CONTEXTUAL_OUTPUT_TOKEN_BUDGETS.into_iter().enumerate() {
        let mut intent = ContextualIntent {
            max_output_tokens,
            ..ContextualIntent::default()
        };
        if attempt > 0 {
            intent.instructions.push_str(CONTEXTUAL_RETRY_INSTRUCTION);
        }
        let response = client
            .contextualize(input, &intent)
            .map_err(CoreError::from)?;
        let decoded = if allow_child_caption {
            crate::contextual::decode_contextual_outline_output(&response.output_text, input)
        } else {
            crate::contextual::decode_contextual_output(&response.output_text, input)
        };
        match decoded {
            Ok(payload) => {
                return Ok(ContextualEvidence {
                    payload,
                    runtime: response.runtime,
                });
            }
            Err(error) => validation_error = Some(error),
        }
    }
    let error = validation_error.expect("at least one contextual validation attempt");
    Err(CoreError::new(
        CoreErrorKind::InferenceRejected,
        error.to_string(),
    ))
}

fn persist_outline(
    catalog: &Catalog,
    asset_id: AssetId,
    level: u32,
    index: u32,
    value: &OutlineValue,
) -> Result<(), CoreError> {
    let contextual = serde_json::to_value(&value.evidence).map_err(|error| json_error(&error))?;
    catalog.with_transaction(|transaction| {
        upsert_long_audio_outline_node(
            transaction,
            &LongAudioOutlineNode {
                asset_id,
                plan_version: LONG_AUDIO_PLAN_VERSION,
                level,
                index,
                start_millis: value.start_millis,
                end_millis: value.end_millis,
                contextual,
                updated_at_millis: crate::util::now_millis(),
            },
        )
        .map_err(CoreError::from)
    })
}

fn update_progress(
    catalog: &Catalog,
    job: &ClaimedJob,
    segment_index: u32,
    stage: u32,
    segment_count: usize,
) -> Result<(), CoreError> {
    let completed = usize::try_from(segment_index).expect("segment index fits usize") * 4
        + usize::try_from(stage).expect("stage fits usize");
    let total = segment_count.saturating_mul(4).saturating_add(1);
    let progress =
        u8::try_from((completed.saturating_mul(98) / total).min(98)).expect("progress fits u8");
    catalog.with_transaction(|transaction| {
        update_job_progress(transaction, &job.id, progress, crate::util::now_millis())
            .map_err(CoreError::from)
    })
}

fn decode_stage<T: serde::de::DeserializeOwned>(
    value: &serde_json::Value,
    label: &str,
) -> Result<T, CoreError> {
    serde_json::from_value(value.clone()).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("stored long-audio {label} is invalid: {error}"),
        )
    })
}

fn json_error(error: &serde_json::Error) -> CoreError {
    CoreError::new(
        CoreErrorKind::Other,
        format!("cannot encode long-audio evidence: {error}"),
    )
}

fn cache_error(error: &echo_cache::CacheError) -> CoreError {
    CoreError::new(CoreErrorKind::Other, error.to_string())
}

#[cfg(test)]
mod tests;
