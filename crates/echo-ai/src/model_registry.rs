//! Model registry: logical model identities mapped to `HuggingFace` repos in a
//! configurable local model root.
//!
//! Echo never downloads models. The user manages a shared HF cache (the
//! default root is the standard `~/.cache/huggingface/hub` layout); this
//! registry resolves a logical model to its local snapshot path and reports
//! missing models with the exact download command.

use std::path::{Path, PathBuf};

use crate::{Capability, InferenceBackend};

/// One cataloged model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelSpec {
    /// Stable logical identity used by routing and the UI.
    pub id: &'static str,
    /// `HuggingFace` repo id (`org/name`), also the cache directory identity.
    pub repo: &'static str,
    /// Optional revision to prefer (full hash or `main`); `None` picks any
    /// available snapshot.
    pub revision: Option<&'static str>,
    pub backend: InferenceBackend,
    pub capability: Capability,
    /// Files that must exist under the snapshot for the model to count as
    /// present.
    pub required_files: &'static [&'static str],
}

/// Every model Echo knows how to use.
pub const MODEL_CATALOG: &[ModelSpec] = &[
    ModelSpec {
        id: "qwen3-asr-1.7b-mlx",
        repo: "mlx-community/Qwen3-ASR-1.7B",
        revision: None,
        backend: InferenceBackend::Mlx,
        capability: Capability::Transcribe,
        required_files: &["config.json", "model.safetensors"],
    },
    ModelSpec {
        id: "qwen3-forced-aligner-0.6b-mlx",
        repo: "mlx-community/Qwen3-ForcedAligner-0.6B",
        revision: None,
        backend: InferenceBackend::Mlx,
        capability: Capability::Align,
        required_files: &["config.json", "model.safetensors"],
    },
    ModelSpec {
        id: "sensevoice-small-mlx",
        repo: "mlx-community/SenseVoiceSmall",
        revision: None,
        backend: InferenceBackend::Mlx,
        capability: Capability::UnderstandAudio,
        required_files: &["config.json", "model.safetensors"],
    },
    ModelSpec {
        id: "whisper-small-ggml",
        repo: "ggerganov/whisper.cpp",
        revision: None,
        backend: InferenceBackend::WhisperCpp,
        capability: Capability::Transcribe,
        required_files: &["ggml-small.bin"],
    },
];

/// Resolution outcome for one model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelStatus {
    /// The model is usable at the given snapshot path.
    Present { snapshot: PathBuf },
    /// The model is absent; `download_command` is the exact `hf download`
    /// invocation the user can run.
    Missing { download_command: String },
}

/// The HF cache directory for one repo.
#[must_use]
pub fn repo_cache_dir(model_root: &Path, spec: &ModelSpec) -> PathBuf {
    let cache_name = format!("models--{}", spec.repo.replace('/', "--"));
    model_root.join(cache_name)
}

/// Resolves `spec` against `model_root`.
///
/// # Errors
///
/// Returns [`std::io::Error`] when the model root cannot be read.
pub fn resolve_model(model_root: &Path, spec: &ModelSpec) -> Result<ModelStatus, std::io::Error> {
    let repo_dir = repo_cache_dir(model_root, spec);
    let snapshots = repo_dir.join("snapshots");
    let mut candidates: Vec<PathBuf> = if snapshots.is_dir() {
        std::fs::read_dir(&snapshots)?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_dir())
            .collect()
    } else {
        Vec::new()
    };
    candidates.sort();
    if let Some(revision) = spec.revision {
        let exact = snapshots.join(revision);
        if let Some(index) = candidates.iter().position(|path| *path == exact) {
            let chosen = candidates.remove(index);
            candidates.insert(0, chosen);
        }
    }
    for candidate in &candidates {
        let present = spec
            .required_files
            .iter()
            .all(|file| candidate.join(file).is_file());
        if present {
            return Ok(ModelStatus::Present {
                snapshot: candidate.clone(),
            });
        }
    }
    Ok(ModelStatus::Missing {
        download_command: format!("hf download {}", spec.repo),
    })
}

/// Resolves every cataloged model; the first failure stops the scan.
///
/// # Errors
///
/// Returns [`std::io::Error`] when the model root cannot be read.
pub fn resolve_all(
    model_root: &Path,
) -> Result<Vec<(&'static ModelSpec, ModelStatus)>, std::io::Error> {
    MODEL_CATALOG
        .iter()
        .map(|spec| resolve_model(model_root, spec).map(|status| (spec, status)))
        .collect()
}
