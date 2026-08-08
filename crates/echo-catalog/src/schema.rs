//! Authoritative construction and identity checks for the current development
//! Catalog schema.
//!
//! Echo has no migration chain before its first compatibility promise. This
//! module creates the current dated revision atomically and rejects every
//! other persisted shape. Revisions follow Shadow's contract: `YYYYMMDDNN`.

pub(crate) const SCHEMA_VERSION: i64 = 2_026_080_802;

pub(crate) const SCHEMA_IDENTITY: &str = "echo-catalog-20260808.2-fts5-transcript-search";

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
