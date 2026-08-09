//! Explainable cross-asset sound-album candidates.
//!
//! These are rebuildable browse projections, never user album membership.
//! A candidate exists only when at least two assets share exact evidence from
//! their newest contextual record or immutable Original metadata.

use std::collections::{BTreeMap, BTreeSet};

use echo_domain::AssetId;
use rusqlite::Transaction;

use crate::{CatalogError, SourceMetadataEntry, contextual_facets::normalize_facet};

const MINIMUM_ALBUM_MEMBERS: usize = 2;
const MAXIMUM_ALBUM_CANDIDATES: usize = 48;

/// The evidence dimension that connects a suggested album.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SmartAlbumFacet {
    Time,
    Place,
    Event,
    Mood,
    Person,
}

impl SmartAlbumFacet {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Time => "time",
            Self::Place => "place",
            Self::Event => "event",
            Self::Mood => "mood",
            Self::Person => "person",
        }
    }
}

/// Whether membership comes from immutable source evidence or AI analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SmartAlbumEvidence {
    Original,
    Ai,
}

impl SmartAlbumEvidence {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::Ai => "ai",
        }
    }
}

/// One suggested album with exact, auditable member identities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartAlbumCandidate {
    pub key: String,
    pub label: String,
    pub facet: SmartAlbumFacet,
    pub evidence: SmartAlbumEvidence,
    pub member_asset_ids: Vec<AssetId>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct GroupKey {
    evidence: SmartAlbumEvidence,
    facet: SmartAlbumFacet,
    normalized_value: String,
}

#[derive(Debug)]
struct Group {
    label: String,
    members: BTreeSet<AssetId>,
}

/// Lists bounded smart-album candidates, largest groups first.
///
/// Model-derived groups use only facets attached to the newest contextual
/// record for each asset. Original groups currently cover recording day and
/// embedded location. Exact normalized values are required: this projection
/// never performs unsupported synonym or identity merging.
///
/// # Errors
///
/// Returns a catalog failure when stored identities or metadata cannot be
/// decoded.
pub fn list_smart_album_candidates(
    transaction: &Transaction<'_>,
) -> Result<Vec<SmartAlbumCandidate>, CatalogError> {
    let mut groups = BTreeMap::new();
    collect_contextual_groups(transaction, &mut groups)?;
    collect_recording_day_groups(transaction, &mut groups)?;
    collect_source_location_groups(transaction, &mut groups)?;

    let mut candidates = groups
        .into_iter()
        .filter_map(|(key, group)| {
            (group.members.len() >= MINIMUM_ALBUM_MEMBERS).then(|| SmartAlbumCandidate {
                key: format!(
                    "{}:{}:{}",
                    key.evidence.as_str(),
                    key.facet.as_str(),
                    key.normalized_value
                ),
                label: group.label,
                facet: key.facet,
                evidence: key.evidence,
                member_asset_ids: group.members.into_iter().collect(),
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .member_asset_ids
            .len()
            .cmp(&left.member_asset_ids.len())
            .then_with(|| left.evidence.cmp(&right.evidence))
            .then_with(|| left.facet.cmp(&right.facet))
            .then_with(|| left.label.cmp(&right.label))
    });
    candidates.truncate(MAXIMUM_ALBUM_CANDIDATES);
    Ok(candidates)
}

fn collect_contextual_groups(
    transaction: &Transaction<'_>,
    groups: &mut BTreeMap<GroupKey, Group>,
) -> Result<(), CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT f.facet_kind, f.normalized_value, f.display_value, f.asset_id \
         FROM contextual_browse_facets f \
         WHERE f.facet_kind <> 'keyword' AND f.analysis_record_id = (\
             SELECT MAX(r.id) FROM analysis_records r \
             WHERE r.asset_id = f.asset_id AND r.kind = 'contextual'\
         ) ORDER BY f.facet_kind, f.normalized_value, f.asset_id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;
    for row in rows {
        let (kind, normalized_value, label, asset_id) = row?;
        let facet = match crate::contextual_facets::ContextualFacetKind::from_str(&kind) {
            Some(crate::contextual_facets::ContextualFacetKind::Mood) => SmartAlbumFacet::Mood,
            Some(crate::contextual_facets::ContextualFacetKind::Place) => SmartAlbumFacet::Place,
            Some(crate::contextual_facets::ContextualFacetKind::Event) => SmartAlbumFacet::Event,
            Some(crate::contextual_facets::ContextualFacetKind::Person) => SmartAlbumFacet::Person,
            _ => continue,
        };
        insert_group(
            groups,
            GroupKey {
                evidence: SmartAlbumEvidence::Ai,
                facet,
                normalized_value,
            },
            label,
            parse_asset_id(&asset_id)?,
        );
    }
    Ok(())
}

fn collect_recording_day_groups(
    transaction: &Transaction<'_>,
    groups: &mut BTreeMap<GroupKey, Group>,
) -> Result<(), CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT id, strftime('%Y-%m-%d', recorded_at_millis / 1000, 'unixepoch') \
         FROM assets WHERE recorded_at_millis > 0 ORDER BY recorded_at_millis, id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (asset_id, day) = row?;
        insert_group(
            groups,
            GroupKey {
                evidence: SmartAlbumEvidence::Original,
                facet: SmartAlbumFacet::Time,
                normalized_value: day.clone(),
            },
            day,
            parse_asset_id(&asset_id)?,
        );
    }
    Ok(())
}

fn collect_source_location_groups(
    transaction: &Transaction<'_>,
    groups: &mut BTreeMap<GroupKey, Group>,
) -> Result<(), CatalogError> {
    let mut statement = transaction
        .prepare("SELECT asset_id, entries_json FROM asset_source_metadata ORDER BY asset_id")?;
    let rows = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (asset_id, entries_json) = row?;
        let entries: Vec<SourceMetadataEntry> =
            serde_json::from_str(&entries_json).map_err(|error| {
                CatalogError::new(
                    crate::CatalogErrorKind::Other,
                    format!("invalid source metadata during smart album projection: {error}"),
                )
            })?;
        let Some(location) = entries
            .iter()
            .find(|entry| entry.key.to_ascii_lowercase().contains("location"))
            .map(|entry| entry.value.as_str())
        else {
            continue;
        };
        let Some((normalized_value, label)) = normalize_facet(location) else {
            continue;
        };
        insert_group(
            groups,
            GroupKey {
                evidence: SmartAlbumEvidence::Original,
                facet: SmartAlbumFacet::Place,
                normalized_value,
            },
            label,
            parse_asset_id(&asset_id)?,
        );
    }
    Ok(())
}

fn insert_group(
    groups: &mut BTreeMap<GroupKey, Group>,
    key: GroupKey,
    label: String,
    asset_id: AssetId,
) {
    let group = groups.entry(key).or_insert_with(|| Group {
        label: label.clone(),
        members: BTreeSet::new(),
    });
    if label < group.label {
        group.label = label;
    }
    group.members.insert(asset_id);
}

fn parse_asset_id(value: &str) -> Result<AssetId, CatalogError> {
    value.parse().map_err(|error| {
        CatalogError::new(
            crate::CatalogErrorKind::Other,
            format!("invalid stored asset id during smart album projection: {error}"),
        )
    })
}

#[cfg(test)]
mod tests;
