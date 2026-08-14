//! Rights-tracked impulse-response identity and preparation evidence.
//!
//! Exact source bytes and rebuildable prepared bytes live in their respective
//! stores. This owner persists only immutable identities, append-only import
//! declarations, and the evidence required to validate a prepared artifact.

use std::str::FromStr;

use echo_domain::ContentHash;
use rusqlite::Transaction;
use uuid::Uuid;

use crate::{CatalogError, CatalogErrorKind};

/// Canonical plane interpretation of one prepared impulse response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpulseResponseLayout {
    Mono,
    StereoParallel,
    TrueStereoLlLrRlRr,
}

impl ImpulseResponseLayout {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Mono => "mono",
            Self::StereoParallel => "stereo_parallel",
            Self::TrueStereoLlLrRlRr => "true_stereo_ll_lr_rl_rr",
        }
    }

    fn from_stored(value: &str) -> rusqlite::Result<Self> {
        match value {
            "mono" => Ok(Self::Mono),
            "stereo_parallel" => Ok(Self::StereoParallel),
            "true_stereo_ll_lr_rl_rr" => Ok(Self::TrueStereoLlLrRlRr),
            _ => Err(rusqlite::Error::InvalidQuery),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImpulseResponseRights {
    Spdx {
        expression: String,
        license_url: Option<String>,
    },
    UserOwnedNoRedistribution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpulseResponseRecord {
    pub import_id: Uuid,
    pub source_hash: ContentHash,
    pub source_size_bytes: u64,
    pub prepared_hash: ContentHash,
    pub prepared_size_bytes: u64,
    pub preparation_version: u32,
    pub source_sample_rate: u32,
    pub channel_count: u32,
    pub layout: ImpulseResponseLayout,
    pub source_frame_count: u64,
    pub prepared_frame_count: u64,
    pub avcodec_version: u32,
    pub swresample_version: u32,
    pub imported_at_millis: u64,
    pub original_path: String,
    pub display_name: String,
    pub creator: Option<String>,
    pub source_url: Option<String>,
    pub attribution: Option<String>,
    pub rights: ImpulseResponseRights,
}

/// Records one already-published import. Repeating the same import id is
/// idempotent only when every immutable field is identical.
///
/// # Errors
///
/// Returns a catalog error when immutable evidence conflicts or persistence
/// fails.
#[allow(clippy::too_many_lines)] // Failure ordering is easier to audit as one transaction projection.
pub fn record_impulse_response(
    transaction: &Transaction<'_>,
    record: &ImpulseResponseRecord,
) -> Result<(), CatalogError> {
    validate_preparation_shape(record)?;
    if let Some(existing) = list_impulse_responses(transaction)?
        .into_iter()
        .find(|existing| existing.import_id == record.import_id)
    {
        if existing == *record {
            return Ok(());
        }
        return Err(CatalogError::new(
            CatalogErrorKind::Other,
            "impulse response import id already names different evidence".to_owned(),
        ));
    }
    let (rights_kind, expression, license_url) = match &record.rights {
        ImpulseResponseRights::Spdx {
            expression,
            license_url,
        } => ("spdx", Some(expression.as_str()), license_url.as_deref()),
        ImpulseResponseRights::UserOwnedNoRedistribution => {
            ("user_owned_no_redistribution", None, None)
        }
    };
    transaction.execute(
        "INSERT INTO impulse_response_sources (source_hash, size_bytes, created_at_millis) \
         VALUES (?1, ?2, ?3) ON CONFLICT(source_hash) DO NOTHING",
        rusqlite::params![
            record.source_hash.to_string(),
            i64::try_from(record.source_size_bytes).map_err(range_error)?,
            i64::try_from(record.imported_at_millis).map_err(range_error)?,
        ],
    )?;
    let stored_source_size: i64 = transaction.query_row(
        "SELECT size_bytes FROM impulse_response_sources WHERE source_hash = ?1",
        [record.source_hash.to_string()],
        |row| row.get(0),
    )?;
    if stored_u64(stored_source_size)? != record.source_size_bytes {
        return Err(CatalogError::new(
            CatalogErrorKind::Other,
            "impulse response source hash has conflicting size evidence".to_owned(),
        ));
    }
    transaction.execute(
        "INSERT INTO impulse_response_preparations (prepared_hash, source_hash, size_bytes, \
         preparation_version, source_sample_rate, channel_count, layout_kind, source_frame_count, \
         prepared_frame_count, avcodec_version, swresample_version, created_at_millis) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12) \
         ON CONFLICT(prepared_hash) DO NOTHING",
        rusqlite::params![
            record.prepared_hash.to_string(),
            record.source_hash.to_string(),
            i64::try_from(record.prepared_size_bytes).map_err(range_error)?,
            i64::from(record.preparation_version),
            i64::from(record.source_sample_rate),
            i64::from(record.channel_count),
            record.layout.as_str(),
            i64::try_from(record.source_frame_count).map_err(range_error)?,
            i64::try_from(record.prepared_frame_count).map_err(range_error)?,
            i64::from(record.avcodec_version),
            i64::from(record.swresample_version),
            i64::try_from(record.imported_at_millis).map_err(range_error)?,
        ],
    )?;
    let stored_preparation: (String, i64, i64, i64, i64, String, i64, i64, i64, i64) = transaction
        .query_row(
            "SELECT source_hash, size_bytes, preparation_version, source_sample_rate, \
             channel_count, layout_kind, source_frame_count, prepared_frame_count, avcodec_version, \
             swresample_version FROM impulse_response_preparations WHERE prepared_hash = ?1",
            [record.prepared_hash.to_string()],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                ))
            },
        )?;
    let expected_preparation = (
        record.source_hash.to_string(),
        i64::try_from(record.prepared_size_bytes).map_err(range_error)?,
        i64::from(record.preparation_version),
        i64::from(record.source_sample_rate),
        i64::from(record.channel_count),
        record.layout.as_str().to_owned(),
        i64::try_from(record.source_frame_count).map_err(range_error)?,
        i64::try_from(record.prepared_frame_count).map_err(range_error)?,
        i64::from(record.avcodec_version),
        i64::from(record.swresample_version),
    );
    if stored_preparation != expected_preparation {
        return Err(CatalogError::new(
            CatalogErrorKind::Other,
            "prepared impulse response hash has conflicting evidence".to_owned(),
        ));
    }
    transaction.execute(
        "INSERT INTO impulse_response_imports (import_id, source_hash, prepared_hash, \
         imported_at_millis, original_path, display_name, creator, source_url, attribution, \
         rights_kind, spdx_expression, license_url) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        rusqlite::params![
            record.import_id.to_string(),
            record.source_hash.to_string(),
            record.prepared_hash.to_string(),
            i64::try_from(record.imported_at_millis).map_err(range_error)?,
            record.original_path,
            record.display_name,
            record.creator,
            record.source_url,
            record.attribution,
            rights_kind,
            expression,
            license_url,
        ],
    )?;
    Ok(())
}

/// Lists rights-tracked imports newest first.
///
/// # Errors
///
/// Returns a catalog error when stored evidence is malformed or unreadable.
pub fn list_impulse_responses(
    transaction: &Transaction<'_>,
) -> Result<Vec<ImpulseResponseRecord>, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT i.import_id, i.source_hash, s.size_bytes, i.prepared_hash, p.size_bytes, \
         p.preparation_version, p.source_sample_rate, p.channel_count, p.layout_kind, p.source_frame_count, \
         p.prepared_frame_count, p.avcodec_version, p.swresample_version, i.imported_at_millis, \
         i.original_path, i.display_name, i.creator, i.source_url, i.attribution, i.rights_kind, \
         i.spdx_expression, i.license_url FROM impulse_response_imports i \
         JOIN impulse_response_sources s ON s.source_hash = i.source_hash \
         JOIN impulse_response_preparations p ON p.prepared_hash = i.prepared_hash \
         ORDER BY i.imported_at_millis DESC, i.import_id DESC",
    )?;
    let rows = statement.query_map([], record_from_row)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(CatalogError::from)
}

fn record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ImpulseResponseRecord> {
    let rights_kind: String = row.get(19)?;
    let rights = match rights_kind.as_str() {
        "spdx" => ImpulseResponseRights::Spdx {
            expression: row.get::<_, Option<String>>(20)?.ok_or_else(|| {
                rusqlite::Error::InvalidColumnType(
                    20,
                    "spdx_expression".to_owned(),
                    rusqlite::types::Type::Null,
                )
            })?,
            license_url: row.get(21)?,
        },
        "user_owned_no_redistribution" => ImpulseResponseRights::UserOwnedNoRedistribution,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    Ok(ImpulseResponseRecord {
        import_id: Uuid::parse_str(&row.get::<_, String>(0)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        source_hash: ContentHash::from_str(&row.get::<_, String>(1)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        source_size_bytes: stored_u64(row.get(2)?)?,
        prepared_hash: ContentHash::from_str(&row.get::<_, String>(3)?)
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
        prepared_size_bytes: stored_u64(row.get(4)?)?,
        preparation_version: stored_u32(row.get(5)?)?,
        source_sample_rate: stored_u32(row.get(6)?)?,
        channel_count: stored_u32(row.get(7)?)?,
        layout: ImpulseResponseLayout::from_stored(&row.get::<_, String>(8)?)?,
        source_frame_count: stored_u64(row.get(9)?)?,
        prepared_frame_count: stored_u64(row.get(10)?)?,
        avcodec_version: stored_u32(row.get(11)?)?,
        swresample_version: stored_u32(row.get(12)?)?,
        imported_at_millis: stored_u64(row.get(13)?)?,
        original_path: row.get(14)?,
        display_name: row.get(15)?,
        creator: row.get(16)?,
        source_url: row.get(17)?,
        attribution: row.get(18)?,
        rights,
    })
}

/// Validates that an authored immutable selection resolves to one import row.
pub(crate) fn validate_impulse_response_selection(
    transaction: &Transaction<'_>,
    selection: echo_domain::ImpulseResponseSelection,
) -> Result<(), CatalogError> {
    let matches: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM impulse_response_imports WHERE import_id = ?1 \
         AND source_hash = ?2 AND prepared_hash = ?3",
        rusqlite::params![
            selection.import_id.to_string(),
            selection.source_hash.to_string(),
            selection.prepared_hash.to_string(),
        ],
        |row| row.get(0),
    )?;
    if matches == 1 {
        Ok(())
    } else {
        Err(CatalogError::new(
            CatalogErrorKind::Other,
            "impulse response selection does not resolve to one immutable import".to_owned(),
        ))
    }
}

fn validate_preparation_shape(record: &ImpulseResponseRecord) -> Result<(), CatalogError> {
    if matches!(
        (
            record.preparation_version,
            record.channel_count,
            record.layout
        ),
        (1, 1, ImpulseResponseLayout::Mono)
            | (1, 2, ImpulseResponseLayout::StereoParallel)
            | (2, 4, ImpulseResponseLayout::TrueStereoLlLrRlRr)
    ) {
        Ok(())
    } else {
        Err(CatalogError::new(
            CatalogErrorKind::Other,
            "impulse response preparation version, channels, and layout conflict".to_owned(),
        ))
    }
}

fn stored_u64(value: i64) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, value))
}

fn stored_u32(value: i64) -> rusqlite::Result<u32> {
    u32::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(0, value))
}

fn range_error(error: std::num::TryFromIntError) -> CatalogError {
    CatalogError::new(CatalogErrorKind::Other, error.to_string())
}

#[cfg(test)]
mod tests;
