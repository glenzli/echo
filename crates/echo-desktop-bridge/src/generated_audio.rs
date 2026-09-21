//! Opaque generation handles keep review JSON separate from mutation authority.
pub struct GeneratedAudioCandidate(echo_core::GeneratedAudioCandidate);
impl GeneratedAudioCandidate {
    pub(crate) fn details_json(&self) -> Result<String, echo_core::CoreError> {
        self.0.details_json()
    }
}
use std::path::Path;

pub(crate) fn generate_narration(
    text: &str,
    directory: &str,
    endpoint: &str,
) -> Result<Box<GeneratedAudioCandidate>, String> {
    echo_core::generate_narration(
        text,
        Path::new(directory),
        echo_core::InferRuntimeConfig {
            base_url: endpoint.into(),
            credential_path: echo_core::infer_runtime_credential_path().unwrap_or_default(),
        },
    )
    .map(|candidate| Box::new(GeneratedAudioCandidate(candidate)))
    .map_err(|e| e.to_string())
}

pub(crate) fn accept(
    catalog: &str,
    candidate: &GeneratedAudioCandidate,
    assembly: &str,
    global: bool,
) -> Result<String, String> {
    let catalog = echo_catalog::open_catalog(Path::new(catalog)).map_err(|e| e.to_string())?;
    echo_core::accept_generated_audio(&catalog, &candidate.0, assembly, global)
        .map_err(|e| e.to_string())
}

pub(crate) fn generate_sound_material(
    model: &str,
    prompt: &str,
    duration_seconds: u32,
    seed: u32,
    ambience: bool,
    directory: &str,
    endpoint: &str,
) -> Result<Box<GeneratedAudioCandidate>, String> {
    let spec = echo_core::SoundMaterialSpec {
        model: echo_core::SoundMaterialModel::from_choice(model).map_err(|e| e.to_string())?,
        prompt: prompt.trim().into(),
        duration_seconds,
        seed,
        ambience,
    };
    echo_core::generate_sound_material(
        &spec,
        Path::new(directory),
        echo_core::InferRuntimeConfig {
            base_url: endpoint.into(),
            credential_path: echo_core::infer_runtime_credential_path().unwrap_or_default(),
        },
    )
    .map(|candidate| Box::new(GeneratedAudioCandidate(candidate)))
    .map_err(|e| e.to_string())
}
