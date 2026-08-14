//! Authored deterministic granular texture intent.
//!
//! The seed and scheduling units are persisted. History buffers and live
//! grains remain execution state owned by the audio engine.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GranularVfxSettings {
    pub enabled: bool,
    pub mix_percent: u8,
    pub grain_millis: u16,
    pub density_tenths_hertz: u16,
    pub lookback_millis: u16,
    pub scatter_millis: u16,
    pub pitch_cents: i16,
    pub stereo_spread_percent: u8,
    pub random_seed: u32,
}

impl Default for GranularVfxSettings {
    fn default() -> Self {
        Self::standard()
    }
}

impl GranularVfxSettings {
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            enabled: false,
            mix_percent: 45,
            grain_millis: 80,
            density_tenths_hertz: 120,
            lookback_millis: 250,
            scatter_millis: 120,
            pitch_cents: 0,
            stereo_spread_percent: 50,
            random_seed: 0x4543_484F,
        }
    }

    #[must_use]
    pub fn is_valid(self) -> bool {
        let pitch_ratio = 2.0_f64.powf(f64::from(self.pitch_cents) / 1_200.0);
        let required_history_millis = f64::from(self.lookback_millis)
            + f64::from(self.scatter_millis)
            + f64::from(self.grain_millis) * pitch_ratio.max(1.0);
        self.mix_percent <= 100
            && (20..=250).contains(&self.grain_millis)
            && (10..=400).contains(&self.density_tenths_hertz)
            && self.lookback_millis <= 1_500
            && self.scatter_millis <= 750
            && (-1_200..=1_200).contains(&self.pitch_cents)
            && self.stereo_spread_percent <= 100
            && required_history_millis <= 2_000.0
    }
}

#[cfg(test)]
mod tests;
