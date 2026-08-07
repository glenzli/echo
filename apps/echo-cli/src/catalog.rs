//! Catalog initialization command.

use std::path::Path;

use anyhow::Context;
use echo_catalog::open_catalog;

pub(crate) fn run_init(catalog: &Path) -> anyhow::Result<()> {
    let catalog = open_catalog(catalog)
        .with_context(|| format!("cannot open catalog at {}", catalog.display()))?;
    let stats = catalog.stats().context("cannot read catalog statistics")?;
    println!(
        "catalog ready: {} assets, {} analysis records, schema v{}",
        stats.asset_count, stats.analysis_record_count, stats.schema_version
    );
    Ok(())
}
