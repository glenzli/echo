//! Opaque generation handles keep review JSON separate from mutation authority.
pub struct NarrationCandidate(echo_core::NarrationCandidate);
impl NarrationCandidate {
    pub(crate) fn details_json(&self) -> Result<String, echo_core::CoreError> {
        self.0.details_json()
    }
}
use std::path::Path;

pub(crate) fn generate(
    text: &str,
    directory: &str,
    endpoint: &str,
) -> Result<Box<NarrationCandidate>, String> {
    echo_core::generate_narration(
        text,
        Path::new(directory),
        echo_core::InferRuntimeConfig {
            base_url: endpoint.into(),
            credential_path: echo_core::infer_runtime_credential_path().unwrap_or_default(),
        },
    )
    .map(|candidate| Box::new(NarrationCandidate(candidate)))
    .map_err(|e| e.to_string())
}

pub(crate) fn accept(
    catalog: &str,
    candidate: &NarrationCandidate,
    assembly: &str,
    global: bool,
) -> Result<String, String> {
    let catalog = echo_catalog::open_catalog(Path::new(catalog)).map_err(|e| e.to_string())?;
    echo_core::accept_narration(&catalog, &candidate.0, assembly, global).map_err(|e| e.to_string())
}
