//! Administrative import for Echo's owner-only Infer Runtime credential.

use std::path::Path;

use anyhow::Context;
use echo_core::InferRuntimeCredentialStore;

pub(crate) fn run_import(source: &Path) -> anyhow::Result<()> {
    let store = InferRuntimeCredentialStore::for_current_user()
        .context("cannot resolve Echo's credential store")?;
    let destination = store
        .import_from(source)
        .context("cannot import Infer Runtime consumer credential")?;
    println!(
        "Infer Runtime consumer credential imported to {} (private directory 0700, file 0600)",
        destination.display()
    );
    Ok(())
}
