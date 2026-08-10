//! Transcription command: import the source, submit the product intent to
//! Infer Runtime, and record transcript plus alignment evidence.

use std::path::Path;

use anyhow::Context;
use echo_catalog::open_catalog;
use echo_core::{
    AlignmentIntent, ImportOutcome, InferRuntimeClient, InferRuntimeConfig, TranscriptionIntent,
    import_asset, load_infer_runtime_credential, record_alignment, record_runtime_transcript,
};

pub(crate) fn run_transcribe(
    catalog_path: &Path,
    source: &Path,
    runtime_endpoint: &str,
) -> anyhow::Result<()> {
    let asset = match import_asset(catalog_path, source)? {
        ImportOutcome::Imported(asset) | ImportOutcome::AlreadyPresent(asset) => asset,
    };
    println!("asset {} ready, transcribing...", asset.id);

    let credential = load_infer_runtime_credential()
        .context("Echo has no usable Infer Runtime consumer credential")?;
    let client = InferRuntimeClient::new(InferRuntimeConfig {
        base_url: runtime_endpoint.to_owned(),
        bearer_token: credential.into_bearer_token(),
    });
    let payload = client
        .transcribe(&asset.original.path, &TranscriptionIntent::default())
        .context("Runtime transcription failed")?;
    let catalog = open_catalog(catalog_path)?;
    record_runtime_transcript(&catalog, asset.id, &payload)?;
    let alignment = client
        .align(
            &asset.original.path,
            &payload.text,
            &AlignmentIntent {
                language: payload.language.clone(),
                ..AlignmentIntent::default()
            },
        )
        .context("Runtime alignment failed")?;
    record_alignment(&catalog, asset.id, &alignment)?;

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

pub(crate) fn default_runtime_endpoint() -> String {
    std::env::var("ECHO_INFER_ENDPOINT").unwrap_or_default()
}
