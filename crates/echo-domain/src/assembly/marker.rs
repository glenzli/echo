//! User-authored navigation annotations in composition time, independent of audio duration.

use serde::{Deserialize, Serialize};

use super::{MAX_ASSEMBLY_DURATION_MILLIS, SoundAssemblyError, validate_name};
use crate::AssemblyMarkerId;

pub const MAX_ASSEMBLY_MARKERS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssemblyMarker {
    id: AssemblyMarkerId,
    name: String,
    start_millis: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    end_millis: Option<u64>,
}

impl AssemblyMarker {
    /// Creates a named point or non-empty range within the four-hour composition coordinate space.
    ///
    /// # Errors
    /// Returns an error for an empty/long name or invalid time bounds.
    pub fn new(
        id: AssemblyMarkerId,
        name: String,
        start_millis: u64,
        end_millis: Option<u64>,
    ) -> Result<Self, SoundAssemblyError> {
        let marker = Self {
            id,
            name,
            start_millis,
            end_millis,
        };
        marker.validate()?;
        Ok(marker)
    }

    #[must_use]
    pub const fn id(&self) -> AssemblyMarkerId {
        self.id
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub const fn start_millis(&self) -> u64 {
        self.start_millis
    }
    #[must_use]
    pub const fn end_millis(&self) -> Option<u64> {
        self.end_millis
    }

    pub(super) fn validate(&self) -> Result<(), SoundAssemblyError> {
        validate_name(&self.name, 120, SoundAssemblyError::InvalidMarker)?;
        if self.start_millis > MAX_ASSEMBLY_DURATION_MILLIS
            || self
                .end_millis
                .is_some_and(|end| end <= self.start_millis || end > MAX_ASSEMBLY_DURATION_MILLIS)
        {
            return Err(SoundAssemblyError::InvalidMarker);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
