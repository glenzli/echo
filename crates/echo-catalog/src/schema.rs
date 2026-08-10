//! Authoritative construction and identity checks for the current development
//! Catalog schema.
//!
//! This module creates the current dated revision atomically and names the
//! immediately preceding compatible revision used by Catalog's focused
//! migration. Revisions use the canonical `YYYYMMDD.N` form.

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

pub(crate) const PREVIOUS_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 5);
pub(crate) const LEGACY_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 4);
pub(crate) const OLDER_COMPATIBLE_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 3);
pub(crate) const OLDEST_COMPATIBLE_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 2);
pub(crate) const ANCIENT_COMPATIBLE_SCHEMA_VERSION: CatalogSchemaRevision =
    CatalogSchemaRevision::new(20_260_811, 1);
pub(crate) const SCHEMA_VERSION: CatalogSchemaRevision = CatalogSchemaRevision::new(20_260_811, 6);

pub(crate) const SCHEMA_IDENTITY: &str = "echo-catalog-20260811.6-semantic-search";

pub(crate) const SEMANTIC_SEARCH_MIGRATION_SQL: &str = r"
CREATE TABLE IF NOT EXISTS semantic_documents (
    asset_id           TEXT PRIMARY KEY REFERENCES assets(id),
    source_revision    TEXT NOT NULL,
    document_text      TEXT NOT NULL,
    embedding_space    TEXT NOT NULL,
    dimensions         INTEGER NOT NULL CHECK (dimensions > 0),
    quantized_vector   BLOB NOT NULL,
    vector_norm_sq     INTEGER NOT NULL CHECK (vector_norm_sq > 0),
    runtime_json       TEXT NOT NULL,
    updated_at_millis  INTEGER NOT NULL,
    CHECK (length(quantized_vector) = dimensions)
);

CREATE INDEX IF NOT EXISTS semantic_documents_space
    ON semantic_documents (embedding_space, asset_id);

CREATE VIRTUAL TABLE IF NOT EXISTS semantic_document_fts USING fts5(
    asset_id UNINDEXED,
    text,
    tokenize = 'unicode61'
);
";

pub(crate) const LONG_AUDIO_MIGRATION_SQL: &str = r"
CREATE TABLE long_audio_segments (
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    plan_version       INTEGER NOT NULL,
    segment_index      INTEGER NOT NULL CHECK (segment_index >= 0),
    start_millis       INTEGER NOT NULL CHECK (start_millis >= 0),
    end_millis         INTEGER NOT NULL CHECK (end_millis > start_millis),
    proxy_content_hash TEXT,
    proxy_size_bytes   INTEGER CHECK (proxy_size_bytes > 0),
    transcript_json    TEXT,
    alignment_json     TEXT,
    contextual_json    TEXT,
    updated_at_millis  INTEGER NOT NULL,
    PRIMARY KEY (asset_id, plan_version, segment_index),
    CHECK ((proxy_content_hash IS NULL) = (proxy_size_bytes IS NULL))
);

CREATE INDEX long_audio_segments_asset_range
    ON long_audio_segments (asset_id, plan_version, start_millis);

CREATE TABLE long_audio_outline_nodes (
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    plan_version       INTEGER NOT NULL,
    level              INTEGER NOT NULL CHECK (level >= 0),
    node_index         INTEGER NOT NULL CHECK (node_index >= 0),
    start_millis       INTEGER NOT NULL CHECK (start_millis >= 0),
    end_millis         INTEGER NOT NULL CHECK (end_millis > start_millis),
    contextual_json    TEXT NOT NULL,
    updated_at_millis  INTEGER NOT NULL,
    PRIMARY KEY (asset_id, plan_version, level, node_index)
);

CREATE INDEX long_audio_outline_asset_level
    ON long_audio_outline_nodes (asset_id, plan_version, level, start_millis);
";

pub(crate) const ADJUSTMENT_EFFECTS_MIGRATION_SQL: &str = r#"
ALTER TABLE asset_adjustment_revisions
    ADD COLUMN reverb_json TEXT NOT NULL DEFAULT
    '{"enabled":false,"mix_percent":18,"pre_delay_millis":20,"decay_millis":1800,"size_percent":55,"damping_percent":45,"low_cut_hertz":120,"high_cut_hertz":10000}';
"#;

pub(crate) const RENDER_EXPORTS_MIGRATION_SQL: &str = r"
CREATE TABLE render_exports (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id               TEXT NOT NULL REFERENCES assets(id),
    adjustment_revision_id INTEGER REFERENCES asset_adjustment_revisions(id),
    output_path            TEXT NOT NULL,
    format                 TEXT NOT NULL CHECK (format = 'wav_pcm24'),
    sample_rate            INTEGER NOT NULL CHECK (sample_rate > 0),
    channel_count          INTEGER NOT NULL CHECK (channel_count > 0),
    bit_depth              INTEGER NOT NULL CHECK (bit_depth = 24),
    frame_count            INTEGER NOT NULL CHECK (frame_count > 0),
    content_hash           TEXT NOT NULL,
    size_bytes             INTEGER NOT NULL CHECK (size_bytes > 0),
    integrated_lufs        REAL NOT NULL,
    true_peak_dbtp         REAL NOT NULL,
    created_at_millis      INTEGER NOT NULL
);

CREATE INDEX render_exports_asset_created
    ON render_exports (asset_id, created_at_millis DESC, id DESC);
";

pub(crate) const USER_ALBUMS_MIGRATION_SQL: &str = r"
CREATE TABLE user_albums (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    name               TEXT NOT NULL COLLATE NOCASE UNIQUE,
    cover_asset_id     TEXT REFERENCES assets(id),
    created_at_millis  INTEGER NOT NULL,
    updated_at_millis  INTEGER NOT NULL
);

CREATE TABLE user_album_members (
    album_id           INTEGER NOT NULL REFERENCES user_albums(id),
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    added_at_millis    INTEGER NOT NULL,
    PRIMARY KEY (album_id, asset_id)
);

CREATE INDEX user_album_members_asset
    ON user_album_members (asset_id, album_id);
";

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

CREATE TABLE IF NOT EXISTS contextual_browse_facets (
    analysis_record_id  INTEGER NOT NULL REFERENCES analysis_records(id) ON DELETE CASCADE,
    asset_id            TEXT NOT NULL REFERENCES assets(id),
    facet_kind          TEXT NOT NULL CHECK (
                        facet_kind IN ('keyword', 'mood', 'place', 'event', 'person')),
    normalized_value    TEXT NOT NULL,
    display_value       TEXT NOT NULL,
    PRIMARY KEY (analysis_record_id, facet_kind, normalized_value)
);

CREATE INDEX IF NOT EXISTS contextual_browse_facets_lookup
    ON contextual_browse_facets (facet_kind, normalized_value, asset_id);

CREATE INDEX IF NOT EXISTS contextual_browse_facets_latest
    ON contextual_browse_facets (asset_id, analysis_record_id DESC);

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

CREATE TABLE IF NOT EXISTS semantic_documents (
    asset_id           TEXT PRIMARY KEY REFERENCES assets(id),
    source_revision    TEXT NOT NULL,
    document_text      TEXT NOT NULL,
    embedding_space    TEXT NOT NULL,
    dimensions         INTEGER NOT NULL CHECK (dimensions > 0),
    quantized_vector   BLOB NOT NULL,
    vector_norm_sq     INTEGER NOT NULL CHECK (vector_norm_sq > 0),
    runtime_json       TEXT NOT NULL,
    updated_at_millis  INTEGER NOT NULL,
    CHECK (length(quantized_vector) = dimensions)
);

CREATE INDEX IF NOT EXISTS semantic_documents_space
    ON semantic_documents (embedding_space, asset_id);

CREATE VIRTUAL TABLE IF NOT EXISTS semantic_document_fts USING fts5(
    asset_id UNINDEXED,
    text,
    tokenize = 'unicode61'
);

CREATE TABLE IF NOT EXISTS asset_user_state (
    asset_id           TEXT PRIMARY KEY REFERENCES assets(id),
    liked              INTEGER NOT NULL DEFAULT 0 CHECK (liked IN (0, 1)),
    rating             INTEGER NOT NULL DEFAULT 0 CHECK (rating BETWEEN 0 AND 5),
    updated_at_millis  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS user_albums (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    name               TEXT NOT NULL COLLATE NOCASE UNIQUE,
    cover_asset_id     TEXT REFERENCES assets(id),
    created_at_millis  INTEGER NOT NULL,
    updated_at_millis  INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS user_album_members (
    album_id           INTEGER NOT NULL REFERENCES user_albums(id),
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    added_at_millis    INTEGER NOT NULL,
    PRIMARY KEY (album_id, asset_id)
);

CREATE INDEX IF NOT EXISTS user_album_members_asset
    ON user_album_members (asset_id, album_id);

CREATE TABLE IF NOT EXISTS long_audio_segments (
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    plan_version       INTEGER NOT NULL,
    segment_index      INTEGER NOT NULL CHECK (segment_index >= 0),
    start_millis       INTEGER NOT NULL CHECK (start_millis >= 0),
    end_millis         INTEGER NOT NULL CHECK (end_millis > start_millis),
    proxy_content_hash TEXT,
    proxy_size_bytes   INTEGER CHECK (proxy_size_bytes > 0),
    transcript_json    TEXT,
    alignment_json     TEXT,
    contextual_json    TEXT,
    updated_at_millis  INTEGER NOT NULL,
    PRIMARY KEY (asset_id, plan_version, segment_index),
    CHECK ((proxy_content_hash IS NULL) = (proxy_size_bytes IS NULL))
);

CREATE INDEX IF NOT EXISTS long_audio_segments_asset_range
    ON long_audio_segments (asset_id, plan_version, start_millis);

CREATE TABLE IF NOT EXISTS long_audio_outline_nodes (
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    plan_version       INTEGER NOT NULL,
    level              INTEGER NOT NULL CHECK (level >= 0),
    node_index         INTEGER NOT NULL CHECK (node_index >= 0),
    start_millis       INTEGER NOT NULL CHECK (start_millis >= 0),
    end_millis         INTEGER NOT NULL CHECK (end_millis > start_millis),
    contextual_json    TEXT NOT NULL,
    updated_at_millis  INTEGER NOT NULL,
    PRIMARY KEY (asset_id, plan_version, level, node_index)
);

CREATE INDEX IF NOT EXISTS long_audio_outline_asset_level
    ON long_audio_outline_nodes (asset_id, plan_version, level, start_millis);

CREATE TABLE IF NOT EXISTS asset_adjustment_revisions (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id              TEXT NOT NULL REFERENCES assets(id),
    trim_start_millis     INTEGER NOT NULL CHECK (trim_start_millis >= 0),
    trim_end_millis       INTEGER NOT NULL CHECK (trim_end_millis > trim_start_millis),
    fade_in_millis        INTEGER NOT NULL CHECK (fade_in_millis >= 0),
    fade_out_millis       INTEGER NOT NULL CHECK (fade_out_millis >= 0),
    fade_in_curve         INTEGER NOT NULL DEFAULT 0 CHECK (fade_in_curve IN (0, 1, 2)),
    fade_out_curve        INTEGER NOT NULL DEFAULT 0 CHECK (fade_out_curve IN (0, 1, 2)),
    gain_centibels        INTEGER NOT NULL CHECK (gain_centibels BETWEEN -2400 AND 1200),
    low_cut_hertz         INTEGER NOT NULL DEFAULT 0
                          CHECK (low_cut_hertz = 0 OR low_cut_hertz BETWEEN 20 AND 240),
    eq_low_gain_centibels INTEGER NOT NULL DEFAULT 0
                          CHECK (eq_low_gain_centibels BETWEEN -1200 AND 1200),
    eq_mid_gain_centibels INTEGER NOT NULL DEFAULT 0
                          CHECK (eq_mid_gain_centibels BETWEEN -1200 AND 1200),
    eq_high_gain_centibels INTEGER NOT NULL DEFAULT 0
                           CHECK (eq_high_gain_centibels BETWEEN -1200 AND 1200),
    parametric_equalizer_json TEXT NOT NULL DEFAULT
                           '{\"bands\":[{\"enabled\":true,\"filter_kind\":\"low_shelf\",\"frequency_hertz\":120,\"q_hundredths\":71,\"gain_centibels\":0},{\"enabled\":false,\"filter_kind\":\"bell\",\"frequency_hertz\":250,\"q_hundredths\":100,\"gain_centibels\":0},{\"enabled\":true,\"filter_kind\":\"bell\",\"frequency_hertz\":1000,\"q_hundredths\":100,\"gain_centibels\":0},{\"enabled\":false,\"filter_kind\":\"bell\",\"frequency_hertz\":3000,\"q_hundredths\":100,\"gain_centibels\":0},{\"enabled\":false,\"filter_kind\":\"bell\",\"frequency_hertz\":5000,\"q_hundredths\":100,\"gain_centibels\":0},{\"enabled\":true,\"filter_kind\":\"high_shelf\",\"frequency_hertz\":8000,\"q_hundredths\":71,\"gain_centibels\":0}]}',
    compressor_enabled     INTEGER NOT NULL DEFAULT 0
                           CHECK (compressor_enabled IN (0, 1)),
    compressor_threshold_centibels INTEGER NOT NULL DEFAULT -1800
                           CHECK (compressor_threshold_centibels BETWEEN -6000 AND 0),
    compressor_ratio_tenths INTEGER NOT NULL DEFAULT 30
                           CHECK (compressor_ratio_tenths BETWEEN 10 AND 200),
    compressor_attack_millis INTEGER NOT NULL DEFAULT 10
                           CHECK (compressor_attack_millis BETWEEN 1 AND 200),
    compressor_release_millis INTEGER NOT NULL DEFAULT 120
                           CHECK (compressor_release_millis BETWEEN 20 AND 2000),
    compressor_makeup_centibels INTEGER NOT NULL DEFAULT 0
                           CHECK (compressor_makeup_centibels BETWEEN 0 AND 2400),
    reverb_json             TEXT NOT NULL DEFAULT
                           '{\"enabled\":false,\"mix_percent\":18,\"pre_delay_millis\":20,\"decay_millis\":1800,\"size_percent\":55,\"damping_percent\":45,\"low_cut_hertz\":120,\"high_cut_hertz\":10000}',
    limiter_enabled        INTEGER NOT NULL DEFAULT 0
                           CHECK (limiter_enabled IN (0, 1)),
    limiter_ceiling_centibels INTEGER NOT NULL DEFAULT -100
                           CHECK (limiter_ceiling_centibels BETWEEN -600 AND 0),
    limiter_release_millis INTEGER NOT NULL DEFAULT 100
                           CHECK (limiter_release_millis BETWEEN 20 AND 1000),
    created_at_millis     INTEGER NOT NULL,
    CHECK (fade_in_millis + fade_out_millis <= trim_end_millis - trim_start_millis)
);

CREATE INDEX IF NOT EXISTS asset_adjustment_revisions_latest
    ON asset_adjustment_revisions (asset_id, id DESC);

CREATE TABLE IF NOT EXISTS render_exports (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id               TEXT NOT NULL REFERENCES assets(id),
    adjustment_revision_id INTEGER REFERENCES asset_adjustment_revisions(id),
    output_path            TEXT NOT NULL,
    format                 TEXT NOT NULL CHECK (format = 'wav_pcm24'),
    sample_rate            INTEGER NOT NULL CHECK (sample_rate > 0),
    channel_count          INTEGER NOT NULL CHECK (channel_count > 0),
    bit_depth              INTEGER NOT NULL CHECK (bit_depth = 24),
    frame_count            INTEGER NOT NULL CHECK (frame_count > 0),
    content_hash           TEXT NOT NULL,
    size_bytes             INTEGER NOT NULL CHECK (size_bytes > 0),
    integrated_lufs        REAL NOT NULL,
    true_peak_dbtp         REAL NOT NULL,
    created_at_millis      INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS render_exports_asset_created
    ON render_exports (asset_id, created_at_millis DESC, id DESC);

CREATE TABLE IF NOT EXISTS asset_source_metadata (
    asset_id           TEXT PRIMARY KEY REFERENCES assets(id),
    container_format   TEXT NOT NULL,
    sample_rate        INTEGER NOT NULL,
    channel_count      INTEGER NOT NULL,
    entries_json       TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS inference_runs (
    local_job_id       TEXT PRIMARY KEY REFERENCES jobs(id) ON DELETE CASCADE,
    asset_id           TEXT NOT NULL REFERENCES assets(id),
    intent             TEXT NOT NULL,
    runtime_job_id     TEXT,
    contract_version   TEXT NOT NULL,
    state              TEXT NOT NULL CHECK (
                       state IN ('submitting', 'succeeded', 'failed', 'cancelled', 'expired')),
    http_status        INTEGER,
    error_code         TEXT,
    snapshot_json      TEXT,
    updated_at_millis  INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS inference_runs_asset_intent
    ON inference_runs (asset_id, intent, updated_at_millis DESC);
";

#[cfg(test)]
mod tests;
