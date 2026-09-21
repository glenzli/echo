//! User-authored memory context, separate from embedded tags and AI evidence.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryInfo {
    pub notes: String,
    pub place: String,
    /// Deliberately descriptive: approximate dates must not become false timestamps.
    pub time_description: String,
    pub moments: Vec<MemoryMoment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemoryMoment {
    pub start_millis: u64,
    pub end_millis: Option<u64>,
    pub note: String,
}

impl MemoryInfo {
    /// Validate and normalize user text and immutable source-time anchors.
    /// # Errors
    /// Rejects excess text, control characters and invalid source coordinates.
    pub fn normalized(mut self, duration: Option<u64>) -> Result<Self, &'static str> {
        fn text(value: &str, limit: usize, multiline: bool) -> Result<String, &'static str> {
            let value = value.trim().replace("\r\n", "\n");
            if value.chars().count() > limit
                || value
                    .chars()
                    .any(|c| c.is_control() && !(multiline && matches!(c, '\n' | '\t')))
            {
                return Err("memory text is too long or contains invalid characters");
            }
            Ok(value)
        }
        self.notes = text(&self.notes, 2000, true)?;
        self.place = text(&self.place, 200, false)?;
        self.time_description = text(&self.time_description, 120, false)?;
        if self.moments.len() > 128 || (duration.is_none() && !self.moments.is_empty()) {
            return Err("memory moments require an original recording and at most 128 entries");
        }
        for moment in &mut self.moments {
            moment.note = text(&moment.note, 500, true)?;
            if moment.note.is_empty()
                || moment.start_millis >= duration.unwrap_or(0)
                || moment
                    .end_millis
                    .is_some_and(|end| end <= moment.start_millis || end > duration.unwrap_or(0))
            {
                return Err("memory moment is empty or outside the original recording");
            }
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests;
