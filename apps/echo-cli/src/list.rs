//! Library listing command.

use std::path::Path;

use anyhow::Context;
use echo_catalog::{list_assets, open_catalog};

pub(crate) fn run_list(catalog: &Path) -> anyhow::Result<()> {
    let catalog = open_catalog(catalog)
        .with_context(|| format!("cannot open catalog at {}", catalog.display()))?;
    let assets = catalog
        .with_transaction(list_assets)
        .context("cannot list assets")?;
    if assets.is_empty() {
        println!("no assets registered");
        return Ok(());
    }
    for asset in assets {
        let duration = asset
            .original
            .duration_millis
            .map_or("unknown".to_owned(), |millis| {
                let whole_seconds = millis / 1000;
                let fraction = millis % 1000;
                format!("{whole_seconds}.{fraction:03}s")
            });
        println!(
            "{}  {}  {}  {:?}",
            asset.id,
            asset.original.path.display(),
            duration,
            asset.max_level
        );
    }
    Ok(())
}
