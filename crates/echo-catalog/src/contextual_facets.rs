//! Derived browse facets for the newest contextual evidence per asset.
//!
//! Contextual Analysis remains the immutable source of truth. This owner
//! atomically appends that evidence and updates the rebuildable browse index.

use std::collections::BTreeSet;

use echo_domain::{AnalysisKind, AssetId};
use rusqlite::Transaction;

use crate::{AppendAnalysisRecord, CatalogError, record_analysis};

/// One contextual append whose validated payload becomes browse facets.
#[derive(Debug)]
pub struct AppendContextualAnalysis {
    pub analysis: AppendAnalysisRecord,
}

/// One keyword browse facet over the newest contextual evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextualKeywordFacet {
    pub key: String,
    pub label: String,
    pub count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ContextualFacetKind {
    Keyword,
    Mood,
    Place,
    Event,
    Person,
}

impl ContextualFacetKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::Mood => "mood",
            Self::Place => "place",
            Self::Event => "event",
            Self::Person => "person",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "keyword" => Some(Self::Keyword),
            "mood" => Some(Self::Mood),
            "place" => Some(Self::Place),
            "event" => Some(Self::Event),
            "person" => Some(Self::Person),
            _ => None,
        }
    }
}

/// Appends contextual evidence and updates its derived browse rows in the
/// caller's transaction.
///
/// # Errors
///
/// Returns a catalog failure for a wrong analysis kind, malformed contextual
/// evidence, or failed write.
pub fn record_contextual_analysis(
    transaction: &Transaction<'_>,
    append: &AppendContextualAnalysis,
) -> Result<(), CatalogError> {
    if append.analysis.record.kind != AnalysisKind::Contextual {
        return Err(CatalogError::new(
            crate::CatalogErrorKind::Other,
            "contextual facet append requires contextual analysis evidence",
        ));
    }
    record_analysis(transaction, &append.analysis)?;
    let analysis_record_id = transaction.last_insert_rowid();
    insert_contextual_rows(
        transaction,
        analysis_record_id,
        append.analysis.asset_id,
        &append.analysis.record.value,
    )
}

/// Lists keyword counts using the newest record that emitted keywords for
/// each asset. An empty refresh is absence of new evidence, not a negation;
/// a later non-empty keyword set supersedes the earlier set.
///
/// # Errors
///
/// Returns a catalog failure when the aggregate query cannot be read.
pub fn list_contextual_keyword_facets(
    transaction: &Transaction<'_>,
) -> Result<Vec<ContextualKeywordFacet>, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT f.normalized_value, MIN(f.display_value), COUNT(DISTINCT f.asset_id) \
         FROM contextual_browse_facets f \
         WHERE f.facet_kind = 'keyword' AND f.analysis_record_id = (\
             SELECT MAX(latest.analysis_record_id) \
             FROM contextual_browse_facets latest \
             WHERE latest.asset_id = f.asset_id \
               AND latest.facet_kind = f.facet_kind\
         ) GROUP BY f.normalized_value \
         ORDER BY COUNT(DISTINCT f.asset_id) DESC, f.normalized_value ASC",
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

pub(crate) fn rebuild_contextual_browse_facets(
    transaction: &Transaction<'_>,
) -> Result<(), CatalogError> {
    transaction.execute("DELETE FROM contextual_browse_facets", [])?;
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
        let value = serde_json::from_str(&value).map_err(|error| {
            CatalogError::new(
                crate::CatalogErrorKind::Other,
                format!("invalid contextual evidence during facet rebuild: {error}"),
            )
        })?;
        insert_contextual_rows(transaction, analysis_record_id, asset_id, &value)?;
    }
    Ok(())
}

fn insert_contextual_rows(
    transaction: &Transaction<'_>,
    analysis_record_id: i64,
    asset_id: AssetId,
    value: &serde_json::Value,
) -> Result<(), CatalogError> {
    let mut facets = Vec::new();
    append_array_values(
        &mut facets,
        ContextualFacetKind::Keyword,
        value.get("keywords"),
    );
    append_optional_value(&mut facets, ContextualFacetKind::Mood, value.get("mood"));
    append_optional_value(
        &mut facets,
        ContextualFacetKind::Place,
        value.get("place_hint"),
    );
    append_optional_value(
        &mut facets,
        ContextualFacetKind::Event,
        value.get("event_type"),
    );
    append_array_values(
        &mut facets,
        ContextualFacetKind::Person,
        value.get("people_hints"),
    );

    let mut seen = BTreeSet::new();
    for (kind, label) in facets {
        let Some((key, label)) = normalize_facet(&label) else {
            continue;
        };
        if !seen.insert((kind, key.clone())) {
            continue;
        }
        transaction.execute(
            "INSERT INTO contextual_browse_facets (analysis_record_id, asset_id, facet_kind, \
             normalized_value, display_value) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                analysis_record_id,
                asset_id.to_string(),
                kind.as_str(),
                key,
                label
            ],
        )?;
    }
    Ok(())
}

fn append_array_values(
    facets: &mut Vec<(ContextualFacetKind, String)>,
    kind: ContextualFacetKind,
    value: Option<&serde_json::Value>,
) {
    facets.extend(
        value
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(|label| (kind, label.to_owned())),
    );
}

fn append_optional_value(
    facets: &mut Vec<(ContextualFacetKind, String)>,
    kind: ContextualFacetKind,
    value: Option<&serde_json::Value>,
) {
    if let Some(label) = value.and_then(serde_json::Value::as_str) {
        facets.push((kind, label.to_owned()));
    }
}

pub(crate) fn normalize_facet(value: &str) -> Option<(String, String)> {
    let label = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if label.is_empty() {
        return None;
    }
    Some((label.to_lowercase(), label))
}

#[cfg(test)]
mod tests;
