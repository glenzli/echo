//! Audio-engine probe command.

use std::path::Path;

use anyhow::Context;

pub(crate) fn run_probe(source: &Path, json: bool) -> anyhow::Result<()> {
    let result =
        echo_bridge::probe(source).with_context(|| format!("cannot probe {}", source.display()))?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "schema_version": 1,
                "has_audio": result.has_audio,
                "codec_name": result.codec_name,
                "container_format": result.container_format,
                "sample_rate_hz": result.sample_rate,
                "channels": result.channel_count,
                "duration_ms": result.duration_millis,
            })
        );
        return Ok(());
    }
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
