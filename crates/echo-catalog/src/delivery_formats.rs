//! Atomic expansion of delivery formats while preserving export IDs and references.
use crate::CatalogError;
use rusqlite::{Connection, OptionalExtension};
pub(crate) fn migrate(connection: &Connection) -> Result<(), CatalogError> {
    let sql: String = connection.query_row(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='render_exports'",
        [],
        |row| row.get(0),
    )?;
    let assembly_sql: String = connection.query_row(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='sound_assembly_exports'",
        [],
        |row| row.get(0),
    )?;
    if sql.contains("'aac_m4a'") && assembly_sql.contains("'aac_m4a'") {
        return Ok(());
    }
    let foreign_keys: bool =
        connection.pragma_query_value(None, "foreign_keys", |row| row.get(0))?;
    connection.pragma_update(None, "foreign_keys", false)?;
    let result = (|| {
        let tx = connection.unchecked_transaction()?;
        let memory_view: Option<String> = tx
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='view' AND name='memory_sources'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        tx.execute_batch("DROP VIEW IF EXISTS memory_sources;")?;
        tx.execute_batch(SQL)?;
        tx.execute_batch(crate::memory_info::SCHEMA_SQL)?;
        if let Some(sql) = memory_view {
            tx.execute_batch(&sql)?;
        }
        let violations: i64 =
            tx.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })?;
        if violations != 0 {
            return Err(crate::CatalogError::new(
                crate::CatalogErrorKind::Constraint,
                "delivery migration contains broken references",
            ));
        }
        tx.execute(
            "UPDATE catalog_meta SET value=?1 WHERE key='schema_version'",
            [crate::schema::SCHEMA_VERSION.to_string()],
        )?;
        tx.execute(
            "UPDATE catalog_meta SET value=?1 WHERE key='schema_identity'",
            [crate::schema::SCHEMA_IDENTITY],
        )?;
        tx.commit()?;
        Ok(())
    })();
    connection.pragma_update(None, "foreign_keys", foreign_keys)?;
    result
}
const SQL: &str = r"
CREATE TABLE render_exports_expanded (
    id                     INTEGER PRIMARY KEY AUTOINCREMENT,
    asset_id               TEXT NOT NULL REFERENCES assets(id),
    adjustment_revision_id INTEGER REFERENCES asset_adjustment_revisions(id),
    output_path            TEXT NOT NULL,
    format                 TEXT NOT NULL CHECK (
                           format IN ('wav_pcm16','wav_pcm24','flac24','wav_float32','mp3','aac_m4a')),
    sample_rate            INTEGER NOT NULL CHECK (sample_rate > 0),
    channel_count          INTEGER NOT NULL CHECK (channel_count > 0),
    bit_depth              INTEGER NOT NULL CHECK (
                           (format = 'wav_pcm16' AND bit_depth = 16) OR
                           (format = 'wav_float32' AND bit_depth = 32) OR
                           (format IN ('mp3','aac_m4a') AND bit_depth = 0) OR
                           (format IN ('wav_pcm24', 'flac24') AND bit_depth = 24)),
    frame_count            INTEGER NOT NULL CHECK (frame_count > 0),
    content_hash           TEXT NOT NULL,
    size_bytes             INTEGER NOT NULL CHECK (size_bytes > 0),
    integrated_lufs        REAL NOT NULL,
    true_peak_dbtp         REAL NOT NULL,
    created_at_millis      INTEGER NOT NULL
);
INSERT INTO render_exports_expanded SELECT * FROM render_exports;
DROP TABLE render_exports;
ALTER TABLE render_exports_expanded RENAME TO render_exports;
CREATE INDEX render_exports_asset_created ON render_exports (asset_id, created_at_millis DESC, id DESC);
CREATE TABLE sound_assembly_exports_expanded (
    id                    INTEGER PRIMARY KEY AUTOINCREMENT,
    assembly_id           TEXT NOT NULL REFERENCES sound_assemblies(id),
    assembly_revision_id  INTEGER NOT NULL REFERENCES sound_assembly_revisions(id),
    output_path           TEXT NOT NULL,
    format                TEXT NOT NULL CHECK (format IN ('wav_pcm16','wav_pcm24','flac24','wav_float32','mp3','aac_m4a')),
    sample_rate           INTEGER NOT NULL CHECK (sample_rate > 0),
    channel_count         INTEGER NOT NULL CHECK (channel_count IN (1, 2)),
    bit_depth             INTEGER NOT NULL CHECK ((format = 'wav_pcm16' AND bit_depth = 16) OR (format IN ('wav_pcm24','flac24') AND bit_depth = 24) OR (format = 'wav_float32' AND bit_depth = 32) OR (format IN ('mp3','aac_m4a') AND bit_depth = 0)),
    frame_count           INTEGER NOT NULL CHECK (frame_count > 0),
    content_hash          TEXT NOT NULL,
    size_bytes            INTEGER NOT NULL CHECK (size_bytes > 0),
    integrated_lufs       REAL NOT NULL,
    true_peak_dbtp        REAL NOT NULL,
    provenance_json       TEXT NOT NULL CHECK (json_valid(provenance_json)),
    created_at_millis     INTEGER NOT NULL CHECK (created_at_millis >= 0)
);
INSERT INTO sound_assembly_exports_expanded SELECT * FROM sound_assembly_exports;
DROP TABLE sound_assembly_exports;
ALTER TABLE sound_assembly_exports_expanded RENAME TO sound_assembly_exports;
CREATE INDEX sound_assembly_exports_created ON sound_assembly_exports (assembly_id, created_at_millis DESC, id DESC);
";

#[cfg(test)]
mod tests;
