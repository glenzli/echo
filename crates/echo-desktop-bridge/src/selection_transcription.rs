//! Explicit editor inference and scoped evidence projection, independent of library jobs.
use crate::session::LibrarySession;
use echo_domain::{AnalysisKind, AssetId};
use std::{path::Path, str::FromStr};

pub(crate) fn transcribe(
    catalog: &str,
    cache: &str,
    id: &str,
    start: u64,
    end: u64,
    endpoint: &str,
) -> Result<String, String> {
    let catalog = echo_catalog::open_catalog(Path::new(catalog)).map_err(|e| e.to_string())?;
    let id = AssetId::from_str(id).map_err(|e| e.to_string())?;
    let result = echo_core::transcribe_selection(
        &catalog,
        Path::new(cache),
        id,
        start,
        end,
        echo_core::InferRuntimeConfig {
            base_url: endpoint.to_owned(),
            credential_path: echo_core::infer_runtime_credential_path().unwrap_or_default(),
        },
    )
    .map_err(|e| e.to_string())?;
    serde_json::to_string(&result).map_err(|e| e.to_string())
}
impl LibrarySession {
    pub(crate) fn accept_selection_transcript(&self, id: &str, value: &str) -> Result<(), String> {
        let id = AssetId::from_str(id).map_err(|e| e.to_string())?;
        let result: echo_core::SelectionTranscript =
            serde_json::from_str(value).map_err(|e| e.to_string())?;
        echo_core::record_selection_transcript(&self.catalog, id, &result)
            .map_err(|e| e.to_string())
    }
    pub(crate) fn selection_transcripts(&self, id: &str) -> Result<String, String> {
        let id = AssetId::from_str(id).map_err(|e| e.to_string())?;
        let records = self
            .catalog
            .with_transaction(|tx| echo_catalog::query_analysis(tx, id))
            .map_err(|e| e.to_string())?;
        let results: Vec<_> = records
            .into_iter()
            .filter(|r| r.kind == AnalysisKind::SelectionTranscript)
            .filter_map(|r| serde_json::from_value::<echo_core::SelectionTranscript>(r.value).ok())
            .filter(|r| r.validate().is_ok())
            .collect();
        serde_json::to_string(&results).map_err(|e| e.to_string())
    }
}
