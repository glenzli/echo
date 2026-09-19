//! User collection identities shared by recordings and authored memories.
//!
//! Audio assets remain immutable source registrations. A sound item says what
//! the user collects; an assembly edition pins a completed render and its
//! provenance without registering that render as a new Original.

use std::{collections::BTreeMap, path::PathBuf};

use rusqlite::{OptionalExtension, Transaction, params};

use crate::{CatalogError, CatalogErrorKind};

pub(crate) const SCHEMA_SQL: &str = r"
CREATE TABLE IF NOT EXISTS sound_items (
    id TEXT PRIMARY KEY,
    asset_id TEXT UNIQUE REFERENCES assets(id),
    assembly_id TEXT UNIQUE REFERENCES sound_assemblies(id),
    in_memory INTEGER NOT NULL DEFAULT 1 CHECK (in_memory IN (0, 1)),
    in_materials INTEGER NOT NULL DEFAULT 0 CHECK (in_materials IN (0, 1)),
    material_category TEXT NOT NULL DEFAULT '',
    listening_export_id INTEGER REFERENCES sound_assembly_exports(id),
    created_at_millis INTEGER NOT NULL CHECK (created_at_millis >= 0),
    CHECK ((asset_id IS NOT NULL AND assembly_id IS NULL AND id = asset_id)
        OR (asset_id IS NULL AND assembly_id IS NOT NULL AND id = assembly_id))
);
CREATE TABLE IF NOT EXISTS memory_waveform_artifacts (
    export_id INTEGER PRIMARY KEY REFERENCES sound_assembly_exports(id),
    content_hash TEXT NOT NULL,
    size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0)
);
CREATE TABLE IF NOT EXISTS project_adjustment_revisions (
    revision_id INTEGER PRIMARY KEY REFERENCES asset_adjustment_revisions(id),
    assembly_id TEXT NOT NULL REFERENCES sound_assemblies(id),
    clip_id TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS project_materials (
    assembly_id TEXT NOT NULL REFERENCES sound_assemblies(id),
    asset_id TEXT NOT NULL REFERENCES assets(id),
    PRIMARY KEY (assembly_id, asset_id)
);
CREATE TABLE IF NOT EXISTS assembly_memory_editions (
    sound_id TEXT NOT NULL REFERENCES sound_items(id),
    export_id INTEGER NOT NULL REFERENCES sound_assembly_exports(id),
    created_at_millis INTEGER NOT NULL,
    PRIMARY KEY (sound_id, export_id)
);
CREATE TABLE IF NOT EXISTS sound_user_state (
    asset_id TEXT PRIMARY KEY REFERENCES sound_items(id),
    liked INTEGER NOT NULL DEFAULT 0 CHECK (liked IN (0, 1)),
    rating INTEGER NOT NULL DEFAULT 0 CHECK (rating BETWEEN 0 AND 5),
    last_listened_at_millis INTEGER NOT NULL DEFAULT 0 CHECK (last_listened_at_millis >= 0),
    resume_position_millis INTEGER NOT NULL DEFAULT 0 CHECK (resume_position_millis >= 0),
    updated_at_millis INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS memory_albums (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL COLLATE NOCASE UNIQUE,
    cover_asset_id TEXT REFERENCES sound_items(id),
    created_at_millis INTEGER NOT NULL,
    updated_at_millis INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS memory_album_members (
    album_id INTEGER NOT NULL REFERENCES memory_albums(id),
    asset_id TEXT NOT NULL REFERENCES sound_items(id),
    added_at_millis INTEGER NOT NULL,
    PRIMARY KEY (album_id, asset_id)
);
CREATE INDEX IF NOT EXISTS memory_album_members_sound ON memory_album_members(asset_id, album_id);
CREATE VIEW IF NOT EXISTS memory_sources AS
    SELECT s.id, a.path, a.path_status, a.duration_millis, a.recorded_at_millis,
           a.imported_at_millis
    FROM sound_items s JOIN assets a ON a.id = s.asset_id WHERE s.in_memory = 1
    UNION ALL
    SELECT s.id, e.output_path, 'present', e.frame_count * 1000 / e.sample_rate,
           NULL, s.created_at_millis
    FROM sound_items s JOIN sound_assembly_exports e ON e.id = s.listening_export_id
    WHERE s.in_memory = 1;
";

pub(crate) fn migrate(transaction: &Transaction<'_>) -> Result<(), CatalogError> {
    transaction.execute_batch(SCHEMA_SQL)?;
    transaction.execute(
        "INSERT OR IGNORE INTO sound_items (id, asset_id, created_at_millis) \
         SELECT id, id, imported_at_millis FROM assets",
        [],
    )?;
    transaction.execute_batch(
        "INSERT OR IGNORE INTO sound_user_state SELECT * FROM asset_user_state;
         INSERT OR IGNORE INTO memory_albums SELECT * FROM user_albums;
         INSERT OR IGNORE INTO memory_album_members SELECT * FROM user_album_members;",
    )?;
    Ok(())
}

/// Collection membership is independent of an immutable source identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoundMembership {
    pub in_memory: bool,
    pub in_materials: bool,
    pub material_category: String,
    pub assembly_id: Option<String>,
}

/// Lists collection identities, including project-only material sources.
///
/// # Errors
/// Returns a catalog error when membership cannot be read.
pub fn sound_memberships(
    transaction: &Transaction<'_>,
) -> Result<BTreeMap<String, SoundMembership>, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT id, in_memory, in_materials, material_category, assembly_id FROM sound_items",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get(0)?,
            SoundMembership {
                in_memory: row.get(1)?,
                in_materials: row.get(2)?,
                material_category: row.get(3)?,
                assembly_id: row.get(4)?,
            },
        ))
    })?;
    rows.collect::<Result<_, _>>().map_err(CatalogError::from)
}

/// Changes collection membership without deleting media, revisions or references.
///
/// # Errors
/// Rejects unknown identities and invalid categories.
pub fn set_sound_membership(
    transaction: &Transaction<'_>,
    id: &str,
    in_memory: bool,
    in_materials: bool,
    category: &str,
) -> Result<(), CatalogError> {
    if !["", "music", "ambience", "effects", "voice"].contains(&category) {
        return Err(library_error("unknown material category"));
    }
    let updated = transaction.execute(
        "UPDATE sound_items SET in_memory = ?2, in_materials = ?3, material_category = ?4 \
         WHERE id = ?1",
        params![id, in_memory, in_materials, category],
    )?;
    if updated != 1 {
        return Err(library_error("sound item does not exist"));
    }
    Ok(())
}

/// Associates an imported source with a project without collecting it globally.
///
/// # Errors
/// Rejects unknown projects or sources.
pub fn attach_project_material(
    transaction: &Transaction<'_>,
    assembly_id: &str,
    asset_id: &str,
) -> Result<(), CatalogError> {
    transaction.execute(
        "INSERT OR IGNORE INTO project_materials (assembly_id, asset_id) VALUES (?1, ?2)",
        params![assembly_id, asset_id],
    )?;
    Ok(())
}

/// Returns explicit project-bin sources, including not-yet-placed material.
///
/// # Errors
/// Returns a catalog error when the project bin cannot be read.
pub fn project_material_ids(
    transaction: &Transaction<'_>,
    assembly_id: &str,
) -> Result<Vec<String>, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT asset_id FROM project_materials WHERE assembly_id = ?1 ORDER BY asset_id",
    )?;
    statement
        .query_map([assembly_id], |row| row.get(0))?
        .collect::<Result<_, _>>()
        .map_err(CatalogError::from)
}

/// One explicitly selected, immutable listening edition of an assembly memory.
#[derive(Debug, Clone, PartialEq)]
pub struct AssemblyMemory {
    pub id: String,
    pub name: String,
    pub assembly_revision_id: i64,
    pub export_id: i64,
    pub path: PathBuf,
    pub content_hash: String,
    pub size_bytes: u64,
    pub duration_millis: u64,
    pub sample_rate: u32,
    pub channel_count: u16,
    pub created_at_millis: i64,
    pub provenance_json: String,
    pub liked: bool,
    pub rating: u8,
    pub last_listened_at_millis: i64,
    pub resume_position_millis: u64,
}

/// Preserves a completed mix as the memory's listening edition. The caller owns
/// copying and verifying durable media before entering this transaction.
///
/// # Errors
/// Rejects unknown exports and cross-project references. Failure rolls back the
/// membership and edition together with the caller's transaction.
pub fn preserve_assembly_memory(
    transaction: &Transaction<'_>,
    assembly_id: &str,
    export_id: i64,
    now_millis: i64,
) -> Result<(), CatalogError> {
    let valid: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM sound_assembly_exports WHERE id = ?1 AND assembly_id = ?2)",
        params![export_id, assembly_id],
        |row| row.get(0),
    )?;
    if !valid || now_millis < 0 {
        return Err(library_error(
            "memory edition must reference an export of the same assembly",
        ));
    }
    let previous: Option<i64> = transaction
        .query_row(
            "SELECT listening_export_id FROM sound_items WHERE id = ?1",
            [assembly_id],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    transaction.execute(
        "INSERT INTO sound_items (id, assembly_id, listening_export_id, created_at_millis) \
         VALUES (?1, ?1, ?2, ?3) ON CONFLICT(id) DO UPDATE SET \
         in_memory = 1, listening_export_id = excluded.listening_export_id",
        params![assembly_id, export_id, now_millis],
    )?;
    transaction.execute(
        "INSERT OR IGNORE INTO assembly_memory_editions (sound_id, export_id, created_at_millis) VALUES (?1, ?2, ?3)",
        params![assembly_id, export_id, now_millis],
    )?;
    // Composition time changes between editions. Keep listening history, but
    // never claim that the old playhead still points to the same content.
    if previous != Some(export_id) {
        transaction.execute(
            "UPDATE sound_user_state SET resume_position_millis = 0 WHERE asset_id = ?1",
            [assembly_id],
        )?;
    }
    Ok(())
}

/// Lists saved assembly memories with the exact accepted render and provenance.
///
/// # Errors
/// Returns a catalog error for malformed or unreadable records.
pub fn assembly_memories(
    transaction: &Transaction<'_>,
) -> Result<Vec<AssemblyMemory>, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT s.id, r.name, r.id, e.id, e.output_path, e.content_hash, e.size_bytes, \
         e.frame_count * 1000 / e.sample_rate, e.sample_rate, e.channel_count, \
         s.created_at_millis, e.provenance_json, COALESCE(u.liked, 0), COALESCE(u.rating, 0), \
         COALESCE(u.last_listened_at_millis, 0), COALESCE(u.resume_position_millis, 0) \
         FROM sound_items s JOIN sound_assembly_exports e ON e.id = s.listening_export_id \
         JOIN sound_assembly_revisions r ON r.id = e.assembly_revision_id \
         LEFT JOIN sound_user_state u ON u.asset_id = s.id ORDER BY s.created_at_millis DESC, s.id",
    )?;
    statement
        .query_map([], |row| {
            Ok(AssemblyMemory {
                id: row.get(0)?,
                name: row.get(1)?,
                assembly_revision_id: row.get(2)?,
                export_id: row.get(3)?,
                path: PathBuf::from(row.get::<_, String>(4)?),
                content_hash: row.get(5)?,
                size_bytes: nonnegative(row, 6)?,
                duration_millis: nonnegative(row, 7)?,
                sample_rate: row.get(8)?,
                channel_count: row.get(9)?,
                created_at_millis: row.get(10)?,
                provenance_json: row.get(11)?,
                liked: row.get(12)?,
                rating: row.get(13)?,
                last_listened_at_millis: row.get(14)?,
                resume_position_millis: nonnegative(row, 15)?,
            })
        })?
        .collect::<Result<_, _>>()
        .map_err(CatalogError::from)
}

/// Resolves the accepted mix of an assembly memory, never a newer draft.
///
/// # Errors
/// Returns a catalog error when the entry cannot be queried.
pub fn assembly_memory_path(
    transaction: &Transaction<'_>,
    id: &str,
) -> Result<Option<PathBuf>, CatalogError> {
    transaction.query_row(
        "SELECT e.output_path FROM sound_items s JOIN sound_assembly_exports e ON e.id = s.listening_export_id WHERE s.id = ?1",
        [id], |row| row.get::<_, String>(0).map(PathBuf::from),
    ).optional().map_err(CatalogError::from)
}

fn nonnegative(row: &rusqlite::Row<'_>, column: usize) -> rusqlite::Result<u64> {
    let value: i64 = row.get(column)?;
    u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(column, value))
}

fn library_error(message: &str) -> CatalogError {
    CatalogError::new(CatalogErrorKind::Other, message)
}

#[cfg(test)]
mod tests;
