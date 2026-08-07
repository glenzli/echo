//! Transcription command: import the source, resolve the ASR model, run the
//! worker, and record the transcript as analysis evidence.

use std::path::{Path, PathBuf};

use anyhow::Context;
use echo_ai::{ModelStatus, resolve_model};
use echo_catalog::open_catalog;
use echo_core::run_transcribe as run_asr_worker;
use echo_core::{ImportOutcome, TranscribeWorker, import_asset, record_transcript};

pub(crate) fn run_transcribe(
    catalog_path: &Path,
    source: &Path,
    model_root: &Path,
    python: &Path,
    worker_script: &Path,
) -> anyhow::Result<()> {
    let asset = match import_asset(catalog_path, source)? {
        ImportOutcome::Imported(asset) | ImportOutcome::AlreadyPresent(asset) => asset,
    };
    println!("asset {} ready, transcribing...", asset.id);

    let asr_spec = echo_ai::MODEL_CATALOG
        .iter()
        .find(|spec| spec.id == "qwen3-asr-1.7b-mlx")
        .expect("catalog declares qwen3-asr");
    let snapshot = match resolve_model(model_root, asr_spec).context("cannot resolve models")? {
        ModelStatus::Present { snapshot } => snapshot,
        ModelStatus::Missing { download_command } => {
            anyhow::bail!("ASR model missing; run: {download_command}");
        }
    };

    let worker = TranscribeWorker {
        python: python.to_owned(),
        script: worker_script.to_owned(),
    };
    let payload =
        run_asr_worker(&asset.original.path, &snapshot, &worker).context("transcription failed")?;
    let version = snapshot
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_owned();
    record_transcript(&open_catalog(catalog_path)?, asset.id, &payload, &version)?;

    println!(
        "transcribed {} segments, {} chars (language {:?})",
        payload.segments.len(),
        payload.text.chars().count(),
        payload.language
    );
    for segment in &payload.segments {
        println!(
            "  [{:.2}-{:.2}] {}",
            segment.start, segment.end, segment.text
        );
    }
    Ok(())
}

pub(crate) fn default_python() -> PathBuf {
    std::env::var_os("ECHO_MLX_PYTHON").map_or_else(|| PathBuf::from("python3"), PathBuf::from)
}
