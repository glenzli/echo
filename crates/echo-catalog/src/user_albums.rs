//! User-authored sound albums and explicit membership.
//!
//! This owner persists user facts only. Rebuildable smart-album candidates
//! remain in `smart_albums` and enter this owner only through an explicit,
//! atomic snapshot requested by the user.

use std::{collections::BTreeSet, str::FromStr};

use echo_domain::AssetId;
use rusqlite::{OptionalExtension, Transaction, params};

use crate::{CatalogError, CatalogErrorKind};

const MAX_ALBUM_NAME_CHARACTERS: usize = 80;
const MAX_INITIAL_MEMBERS: usize = 10_000;

/// Stable Catalog identity for one user-authored album.
pub type UserAlbumId = i64;

/// One user album with its explicit member snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserAlbum {
    pub id: UserAlbumId,
    pub name: String,
    pub cover_asset_id: Option<AssetId>,
    pub member_asset_ids: Vec<AssetId>,
    pub created_at_millis: i64,
    pub updated_at_millis: i64,
}

/// Atomic input for creating an empty album or saving a suggestion snapshot.
#[derive(Debug, Clone, Copy)]
pub struct CreateUserAlbum<'a> {
    pub name: &'a str,
    pub member_asset_ids: &'a [AssetId],
}

/// Lists user albums by most recent user change.
///
/// # Errors
///
/// Returns a Catalog failure when stored identities cannot be decoded.
pub fn list_user_albums(transaction: &Transaction<'_>) -> Result<Vec<UserAlbum>, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT id, name, cover_asset_id, created_at_millis, updated_at_millis \
         FROM user_albums ORDER BY updated_at_millis DESC, id DESC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;
    let mut albums = Vec::new();
    for row in rows {
        let (id, name, cover_asset_id, created_at_millis, updated_at_millis) = row?;
        let mut member_statement = transaction.prepare(
            "SELECT asset_id FROM user_album_members WHERE album_id = ?1 \
             ORDER BY added_at_millis, asset_id",
        )?;
        let member_rows =
            member_statement.query_map([id], |member_row| member_row.get::<_, String>(0))?;
        let member_asset_ids = member_rows
            .map(|member| parse_asset_id(&member?))
            .collect::<Result<Vec<_>, _>>()?;
        albums.push(UserAlbum {
            id,
            name,
            cover_asset_id: cover_asset_id.as_deref().map(parse_asset_id).transpose()?,
            member_asset_ids,
            created_at_millis,
            updated_at_millis,
        });
    }
    Ok(albums)
}

/// Creates one user album and snapshots all supplied members atomically.
///
/// # Errors
///
/// Rejects invalid or duplicate names, unknown assets, excessive member
/// counts, or persistence failures.
pub fn create_user_album(
    transaction: &Transaction<'_>,
    input: CreateUserAlbum<'_>,
    now_millis: i64,
) -> Result<UserAlbumId, CatalogError> {
    validate_album_name(input.name)?;
    if input.member_asset_ids.len() > MAX_INITIAL_MEMBERS {
        return Err(album_error("album member snapshot is too large"));
    }
    ensure_unique_name(transaction, input.name, None)?;

    let mut seen = BTreeSet::new();
    let members = input
        .member_asset_ids
        .iter()
        .copied()
        .filter(|asset_id| seen.insert(*asset_id))
        .collect::<Vec<_>>();
    for asset_id in &members {
        ensure_asset_exists(transaction, *asset_id)?;
    }

    transaction.execute(
        "INSERT INTO user_albums \
         (name, cover_asset_id, created_at_millis, updated_at_millis) \
         VALUES (?1, ?2, ?3, ?3)",
        params![
            input.name,
            members.first().map(ToString::to_string),
            now_millis
        ],
    )?;
    let album_id = transaction.last_insert_rowid();
    for asset_id in members {
        transaction.execute(
            "INSERT INTO user_album_members (album_id, asset_id, added_at_millis) \
             VALUES (?1, ?2, ?3)",
            params![album_id, asset_id.to_string(), now_millis],
        )?;
    }
    Ok(album_id)
}

/// Renames an existing user album while preserving authored text exactly.
///
/// # Errors
///
/// Rejects invalid or duplicate names, unknown albums, or persistence
/// failures.
pub fn rename_user_album(
    transaction: &Transaction<'_>,
    album_id: UserAlbumId,
    name: &str,
    now_millis: i64,
) -> Result<(), CatalogError> {
    validate_album_name(name)?;
    ensure_album_exists(transaction, album_id)?;
    ensure_unique_name(transaction, name, Some(album_id))?;
    transaction.execute(
        "UPDATE user_albums SET name = ?1, updated_at_millis = ?2 WHERE id = ?3",
        params![name, now_millis, album_id],
    )?;
    Ok(())
}

/// Deletes an album and its explicit memberships without touching assets.
///
/// # Errors
///
/// Rejects unknown albums or persistence failures.
pub fn delete_user_album(
    transaction: &Transaction<'_>,
    album_id: UserAlbumId,
) -> Result<(), CatalogError> {
    ensure_album_exists(transaction, album_id)?;
    transaction.execute(
        "DELETE FROM user_album_members WHERE album_id = ?1",
        [album_id],
    )?;
    transaction.execute("DELETE FROM user_albums WHERE id = ?1", [album_id])?;
    Ok(())
}

/// Adds or removes one explicit album member.
///
/// The first added sound becomes the cover. Removing the current cover picks
/// the oldest remaining member, or leaves an empty album without a cover.
///
/// # Errors
///
/// Rejects unknown albums or assets and propagates persistence failures.
pub fn set_user_album_membership(
    transaction: &Transaction<'_>,
    album_id: UserAlbumId,
    asset_id: AssetId,
    included: bool,
    now_millis: i64,
) -> Result<bool, CatalogError> {
    ensure_album_exists(transaction, album_id)?;
    ensure_asset_exists(transaction, asset_id)?;
    let changed = if included {
        transaction.execute(
            "INSERT OR IGNORE INTO user_album_members \
             (album_id, asset_id, added_at_millis) VALUES (?1, ?2, ?3)",
            params![album_id, asset_id.to_string(), now_millis],
        )? > 0
    } else {
        transaction.execute(
            "DELETE FROM user_album_members WHERE album_id = ?1 AND asset_id = ?2",
            params![album_id, asset_id.to_string()],
        )? > 0
    };
    if !changed {
        return Ok(false);
    }

    let cover_asset_id = transaction
        .query_row(
            "SELECT cover_asset_id FROM user_albums WHERE id = ?1",
            [album_id],
            |row| row.get::<_, Option<String>>(0),
        )?
        .as_deref()
        .map(parse_asset_id)
        .transpose()?;
    let next_cover = if included && cover_asset_id.is_none() {
        Some(asset_id)
    } else if !included && cover_asset_id == Some(asset_id) {
        oldest_member(transaction, album_id)?
    } else {
        cover_asset_id
    };
    transaction.execute(
        "UPDATE user_albums SET cover_asset_id = ?1, updated_at_millis = ?2 WHERE id = ?3",
        params![next_cover.map(|id| id.to_string()), now_millis, album_id],
    )?;
    Ok(true)
}

fn validate_album_name(name: &str) -> Result<(), CatalogError> {
    if name.is_empty()
        || name.trim() != name
        || name.chars().count() > MAX_ALBUM_NAME_CHARACTERS
        || name.chars().any(char::is_control)
    {
        return Err(album_error(
            "album name must be non-empty, trimmed, free of control characters, and at most 80 characters",
        ));
    }
    Ok(())
}

fn ensure_unique_name(
    transaction: &Transaction<'_>,
    name: &str,
    except_album_id: Option<UserAlbumId>,
) -> Result<(), CatalogError> {
    let duplicate = transaction
        .query_row(
            "SELECT id FROM user_albums WHERE name = ?1 COLLATE NOCASE AND id != ?2",
            params![name, except_album_id.unwrap_or(-1)],
            |row| row.get::<_, i64>(0),
        )
        .optional()?;
    if duplicate.is_some() {
        return Err(album_error("album name already exists"));
    }
    Ok(())
}

fn ensure_album_exists(
    transaction: &Transaction<'_>,
    album_id: UserAlbumId,
) -> Result<(), CatalogError> {
    let exists = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM user_albums WHERE id = ?1)",
        [album_id],
        |row| row.get::<_, bool>(0),
    )?;
    if !exists {
        return Err(album_error("user album does not exist"));
    }
    Ok(())
}

fn ensure_asset_exists(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
) -> Result<(), CatalogError> {
    let exists = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM assets WHERE id = ?1)",
        [asset_id.to_string()],
        |row| row.get::<_, bool>(0),
    )?;
    if !exists {
        return Err(album_error("album member asset does not exist"));
    }
    Ok(())
}

fn oldest_member(
    transaction: &Transaction<'_>,
    album_id: UserAlbumId,
) -> Result<Option<AssetId>, CatalogError> {
    transaction
        .query_row(
            "SELECT asset_id FROM user_album_members WHERE album_id = ?1 \
             ORDER BY added_at_millis, asset_id LIMIT 1",
            [album_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .as_deref()
        .map(parse_asset_id)
        .transpose()
}

fn parse_asset_id(value: &str) -> Result<AssetId, CatalogError> {
    AssetId::from_str(value)
        .map_err(|error| album_error(format!("invalid stored album member identity: {error}")))
}

fn album_error(message: impl Into<String>) -> CatalogError {
    CatalogError::new(CatalogErrorKind::Other, message)
}

#[cfg(test)]
mod tests;
