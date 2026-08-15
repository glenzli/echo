//! Durable CLAP audio-segment vectors, separate from text semantic evidence.

use echo_domain::AssetId;
use rusqlite::Transaction;

use crate::{CatalogError, CatalogErrorKind};

const CLAP_DIMENSIONS: usize = 512;

/// A short original asset eligible for the first CLAP indexing slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioSemanticSource {
    pub asset_id: AssetId,
    pub source_revision: String,
    pub path: String,
    pub duration_millis: u64,
}

/// One stored result from an exact CLAP embedding space.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioSemanticSearchHit {
    pub asset_id: String,
    pub start_millis: u64,
    pub score: f64,
}

/// Finds present, speech-empty short assets that still need a CLAP vector.
pub fn list_audio_sources_needing_embedding(
    transaction: &Transaction<'_>,
) -> Result<Vec<AudioSemanticSource>, CatalogError> {
    let mut statement = transaction.prepare(
        "SELECT a.id, a.content_hash, a.path, a.duration_millis \
         FROM assets a WHERE a.path_status = 'present' \
         AND a.duration_millis > 0 AND a.duration_millis <= 10000 \
         AND TRIM(COALESCE(json_extract((SELECT r.value FROM analysis_records r \
             WHERE r.asset_id = a.id AND r.kind = 'transcript' ORDER BY r.id DESC LIMIT 1), \
             '$.text'), '')) = '' \
         AND EXISTS(SELECT 1 FROM analysis_records r WHERE r.asset_id = a.id \
             AND r.kind = 'transcript') \
         AND NOT EXISTS(SELECT 1 FROM audio_semantic_segments s \
             WHERE s.asset_id = a.id AND s.source_revision = \
             ('echo:clap-audio:v1:' || a.content_hash)) \
         ORDER BY a.imported_at_millis, a.id",
    )?;
    let mut rows = statement.query([])?;
    let mut sources = Vec::new();
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let asset_id = id
            .parse()
            .map_err(|error| invalid(&format!("invalid asset id {id}: {error}")))?;
        let duration: i64 = row.get(3)?;
        sources.push(AudioSemanticSource {
            asset_id,
            source_revision: format!("echo:clap-audio:v1:{}", row.get::<_, String>(1)?),
            path: row.get(2)?,
            duration_millis: u64::try_from(duration).map_err(|_| invalid("negative duration"))?,
        });
    }
    Ok(sources)
}

/// Publishes one complete, normalized 512d audio segment vector.
pub fn upsert_audio_semantic_segment(
    transaction: &Transaction<'_>,
    source: &AudioSemanticSource,
    embedding_space: &str,
    values: &[f32],
    runtime: &serde_json::Value,
    updated_at_millis: i64,
) -> Result<(), CatalogError> {
    if embedding_space.trim().is_empty()
        || values.len() != CLAP_DIMENSIONS
        || values.iter().any(|v| !v.is_finite())
    {
        return Err(invalid("invalid CLAP audio embedding"));
    }
    let bytes = values
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect::<Vec<_>>();
    let runtime = serde_json::to_string(runtime).map_err(|e| invalid(&e.to_string()))?;
    transaction.execute(
        "INSERT INTO audio_semantic_segments \
         (asset_id, start_millis, end_millis, source_revision, embedding_space, vector, runtime_json, updated_at_millis) \
         VALUES (?1, 0, ?2, ?3, ?4, ?5, ?6, ?7) \
         ON CONFLICT(asset_id, start_millis, end_millis) DO UPDATE SET \
         source_revision=excluded.source_revision, embedding_space=excluded.embedding_space, \
         vector=excluded.vector, runtime_json=excluded.runtime_json, updated_at_millis=excluded.updated_at_millis",
        rusqlite::params![source.asset_id.to_string(), i64::try_from(source.duration_millis).map_err(|_| invalid("duration exceeds catalog range"))?, source.source_revision, embedding_space, bytes, runtime, updated_at_millis],
    )?;
    Ok(())
}

/// Searches only vectors from the supplied exact CLAP embedding space.
pub fn search_audio_semantic_segments(
    transaction: &Transaction<'_>,
    query: &[f32],
    embedding_space: &str,
    limit: u64,
) -> Result<Vec<AudioSemanticSearchHit>, CatalogError> {
    if query.len() != CLAP_DIMENSIONS
        || query.iter().any(|v| !v.is_finite())
        || embedding_space.trim().is_empty()
    {
        return Err(invalid("invalid CLAP audio query"));
    }
    let mut statement = transaction.prepare(
        "SELECT asset_id, start_millis, vector FROM audio_semantic_segments \
         WHERE embedding_space = ?1",
    )?;
    let mut rows = statement.query([embedding_space])?;
    let mut hits = Vec::new();
    while let Some(row) = rows.next()? {
        let bytes: Vec<u8> = row.get(2)?;
        if bytes.len() != CLAP_DIMENSIONS * 4 {
            return Err(invalid("stored CLAP vector length is invalid"));
        }
        let score = bytes
            .chunks_exact(4)
            .zip(query)
            .map(|(chunk, q)| {
                f64::from(f32::from_le_bytes(chunk.try_into().expect("four bytes"))) * f64::from(*q)
            })
            .sum();
        hits.push(AudioSemanticSearchHit {
            asset_id: row.get(0)?,
            start_millis: u64::try_from(row.get::<_, i64>(1)?)
                .map_err(|_| invalid("negative segment start"))?,
            score,
        });
    }
    hits.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.asset_id.cmp(&b.asset_id))
    });
    hits.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    Ok(hits)
}

fn invalid(message: &str) -> CatalogError {
    CatalogError::new(CatalogErrorKind::Other, message)
}

#[cfg(test)]
mod tests;
