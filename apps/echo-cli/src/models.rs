//! Model registry command: reports the presence of every cataloged model in
//! the configured model root (default: the standard HF cache).

use std::path::Path;

use anyhow::Context;
use echo_ai::{ModelStatus, resolve_all};

/// Default model root: the standard `HuggingFace` hub cache layout.
pub(crate) fn default_model_root() -> std::path::PathBuf {
    std::env::var_os("HF_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HF_HUB_CACHE").map(std::path::PathBuf::from))
        .unwrap_or_else(|| {
            std::env::var_os("HOME").map_or_else(
                || std::path::PathBuf::from(".cache/huggingface/hub"),
                |home| std::path::PathBuf::from(home).join(".cache/huggingface/hub"),
            )
        })
}

pub(crate) fn run_models(root: &Path) -> anyhow::Result<()> {
    let resolved = resolve_all(root).context("cannot scan the model root")?;
    for (spec, status) in resolved {
        if spec.backend == echo_ai::InferenceBackend::Ollama {
            let endpoint = std::env::var("ECHO_OLLAMA_ENDPOINT")
                .unwrap_or_else(|_| "http://127.0.0.1:11434".to_owned());
            let model =
                std::env::var("ECHO_OLLAMA_MODEL").unwrap_or_else(|_| "qwen3.5:4b-mlx".to_owned());
            match echo_ai::resolve_ollama_model(&endpoint, &model) {
                Ok(ModelStatus::Present { .. }) => {
                    println!("[ok ] {}  ({endpoint}::{model})", spec.id);
                }
                Ok(ModelStatus::Missing { download_command }) => {
                    println!("[missing] {}  ->  {download_command}", spec.id);
                }
                Err(error) => {
                    println!("[missing] {}  ->  {error}", spec.id);
                }
            }
            continue;
        }
        match status {
            ModelStatus::Present { snapshot } => {
                println!("[ok ] {}  {}", spec.id, snapshot.display());
            }
            ModelStatus::Missing { download_command } => {
                println!("[missing] {}  ->  {download_command}", spec.id);
            }
        }
    }
    Ok(())
}
