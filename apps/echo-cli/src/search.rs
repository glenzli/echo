//! Transcript search command.

use std::path::Path;

use anyhow::Context;
use echo_catalog::{open_catalog, search_transcripts};

pub(crate) fn run_search(catalog_path: &Path, query: &str, limit: u64) -> anyhow::Result<()> {
    let catalog = open_catalog(catalog_path)
        .with_context(|| format!("cannot open catalog at {}", catalog_path.display()))?;
    let hits = catalog
        .with_transaction(|transaction| search_transcripts(transaction, query, limit))
        .context("search failed")?;
    if hits.is_empty() {
        println!("no matches");
        return Ok(());
    }
    for hit in hits {
        println!("{}  {}", hit.asset_id, hit.snippet);
    }
    Ok(())
}
