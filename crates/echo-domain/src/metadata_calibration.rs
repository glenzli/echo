//! User-authored calibration over model-derived descriptive metadata.
//!
//! Analysis remains immutable evidence. A calibration stores only fields that
//! differ from the current model projection, so future analysis may improve
//! every field the user did not explicitly correct.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const MAX_METADATA_CAPTION_CHARACTERS: usize = 120;
pub const MAX_METADATA_SUMMARY_CHARACTERS: usize = 1_000;
pub const MAX_METADATA_TEXT_CHARACTERS: usize = 131_072;
pub const MAX_METADATA_LABEL_CHARACTERS: usize = 80;
pub const MAX_METADATA_KEYWORDS: usize = 32;

/// Complete descriptive values presented to a user or supplied as a desired
/// calibration result.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataFields {
    pub sound_caption: String,
    pub summary: String,
    pub event_type: String,
    pub mood: String,
    pub keywords: Vec<String>,
    pub transcript_text: String,
    pub language: String,
}

/// Sparse user-authored overlay. `None` falls back to model evidence while an
/// explicit empty string or empty keyword list intentionally clears a value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataCalibration {
    pub sound_caption: Option<String>,
    pub summary: Option<String>,
    pub event_type: Option<String>,
    pub mood: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub transcript_text: Option<String>,
    pub language: Option<String>,
}

/// Stable field identity used for provenance presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MetadataField {
    SoundCaption,
    Summary,
    EventType,
    Mood,
    Keywords,
    TranscriptText,
    Language,
}

impl MetadataField {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SoundCaption => "sound_caption",
            Self::Summary => "summary",
            Self::EventType => "event_type",
            Self::Mood => "mood",
            Self::Keywords => "keywords",
            Self::TranscriptText => "transcript_text",
            Self::Language => "language",
        }
    }

    #[must_use]
    pub fn from_wire_name(value: &str) -> Option<Self> {
        match value {
            "sound_caption" => Some(Self::SoundCaption),
            "summary" => Some(Self::Summary),
            "event_type" => Some(Self::EventType),
            "mood" => Some(Self::Mood),
            "keywords" => Some(Self::Keywords),
            "transcript_text" => Some(Self::TranscriptText),
            "language" => Some(Self::Language),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataCalibrationError {
    message: String,
}

impl MetadataCalibrationError {
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for MetadataCalibrationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for MetadataCalibrationError {}

impl MetadataFields {
    /// Normalizes and validates user-facing metadata values.
    ///
    /// # Errors
    ///
    /// Returns an error when a field exceeds the bounded product contract.
    pub fn normalized(mut self) -> Result<Self, MetadataCalibrationError> {
        self.sound_caption = normalize_single_line(&self.sound_caption);
        self.summary = normalize_multiline(&self.summary);
        self.event_type = normalize_single_line(&self.event_type);
        self.mood = normalize_single_line(&self.mood);
        self.transcript_text = normalize_multiline(&self.transcript_text);
        self.language = normalize_single_line(&self.language);
        self.keywords = normalize_keywords(self.keywords)?;
        validate_length(
            "sound caption",
            &self.sound_caption,
            MAX_METADATA_CAPTION_CHARACTERS,
        )?;
        validate_length("summary", &self.summary, MAX_METADATA_SUMMARY_CHARACTERS)?;
        validate_length("event", &self.event_type, MAX_METADATA_LABEL_CHARACTERS)?;
        validate_length("mood", &self.mood, MAX_METADATA_LABEL_CHARACTERS)?;
        validate_length("language", &self.language, MAX_METADATA_LABEL_CHARACTERS)?;
        validate_length(
            "transcript text",
            &self.transcript_text,
            MAX_METADATA_TEXT_CHARACTERS,
        )?;
        Ok(self)
    }
}

impl MetadataCalibration {
    /// Produces the sparse overlay needed to turn `model` into `desired`.
    ///
    /// # Errors
    ///
    /// Returns an error when either input violates the bounded metadata
    /// contract.
    pub fn between(
        model: MetadataFields,
        desired: MetadataFields,
    ) -> Result<Self, MetadataCalibrationError> {
        Self::between_with_explicit_fields(model, desired, &[])
    }

    /// Produces a sparse overlay while retaining fields the user explicitly
    /// owns even when their desired value currently equals the model value.
    /// This is what makes an intentional empty value survive later analysis.
    ///
    /// # Errors
    ///
    /// Returns an error when either input violates the bounded metadata
    /// contract.
    pub fn between_with_explicit_fields(
        model: MetadataFields,
        desired: MetadataFields,
        explicit_fields: &[MetadataField],
    ) -> Result<Self, MetadataCalibrationError> {
        let model = model.normalized()?;
        let desired = desired.normalized()?;
        let explicit_fields = explicit_fields.iter().copied().collect::<BTreeSet<_>>();
        Ok(Self {
            sound_caption: different_or_explicit(
                &model.sound_caption,
                &desired.sound_caption,
                explicit_fields.contains(&MetadataField::SoundCaption),
            ),
            summary: different_or_explicit(
                &model.summary,
                &desired.summary,
                explicit_fields.contains(&MetadataField::Summary),
            ),
            event_type: different_or_explicit(
                &model.event_type,
                &desired.event_type,
                explicit_fields.contains(&MetadataField::EventType),
            ),
            mood: different_or_explicit(
                &model.mood,
                &desired.mood,
                explicit_fields.contains(&MetadataField::Mood),
            ),
            keywords: (model.keywords != desired.keywords
                || explicit_fields.contains(&MetadataField::Keywords))
            .then_some(desired.keywords),
            transcript_text: different_or_explicit(
                &model.transcript_text,
                &desired.transcript_text,
                explicit_fields.contains(&MetadataField::TranscriptText),
            ),
            language: different_or_explicit(
                &model.language,
                &desired.language,
                explicit_fields.contains(&MetadataField::Language),
            ),
        })
    }

    #[must_use]
    pub fn apply_to(&self, mut model: MetadataFields) -> MetadataFields {
        if let Some(value) = &self.sound_caption {
            model.sound_caption.clone_from(value);
        }
        if let Some(value) = &self.summary {
            model.summary.clone_from(value);
        }
        if let Some(value) = &self.event_type {
            model.event_type.clone_from(value);
        }
        if let Some(value) = &self.mood {
            model.mood.clone_from(value);
        }
        if let Some(value) = &self.keywords {
            model.keywords.clone_from(value);
        }
        if let Some(value) = &self.transcript_text {
            model.transcript_text.clone_from(value);
        }
        if let Some(value) = &self.language {
            model.language.clone_from(value);
        }
        model
    }

    #[must_use]
    pub fn calibrated_fields(&self) -> Vec<MetadataField> {
        [
            (MetadataField::SoundCaption, self.sound_caption.is_some()),
            (MetadataField::Summary, self.summary.is_some()),
            (MetadataField::EventType, self.event_type.is_some()),
            (MetadataField::Mood, self.mood.is_some()),
            (MetadataField::Keywords, self.keywords.is_some()),
            (
                MetadataField::TranscriptText,
                self.transcript_text.is_some(),
            ),
            (MetadataField::Language, self.language.is_some()),
        ]
        .into_iter()
        .filter_map(|(field, present)| present.then_some(field))
        .collect()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.calibrated_fields().is_empty()
    }
}

fn different_or_explicit(model: &str, desired: &str, explicit: bool) -> Option<String> {
    (model != desired || explicit).then(|| desired.to_owned())
}

fn normalize_single_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_multiline(value: &str) -> String {
    value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

fn normalize_keywords(values: Vec<String>) -> Result<Vec<String>, MetadataCalibrationError> {
    if values.len() > MAX_METADATA_KEYWORDS {
        return Err(invalid(format!(
            "metadata has more than {MAX_METADATA_KEYWORDS} keywords"
        )));
    }
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let value = normalize_single_line(&value);
        if value.is_empty() {
            continue;
        }
        validate_length("keyword", &value, MAX_METADATA_LABEL_CHARACTERS)?;
        if seen.insert(value.to_lowercase()) {
            normalized.push(value);
        }
    }
    Ok(normalized)
}

fn validate_length(
    field: &str,
    value: &str,
    maximum: usize,
) -> Result<(), MetadataCalibrationError> {
    if value.chars().count() > maximum {
        return Err(invalid(format!(
            "metadata {field} exceeds {maximum} characters"
        )));
    }
    Ok(())
}

fn invalid(message: String) -> MetadataCalibrationError {
    MetadataCalibrationError { message }
}

#[cfg(test)]
mod tests;
