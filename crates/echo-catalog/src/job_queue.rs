//! The persistent background job queue.
//!
//! Jobs are the durable unit of background work (folder scans, imports,
//! waveform analysis, transcription). State transitions commit atomically
//! with the catalog, so a crash simply leaves jobs in `running`; recovery
//! resets them to `pending` and work resumes.

use std::path::PathBuf;

use rusqlite::{OptionalExtension, Transaction};

use crate::error::CatalogError;

/// Job family; each kind owns its payload schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobKind {
    /// Walks a scan root, journals files, and queues imports.
    ScanRoot,
    /// Registers one discovered file (hash + probe).
    ImportFile,
    /// Builds and caches the waveform pyramid for an asset.
    AnalyzeWaveform,
    /// Submits audio transcription and records transcript evidence.
    Transcribe,
    /// Runs local LLM contextual understanding over a transcript.
    Contextual,
}

/// Job lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Pending,
    Running,
    Done,
    Failed,
    Cancelled,
}

/// One queued job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub id: String,
    pub kind: JobKind,
    pub payload: serde_json::Value,
    pub state: JobState,
    pub progress: u8,
    pub attempts: u32,
    pub created_at_millis: i64,
    pub updated_at_millis: i64,
    pub error: Option<String>,
}

/// A job claimed for execution (was `pending`, now `running`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimedJob {
    pub id: String,
    pub kind: JobKind,
    pub payload: serde_json::Value,
}

/// Aggregate queue statistics for progress display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct JobStats {
    pub pending: u64,
    pub running: u64,
    pub done: u64,
    pub failed: u64,
}

/// Enqueues one job.
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn enqueue_job(
    transaction: &Transaction<'_>,
    id: &str,
    kind: JobKind,
    payload: &serde_json::Value,
    now_millis: i64,
) -> Result<(), CatalogError> {
    transaction.execute(
        "INSERT INTO jobs (id, kind, payload, state, progress, attempts, created_at_millis, \
         updated_at_millis) VALUES (?1, ?2, ?3, 'pending', 0, 0, ?4, ?4) \
         ON CONFLICT(id) DO NOTHING",
        rusqlite::params![id, kind_text(kind), payload.to_string(), now_millis],
    )?;
    Ok(())
}

/// Queues (or re-queues) a scan job for one root. Unlike ordinary jobs, a
/// scan re-runs on every startup: the journal makes it cheap, and it is what
/// powers silent incremental detection. An already-running scan is left alone.
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn requeue_scan_job(
    transaction: &Transaction<'_>,
    id: &str,
    payload: &serde_json::Value,
    now_millis: i64,
) -> Result<(), CatalogError> {
    transaction.execute(
        "INSERT INTO jobs (id, kind, payload, state, progress, attempts, created_at_millis, \
         updated_at_millis) VALUES (?1, 'scan_root', ?2, 'pending', 0, 0, ?3, ?3) \
         ON CONFLICT(id) DO UPDATE SET state = 'pending', error = NULL, \
         updated_at_millis = excluded.updated_at_millis WHERE jobs.state != 'running'",
        rusqlite::params![id, payload.to_string(), now_millis],
    )?;
    Ok(())
}

/// Atomically claims the oldest pending job (pending -> running).
///
/// # Errors
///
/// Returns a catalog failure when the read or write cannot be applied.
pub fn claim_next_job(
    transaction: &Transaction<'_>,
    now_millis: i64,
) -> Result<Option<ClaimedJob>, CatalogError> {
    let candidate: Option<(String, String, String, u32)> = transaction
        .query_row(
            "SELECT id, kind, payload, attempts FROM jobs WHERE state = 'pending' \
             ORDER BY created_at_millis ASC, id ASC LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()?;
    let Some((id, kind, payload, attempts)) = candidate else {
        return Ok(None);
    };
    transaction.execute(
        "UPDATE jobs SET state = 'running', attempts = ?2, updated_at_millis = ?3 WHERE id = ?1",
        rusqlite::params![id.as_str(), attempts + 1, now_millis],
    )?;
    let payload = serde_json::from_str(&payload).map_err(|error| {
        CatalogError::new(
            crate::error::CatalogErrorKind::Other,
            format!("corrupt job payload for {id}: {error}"),
        )
    })?;
    Ok(Some(ClaimedJob {
        id,
        kind: parse_kind(&kind)?,
        payload,
    }))
}

/// Marks a claimed job done.
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn complete_job(
    transaction: &Transaction<'_>,
    id: &str,
    now_millis: i64,
) -> Result<(), CatalogError> {
    transaction.execute(
        "UPDATE jobs SET state = 'done', progress = 100, updated_at_millis = ?2 WHERE id = ?1",
        rusqlite::params![id, now_millis],
    )?;
    Ok(())
}

/// Marks a claimed job failed with an error message.
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn fail_job(
    transaction: &Transaction<'_>,
    id: &str,
    error: &str,
    now_millis: i64,
) -> Result<(), CatalogError> {
    transaction.execute(
        "UPDATE jobs SET state = 'failed', error = ?2, updated_at_millis = ?3 WHERE id = ?1",
        rusqlite::params![id, error, now_millis],
    )?;
    Ok(())
}

/// Updates the progress of a running job.
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn update_job_progress(
    transaction: &Transaction<'_>,
    id: &str,
    progress: u8,
    now_millis: i64,
) -> Result<(), CatalogError> {
    transaction.execute(
        "UPDATE jobs SET progress = ?2, updated_at_millis = ?3 WHERE id = ?1",
        rusqlite::params![id, progress, now_millis],
    )?;
    Ok(())
}

/// Resets interrupted jobs after a crash (running -> pending).
///
/// # Panics
///
/// Panics when the affected row count exceeds `u64`.
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn recover_interrupted_jobs(
    transaction: &Transaction<'_>,
    now_millis: i64,
) -> Result<u64, CatalogError> {
    let affected = transaction.execute(
        "UPDATE jobs SET state = 'pending', updated_at_millis = ?1 WHERE state = 'running'",
        [now_millis],
    )?;
    Ok(u64::try_from(affected).expect("affected row count fits u64"))
}

/// Reads queue statistics.
///
/// # Panics
///
/// Panics when a count exceeds `u64`.
///
/// # Errors
///
/// Returns a catalog failure when the read cannot be applied.
pub fn job_stats(transaction: &Transaction<'_>) -> Result<JobStats, CatalogError> {
    let mut statement = transaction.prepare("SELECT state, COUNT(*) FROM jobs GROUP BY state")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })?;
    let mut stats = JobStats::default();
    for row in rows {
        let (kind, count) = row?;
        let count = u64::try_from(count).expect("count fits u64");
        match kind.as_str() {
            "pending" => stats.pending = count,
            "running" => stats.running = count,
            "done" => stats.done = count,
            "failed" => stats.failed = count,
            _ => {}
        }
    }
    Ok(stats)
}

/// Reads all failed jobs (for retry/display).
///
/// # Errors
///
/// Returns a catalog failure when the read cannot be applied.
pub fn list_failed_jobs(transaction: &Transaction<'_>) -> Result<Vec<Job>, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT id, kind, payload, state, progress, attempts, created_at_millis, \
         updated_at_millis, error FROM jobs WHERE state = 'failed' \
         ORDER BY updated_at_millis DESC LIMIT 50",
    )?;
    let rows = statement.query_map([], |row| {
        let kind = parse_kind(&row.get::<_, String>(1)?).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                1,
                rusqlite::types::Type::Text,
                error.message.into(),
            )
        })?;
        Ok(Job {
            id: row.get(0)?,
            kind,
            payload: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or_default(),
            state: JobState::Failed,
            progress: row.get(4)?,
            attempts: row.get(5)?,
            created_at_millis: row.get(6)?,
            updated_at_millis: row.get(7)?,
            error: row.get(8)?,
        })
    })?;
    let mut jobs = Vec::new();
    for row in rows {
        jobs.push(row?);
    }
    Ok(jobs)
}

/// The on-disk payload helper: a job targeting one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileJobPayload {
    pub path: PathBuf,
}

impl FileJobPayload {
    /// Encodes the payload.
    #[must_use]
    pub fn encode(&self) -> serde_json::Value {
        serde_json::json!({ "path": self.path.to_string_lossy() })
    }

    /// Decodes the payload.
    ///
    /// # Errors
    ///
    /// Returns a catalog failure when the payload lacks a path.
    pub fn decode(value: &serde_json::Value) -> Result<Self, CatalogError> {
        let path = value
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                CatalogError::new(
                    crate::error::CatalogErrorKind::Other,
                    "file job payload lacks a path",
                )
            })?;
        Ok(Self {
            path: PathBuf::from(path),
        })
    }
}

/// The on-disk payload helper: a job targeting one scan root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanRootJobPayload {
    pub root: PathBuf,
}

impl ScanRootJobPayload {
    /// Encodes the payload.
    #[must_use]
    pub fn encode(&self) -> serde_json::Value {
        serde_json::json!({ "root": self.root.to_string_lossy() })
    }

    /// Decodes the payload.
    ///
    /// # Errors
    ///
    /// Returns a catalog failure when the payload lacks a root.
    pub fn decode(value: &serde_json::Value) -> Result<Self, CatalogError> {
        let root = value
            .get("root")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                CatalogError::new(
                    crate::error::CatalogErrorKind::Other,
                    "scan job payload lacks a root",
                )
            })?;
        Ok(Self {
            root: PathBuf::from(root),
        })
    }
}

pub(crate) const fn kind_text(kind: JobKind) -> &'static str {
    match kind {
        JobKind::ScanRoot => "scan_root",
        JobKind::ImportFile => "import_file",
        JobKind::AnalyzeWaveform => "analyze_waveform",
        JobKind::Transcribe => "transcribe",
        JobKind::Contextual => "contextual",
    }
}

pub(crate) fn parse_kind(text: &str) -> Result<JobKind, CatalogError> {
    match text {
        "scan_root" => Ok(JobKind::ScanRoot),
        "import_file" => Ok(JobKind::ImportFile),
        "analyze_waveform" => Ok(JobKind::AnalyzeWaveform),
        "transcribe" => Ok(JobKind::Transcribe),
        "contextual" => Ok(JobKind::Contextual),
        other => Err(CatalogError::new(
            crate::error::CatalogErrorKind::Other,
            format!("unknown job kind {other}"),
        )),
    }
}

#[cfg(test)]
mod tests;
