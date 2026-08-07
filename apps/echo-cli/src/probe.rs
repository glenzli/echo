//! Audio-engine probe command.

use std::path::Path;

use anyhow::Context;

pub(crate) fn run_probe(source: &Path) -> anyhow::Result<()> {
    let result =
        echo_bridge::probe(source).with_context(|| format!("cannot probe {}", source.display()))?;
    if !result.has_audio {
        println!("no audio stream in {}", source.display());
        return Ok(());
    }
    println!(
        "{}: {} ({}), {} Hz, {} channel(s), {} ms",
        source.display(),
        result.codec_name,
        result.container_format,
        result.sample_rate,
        result.channel_count,
        result.duration_millis
    );
    Ok(())
}
