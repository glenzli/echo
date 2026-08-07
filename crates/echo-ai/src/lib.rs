//! AI capability routing and inference backend identities.
//!
//! The business layer never knows which GPU or runtime produces evidence:
//! a capability request maps to a backend through this crate's routing
//! contract. Backend implementations (`MLX`, `CoreML`, `CUDA`, `ONNX Runtime`,
//! whisper.cpp, Cloud) arrive with their first real consumer; this crate owns
//! the stable vocabulary and the routing table only.

use serde::{Deserialize, Serialize};

/// An analysis capability Echo can request from an inference backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Speech-to-text transcription.
    Transcribe,
    /// Word/sentence forced alignment.
    Align,
    /// Emotion and audio-event recognition (e.g. `SenseVoice`).
    UnderstandAudio,
    /// Speaker diarization.
    Diarize,
    /// Text or audio embedding for semantic search (e.g. CLAP).
    Embed,
}

/// A concrete inference backend family. Identity only: capability-specific
/// configuration lives with the backend implementation when one exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InferenceBackend {
    /// Apple Silicon native runtime (macOS-only).
    Mlx,
    /// Apple Neural Engine / Core ML (macOS-only).
    CoreMl,
    /// NVIDIA CUDA.
    Cuda,
    /// Cross-hardware ONNX Runtime with execution providers.
    OnnxRuntime,
    /// Stable baseline transcription engine.
    WhisperCpp,
    /// Remote model service.
    Cloud,
}

/// One supported mapping from capability to backend. Later versions of a
/// backend replace earlier ones by lowering a preference rank.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityRoute {
    pub capability: Capability,
    pub backend: InferenceBackend,
    /// Higher means preferred; ties resolve to the first declared route.
    pub preference: u8,
}

/// Declares the supported routes for one deployment. Routing is data, not
/// code: a minimal install declares only `WhisperCpp`, an enhanced install
/// adds MLX/CoreML routes.
pub const ROUTES_MINIMAL: &[CapabilityRoute] = &[CapabilityRoute {
    capability: Capability::Transcribe,
    backend: InferenceBackend::WhisperCpp,
    preference: 1,
}];

/// Resolves the preferred backend for a capability from an ordered route list.
#[must_use]
pub fn resolve(capability: Capability, routes: &[CapabilityRoute]) -> Option<InferenceBackend> {
    routes
        .iter()
        .filter(|route| route.capability == capability)
        .max_by_key(|route| route.preference)
        .map(|route| route.backend)
}
