//! Explicit lifecycle for destructive spectral work performed after an
//! already-rendered adjustment graph.
//!
//! A working copy is not an alternate adjustment graph and never writes an
//! immutable Original. It freezes one internal render's content identity and
//! upstream adjustment revision. When that revision stops being
//! current, the copy remains preserved but is unavailable until the user
//! explicitly renders and creates a new copy.

use std::str::FromStr;

use echo_domain::{AssetId, ContentHash};
use rusqlite::Transaction;

use crate::{CatalogError, CatalogErrorKind, latest_adjustment_graph};

const TILE_MANIFEST_SCHEMA_VERSION: u32 = 1;
const INITIAL_TILE_MANIFEST_JSON: &str = r#"{"schema":1,"tiles":[]}"#;
const INITIAL_TOOL_VERSION: &str = "rendered-spectral-working-copy-v1";

/// Evidence needed to create an empty destructive working copy from one
/// completed internal render cache entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateRenderedSpectralWorkingCopy {
    pub asset_id: AssetId,
    /// Zero identifies an untouched immutable Original.
    pub parent_adjustment_revision_id: i64,
    pub parent_render_content_hash: ContentHash,
    pub created_at_millis: i64,
}

/// Whether the frozen upstream graph still matches the asset's current graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderedSpectralWorkingCopyAvailability {
    /// The copy may be rendered and edited when its tile processor arrives.
    Available,
    /// The prior adjustment graph changed. Preserve the copy; never rebase it.
    UpstreamChanged,
}

/// One asset-local working copy and its immutable upstream render evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedSpectralWorkingCopy {
    pub id: i64,
    pub asset_id: AssetId,
    /// Zero means the immutable Original had no saved adjustment graph.
    pub parent_adjustment_revision_id: i64,
    pub parent_render_content_hash: ContentHash,
    pub manifest_schema_version: u32,
    pub tile_manifest_json: String,
    pub tool_version: String,
    pub enabled: bool,
    pub availability: RenderedSpectralWorkingCopyAvailability,
    pub created_at_millis: i64,
}

/// Creates an empty working copy only from a still-current internal render.
///
/// The parent hash identifies a cache payload owned by the renderer. The
/// catalog deliberately never treats a user-visible delivery file as cache.
pub fn create_rendered_spectral_working_copy(
    transaction: &Transaction<'_>,
    request: CreateRenderedSpectralWorkingCopy,
) -> Result<RenderedSpectralWorkingCopy, CatalogError> {
    if request.parent_adjustment_revision_id < 0 || request.created_at_millis < 0 {
        return Err(invalid(
            "rendered spectral working-copy evidence is outside the supported range",
        ));
    }
    if current_revision_id(transaction, request.asset_id)? != request.parent_adjustment_revision_id
    {
        return Err(invalid(
            "rendered spectral working copy requires the current upstream render revision",
        ));
    }
    transaction.execute(
        "INSERT INTO rendered_spectral_working_copies \
         (asset_id, parent_adjustment_revision_id, parent_render_content_hash, \
          manifest_schema_version, tile_manifest_json, tool_version, \
          enabled, created_at_millis) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7)",
        rusqlite::params![
            request.asset_id.to_string(),
            optional_revision(request.parent_adjustment_revision_id),
            request.parent_render_content_hash.to_string(),
            i64::from(TILE_MANIFEST_SCHEMA_VERSION),
            INITIAL_TILE_MANIFEST_JSON,
            INITIAL_TOOL_VERSION,
            request.created_at_millis,
        ],
    )?;
    Ok(RenderedSpectralWorkingCopy {
        id: transaction.last_insert_rowid(),
        asset_id: request.asset_id,
        parent_adjustment_revision_id: request.parent_adjustment_revision_id,
        parent_render_content_hash: request.parent_render_content_hash,
        manifest_schema_version: TILE_MANIFEST_SCHEMA_VERSION,
        tile_manifest_json: INITIAL_TILE_MANIFEST_JSON.to_owned(),
        tool_version: INITIAL_TOOL_VERSION.to_owned(),
        enabled: true,
        availability: RenderedSpectralWorkingCopyAvailability::Available,
        created_at_millis: request.created_at_millis,
    })
}

/// Lists preserved working copies, newest first, with current availability.
pub fn list_rendered_spectral_working_copies(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
) -> Result<Vec<RenderedSpectralWorkingCopy>, CatalogError> {
    let current_revision_id = current_revision_id(transaction, asset_id)?;
    let mut statement = transaction.prepare(
        "SELECT id, parent_adjustment_revision_id, parent_render_content_hash, \
         manifest_schema_version, tile_manifest_json, tool_version, \
         enabled, created_at_millis FROM rendered_spectral_working_copies \
         WHERE asset_id = ?1 ORDER BY created_at_millis DESC, id DESC",
    )?;
    let rows = statement.query_map([asset_id.to_string()], |row| {
        Ok(StoredWorkingCopy {
            id: row.get(0)?,
            parent_adjustment_revision_id: row.get::<_, Option<i64>>(1)?.unwrap_or(0),
            parent_render_content_hash: row.get(2)?,
            manifest_schema_version: row.get(3)?,
            tile_manifest_json: row.get(4)?,
            tool_version: row.get(5)?,
            enabled: row.get(6)?,
            created_at_millis: row.get(7)?,
        })
    })?;
    rows.map(|row| restore_working_copy(asset_id, row?, current_revision_id))
        .collect()
}

/// Bypasses or restores the complete working copy without deleting its tiles.
pub fn set_rendered_spectral_working_copy_enabled(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    working_copy_id: i64,
    enabled: bool,
) -> Result<(), CatalogError> {
    if working_copy_id <= 0 {
        return Err(invalid(
            "rendered spectral working-copy id must be positive",
        ));
    }
    let updated = transaction.execute(
        "UPDATE rendered_spectral_working_copies SET enabled = ?1 \
         WHERE id = ?2 AND asset_id = ?3",
        rusqlite::params![i64::from(enabled), working_copy_id, asset_id.to_string()],
    )?;
    if updated == 1 {
        Ok(())
    } else {
        Err(invalid(
            "rendered spectral working copy is not owned by the asset",
        ))
    }
}

/// Removes the whole copy. Its future derived render cache is disposable and
/// must be collected by the cache owner, never by this provenance transaction.
pub fn remove_rendered_spectral_working_copy(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
    working_copy_id: i64,
) -> Result<(), CatalogError> {
    if working_copy_id <= 0 {
        return Err(invalid(
            "rendered spectral working-copy id must be positive",
        ));
    }
    let deleted = transaction.execute(
        "DELETE FROM rendered_spectral_working_copies WHERE id = ?1 AND asset_id = ?2",
        rusqlite::params![working_copy_id, asset_id.to_string()],
    )?;
    if deleted == 1 {
        Ok(())
    } else {
        Err(invalid(
            "rendered spectral working copy is not owned by the asset",
        ))
    }
}

struct StoredWorkingCopy {
    id: i64,
    parent_adjustment_revision_id: i64,
    parent_render_content_hash: String,
    manifest_schema_version: i64,
    tile_manifest_json: String,
    tool_version: String,
    enabled: i64,
    created_at_millis: i64,
}

fn current_revision_id(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
) -> Result<i64, CatalogError> {
    Ok(latest_adjustment_graph(transaction, asset_id)?.map_or(0, |revision| revision.revision_id))
}

fn restore_working_copy(
    asset_id: AssetId,
    stored: StoredWorkingCopy,
    current_revision_id: i64,
) -> Result<RenderedSpectralWorkingCopy, CatalogError> {
    let manifest_schema_version = u32::try_from(stored.manifest_schema_version)
        .ok()
        .filter(|version| *version > 0)
        .ok_or_else(|| malformed("manifest schema version"))?;
    if !serde_json::from_str::<serde_json::Value>(&stored.tile_manifest_json).is_ok()
        || stored.tool_version.trim().is_empty()
        || !matches!(stored.enabled, 0 | 1)
        || stored.created_at_millis < 0
    {
        return Err(malformed("rendered spectral working-copy row"));
    }
    Ok(RenderedSpectralWorkingCopy {
        id: stored.id,
        asset_id,
        parent_adjustment_revision_id: stored.parent_adjustment_revision_id,
        parent_render_content_hash: ContentHash::from_str(&stored.parent_render_content_hash)
            .map_err(|_| malformed("parent render content hash"))?,
        manifest_schema_version,
        tile_manifest_json: stored.tile_manifest_json,
        tool_version: stored.tool_version,
        enabled: stored.enabled == 1,
        availability: (current_revision_id == stored.parent_adjustment_revision_id)
            .then_some(RenderedSpectralWorkingCopyAvailability::Available)
            .unwrap_or(RenderedSpectralWorkingCopyAvailability::UpstreamChanged),
        created_at_millis: stored.created_at_millis,
    })
}

fn optional_revision(revision_id: i64) -> Option<i64> {
    (revision_id != 0).then_some(revision_id)
}

fn invalid(message: &str) -> CatalogError {
    CatalogError::new(CatalogErrorKind::Constraint, message)
}

fn malformed(field: &str) -> CatalogError {
    CatalogError::new(
        CatalogErrorKind::Other,
        format!("malformed rendered spectral working copy {field}"),
    )
}

#[cfg(test)]
mod tests;
