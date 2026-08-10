//! Text-evidence documents and a compact local semantic vector index.
//!
//! Runtime owns embedding execution; Echo owns source revision, quantization,
//! exact embedding-space matching, hybrid retrieval, and stale publication.

use echo_domain::AssetId;
use rusqlite::Transaction;

use crate::{CatalogError, segment_cjk};

const MAX_DOCUMENT_BYTES: usize = 4_096;
const MAX_EMBEDDING_DIMENSIONS: usize = 4_096;

/// Bounded evidence text whose immutable analysis ids form its revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticSource {
    pub asset_id: AssetId,
    pub revision: String,
    pub text: String,
}

/// Persisted semantic vector publication.
#[derive(Debug)]
pub struct UpsertSemanticDocument<'a> {
    pub source: &'a SemanticSource,
    pub embedding_space: &'a str,
    pub values: &'a [f32],
    pub runtime: &'a serde_json::Value,
    pub updated_at_millis: i64,
}

/// One approximate cosine result from the compact vector index.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticSearchHit {
    pub asset_id: String,
    pub score: f64,
}

/// Replaces the literal index row for one current semantic source without
/// waiting for Runtime vector execution.
///
/// # Errors
///
/// Returns a catalog failure when the FTS projection cannot be updated.
pub fn index_semantic_source_text(
    transaction: &Transaction<'_>,
    source: &SemanticSource,
) -> Result<(), CatalogError> {
    transaction.execute(
        "DELETE FROM semantic_document_fts WHERE asset_id = ?1",
        [source.asset_id.to_string()],
    )?;
    transaction.execute(
        "INSERT INTO semantic_document_fts (asset_id, text) VALUES (?1, ?2)",
        rusqlite::params![source.asset_id.to_string(), segment_cjk(&source.text)],
    )?;
    Ok(())
}

/// Lists current contextual documents whose exact source revision is not yet
/// represented in the semantic index.
///
/// # Errors
///
/// Returns a catalog failure when stored evidence cannot be read.
pub fn list_semantic_sources_needing_embedding(
    transaction: &Transaction<'_>,
) -> Result<Vec<SemanticSource>, CatalogError> {
    semantic_sources(transaction, true, None)
}

/// Returns the current bounded semantic source for one asset.
///
/// # Errors
///
/// Returns a catalog failure when stored evidence cannot be read.
pub fn semantic_source(
    transaction: &Transaction<'_>,
    asset_id: AssetId,
) -> Result<Option<SemanticSource>, CatalogError> {
    Ok(semantic_sources(transaction, false, Some(asset_id))?
        .into_iter()
        .next())
}

fn semantic_sources(
    transaction: &Transaction<'_>,
    only_stale: bool,
    asset_id: Option<AssetId>,
) -> Result<Vec<SemanticSource>, CatalogError> {
    let asset_filter = if asset_id.is_some() {
        "AND a.id = ?1"
    } else {
        ""
    };
    let sql = format!(
        "SELECT a.id, contextual.id, COALESCE(transcript.id, 0), \
         COALESCE(json_extract(contextual.value, '$.sound_caption'), ''), \
         COALESCE(json_extract(contextual.value, '$.summary'), ''), \
         COALESCE(json_extract(contextual.value, '$.keywords'), '[]'), \
         COALESCE(json_extract(contextual.value, '$.mood'), ''), \
         COALESCE(json_extract(contextual.value, '$.place_hint'), ''), \
         COALESCE(json_extract(contextual.value, '$.event_type'), ''), \
         COALESCE(json_extract(contextual.value, '$.people_hints'), '[]'), \
         COALESCE(substr(json_extract(transcript.value, '$.text'), 1, 2048), ''), \
         COALESCE(document.source_revision, '') \
         FROM assets a \
         JOIN analysis_records contextual ON contextual.id = (\
             SELECT candidate.id FROM analysis_records candidate \
             WHERE candidate.asset_id = a.id AND candidate.kind = 'contextual' \
             ORDER BY candidate.id DESC LIMIT 1\
         ) \
         LEFT JOIN analysis_records transcript ON transcript.id = (\
             SELECT candidate.id FROM analysis_records candidate \
             WHERE candidate.asset_id = a.id AND candidate.kind = 'transcript' \
             ORDER BY candidate.id DESC LIMIT 1\
         ) \
         LEFT JOIN semantic_documents document ON document.asset_id = a.id \
         WHERE a.path_status = 'present' \
         AND TRIM(COALESCE(json_extract(contextual.value, '$.sound_caption'), '')) <> '' \
         {asset_filter} ORDER BY a.imported_at_millis, a.id"
    );
    let mut statement = transaction.prepare(&sql)?;
    let mut rows = if let Some(asset_id) = asset_id {
        statement.query([asset_id.to_string()])?
    } else {
        statement.query([])?
    };
    let mut sources = Vec::new();
    while let Some(row) = rows.next()? {
        let id_text: String = row.get(0)?;
        let asset_id = id_text.parse::<AssetId>().map_err(|error| {
            CatalogError::new(
                crate::CatalogErrorKind::Other,
                format!("invalid stored asset id {id_text}: {error}"),
            )
        })?;
        let contextual_id: i64 = row.get(1)?;
        let transcript_id: i64 = row.get(2)?;
        let revision =
            format!("echo:semantic-document:v1:{asset_id}:{contextual_id}:{transcript_id}");
        let stored_revision: String = row.get(11)?;
        if only_stale && stored_revision == revision {
            continue;
        }
        let keywords_json = row.get::<_, String>(5)?;
        let people_json = row.get::<_, String>(9)?;
        let keywords = json_strings(&keywords_json);
        let people = json_strings(&people_json);
        let text = bounded_document([
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            keywords.join(" "),
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            people.join(" "),
            row.get::<_, String>(10)?,
        ]);
        sources.push(SemanticSource {
            asset_id,
            revision,
            text,
        });
    }
    Ok(sources)
}

/// Atomically publishes one quantized vector and its richer text index row.
///
/// # Errors
///
/// Returns a catalog failure for invalid dimensions, vector values, or JSON.
pub fn upsert_semantic_document(
    transaction: &Transaction<'_>,
    input: &UpsertSemanticDocument<'_>,
) -> Result<(), CatalogError> {
    let (vector, norm_sq) = quantize(input.values)?;
    if input.embedding_space.trim().is_empty() || input.source.text.trim().is_empty() {
        return Err(invalid("semantic document identity and text are required"));
    }
    let runtime = serde_json::to_string(input.runtime)
        .map_err(|error| invalid(&format!("cannot encode semantic provenance: {error}")))?;
    let dimensions = i64::try_from(input.values.len())
        .map_err(|_| invalid("semantic embedding dimensions exceed catalog range"))?;
    transaction.execute(
        "INSERT INTO semantic_documents \
         (asset_id, source_revision, document_text, embedding_space, dimensions, \
          quantized_vector, vector_norm_sq, runtime_json, updated_at_millis) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
         ON CONFLICT(asset_id) DO UPDATE SET source_revision = excluded.source_revision, \
         document_text = excluded.document_text, embedding_space = excluded.embedding_space, \
         dimensions = excluded.dimensions, quantized_vector = excluded.quantized_vector, \
         vector_norm_sq = excluded.vector_norm_sq, runtime_json = excluded.runtime_json, \
         updated_at_millis = excluded.updated_at_millis",
        rusqlite::params![
            input.source.asset_id.to_string(),
            input.source.revision,
            input.source.text,
            input.embedding_space,
            dimensions,
            vector,
            norm_sq,
            runtime,
            input.updated_at_millis,
        ],
    )?;
    index_semantic_source_text(transaction, input.source)?;
    Ok(())
}

/// Searches only vectors from the exact embedding-space identity.
///
/// # Errors
///
/// Returns a catalog failure for an invalid query or corrupt stored vector.
pub fn search_semantic_documents(
    transaction: &Transaction<'_>,
    query: &[f32],
    embedding_space: &str,
    limit: u64,
) -> Result<Vec<SemanticSearchHit>, CatalogError> {
    let (query, query_norm_sq) = quantize(query)?;
    let dimensions = i64::try_from(query.len())
        .map_err(|_| invalid("semantic query dimensions exceed catalog range"))?;
    let mut statement = transaction.prepare(
        "SELECT document.asset_id, document.quantized_vector, document.vector_norm_sq \
         FROM semantic_documents document JOIN assets asset ON asset.id = document.asset_id \
         WHERE document.embedding_space = ?1 AND document.dimensions = ?2 \
         AND asset.path_status = 'present'",
    )?;
    let rows = statement.query_map(rusqlite::params![embedding_space, dimensions], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Vec<u8>>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;
    let mut hits = Vec::new();
    for row in rows {
        let (asset_id, vector, norm_sq) = row?;
        if vector.len() != query.len() || norm_sq <= 0 {
            return Err(invalid("stored semantic vector is corrupt"));
        }
        let dot = vector
            .iter()
            .zip(&query)
            .map(|(left, right)| i64::from(left.cast_signed()) * i64::from(right.cast_signed()))
            .sum::<i64>();
        // Values are bounded to 4,096 dimensions of signed 8-bit samples, so
        // each integer is exactly representable in f64 before cosine math.
        #[allow(clippy::cast_precision_loss)]
        let denominator = ((norm_sq as f64) * (query_norm_sq as f64)).sqrt();
        #[allow(clippy::cast_precision_loss)]
        let score = dot as f64 / denominator;
        hits.push(SemanticSearchHit { asset_id, score });
    }
    hits.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.asset_id.cmp(&right.asset_id))
    });
    hits.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    Ok(hits)
}

#[allow(clippy::cast_possible_truncation)]
fn quantize(values: &[f32]) -> Result<(Vec<u8>, i64), CatalogError> {
    if values.is_empty() || values.len() > MAX_EMBEDDING_DIMENSIONS {
        return Err(invalid("semantic embedding dimensions are invalid"));
    }
    let mut norm_sq = 0i64;
    let vector = values
        .iter()
        .map(|value| {
            if !value.is_finite() {
                return Err(invalid("semantic embedding contains a non-finite value"));
            }
            let quantized = (value.clamp(-1.0, 1.0) * 127.0).round() as i8;
            norm_sq += i64::from(quantized) * i64::from(quantized);
            Ok(quantized.cast_unsigned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if norm_sq == 0 {
        return Err(invalid("semantic embedding has zero norm"));
    }
    Ok((vector, norm_sq))
}

fn bounded_document(parts: impl IntoIterator<Item = String>) -> String {
    let joined = parts
        .into_iter()
        .map(|part| part.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(". ");
    if joined.len() <= MAX_DOCUMENT_BYTES {
        return joined;
    }
    let mut end = MAX_DOCUMENT_BYTES;
    while !joined.is_char_boundary(end) {
        end -= 1;
    }
    joined[..end].trim().to_owned()
}

fn json_strings(value: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(value).unwrap_or_default()
}

fn invalid(message: &str) -> CatalogError {
    CatalogError::new(crate::CatalogErrorKind::Other, message)
}

#[cfg(test)]
mod tests;
