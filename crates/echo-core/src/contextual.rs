//! Contextual understanding evidence. Runtime does not yet expose the
//! structured-output contract Echo needs, so execution remains deliberately
//! unavailable instead of falling back to a naked model call.

use echo_catalog::{AppendAnalysisRecord, record_analysis};
use echo_domain::{AnalysisKind, AnalysisRecord, AssetId, ModelIdentity};
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreErrorKind};

/// The canonical contextual payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextualPayload {
    pub summary: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub mood: Option<String>,
    #[serde(default)]
    pub place_hint: Option<String>,
    #[serde(default)]
    pub event_type: Option<String>,
    #[serde(default)]
    pub people_hints: Vec<String>,
}

/// Records contextual evidence for an asset.
///
/// # Errors
///
/// Returns a catalog failure when the write cannot be applied.
pub fn record_contextual(
    catalog: &echo_catalog::Catalog,
    asset_id: AssetId,
    payload: &ContextualPayload,
    model_name: &str,
) -> Result<(), CoreError> {
    let value = serde_json::to_value(payload).map_err(|error| {
        CoreError::new(
            CoreErrorKind::Other,
            format!("cannot encode contextual payload: {error}"),
        )
    })?;
    let now = crate::import::now_millis();
    catalog
        .with_transaction(|transaction| {
            record_analysis(
                transaction,
                &AppendAnalysisRecord {
                    asset_id,
                    record: AnalysisRecord::new(
                        AnalysisKind::Contextual,
                        value,
                        ModelIdentity::new(model_name.to_owned(), "ollama".to_owned()),
                        None,
                        now,
                    ),
                },
            )
        })
        .map_err(CoreError::from)
}

#[cfg(test)]
mod tests;
