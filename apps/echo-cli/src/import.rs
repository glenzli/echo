//! Asset import command.

use std::path::Path;

use anyhow::Context;
use echo_core::{ImportOutcome, import_asset};

pub(crate) fn run_import(catalog: &Path, source: &Path) -> anyhow::Result<()> {
    match import_asset(catalog, source) {
        Ok(ImportOutcome::Imported(asset)) => {
            println!(
                "imported {} (id {}, {} bytes, level {:?})",
                source.display(),
                asset.id,
                asset.original.size_bytes,
                asset.max_level
            );
        }
        Ok(ImportOutcome::AlreadyPresent(asset)) => {
            println!(
                "already present: {} (id {}, path refreshed)",
                source.display(),
                asset.id
            );
        }
        Err(error) => {
            return Err(anyhow::anyhow!(
                "import failed for {}: {error}",
                source.display()
            ))
            .context("import");
        }
    }
    Ok(())
}
