//! On-disk schema ownership. Schema versions advance when the shape changes;
//! this binary requires an exact match and never migrates silently.

pub(crate) const SCHEMA_VERSION: i64 = 1;

pub(crate) const SCHEMA_V1: &str = "
CREATE TABLE IF NOT EXISTS catalog_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS assets (
    id                 TEXT PRIMARY KEY,
    content_hash       TEXT NOT NULL UNIQUE,
    path               TEXT NOT NULL,
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
";
