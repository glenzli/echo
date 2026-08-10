//! Ephemeral natural-language query over Echo-owned semantic documents.

use std::path::Path;

use anyhow::Context;
use echo_catalog::open_catalog;
use echo_core::{InferRuntimeConfig, load_infer_runtime_credential};

pub(crate) fn run_search(
    catalog_path: &Path,
    query: &str,
    runtime_endpoint: &str,
    limit: u64,
) -> anyhow::Result<()> {
    let catalog = open_catalog(catalog_path)
        .with_context(|| format!("cannot open catalog at {}", catalog_path.display()))?;
    let credential = load_infer_runtime_credential()
        .context("Echo has no usable Infer Runtime consumer credential")?;
    let hits = echo_core::semantic_search(
        &catalog,
        &InferRuntimeConfig {
            base_url: runtime_endpoint.to_owned(),
            bearer_token: credential.into_bearer_token(),
        },
        query,
        limit,
    )
    .context("semantic search failed")?;
    if hits.is_empty() {
        println!("no indexed semantic matches");
    }
    for hit in hits {
        println!("{}  {:.4}", hit.asset_id, hit.score);
    }
    Ok(())
}
