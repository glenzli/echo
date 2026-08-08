//! FTS5 transcript search.
//!
//! Chinese has no whitespace, so `unicode61` would treat a whole sentence as
//! one token. The index pre-segments CJK runs into per-character tokens
//! (`小火车` -> `小 火 车`); queries go through the same segmentation and are
//! matched as exact phrases, which makes short queries (`火车`) work too.

use rusqlite::Transaction;

use crate::error::CatalogError;

/// One search hit over the transcript index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchHit {
    pub asset_id: String,
    pub snippet: String,
}

/// Splits a mixed CJK/space-separated string into per-character tokens for
/// CJK runs, preserving Latin words and existing whitespace.
#[must_use]
pub fn segment_cjk(text: &str) -> String {
    let mut segmented = String::with_capacity(text.len() * 2);
    let mut previous_was_cjk = false;
    for character in text.chars() {
        let is_cjk = is_cjk(character);
        if segmented.is_empty() {
            segmented.push(character);
        } else if is_cjk && previous_was_cjk {
            segmented.push(' ');
            segmented.push(character);
        } else if !is_cjk && (character == ' ' || character == '\t' || character == '\n') {
            // Keep existing whitespace as a separator; avoid doubles.
            if !segmented.ends_with(' ') {
                segmented.push(' ');
            }
        } else {
            segmented.push(character);
        }
        previous_was_cjk = is_cjk;
    }
    segmented
}

fn is_cjk(character: char) -> bool {
    matches!(character as u32,
        0x3400..=0x4DBF    // CJK Extension A
        | 0x4E00..=0x9FFF  // CJK Unified
        | 0xF900..=0xFAFF  // CJK Compatibility
        | 0x20000..=0x2A6DF // CJK Extension B
    )
}

/// Replaces the transcript index row for an asset (re-transcription replaces
/// the previous evidence's search entry).
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn index_transcript(
    transaction: &Transaction<'_>,
    asset_id: &str,
    text: &str,
) -> Result<(), CatalogError> {
    transaction.execute("DELETE FROM transcript_fts WHERE asset_id = ?1", [asset_id])?;
    transaction.execute(
        "INSERT INTO transcript_fts (asset_id, text) VALUES (?1, ?2)",
        rusqlite::params![asset_id, segment_cjk(text)],
    )?;
    Ok(())
}

/// Full-text search over indexed transcripts.
///
/// # Errors
///
/// Returns a catalog failure when the query cannot be applied.
pub fn search_transcripts(
    transaction: &Transaction<'_>,
    query: &str,
    limit: u64,
) -> Result<Vec<SearchHit>, CatalogError> {
    let segmented = segment_cjk(query.trim());
    if segmented.is_empty() {
        return Ok(Vec::new());
    }
    let mut statement = transaction.prepare(
        "SELECT asset_id, snippet(transcript_fts, 1, '…', '…', '…', 24) \
         FROM transcript_fts WHERE transcript_fts MATCH ?1 ORDER BY rank LIMIT ?2",
    )?;
    let rows = statement.query_map(
        rusqlite::params![
            format!("\"{segmented}\""),
            i64::try_from(limit).unwrap_or(i64::MAX)
        ],
        |row| {
            Ok(SearchHit {
                asset_id: row.get(0)?,
                snippet: row.get(1)?,
            })
        },
    )?;
    let mut hits = Vec::new();
    for row in rows {
        hits.push(row?);
    }
    Ok(hits)
}

/// Removes the index row for an asset (used when evidence is superseded).
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn remove_transcript_index(
    transaction: &Transaction<'_>,
    asset_id: &str,
) -> Result<(), CatalogError> {
    transaction.execute("DELETE FROM transcript_fts WHERE asset_id = ?1", [asset_id])?;
    Ok(())
}
