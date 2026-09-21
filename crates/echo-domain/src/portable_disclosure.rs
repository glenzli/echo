//! Compact, unauthenticated source labels carried by exported audio containers.
//! Only source kinds travel: never local paths, private notes or inferred facts.
use crate::SourceDisclosureKind;
use serde::{Deserialize, Serialize};

const PREFIX: &str = "Echo source disclosure: ";
const SCHEMA: &str = "echo.source-disclosure.v1";
const SCOPE: &str = "referenced_sources";
const MAX_BYTES: usize = 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Declaration {
    schema: String,
    scope: String,
    kinds: Vec<SourceDisclosureKind>,
}

/// Encode a deterministic, bounded union of declared source kinds.
/// An empty union means unmarked, never verified recording.
#[must_use]
pub fn encode_portable_disclosure(kinds: impl IntoIterator<Item = SourceDisclosureKind>) -> String {
    let mut present = [false; 3];
    for kind in kinds {
        present[match kind {
            SourceDisclosureKind::AiProcessed => 0,
            SourceDisclosureKind::AiGenerated => 1,
            SourceDisclosureKind::ReconstructedSpeech => 2,
        }] = true;
    }
    let kinds = [
        SourceDisclosureKind::AiProcessed,
        SourceDisclosureKind::AiGenerated,
        SourceDisclosureKind::ReconstructedSpeech,
    ]
    .into_iter()
    .zip(present)
    .filter_map(|(kind, present)| present.then_some(kind))
    .collect();
    let declaration = Declaration {
        schema: SCHEMA.into(),
        scope: SCOPE.into(),
        kinds,
    };
    // This struct contains strings and enum values only; serialization cannot fail.
    format!(
        "{PREFIX}{}",
        serde_json::to_string(&declaration).expect("portable disclosure is serializable")
    )
}

/// Recognize only supported, bounded Echo declarations. Other comments and
/// malformed/future declarations cannot upgrade a source to a known category.
#[must_use]
pub fn decode_portable_disclosure(comment: &str) -> Option<Vec<SourceDisclosureKind>> {
    if comment.len() > 32768 {
        return None;
    }
    let comment = comment.lines().next()?;
    if comment.len() > MAX_BYTES {
        return None;
    }
    let value: Declaration = serde_json::from_str(comment.strip_prefix(PREFIX)?).ok()?;
    if value.schema != SCHEMA || value.scope != SCOPE || value.kinds.len() > 3 {
        return None;
    }
    for (i, kind) in value.kinds.iter().enumerate() {
        if value.kinds[..i].contains(kind) {
            return None;
        }
    }
    Some(value.kinds)
}

#[cfg(test)]
mod tests;
