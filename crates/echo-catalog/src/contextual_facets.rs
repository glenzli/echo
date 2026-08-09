//! Derived keyword facets for the newest contextual evidence per asset.
//!
//! Contextual Analysis remains the immutable source of truth. This owner
//! atomically appends that evidence and updates the rebuildable browse index.

use std::collections::BTreeSet;

use echo_domain::{AnalysisKind, AssetId};
use rusqlite::Transaction;

use crate::{AppendAnalysisRecord, CatalogError, record_analysis};

/// One contextual append plus the keywords projected from its validated JSON.
#[derive(Debug)]
pub struct AppendContextualAnalysis<'a> {
    pub analysis: AppendAnalysisRecord,
    pub keywords: &'a [String],
}

/// One keyword browse facet over the newest contextual evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextualKeywordFacet {
    pub key: String,
    pub label: String,
    pub count: u64,
}

/// Appends contextual evidence and updates its derived keyword rows in the
/// caller's transaction.
///
/// # Errors
///
/// Returns a catalog failure for a wrong analysis kind or failed write.
pub fn record_contextual_analysis(
    transaction: &Transaction<'_>,
    append: &AppendContextualAnalysis<'_>,
) -> Result<(), CatalogError> {
    if append.analysis.record.kind != AnalysisKind::Contextual {
        return Err(CatalogError::new(
            crate::CatalogErrorKind::Other,
            "contextual facet append requires contextual analysis evidence",
        ));
    }
    record_analysis(transaction, &append.analysis)?;
    let analysis_record_id = transaction.last_insert_rowid();
    insert_keyword_rows(
        transaction,
        analysis_record_id,
        append.analysis.asset_id,
        append.keywords,
    )
}

/// Lists keyword counts using only the newest contextual record for each
/// asset, so re-analysis cannot leave stale browse membership visible.
///
/// # Errors
///
/// Returns a catalog failure when the aggregate query cannot be read.
pub fn list_contextual_keyword_facets(
    transaction: &Transaction<'_>,
) -> Result<Vec<ContextualKeywordFacet>, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT f.normalized_keyword, MIN(f.display_keyword), COUNT(DISTINCT f.asset_id) \
         FROM contextual_keyword_facets f \
         WHERE f.analysis_record_id = (\
             SELECT MAX(r.id) FROM analysis_records r \
             WHERE r.asset_id = f.asset_id AND r.kind = 'contextual'\
         ) GROUP BY f.normalized_keyword \
         ORDER BY COUNT(DISTINCT f.asset_id) DESC, f.normalized_keyword ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;
    let mut facets = Vec::new();
    for row in rows {
        let (key, label, count) = row?;
        let count = u64::try_from(count).map_err(|_| {
            CatalogError::new(
                crate::CatalogErrorKind::Other,
                "contextual keyword facet count was negative",
            )
        })?;
        facets.push(ContextualKeywordFacet { key, label, count });
    }
    Ok(facets)
}

pub(crate) fn rebuild_contextual_keyword_facets(
    transaction: &Transaction<'_>,
) -> Result<(), CatalogError> {
    transaction.execute("DELETE FROM contextual_keyword_facets", [])?;
    let mut statement = transaction.prepare(
        "SELECT id, asset_id, value FROM analysis_records \
         WHERE kind = 'contextual' ORDER BY id ASC",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (analysis_record_id, asset_id, value) = row?;
        let asset_id = asset_id.parse::<AssetId>().map_err(|error| {
            CatalogError::new(
                crate::CatalogErrorKind::Other,
                format!("invalid stored asset id during facet rebuild: {error}"),
            )
        })?;
        let value: serde_json::Value = serde_json::from_str(&value).map_err(|error| {
            CatalogError::new(
                crate::CatalogErrorKind::Other,
                format!("invalid contextual evidence during facet rebuild: {error}"),
            )
        })?;
        let keywords = value
            .get("keywords")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_owned)
            .collect::<Vec<_>>();
        insert_keyword_rows(transaction, analysis_record_id, asset_id, &keywords)?;
    }
    Ok(())
}

fn insert_keyword_rows(
    transaction: &Transaction<'_>,
    analysis_record_id: i64,
    asset_id: AssetId,
    keywords: &[String],
) -> Result<(), CatalogError> {
    let mut seen = BTreeSet::new();
    for keyword in keywords {
        let Some((key, label)) = normalize_keyword(keyword) else {
            continue;
        };
        if !seen.insert(key.clone()) {
            continue;
        }
        transaction.execute(
            "INSERT INTO contextual_keyword_facets (analysis_record_id, asset_id, \
             normalized_keyword, display_keyword) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![analysis_record_id, asset_id.to_string(), key, label],
        )?;
    }
    Ok(())
}

fn normalize_keyword(keyword: &str) -> Option<(String, String)> {
    let label = keyword.split_whitespace().collect::<Vec<_>>().join(" ");
    if label.is_empty() {
        return None;
    }
    Some((label.to_lowercase(), label))
}

#[cfg(test)]
mod tests;
