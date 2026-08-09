//! Authoritative construction and identity checks for the current development
//! Catalog schema.
//!
//! Echo has no migration chain before its first compatibility promise. This
//! module creates the current dated revision atomically and rejects every
//! other persisted shape. Revisions use the canonical `YYYYMMDD.N` form.

use std::str::FromStr;

/// One dated Catalog contract revision: calendar date plus that day's
/// positive sequence number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CatalogSchemaRevision {
    date: u32,
    daily_sequence: u16,
}

impl CatalogSchemaRevision {
    /// Creates a canonical dated revision.
    #[must_use]
    pub(crate) const fn new(date: u32, daily_sequence: u16) -> Self {
        Self {
            date,
            daily_sequence,
        }
    }

    /// Eight-digit calendar identity in `YYYYMMDD` form.
    #[must_use]
    pub const fn date(self) -> u32 {
        self.date
    }

    /// Positive revision sequence within the date.
    #[must_use]
    pub const fn daily_sequence(self) -> u16 {
        self.daily_sequence
    }
}

impl std::fmt::Display for CatalogSchemaRevision {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{:08}.{}", self.date, self.daily_sequence)
    }
}

/// A persisted schema revision that is not canonical `YYYYMMDD.N`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("schema revision must use canonical YYYYMMDD.N form")]
pub struct CatalogSchemaRevisionParseError;

impl FromStr for CatalogSchemaRevision {
    type Err = CatalogSchemaRevisionParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (date, sequence) = text
            .split_once('.')
            .ok_or(CatalogSchemaRevisionParseError)?;
        if date.len() != 8
            || !date.bytes().all(|byte| byte.is_ascii_digit())
            || sequence.is_empty()
            || !sequence.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(CatalogSchemaRevisionParseError);
        }
        let revision = Self::new(
            date.parse().map_err(|_| CatalogSchemaRevisionParseError)?,
            sequence
                .parse()
                .map_err(|_| CatalogSchemaRevisionParseError)?,
        );
        if !valid_calendar_date(revision.date)
            || revision.daily_sequence == 0
            || revision.to_string() != text
        {
            return Err(CatalogSchemaRevisionParseError);
        }
        Ok(revision)
    }
}

fn valid_calendar_date(date: u32) -> bool {
    let year = date / 10_000;
    let month = (date / 100) % 100;
    let day = date % 100;
    if year == 0 || !(1..=12).contains(&month) {
        return false;
    }
    let leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days_in_month = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days_in_month).contains(&day)
}

pub(crate) const SCHEMA_VERSION: CatalogSchemaRevision = CatalogSchemaRevision::new(20_260_809, 2);

pub(crate) const SCHEMA_IDENTITY: &str =
    "echo-catalog-20260809.2-derived-artifacts-canonical-revision";

pub(crate) const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS catalog_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS assets (
    id                 TEXT PRIMARY KEY,
    content_hash       TEXT NOT NULL UNIQUE,
    path               TEXT NOT NULL,
    path_status        TEXT NOT NULL DEFAULT 'present'
                       CHECK (path_status IN ('present', 'missing')),
    size_bytes         INTEGER NOT NULL,
    codec              TEXT,
    duration_millis    INTEGER,
    recorded_at_millis INTEGER,
    imported_at_millis INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS analysis_records (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    kind               TEXT NOT NULL,
    value              TEXT NOT NULL,
    model              TEXT NOT NULL,
    model_version      TEXT NOT NULL,
    confidence         REAL,
    recorded_at_millis INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS analysis_records_asset_kind
    ON analysis_records (asset_id, kind);

CREATE TABLE IF NOT EXISTS asset_levels (
    asset_id   TEXT PRIMARY KEY REFERENCES assets(id),
    max_level  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS derived_artifacts (
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    kind               TEXT NOT NULL,
    schema_version     INTEGER NOT NULL,
    content_hash       TEXT NOT NULL,
    size_bytes         INTEGER NOT NULL,
    created_at_millis  INTEGER NOT NULL,
    PRIMARY KEY (asset_id, kind, schema_version)
);

CREATE TABLE IF NOT EXISTS scan_roots (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    root             TEXT NOT NULL UNIQUE,
    enabled          INTEGER NOT NULL DEFAULT 1,
    added_at_millis  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS jobs (
    id                TEXT PRIMARY KEY,
    kind              TEXT NOT NULL,
    payload           TEXT NOT NULL,
    state             TEXT NOT NULL
                      CHECK (state IN ('pending', 'running', 'done', 'failed', 'cancelled')),
    progress          INTEGER NOT NULL DEFAULT 0,
    attempts          INTEGER NOT NULL DEFAULT 0,
    created_at_millis INTEGER NOT NULL,
    updated_at_millis INTEGER NOT NULL,
    error             TEXT
);

CREATE INDEX IF NOT EXISTS jobs_state_kind ON jobs (state, kind);

CREATE TABLE IF NOT EXISTS scan_journal (
    path            TEXT PRIMARY KEY,
    size_bytes      INTEGER NOT NULL,
    mtime_millis    INTEGER NOT NULL,
    content_hash    TEXT NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS transcript_fts USING fts5(
    asset_id UNINDEXED,
    text,
    tokenize = 'unicode61'
);
";

#[cfg(test)]
mod tests;
