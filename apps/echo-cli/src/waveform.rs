//! Waveform artifact command: build the pyramid and publish it into the cache.

use std::path::Path;

use anyhow::Context;
use echo_core::build_and_cache_waveform;

pub(crate) fn run_waveform(cache: &Path, source: &Path) -> anyhow::Result<()> {
    let artifact = build_and_cache_waveform(source, cache, 8)
        .with_context(|| format!("cannot build waveform for {}", source.display()))?;
    println!(
        "waveform: {} level(s), {} base buckets, {} Hz canonical, digest {}",
        artifact.level_count,
        artifact.bucket_count,
        artifact.canonical_sample_rate,
        artifact.content_hash
    );
    Ok(())
}
