//! Durable product projection and recovery policy for background analysis.
//!
//! Accepted evidence selects the next stage. The Job and Inference Run rows
//! then describe execution state and a stable recovery disposition. UI and
//! worker orchestration consume this owner instead of treating absent text as
//! an analysis failure.

use std::str::FromStr;

use echo_domain::AssetId;
use rusqlite::Transaction;

use crate::{CatalogError, CatalogErrorKind, JobState, retry_job};

/// Product stage currently expected for one asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisStage {
    Text,
    SoundEvents,
    Alignment,
    Contextual,
    LongAudio,
    Complete,
}

impl AnalysisStage {
    /// Stable desktop wire value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::SoundEvents => "sound_events",
            Self::Alignment => "alignment",
            Self::Contextual => "contextual",
            Self::LongAudio => "long_audio",
            Self::Complete => "complete",
        }
    }
}

/// Product action associated with an incomplete or failed stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisRecoveryMode {
    None,
    Automatic,
    Manual,
    Source,
}

impl AnalysisRecoveryMode {
    /// Stable desktop wire value.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Automatic => "automatic",
            Self::Manual => "manual",
            Self::Source => "source",
        }
    }
}

/// One lightweight, payload-free status row for Library presentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetAnalysisStatus {
    pub asset_id: AssetId,
    pub job_id: String,
    pub stage: AnalysisStage,
    pub state: Option<JobState>,
    pub progress: u8,
    pub attempts: u32,
    pub error_code: Option<String>,
    pub runtime_job_id: Option<String>,
    pub contract_version: String,
    pub recovery: AnalysisRecoveryMode,
}

impl AssetAnalysisStatus {
    /// Stable desktop state. Accepted complete evidence wins over stale jobs.
    #[must_use]
    pub const fn state_str(&self) -> &'static str {
        if matches!(self.stage, AnalysisStage::Complete) {
            return "done";
        }
        match self.state {
            Some(JobState::Pending) => "pending",
            Some(JobState::Running) => "running",
            Some(JobState::Done) => "done",
            Some(JobState::Failed) => "failed",
            Some(JobState::Cancelled) => "cancelled",
            None => "missing",
        }
    }
}

type StoredStatus = (
    String,
    String,
    String,
    Option<String>,
    Option<u8>,
    Option<u32>,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
);

/// Projects every asset's next analysis stage with one bounded SQL query.
///
/// # Errors
///
/// Returns a catalog failure when stored identities or job states are invalid.
pub fn list_asset_analysis_statuses(
    transaction: &Transaction<'_>,
    contextual_schema_version: u32,
    contextual_job_revision: u32,
    long_audio_plan_version: u32,
) -> Result<Vec<AssetAnalysisStatus>, CatalogError> {
    let mut statement = transaction.prepare(
        r"WITH evidence AS (
            SELECT a.id, a.path_status,
                EXISTS(SELECT 1 FROM analysis_records transcript
                    WHERE transcript.asset_id = a.id AND transcript.kind = 'transcript')
                    AS has_transcript,
                TRIM(COALESCE(json_extract((SELECT transcript.value
                    FROM analysis_records transcript WHERE transcript.asset_id = a.id
                    AND transcript.kind = 'transcript' ORDER BY transcript.id DESC LIMIT 1),
                    '$.text'), '')) = '' AS transcript_empty,
                EXISTS(SELECT 1 FROM analysis_records aligned
                    WHERE aligned.asset_id = a.id AND aligned.kind = 'alignment') AS has_alignment,
                EXISTS(SELECT 1 FROM analysis_records events
                    WHERE events.asset_id = a.id AND events.kind = 'audio_events') AS has_events,
                COALESCE(CAST(json_extract((SELECT contextual.value
                    FROM analysis_records contextual WHERE contextual.asset_id = a.id
                    AND contextual.kind = 'contextual' ORDER BY contextual.id DESC LIMIT 1),
                    '$.schema_version') AS INTEGER), 0) = ?1
                AND TRIM(COALESCE(json_extract((SELECT contextual.value
                    FROM analysis_records contextual WHERE contextual.asset_id = a.id
                    AND contextual.kind = 'contextual' ORDER BY contextual.id DESC LIMIT 1),
                    '$.sound_caption'), '')) <> '' AS has_contextual,
                EXISTS(SELECT 1 FROM long_audio_segments segment
                    WHERE segment.asset_id = a.id AND segment.plan_version = ?3) AS has_long_plan
            FROM assets a
        ), selected AS (
            SELECT id, path_status,
                CASE
                    WHEN has_transcript AND transcript_empty AND has_events THEN 'complete'
                    WHEN has_transcript AND transcript_empty THEN 'sound_events'
                    WHEN has_long_plan AND has_contextual THEN 'complete'
                    WHEN has_long_plan THEN 'long_audio'
                    WHEN has_contextual THEN 'complete'
                    WHEN has_alignment THEN 'contextual'
                    WHEN has_transcript THEN 'alignment'
                    ELSE 'text'
                END AS stage,
                CASE
                    WHEN has_transcript AND transcript_empty THEN 'detect-audio-events-v1-' || id
                    WHEN has_long_plan THEN 'transcribe-' || id
                    WHEN has_contextual OR has_alignment
                        THEN 'contextual-v' || ?1 || '-r' || ?2 || '-' || id
                    WHEN has_transcript THEN 'align-' || id
                    ELSE 'transcribe-' || id
                END AS job_id
            FROM evidence
        )
        SELECT selected.id, selected.stage, selected.job_id, jobs.state, jobs.progress,
               jobs.attempts, runs.error_code, runs.runtime_job_id, runs.contract_version,
               selected.path_status
        FROM selected
        LEFT JOIN jobs ON jobs.id = selected.job_id
        LEFT JOIN inference_runs runs ON runs.local_job_id = selected.job_id
        ORDER BY selected.id",
    )?;
    let rows = statement.query_map(
        rusqlite::params![
            i64::from(contextual_schema_version),
            i64::from(contextual_job_revision),
            i64::from(long_audio_plan_version),
        ],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
            ))
        },
    )?;
    rows.map(parse_status).collect()
}

/// Requeues only current failed stages whose stable error is transient.
///
/// # Errors
///
/// Returns a catalog failure when projection or updates fail.
pub fn requeue_automatic_analysis(
    transaction: &Transaction<'_>,
    contextual_schema_version: u32,
    contextual_job_revision: u32,
    long_audio_plan_version: u32,
    now_millis: i64,
) -> Result<u64, CatalogError> {
    requeue_matching(
        transaction,
        contextual_schema_version,
        contextual_job_revision,
        long_audio_plan_version,
        now_millis,
        AnalysisRecoveryMode::Automatic,
    )
}

/// Requeues current failed or cancelled stages that require a user decision.
/// Missing Originals remain blocked until relinked.
///
/// # Errors
///
/// Returns a catalog failure when projection or updates fail.
pub fn requeue_manual_analysis(
    transaction: &Transaction<'_>,
    contextual_schema_version: u32,
    contextual_job_revision: u32,
    long_audio_plan_version: u32,
    now_millis: i64,
) -> Result<u64, CatalogError> {
    requeue_matching(
        transaction,
        contextual_schema_version,
        contextual_job_revision,
        long_audio_plan_version,
        now_millis,
        AnalysisRecoveryMode::Manual,
    )
}

/// Counts current stages eligible for automatic Runtime recovery.
///
/// # Errors
///
/// Returns a catalog failure when the query fails.
pub fn automatic_analysis_recovery_count(
    transaction: &Transaction<'_>,
    contextual_schema_version: u32,
    contextual_job_revision: u32,
    long_audio_plan_version: u32,
) -> Result<u64, CatalogError> {
    let count = list_asset_analysis_statuses(
        transaction,
        contextual_schema_version,
        contextual_job_revision,
        long_audio_plan_version,
    )?
    .into_iter()
    .filter(|status| status.recovery == AnalysisRecoveryMode::Automatic)
    .count();
    u64::try_from(count).map_err(|error| {
        CatalogError::new(
            CatalogErrorKind::Other,
            format!("automatic analysis recovery count does not fit u64: {error}"),
        )
    })
}

/// Stable error-code classification shared by status and retry behavior.
#[must_use]
pub fn analysis_recovery_mode(error_code: Option<&str>) -> AnalysisRecoveryMode {
    match error_code {
        Some(
            "runtime_discovery_unavailable"
            | "runtime_unavailable"
            | "provider_unavailable"
            | "queue_full"
            | "app_queue_full"
            | "quota_exceeded"
            | "deadline_exceeded",
        ) => AnalysisRecoveryMode::Automatic,
        Some(_) | None => AnalysisRecoveryMode::Manual,
    }
}

fn requeue_matching(
    transaction: &Transaction<'_>,
    contextual_schema_version: u32,
    contextual_job_revision: u32,
    long_audio_plan_version: u32,
    now_millis: i64,
    recovery: AnalysisRecoveryMode,
) -> Result<u64, CatalogError> {
    let statuses = list_asset_analysis_statuses(
        transaction,
        contextual_schema_version,
        contextual_job_revision,
        long_audio_plan_version,
    )?;
    let mut count = 0_u64;
    for status in statuses {
        if status.recovery == recovery && retry_job(transaction, &status.job_id, now_millis)? {
            count += 1;
        }
    }
    Ok(count)
}

fn parse_status(row: rusqlite::Result<StoredStatus>) -> Result<AssetAnalysisStatus, CatalogError> {
    let (
        asset_id,
        stage_wire,
        job_id,
        queue_status_wire,
        progress,
        attempts,
        error_code,
        runtime_job_id,
        contract_version,
        path_status,
    ) = row?;
    let asset_id = AssetId::from_str(&asset_id).map_err(|error| {
        CatalogError::new(
            CatalogErrorKind::Other,
            format!("invalid stored analysis asset id {asset_id}: {error}"),
        )
    })?;
    let analysis_stage = parse_stage(&stage_wire)?;
    let job_state = queue_status_wire
        .as_deref()
        .map(parse_job_state)
        .transpose()?;
    let recovery = if matches!(analysis_stage, AnalysisStage::Complete) {
        AnalysisRecoveryMode::None
    } else if path_status == "missing" {
        AnalysisRecoveryMode::Source
    } else if matches!(job_state, Some(JobState::Failed | JobState::Cancelled)) {
        analysis_recovery_mode(error_code.as_deref())
    } else {
        AnalysisRecoveryMode::None
    };
    Ok(AssetAnalysisStatus {
        asset_id,
        job_id,
        stage: analysis_stage,
        state: job_state,
        progress: progress.unwrap_or(0),
        attempts: attempts.unwrap_or(0),
        error_code,
        runtime_job_id,
        contract_version: contract_version.unwrap_or_default(),
        recovery,
    })
}

fn parse_stage(stage: &str) -> Result<AnalysisStage, CatalogError> {
    match stage {
        "text" => Ok(AnalysisStage::Text),
        "sound_events" => Ok(AnalysisStage::SoundEvents),
        "alignment" => Ok(AnalysisStage::Alignment),
        "contextual" => Ok(AnalysisStage::Contextual),
        "long_audio" => Ok(AnalysisStage::LongAudio),
        "complete" => Ok(AnalysisStage::Complete),
        _ => Err(CatalogError::new(
            CatalogErrorKind::Other,
            format!("unknown analysis stage {stage}"),
        )),
    }
}

fn parse_job_state(state: &str) -> Result<JobState, CatalogError> {
    match state {
        "pending" => Ok(JobState::Pending),
        "running" => Ok(JobState::Running),
        "done" => Ok(JobState::Done),
        "failed" => Ok(JobState::Failed),
        "cancelled" => Ok(JobState::Cancelled),
        _ => Err(CatalogError::new(
            CatalogErrorKind::Other,
            format!("unknown analysis job state {state}"),
        )),
    }
}

#[cfg(test)]
mod tests;
