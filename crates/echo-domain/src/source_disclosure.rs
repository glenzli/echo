//! User-declared source properties. These describe immutable source intervals,
//! not verified recording facts or model execution receipts.
use serde::{Deserialize, Serialize};

/// Why an interval needs provenance disclosure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceDisclosureKind {
    AiProcessed,
    AiGenerated,
    ReconstructedSpeech,
}
impl SourceDisclosureKind {
    #[must_use]
    pub const fn is_generated(self) -> bool {
        matches!(self, Self::AiGenerated | Self::ReconstructedSpeech)
    }
}

/// A declaration in Original milliseconds, independent of memory/material role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceDisclosureSpan {
    pub kind: SourceDisclosureKind,
    pub start_millis: u64,
    pub end_millis: u64,
    pub note: String,
}

/// Validate a complete bounded replacement before appending its revision.
///
/// # Errors
/// Rejects invalid source ranges, excess declarations and oversized notes.
pub fn validate_source_disclosures(
    spans: &[SourceDisclosureSpan],
    duration_millis: u64,
) -> Result<(), &'static str> {
    if spans.len() > 64 {
        return Err("source disclosure exceeds 64 intervals");
    }
    for span in spans {
        if span.start_millis >= span.end_millis || span.end_millis > duration_millis {
            return Err("source disclosure interval is outside the original");
        }
        if span.note.chars().count() > 240 || span.note.contains('\0') {
            return Err("source disclosure note is too long or invalid");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
