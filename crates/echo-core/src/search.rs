//! Source-aware fusion of independent search evidence spaces.

use std::collections::BTreeMap;

use echo_catalog::Catalog;

use crate::{CoreError, InferRuntimeConfig};

/// Merges text-evidence and CLAP candidates by rank, never by vector score.
pub fn search(
    catalog: &Catalog,
    config: &InferRuntimeConfig,
    query: &str,
    limit: u64,
) -> Result<Vec<echo_catalog::SemanticSearchHit>, CoreError> {
    let text = crate::semantic_search::search(catalog, config, query, limit);
    let audio = crate::audio_semantic_search::search(catalog, config, query, limit);
    if text.is_err() && audio.is_err() {
        return Err(text.expect_err("both failed"));
    }
    let mut ranks = BTreeMap::<String, f64>::new();
    if let Ok(hits) = text {
        for (rank, hit) in hits.into_iter().enumerate() {
            *ranks.entry(hit.asset_id).or_default() += 1.0 / (60.0 + rank as f64 + 1.0);
        }
    }
    if let Ok(hits) = audio {
        for (rank, hit) in hits.into_iter().enumerate() {
            *ranks.entry(hit.asset_id).or_default() += 1.0 / (60.0 + rank as f64 + 1.0);
        }
    }
    let mut hits = ranks
        .into_iter()
        .map(|(asset_id, score)| echo_catalog::SemanticSearchHit { asset_id, score })
        .collect::<Vec<_>>();
    hits.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.asset_id.cmp(&b.asset_id))
    });
    hits.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    Ok(hits)
}

#[cfg(test)]
mod tests;
