//! Idempotent asset registration. Registration is keyed by content hash: the
//! same bytes import once, later imports become lookups that update the
//! descriptive path.

use std::path::{Path, PathBuf};

use echo_domain::{AnalysisLevel, AssetId, AudioAsset, ContentHash, OriginalRef};
use rusqlite::{OptionalExtension, Transaction};
use std::str::FromStr;

use crate::error::{CatalogError, CatalogErrorKind};

/// `SQLite` stores `INTEGER` as i64; sizes that exceed it cannot be persisted.
fn sqlite_integer_overflow(error: std::num::TryFromIntError) -> CatalogError {
    CatalogError::new(
        crate::error::CatalogErrorKind::Other,
        format!("value exceeds SQLite integer range: {error}"),
    )
}

/// Outcome of registering one source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegisterAsset {
    /// The content was unknown and a new asset was created.
    Created(AudioAsset),
    /// The content already existed; the existing asset's path was refreshed.
    Existed(AudioAsset),
}

/// Lookup of an asset by content identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetLookup {
    Found(AudioAsset),
    NotFound,
}

/// Inputs for one registration write.
#[derive(Debug, Clone)]
pub struct AssetRegistrationInput<'a> {
    pub content_hash: ContentHash,
    pub path: &'a Path,
    pub size_bytes: u64,
    pub codec: Option<&'a str>,
    pub duration_millis: Option<u64>,
    pub recorded_at_millis: Option<i64>,
    pub imported_at_millis: i64,
}

/// Registers `path` under `content_hash`. Idempotent by content identity.
///
/// When the content already exists, the stored path refreshes to `path`
/// (files move) and the existing identity is returned.
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn register_asset(
    transaction: &Transaction<'_>,
    input: &AssetRegistrationInput<'_>,
) -> Result<RegisterAsset, CatalogError> {
    let AssetRegistrationInput {
        content_hash,
        path,
        size_bytes,
        codec,
        duration_millis,
        recorded_at_millis,
        imported_at_millis,
    } = *input;
    let hash_text = content_hash.to_string();
    if let Some(existing) = find_by_hash(transaction, &hash_text)? {
        update_path(transaction, &hash_text, path)?;
        return Ok(RegisterAsset::Existed(existing));
    }
    let id = AssetId::new();
    transaction.execute(
        "INSERT INTO assets (id, content_hash, path, path_status, size_bytes, codec, \
         duration_millis, recorded_at_millis, imported_at_millis) \
         VALUES (?1, ?2, ?3, 'present', ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            id.to_string(),
            hash_text,
            path.to_string_lossy(),
            i64::try_from(size_bytes).map_err(sqlite_integer_overflow)?,
            codec,
            duration_millis.and_then(|millis| i64::try_from(millis).ok()),
            recorded_at_millis,
            imported_at_millis,
        ],
    )?;
    transaction.execute(
        "INSERT INTO asset_levels (asset_id, max_level) VALUES (?1, 0)",
        [id.to_string()],
    )?;
    transaction.execute(
        "INSERT INTO sound_items (id, asset_id, created_at_millis) VALUES (?1, ?1, ?2)",
        rusqlite::params![id.to_string(), imported_at_millis],
    )?;
    let asset = load_asset(transaction, &id.to_string())?.ok_or_else(|| {
        CatalogError::new(
            CatalogErrorKind::Other,
            "inserted asset row vanished before read-back",
        )
    })?;
    Ok(RegisterAsset::Created(asset))
}

/// Looks up an asset by content hash.
///
/// # Errors
///
/// Returns a catalog failure when the read cannot be applied.
pub fn find_by_content_hash(
    transaction: &Transaction<'_>,
    content_hash: ContentHash,
) -> Result<AssetLookup, CatalogError> {
    let hash_text = content_hash.to_string();
    match find_by_hash(transaction, &hash_text)? {
        Some(asset) => Ok(AssetLookup::Found(asset)),
        None => Ok(AssetLookup::NotFound),
    }
}

/// Looks up an asset by id.
///
/// # Errors
///
/// Returns a catalog failure when the read cannot be applied.
pub fn find_by_id(transaction: &Transaction<'_>, id: AssetId) -> Result<AssetLookup, CatalogError> {
    match load_asset(transaction, &id.to_string())? {
        Some(asset) => Ok(AssetLookup::Found(asset)),
        None => Ok(AssetLookup::NotFound),
    }
}

/// Lists every registered asset ordered by import recency.
///
/// # Errors
///
/// Returns a catalog failure when the read cannot be applied.
pub fn list_assets(transaction: &Transaction<'_>) -> Result<Vec<AudioAsset>, CatalogError> {
    let mut statement =
        transaction.prepare("SELECT id FROM assets ORDER BY imported_at_millis DESC, id DESC")?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
    let mut assets = Vec::new();
    for id in rows {
        let id = id?;
        if let Some(asset) = load_asset(transaction, &id)? {
            assets.push(asset);
        }
    }
    Ok(assets)
}

fn find_by_hash(
    transaction: &Transaction<'_>,
    hash_text: &str,
) -> Result<Option<AudioAsset>, CatalogError> {
    let id: Option<String> = transaction
        .query_row(
            "SELECT id FROM assets WHERE content_hash = ?1",
            [hash_text],
            |row| row.get(0),
        )
        .optional()?;
    match id {
        Some(id) => load_asset(transaction, &id),
        None => Ok(None),
    }
}

fn update_path(
    transaction: &Transaction<'_>,
    hash_text: &str,
    path: &Path,
) -> Result<(), CatalogError> {
    transaction.execute(
        "UPDATE assets SET path = ?1 WHERE content_hash = ?2",
        rusqlite::params![path.to_string_lossy(), hash_text],
    )?;
    Ok(())
}

fn load_asset(transaction: &Transaction<'_>, id: &str) -> Result<Option<AudioAsset>, CatalogError> {
    transaction
        .query_row(
            "SELECT id, content_hash, path, path_status, size_bytes, codec, duration_millis, \
             recorded_at_millis, imported_at_millis, (SELECT max_level FROM asset_levels \
             WHERE asset_id = assets.id) FROM assets WHERE id = ?1",
            [id],
            |row| {
                let content_hash: String = row.get(1)?;
                let parsed_hash = parse_hash(&content_hash).ok_or_else(|| {
                    rusqlite::Error::FromSqlConversionFailure(
                        1,
                        rusqlite::types::Type::Text,
                        format!("invalid stored content hash {content_hash}").into(),
                    )
                })?;
                let size_bytes = u64::try_from(row.get::<_, i64>(4)?).map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Integer,
                        "stored size_bytes is negative".into(),
                    )
                })?;
                let duration_millis = match row.get::<_, Option<i64>>(6)? {
                    Some(millis) => Some(u64::try_from(millis).map_err(|_| {
                        rusqlite::Error::FromSqlConversionFailure(
                            6,
                            rusqlite::types::Type::Integer,
                            "stored duration_millis is negative".into(),
                        )
                    })?),
                    None => None,
                };
                let max_level = AnalysisLevel::try_from(row.get::<_, u8>(9)?).map_err(|level| {
                    rusqlite::Error::FromSqlConversionFailure(
                        9,
                        rusqlite::types::Type::Integer,
                        format!("invalid stored analysis level {level}").into(),
                    )
                })?;
                let id = AssetId::from_str(id).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        error.to_string().into(),
                    )
                })?;
                let path_status = match row.get::<_, String>(3)?.as_str() {
                    "present" => echo_domain::AssetPathStatus::Present,
                    "missing" => echo_domain::AssetPathStatus::Missing,
                    other => {
                        return Err(rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            format!("invalid stored path status {other}").into(),
                        ));
                    }
                };
                Ok(AudioAsset {
                    id,
                    original: OriginalRef {
                        path: PathBuf::from(row.get::<_, String>(2)?),
                        content_hash: parsed_hash,
                        path_status,
                        size_bytes,
                        codec: row.get(5)?,
                        duration_millis,
                        recorded_at_millis: row.get(7)?,
                        imported_at_millis: row.get(8)?,
                    },
                    max_level,
                })
            },
        )
        .optional()
        .map_err(CatalogError::from)
}

fn parse_hash(text: &str) -> Option<ContentHash> {
    ContentHash::from_str(text).ok()
}
