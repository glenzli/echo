//! Authored source-anchored spectral Freeze intent.
//!
//! This owner stores only user intent. FFT state and phase history remain
//! execution details rebuilt from immutable-Original source pre-roll.

use serde::{Deserialize, Serialize};

/// Minimum whole-millisecond source history needed by the canonical
/// 4096-frame Freeze analysis window at 48 kHz.
pub const FREEZE_CAPTURE_PRE_ROLL_MILLIS: u64 = 86;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FreezeVfxSettings {
    pub enabled: bool,
    pub mix_percent: u8,
    pub capture_source_millis: u64,
}

impl Default for FreezeVfxSettings {
    fn default() -> Self {
        Self::standard()
    }
}

impl FreezeVfxSettings {
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            enabled: false,
            mix_percent: 70,
            capture_source_millis: 100,
        }
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.mix_percent <= 100
            && (!self.enabled || self.capture_source_millis >= FREEZE_CAPTURE_PRE_ROLL_MILLIS)
    }
}

#[cfg(test)]
mod tests;
