//! Non-destructive restoration intent for one immutable original.
//!
//! The graph stores authored time-domain bounds, amplitude envelopes, and a
//! bounded low-cut frequency in stable integer units. Execution-specific
//! sample positions and filter coefficients are prepared by the audio engine
//! and are never persisted as user intent.

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};

use crate::source_edit::{EditTimeline, EffectMask, MAX_EFFECT_MASKS};
use crate::{
    SpaceSettings, creative_vfx::CreativeVfxSettings, freeze_vfx::FREEZE_CAPTURE_PRE_ROLL_MILLIS,
    spectral_repair::SpectralRepairSettings,
};

/// Lowest supported output gain in hundredths of one decibel.
pub const MIN_GAIN_CENTIBELS: i16 = -2_400;
/// Highest supported output gain in hundredths of one decibel.
pub const MAX_GAIN_CENTIBELS: i16 = 1_200;
/// Lowest supported enabled low-cut frequency in hertz.
pub const MIN_LOW_CUT_HERTZ: u16 = 20;
/// Highest supported low-cut frequency in hertz.
pub const MAX_LOW_CUT_HERTZ: u16 = 240;
/// Lowest supported gain for one equalizer band, in hundredths of a decibel.
pub const MIN_EQ_GAIN_CENTIBELS: i16 = -1_200;
/// Highest supported gain for one equalizer band, in hundredths of a decibel.
pub const MAX_EQ_GAIN_CENTIBELS: i16 = 1_200;
/// Echo's authored parametric equalizer has a fixed, bounded band count.
pub const PARAMETRIC_EQ_BAND_COUNT: usize = 6;
pub const MIN_EQ_FREQUENCY_HERTZ: u16 = 20;
pub const MAX_EQ_FREQUENCY_HERTZ: u16 = 20_000;
/// Parametric equalizer Q is stored in hundredths.
pub const MIN_EQ_Q_HUNDREDTHS: u16 = 10;
pub const MAX_EQ_Q_HUNDREDTHS: u16 = 2_000;
pub const MIN_COMPRESSOR_THRESHOLD_CENTIBELS: i16 = -6_000;
pub const MAX_COMPRESSOR_THRESHOLD_CENTIBELS: i16 = 0;
pub const MIN_COMPRESSOR_RATIO_TENTHS: u16 = 10;
pub const MAX_COMPRESSOR_RATIO_TENTHS: u16 = 200;
pub const MIN_COMPRESSOR_ATTACK_MILLIS: u16 = 1;
pub const MAX_COMPRESSOR_ATTACK_MILLIS: u16 = 200;
pub const MIN_COMPRESSOR_RELEASE_MILLIS: u16 = 20;
pub const MAX_COMPRESSOR_RELEASE_MILLIS: u16 = 2_000;
pub const MAX_COMPRESSOR_MAKEUP_CENTIBELS: i16 = 2_400;
pub const MIN_LIMITER_CEILING_CENTIBELS: i16 = -600;
pub const MAX_LIMITER_CEILING_CENTIBELS: i16 = 0;
pub const MIN_LIMITER_RELEASE_MILLIS: u16 = 20;
pub const MAX_LIMITER_RELEASE_MILLIS: u16 = 1_000;
pub const MAX_REVERB_MIX_PERCENT: u8 = 100;
pub const MAX_REVERB_PRE_DELAY_MILLIS: u16 = 200;
pub const MIN_REVERB_DECAY_MILLIS: u16 = 100;
pub const MAX_REVERB_DECAY_MILLIS: u16 = 12_000;
pub const MIN_REVERB_SIZE_PERCENT: u8 = 10;
pub const MAX_REVERB_SIZE_PERCENT: u8 = 100;
pub const MAX_REVERB_DAMPING_PERCENT: u8 = 100;
pub const MIN_REVERB_LOW_CUT_HERTZ: u16 = 20;
pub const MAX_REVERB_LOW_CUT_HERTZ: u16 = 1_000;
pub const MIN_REVERB_HIGH_CUT_HERTZ: u16 = 1_000;
pub const MAX_REVERB_HIGH_CUT_HERTZ: u16 = 20_000;
pub const MAX_REVERB_DUCKING_AMOUNT_PERCENT: u8 = 100;
pub const MIN_REVERB_DUCKING_ATTACK_MILLIS: u16 = 1;
pub const MAX_REVERB_DUCKING_ATTACK_MILLIS: u16 = 200;
pub const MIN_REVERB_DUCKING_RELEASE_MILLIS: u16 = 20;
pub const MAX_REVERB_DUCKING_RELEASE_MILLIS: u16 = 2_000;
pub const MAX_NOISE_REDUCTION_CENTIBELS: u16 = 2_400;
pub const MAX_NOISE_REDUCTION_SENSITIVITY_PERCENT: u8 = 100;
pub const MIN_NOISE_REDUCTION_SMOOTHING_MILLIS: u16 = 20;
pub const MAX_NOISE_REDUCTION_SMOOTHING_MILLIS: u16 = 1_000;
pub const MIN_DE_ESSER_FREQUENCY_HERTZ: u16 = 3_000;
pub const MAX_DE_ESSER_FREQUENCY_HERTZ: u16 = 12_000;
pub const MIN_DE_ESSER_THRESHOLD_CENTIBELS: i16 = -6_000;
pub const MAX_DE_ESSER_THRESHOLD_CENTIBELS: i16 = 0;
pub const MAX_DE_ESSER_REDUCTION_CENTIBELS: u16 = 1_800;
pub const MIN_DE_PLOSIVE_FREQUENCY_HERTZ: u16 = 80;
pub const MAX_DE_PLOSIVE_FREQUENCY_HERTZ: u16 = 240;
pub const MAX_DE_PLOSIVE_SENSITIVITY_PERCENT: u8 = 100;
pub const MAX_DE_PLOSIVE_REDUCTION_CENTIBELS: u16 = 1_800;
pub const MIN_DE_PLOSIVE_RELEASE_MILLIS: u16 = 40;
pub const MAX_DE_PLOSIVE_RELEASE_MILLIS: u16 = 500;
pub const MIN_DE_HUM_HARMONIC_COUNT: u8 = 1;
pub const MAX_DE_HUM_HARMONIC_COUNT: u8 = 8;
/// De-hum Q is stored in tenths.
pub const MIN_DE_HUM_QUALITY_TENTHS: u16 = 50;
pub const MAX_DE_HUM_QUALITY_TENTHS: u16 = 1_000;
pub const MAX_DE_HUM_DEPTH_CENTIBELS: u16 = 4_800;
pub const MAX_DE_CLICK_SENSITIVITY_PERCENT: u8 = 100;
pub const MIN_DE_CLICK_DURATION_MICROSECONDS: u16 = 50;
pub const MAX_DE_CLICK_DURATION_MICROSECONDS: u16 = 2_000;
pub const MAX_DE_CLICK_REPAIR_PERCENT: u8 = 100;
pub const MIN_CHANNEL_BALANCE_PERCENT: i8 = -100;
pub const MAX_CHANNEL_BALANCE_PERCENT: i8 = 100;
/// Echo's authored chain is deliberately bounded to singleton effects.
pub const EFFECT_NODE_COUNT: usize = 22;
const STANDARD_EFFECT_NODE_COUNT: u8 = 5;

const fn enabled_by_default() -> bool {
    true
}

/// Stable identity for one authored effect node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum EffectNodeKind {
    Restoration = 0,
    Equalizer = 1,
    Dynamics = 2,
    Space = 3,
    Master = 4,
    DeHum = 5,
    DeClick = 6,
    ChannelRepair = 7,
    SceneVfx = 8,
    DelayVfx = 9,
    ModulationVfx = 10,
    TransformVfx = 11,
    DigitalDegradeVfx = 12,
    DriveVfx = 13,
    RotaryVfx = 14,
    FreezeVfx = 15,
    GranularVfx = 16,
    TapeVfx = 17,
    PitchVfx = 18,
    AutoWahVfx = 19,
    StereoVfx = 20,
    BeatRepeatVfx = 21,
}

impl EffectNodeKind {
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        self as u8
    }

    /// Restores the stable desktop wire representation.
    ///
    /// # Errors
    ///
    /// Returns [`EffectNodeKindValueError`] for an unknown value.
    pub const fn from_wire_value(value: u8) -> Result<Self, EffectNodeKindValueError> {
        match value {
            0 => Ok(Self::Restoration),
            1 => Ok(Self::Equalizer),
            2 => Ok(Self::Dynamics),
            3 => Ok(Self::Space),
            4 => Ok(Self::Master),
            5 => Ok(Self::DeHum),
            6 => Ok(Self::DeClick),
            7 => Ok(Self::ChannelRepair),
            8 => Ok(Self::SceneVfx),
            9 => Ok(Self::DelayVfx),
            10 => Ok(Self::ModulationVfx),
            11 => Ok(Self::TransformVfx),
            12 => Ok(Self::DigitalDegradeVfx),
            13 => Ok(Self::DriveVfx),
            14 => Ok(Self::RotaryVfx),
            15 => Ok(Self::FreezeVfx),
            16 => Ok(Self::GranularVfx),
            17 => Ok(Self::TapeVfx),
            18 => Ok(Self::PitchVfx),
            19 => Ok(Self::AutoWahVfx),
            20 => Ok(Self::StereoVfx),
            21 => Ok(Self::BeatRepeatVfx),
            _ => Err(EffectNodeKindValueError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectNodeKindValueError;

impl std::fmt::Display for EffectNodeKindValueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("effect node kind is outside the stable chain contract")
    }
}

impl std::error::Error for EffectNodeKindValueError {}

/// A bounded linear effect chain. Insert effects may be omitted or reordered;
/// master output remains the unique active terminal node. Omitted singleton
/// identities remain in the private tail so this compact value stays `Copy`
/// without allocating in adjustment snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EffectChain {
    nodes: [EffectNodeKind; EFFECT_NODE_COUNT],
    active_count: u8,
}

#[derive(Deserialize)]
struct StoredEffectChain {
    nodes: Vec<EffectNodeKind>,
    #[serde(default)]
    active_count: Option<u8>,
}

impl<'de> Deserialize<'de> for EffectChain {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let stored = StoredEffectChain::deserialize(deserializer)?;
        let legacy_node_count = stored.nodes.len();
        if !matches!(
            legacy_node_count,
            5 | 7 | 8 | 12 | 13 | 15 | 17 | 20 | EFFECT_NODE_COUNT
        ) {
            return Err(D::Error::custom(
                "effect chain has an unsupported stable node count",
            ));
        }
        let active_count = stored.active_count.unwrap_or(STANDARD_EFFECT_NODE_COUNT);
        let mut nodes = Self::standard().nodes;
        nodes[..legacy_node_count].copy_from_slice(&stored.nodes);
        let chain = Self {
            nodes,
            active_count,
        };
        if !chain.is_valid() {
            return Err(D::Error::custom(EffectChainError));
        }
        Ok(chain)
    }
}

impl Default for EffectChain {
    fn default() -> Self {
        Self::standard()
    }
}

impl EffectChain {
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            nodes: [
                EffectNodeKind::Restoration,
                EffectNodeKind::Equalizer,
                EffectNodeKind::Dynamics,
                EffectNodeKind::Space,
                EffectNodeKind::Master,
                EffectNodeKind::DeHum,
                EffectNodeKind::DeClick,
                EffectNodeKind::ChannelRepair,
                EffectNodeKind::SceneVfx,
                EffectNodeKind::DelayVfx,
                EffectNodeKind::ModulationVfx,
                EffectNodeKind::TransformVfx,
                EffectNodeKind::DigitalDegradeVfx,
                EffectNodeKind::DriveVfx,
                EffectNodeKind::RotaryVfx,
                EffectNodeKind::FreezeVfx,
                EffectNodeKind::GranularVfx,
                EffectNodeKind::TapeVfx,
                EffectNodeKind::PitchVfx,
                EffectNodeKind::AutoWahVfx,
                EffectNodeKind::StereoVfx,
                EffectNodeKind::BeatRepeatVfx,
            ],
            active_count: STANDARD_EFFECT_NODE_COUNT,
        }
    }

    /// Creates an authored singleton chain from its active nodes.
    ///
    /// # Errors
    ///
    /// Returns [`EffectChainError`] for duplicates, an empty chain, a missing
    /// terminal master node, or more nodes than Echo supports.
    pub fn new<const N: usize>(
        active_nodes: [EffectNodeKind; N],
    ) -> Result<Self, EffectChainError> {
        Self::from_active_nodes(&active_nodes)
    }

    /// Restores an authored chain from a variable-length wire projection.
    ///
    /// # Errors
    ///
    /// Returns [`EffectChainError`] under the same conditions as [`Self::new`].
    pub fn from_active_nodes(active_nodes: &[EffectNodeKind]) -> Result<Self, EffectChainError> {
        let active_count = active_nodes.len();
        if active_count == 0
            || active_count > EFFECT_NODE_COUNT
            || !matches!(active_nodes[active_count - 1], EffectNodeKind::Master)
        {
            return Err(EffectChainError);
        }
        let mut seen = [false; EFFECT_NODE_COUNT];
        for &node in active_nodes {
            let value = node as usize;
            if seen[value] {
                return Err(EffectChainError);
            }
            seen[value] = true;
        }

        let standard = Self::standard().nodes;
        let mut nodes = standard;
        nodes[..active_count].copy_from_slice(active_nodes);
        let mut write_index = active_count;
        for node in standard {
            if !seen[node as usize] {
                nodes[write_index] = node;
                write_index += 1;
            }
        }
        let chain = Self {
            nodes,
            active_count: u8::try_from(active_count).map_err(|_| EffectChainError)?,
        };
        if !chain.is_valid() {
            return Err(EffectChainError);
        }
        Ok(chain)
    }

    #[must_use]
    pub fn nodes(&self) -> &[EffectNodeKind] {
        &self.nodes[..usize::from(self.active_count)]
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        valid_effect_chain(self.nodes, self.active_count)
    }
}

const fn valid_effect_chain(nodes: [EffectNodeKind; EFFECT_NODE_COUNT], active_count: u8) -> bool {
    if active_count == 0 || active_count as usize > EFFECT_NODE_COUNT {
        return false;
    }
    if !matches!(nodes[active_count as usize - 1], EffectNodeKind::Master) {
        return false;
    }
    let mut seen = [false; EFFECT_NODE_COUNT];
    let mut index = 0;
    while index < EFFECT_NODE_COUNT {
        let value = nodes[index] as usize;
        if seen[value] {
            return false;
        }
        seen[value] = true;
        index += 1;
    }
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectChainError;

impl std::fmt::Display for EffectChainError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("effect chain must contain unique singleton nodes with master last")
    }
}

impl std::error::Error for EffectChainError {}

/// Authored adaptive broadband noise-reduction intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoiseReductionSettings {
    pub enabled: bool,
    pub reduction_centibels: u16,
    pub sensitivity_percent: u8,
    pub smoothing_millis: u16,
}

impl Default for NoiseReductionSettings {
    fn default() -> Self {
        Self::gentle()
    }
}

impl NoiseReductionSettings {
    #[must_use]
    pub const fn gentle() -> Self {
        Self {
            enabled: false,
            reduction_centibels: 900,
            sensitivity_percent: 50,
            smoothing_millis: 240,
        }
    }
}

/// Authored high-frequency de-essing intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeEsserSettings {
    pub enabled: bool,
    pub frequency_hertz: u16,
    pub threshold_centibels: i16,
    pub reduction_centibels: u16,
}

impl Default for DeEsserSettings {
    fn default() -> Self {
        Self::speech()
    }
}

impl DeEsserSettings {
    #[must_use]
    pub const fn speech() -> Self {
        Self {
            enabled: false,
            frequency_hertz: 6_500,
            threshold_centibels: -2_400,
            reduction_centibels: 600,
        }
    }
}

/// Authored low-frequency speech-plosive suppression intent. Detection and
/// band splitting remain execution details owned by the audio engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DePlosiveSettings {
    pub enabled: bool,
    pub frequency_hertz: u16,
    pub sensitivity_percent: u8,
    pub reduction_centibels: u16,
    pub release_millis: u16,
}

impl Default for DePlosiveSettings {
    fn default() -> Self {
        Self::speech()
    }
}

impl DePlosiveSettings {
    #[must_use]
    pub const fn speech() -> Self {
        Self {
            enabled: false,
            frequency_hertz: 140,
            sensitivity_percent: 50,
            reduction_centibels: 1_200,
            release_millis: 160,
        }
    }
}

/// Authored mains-hum removal intent. The audio engine derives the bounded
/// harmonic notch bank and smooths changes between these stable controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeHumSettings {
    pub enabled: bool,
    pub fundamental_hertz: u16,
    pub harmonic_count: u8,
    pub quality_tenths: u16,
    pub depth_centibels: u16,
}

impl Default for DeHumSettings {
    fn default() -> Self {
        Self::standard()
    }
}

impl DeHumSettings {
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            enabled: false,
            fundamental_hertz: 50,
            harmonic_count: 4,
            quality_tenths: 300,
            depth_centibels: 2_400,
        }
    }
}

/// Authored short-transient repair intent. The audio engine owns detection,
/// lookahead, and interpolation; persistence stores only bounded user intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeClickSettings {
    pub enabled: bool,
    pub sensitivity_percent: u8,
    pub maximum_click_microseconds: u16,
    pub repair_percent: u8,
}

/// Authored recovery for common stereo-channel faults. The audio engine
/// compiles these controls to one smoothed matrix; no execution coefficient is
/// persisted as user intent.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelRepairSettings {
    pub enabled: bool,
    pub invert_left: bool,
    pub invert_right: bool,
    pub swap_channels: bool,
    pub mono_fold_down: bool,
    pub balance_percent: i8,
}

impl Default for ChannelRepairSettings {
    fn default() -> Self {
        Self::identity()
    }
}

impl ChannelRepairSettings {
    #[must_use]
    pub const fn identity() -> Self {
        Self {
            enabled: false,
            invert_left: false,
            invert_right: false,
            swap_channels: false,
            mono_fold_down: false,
            balance_percent: 0,
        }
    }
}

impl Default for DeClickSettings {
    fn default() -> Self {
        Self::standard()
    }
}

impl DeClickSettings {
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            enabled: false,
            sensitivity_percent: 50,
            maximum_click_microseconds: 1_000,
            repair_percent: 100,
        }
    }
}

/// Stable restoration chain authored before tone and dynamics processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestorationSettings {
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
    #[serde(default)]
    pub de_plosive: DePlosiveSettings,
    pub noise_reduction: NoiseReductionSettings,
    pub de_esser: DeEsserSettings,
}

impl Default for RestorationSettings {
    fn default() -> Self {
        Self::standard()
    }
}

impl RestorationSettings {
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            enabled: true,
            de_plosive: DePlosiveSettings::speech(),
            noise_reduction: NoiseReductionSettings::gentle(),
            de_esser: DeEsserSettings::speech(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressorSettings {
    pub enabled: bool,
    pub threshold_centibels: i16,
    pub ratio_tenths: u16,
    pub attack_millis: u16,
    pub release_millis: u16,
    pub makeup_centibels: i16,
}

impl Default for CompressorSettings {
    fn default() -> Self {
        Self::standard()
    }
}

impl CompressorSettings {
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            enabled: false,
            threshold_centibels: -1_800,
            ratio_tenths: 30,
            attack_millis: 10,
            release_millis: 120,
            makeup_centibels: 0,
        }
    }
}

/// Authored final-output peak limiter intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LimiterSettings {
    pub enabled: bool,
    pub ceiling_centibels: i16,
    pub release_millis: u16,
}

impl Default for LimiterSettings {
    fn default() -> Self {
        Self::standard()
    }
}

impl LimiterSettings {
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            enabled: false,
            ceiling_centibels: -100,
            release_millis: 100,
        }
    }
}

/// Stable acoustic character for Echo's bounded algorithmic space effect.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReverbCharacter {
    #[default]
    Room,
    Hall,
    Plate,
    Spring,
}

impl ReverbCharacter {
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        match self {
            Self::Room => 0,
            Self::Hall => 1,
            Self::Plate => 2,
            Self::Spring => 3,
        }
    }

    /// Restores the stable desktop ABI representation.
    ///
    /// # Errors
    ///
    /// Returns [`ReverbCharacterValueError`] for unknown values.
    pub const fn from_wire_value(value: u8) -> Result<Self, ReverbCharacterValueError> {
        match value {
            0 => Ok(Self::Room),
            1 => Ok(Self::Hall),
            2 => Ok(Self::Plate),
            3 => Ok(Self::Spring),
            _ => Err(ReverbCharacterValueError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReverbCharacterValueError;

impl std::fmt::Display for ReverbCharacterValueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("reverb character must be room, hall, plate, or spring")
    }
}

impl std::error::Error for ReverbCharacterValueError {}

/// Authored algorithmic space intent. The audio engine owns delay lines and
/// filter coefficients; the Catalog only persists these stable controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReverbDuckingSettings {
    pub enabled: bool,
    pub amount_percent: u8,
    pub attack_millis: u16,
    pub release_millis: u16,
}

impl Default for ReverbDuckingSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            amount_percent: 65,
            attack_millis: 10,
            release_millis: 250,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReverbSettings {
    #[serde(default)]
    pub character: ReverbCharacter,
    pub enabled: bool,
    pub mix_percent: u8,
    pub pre_delay_millis: u16,
    pub decay_millis: u16,
    pub size_percent: u8,
    pub damping_percent: u8,
    pub low_cut_hertz: u16,
    pub high_cut_hertz: u16,
    #[serde(default)]
    pub ducking: ReverbDuckingSettings,
}

impl Default for ReverbSettings {
    fn default() -> Self {
        Self::studio_room()
    }
}

impl ReverbSettings {
    #[must_use]
    pub const fn studio_room() -> Self {
        Self {
            character: ReverbCharacter::Room,
            enabled: false,
            mix_percent: 18,
            pre_delay_millis: 20,
            decay_millis: 1_800,
            size_percent: 55,
            damping_percent: 45,
            low_cut_hertz: 120,
            high_cut_hertz: 10_000,
            ducking: ReverbDuckingSettings {
                enabled: false,
                amount_percent: 65,
                attack_millis: 10,
                release_millis: 250,
            },
        }
    }
}

/// Stable filter shape for one authored parametric equalizer band.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EqualizerFilterKind {
    #[default]
    Bell,
    LowShelf,
    HighShelf,
    Notch,
}

impl EqualizerFilterKind {
    #[must_use]
    pub const fn catalog_value(self) -> i64 {
        match self {
            Self::Bell => 0,
            Self::LowShelf => 1,
            Self::HighShelf => 2,
            Self::Notch => 3,
        }
    }

    /// Restores the stable Catalog representation.
    ///
    /// # Errors
    ///
    /// Returns [`EqualizerFilterKindValueError`] for unknown values.
    pub const fn from_catalog_value(value: i64) -> Result<Self, EqualizerFilterKindValueError> {
        match value {
            0 => Ok(Self::Bell),
            1 => Ok(Self::LowShelf),
            2 => Ok(Self::HighShelf),
            3 => Ok(Self::Notch),
            _ => Err(EqualizerFilterKindValueError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EqualizerFilterKindValueError;

impl std::fmt::Display for EqualizerFilterKindValueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("equalizer filter must be bell, low shelf, high shelf, or notch")
    }
}

impl std::error::Error for EqualizerFilterKindValueError {}

/// One stable authored band. Coefficients remain an audio-engine concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParametricEqualizerBand {
    pub enabled: bool,
    pub filter_kind: EqualizerFilterKind,
    pub frequency_hertz: u16,
    pub q_hundredths: u16,
    pub gain_centibels: i16,
}

impl ParametricEqualizerBand {
    #[must_use]
    pub const fn new(
        enabled: bool,
        filter_kind: EqualizerFilterKind,
        frequency_hertz: u16,
        q_hundredths: u16,
        gain_centibels: i16,
    ) -> Self {
        Self {
            enabled,
            filter_kind,
            frequency_hertz,
            q_hundredths,
            gain_centibels,
        }
    }
}

/// Six-band authored equalizer intent shared by persistence and execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParametricEqualizer {
    #[serde(default = "enabled_by_default")]
    enabled: bool,
    bands: [ParametricEqualizerBand; PARAMETRIC_EQ_BAND_COUNT],
}

impl Default for ParametricEqualizer {
    fn default() -> Self {
        Self::flat()
    }
}

impl ParametricEqualizer {
    #[must_use]
    pub const fn new(bands: [ParametricEqualizerBand; PARAMETRIC_EQ_BAND_COUNT]) -> Self {
        Self {
            enabled: true,
            bands,
        }
    }

    #[must_use]
    pub const fn flat() -> Self {
        use EqualizerFilterKind::{Bell, HighShelf, LowShelf};
        Self::new([
            ParametricEqualizerBand::new(true, LowShelf, 120, 71, 0),
            ParametricEqualizerBand::new(false, Bell, 250, 100, 0),
            ParametricEqualizerBand::new(true, Bell, 1_000, 100, 0),
            ParametricEqualizerBand::new(false, Bell, 3_000, 100, 0),
            ParametricEqualizerBand::new(false, Bell, 5_000, 100, 0),
            ParametricEqualizerBand::new(true, HighShelf, 8_000, 71, 0),
        ])
    }

    /// Losslessly lifts Echo's former fixed-band gains into the new topology.
    #[must_use]
    pub const fn from_legacy_gains(low: i16, mid: i16, high: i16) -> Self {
        let mut equalizer = Self::flat();
        equalizer.bands[0].gain_centibels = low;
        equalizer.bands[2].gain_centibels = mid;
        equalizer.bands[5].gain_centibels = high;
        equalizer
    }

    #[must_use]
    pub const fn bands(self) -> [ParametricEqualizerBand; PARAMETRIC_EQ_BAND_COUNT] {
        self.bands
    }

    #[must_use]
    pub const fn enabled(self) -> bool {
        self.enabled
    }

    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub fn is_flat(self) -> bool {
        self.bands.iter().all(|band| {
            !band.enabled
                || (band.filter_kind != EqualizerFilterKind::Notch && band.gain_centibels == 0)
        })
    }
}

/// Stable fade interpolation authored independently for each edge.
///
/// The variants describe time-domain intent. The audio engine owns their
/// sample-domain evaluation so persistence never stores sampled envelopes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FadeCurve {
    #[default]
    Linear,
    Smooth,
    EqualPower,
}

impl FadeCurve {
    #[must_use]
    pub const fn catalog_value(self) -> i64 {
        match self {
            Self::Linear => 0,
            Self::Smooth => 1,
            Self::EqualPower => 2,
        }
    }

    /// Restores the stable Catalog representation.
    ///
    /// # Errors
    ///
    /// Returns [`FadeCurveValueError`] for unknown persisted values.
    pub const fn from_catalog_value(value: i64) -> Result<Self, FadeCurveValueError> {
        match value {
            0 => Ok(Self::Linear),
            1 => Ok(Self::Smooth),
            2 => Ok(Self::EqualPower),
            _ => Err(FadeCurveValueError),
        }
    }
}

/// A persisted fade curve value outside the stable enum contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FadeCurveValueError;

impl std::fmt::Display for FadeCurveValueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("fade curve must be linear, smooth, or equal power")
    }
}

impl std::error::Error for FadeCurveValueError {}

/// Fade interpolation selected independently for the two clip edges.
///
/// Keeping the pair as one value prevents bridge and persistence callers from
/// silently swapping two adjacent curve arguments.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FadeCurves {
    pub fade_in: FadeCurve,
    pub fade_out: FadeCurve,
}

impl FadeCurves {
    #[must_use]
    pub const fn new(fade_in: FadeCurve, fade_out: FadeCurve) -> Self {
        Self { fade_in, fade_out }
    }

    #[must_use]
    pub const fn linear() -> Self {
        Self::new(FadeCurve::Linear, FadeCurve::Linear)
    }
}

/// Authored processing that applies inside the selected clip range.
///
/// This value keeps effect intent distinct from time-domain trim bounds and
/// avoids an order-sensitive sequence of scalar effect parameters.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AdjustmentEffects {
    pub fade_curves: FadeCurves,
    pub gain_centibels: i16,
    pub low_cut_hertz: u16,
    pub restoration: RestorationSettings,
    pub de_hum: DeHumSettings,
    pub de_click: DeClickSettings,
    pub channel_repair: ChannelRepairSettings,
    pub equalizer: ParametricEqualizer,
    pub compressor: CompressorSettings,
    pub reverb: ReverbSettings,
    pub space: SpaceSettings,
    pub creative_vfx: CreativeVfxSettings,
    pub spectral_repair: SpectralRepairSettings,
    pub limiter: LimiterSettings,
    pub effect_chain: EffectChain,
    pub edit_timeline: Option<EditTimeline>,
    pub effect_masks: Vec<EffectMask>,
}

impl AdjustmentEffects {
    #[must_use]
    pub const fn new(fade_curves: FadeCurves, gain_centibels: i16, low_cut_hertz: u16) -> Self {
        Self {
            fade_curves,
            gain_centibels,
            low_cut_hertz,
            restoration: RestorationSettings::standard(),
            de_hum: DeHumSettings::standard(),
            de_click: DeClickSettings::standard(),
            channel_repair: ChannelRepairSettings::identity(),
            equalizer: ParametricEqualizer::flat(),
            compressor: CompressorSettings::standard(),
            reverb: ReverbSettings::studio_room(),
            space: SpaceSettings::algorithmic(),
            creative_vfx: CreativeVfxSettings::standard(),
            spectral_repair: SpectralRepairSettings::identity(),
            limiter: LimiterSettings::standard(),
            effect_chain: EffectChain::standard(),
            edit_timeline: None,
            effect_masks: Vec::new(),
        }
    }

    #[must_use]
    pub const fn with_restoration(mut self, restoration: RestorationSettings) -> Self {
        self.restoration = restoration;
        self
    }

    #[must_use]
    pub const fn with_de_hum(mut self, de_hum: DeHumSettings) -> Self {
        self.de_hum = de_hum;
        self
    }

    #[must_use]
    pub const fn with_de_click(mut self, de_click: DeClickSettings) -> Self {
        self.de_click = de_click;
        self
    }

    #[must_use]
    pub const fn with_channel_repair(mut self, channel_repair: ChannelRepairSettings) -> Self {
        self.channel_repair = channel_repair;
        self
    }

    #[must_use]
    pub const fn with_equalizer(mut self, equalizer: ParametricEqualizer) -> Self {
        self.equalizer = equalizer;
        self
    }

    #[must_use]
    pub const fn with_compressor(mut self, compressor: CompressorSettings) -> Self {
        self.compressor = compressor;
        self
    }

    #[must_use]
    pub const fn with_reverb(mut self, reverb: ReverbSettings) -> Self {
        self.reverb = reverb;
        self
    }

    #[must_use]
    pub const fn with_space(mut self, space: SpaceSettings) -> Self {
        self.space = space;
        self
    }

    #[must_use]
    pub const fn with_creative_vfx(mut self, creative_vfx: CreativeVfxSettings) -> Self {
        self.creative_vfx = creative_vfx;
        self
    }

    #[must_use]
    pub fn with_spectral_repair(mut self, spectral_repair: SpectralRepairSettings) -> Self {
        self.spectral_repair = spectral_repair;
        self
    }

    #[must_use]
    pub const fn with_limiter(mut self, limiter: LimiterSettings) -> Self {
        self.limiter = limiter;
        self
    }

    #[must_use]
    pub const fn with_effect_chain(mut self, effect_chain: EffectChain) -> Self {
        self.effect_chain = effect_chain;
        self
    }

    #[must_use]
    pub fn with_edit_timeline(mut self, edit_timeline: EditTimeline) -> Self {
        self.edit_timeline = Some(edit_timeline);
        self
    }

    fn with_optional_edit_timeline(mut self, edit_timeline: Option<EditTimeline>) -> Self {
        self.edit_timeline = edit_timeline;
        self
    }

    #[must_use]
    pub fn with_effect_masks(mut self, effect_masks: Vec<EffectMask>) -> Self {
        self.effect_masks = effect_masks;
        self
    }
}

/// One validated, non-destructive adjustment graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdjustmentGraph {
    trim_start_millis: u64,
    trim_end_millis: u64,
    fade_in_millis: u64,
    fade_out_millis: u64,
    fade_in_curve: FadeCurve,
    fade_out_curve: FadeCurve,
    gain_centibels: i16,
    low_cut_hertz: u16,
    #[serde(default)]
    restoration: RestorationSettings,
    #[serde(default)]
    de_hum: DeHumSettings,
    #[serde(default)]
    de_click: DeClickSettings,
    #[serde(default)]
    channel_repair: ChannelRepairSettings,
    equalizer: ParametricEqualizer,
    compressor: CompressorSettings,
    #[serde(default)]
    reverb: ReverbSettings,
    #[serde(default)]
    space: SpaceSettings,
    #[serde(default)]
    creative_vfx: CreativeVfxSettings,
    #[serde(default)]
    spectral_repair: SpectralRepairSettings,
    #[serde(default)]
    limiter: LimiterSettings,
    #[serde(default)]
    effect_chain: EffectChain,
    edit_timeline: EditTimeline,
    effect_masks: Vec<EffectMask>,
}

#[derive(Deserialize)]
struct StoredAdjustmentGraph {
    trim_start_millis: u64,
    trim_end_millis: u64,
    fade_in_millis: u64,
    fade_out_millis: u64,
    fade_in_curve: FadeCurve,
    fade_out_curve: FadeCurve,
    gain_centibels: i16,
    low_cut_hertz: u16,
    #[serde(default)]
    restoration: RestorationSettings,
    #[serde(default)]
    de_hum: DeHumSettings,
    #[serde(default)]
    de_click: DeClickSettings,
    #[serde(default)]
    channel_repair: ChannelRepairSettings,
    equalizer: ParametricEqualizer,
    compressor: CompressorSettings,
    #[serde(default)]
    reverb: ReverbSettings,
    #[serde(default)]
    space: SpaceSettings,
    #[serde(default)]
    creative_vfx: CreativeVfxSettings,
    #[serde(default)]
    spectral_repair: SpectralRepairSettings,
    #[serde(default)]
    limiter: LimiterSettings,
    #[serde(default)]
    effect_chain: EffectChain,
    #[serde(default)]
    edit_timeline: Option<EditTimeline>,
    #[serde(default)]
    effect_masks: Vec<EffectMask>,
}

impl<'de> Deserialize<'de> for AdjustmentGraph {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let stored = StoredAdjustmentGraph::deserialize(deserializer)?;
        Self::new(
            stored.trim_end_millis,
            stored.trim_start_millis,
            stored.trim_end_millis,
            stored.fade_in_millis,
            stored.fade_out_millis,
            AdjustmentEffects::new(
                FadeCurves::new(stored.fade_in_curve, stored.fade_out_curve),
                stored.gain_centibels,
                stored.low_cut_hertz,
            )
            .with_restoration(stored.restoration)
            .with_de_hum(stored.de_hum)
            .with_de_click(stored.de_click)
            .with_channel_repair(stored.channel_repair)
            .with_equalizer(stored.equalizer)
            .with_compressor(stored.compressor)
            .with_reverb(stored.reverb)
            .with_space(stored.space)
            .with_creative_vfx(stored.creative_vfx)
            .with_spectral_repair(stored.spectral_repair)
            .with_limiter(stored.limiter)
            .with_effect_chain(stored.effect_chain)
            .with_optional_edit_timeline(stored.edit_timeline)
            .with_effect_masks(stored.effect_masks),
        )
        .map_err(D::Error::custom)
    }
}

impl AdjustmentGraph {
    /// Creates a graph bounded by the immutable source duration.
    ///
    /// # Errors
    ///
    /// Returns [`AdjustmentGraphError`] when the range is empty, extends past
    /// the source, the fades overlap, or gain is outside the product range.
    pub fn new(
        source_duration_millis: u64,
        trim_start_millis: u64,
        trim_end_millis: u64,
        fade_in_millis: u64,
        fade_out_millis: u64,
        effects: AdjustmentEffects,
    ) -> Result<Self, AdjustmentGraphError> {
        if source_duration_millis == 0
            || trim_start_millis >= trim_end_millis
            || trim_end_millis > source_duration_millis
        {
            return Err(AdjustmentGraphError::InvalidTrimRange);
        }
        if fade_in_millis.saturating_add(fade_out_millis) > trim_end_millis - trim_start_millis {
            return Err(AdjustmentGraphError::OverlappingFades);
        }
        if !(MIN_GAIN_CENTIBELS..=MAX_GAIN_CENTIBELS).contains(&effects.gain_centibels) {
            return Err(AdjustmentGraphError::GainOutOfRange);
        }
        if effects.low_cut_hertz != 0
            && !(MIN_LOW_CUT_HERTZ..=MAX_LOW_CUT_HERTZ).contains(&effects.low_cut_hertz)
        {
            return Err(AdjustmentGraphError::LowCutOutOfRange);
        }
        if !effects.effect_chain.is_valid() {
            return Err(AdjustmentGraphError::InvalidEffectChain);
        }
        validate_restorative_effects(&effects)?;
        for band in effects.equalizer.bands {
            if !(MIN_EQ_GAIN_CENTIBELS..=MAX_EQ_GAIN_CENTIBELS).contains(&band.gain_centibels)
                || !(MIN_EQ_FREQUENCY_HERTZ..=MAX_EQ_FREQUENCY_HERTZ)
                    .contains(&band.frequency_hertz)
                || !(MIN_EQ_Q_HUNDREDTHS..=MAX_EQ_Q_HUNDREDTHS).contains(&band.q_hundredths)
            {
                return Err(AdjustmentGraphError::EqualizerBandOutOfRange);
            }
        }
        let compressor = effects.compressor;
        if !(MIN_COMPRESSOR_THRESHOLD_CENTIBELS..=MAX_COMPRESSOR_THRESHOLD_CENTIBELS)
            .contains(&compressor.threshold_centibels)
            || !(MIN_COMPRESSOR_RATIO_TENTHS..=MAX_COMPRESSOR_RATIO_TENTHS)
                .contains(&compressor.ratio_tenths)
            || !(MIN_COMPRESSOR_ATTACK_MILLIS..=MAX_COMPRESSOR_ATTACK_MILLIS)
                .contains(&compressor.attack_millis)
            || !(MIN_COMPRESSOR_RELEASE_MILLIS..=MAX_COMPRESSOR_RELEASE_MILLIS)
                .contains(&compressor.release_millis)
            || !(0..=MAX_COMPRESSOR_MAKEUP_CENTIBELS).contains(&compressor.makeup_centibels)
        {
            return Err(AdjustmentGraphError::CompressorOutOfRange);
        }
        let limiter = effects.limiter;
        if !(MIN_LIMITER_CEILING_CENTIBELS..=MAX_LIMITER_CEILING_CENTIBELS)
            .contains(&limiter.ceiling_centibels)
            || !(MIN_LIMITER_RELEASE_MILLIS..=MAX_LIMITER_RELEASE_MILLIS)
                .contains(&limiter.release_millis)
        {
            return Err(AdjustmentGraphError::LimiterOutOfRange);
        }
        let reverb = effects.reverb;
        if reverb.mix_percent > MAX_REVERB_MIX_PERCENT
            || reverb.pre_delay_millis > MAX_REVERB_PRE_DELAY_MILLIS
            || !(MIN_REVERB_DECAY_MILLIS..=MAX_REVERB_DECAY_MILLIS).contains(&reverb.decay_millis)
            || !(MIN_REVERB_SIZE_PERCENT..=MAX_REVERB_SIZE_PERCENT).contains(&reverb.size_percent)
            || reverb.damping_percent > MAX_REVERB_DAMPING_PERCENT
            || !(MIN_REVERB_LOW_CUT_HERTZ..=MAX_REVERB_LOW_CUT_HERTZ)
                .contains(&reverb.low_cut_hertz)
            || !(MIN_REVERB_HIGH_CUT_HERTZ..=MAX_REVERB_HIGH_CUT_HERTZ)
                .contains(&reverb.high_cut_hertz)
            || reverb.low_cut_hertz >= reverb.high_cut_hertz
            || reverb.ducking.amount_percent > MAX_REVERB_DUCKING_AMOUNT_PERCENT
            || !(MIN_REVERB_DUCKING_ATTACK_MILLIS..=MAX_REVERB_DUCKING_ATTACK_MILLIS)
                .contains(&reverb.ducking.attack_millis)
            || !(MIN_REVERB_DUCKING_RELEASE_MILLIS..=MAX_REVERB_DUCKING_RELEASE_MILLIS)
                .contains(&reverb.ducking.release_millis)
        {
            return Err(AdjustmentGraphError::ReverbOutOfRange);
        }
        if !effects.space.is_valid() {
            return Err(AdjustmentGraphError::SpaceOutOfRange);
        }
        effects
            .creative_vfx
            .validate()
            .map_err(|_| AdjustmentGraphError::CreativeVfxOutOfRange)?;
        effects
            .spectral_repair
            .validate(source_duration_millis)
            .map_err(|_| AdjustmentGraphError::SpectralRepairOutOfRange)?;
        validate_freeze_anchor(trim_start_millis, trim_end_millis, effects.creative_vfx)?;
        let edit_timeline = validated_asset_regions(trim_start_millis, trim_end_millis, &effects)?;
        Ok(Self {
            trim_start_millis,
            trim_end_millis,
            fade_in_millis,
            fade_out_millis,
            fade_in_curve: effects.fade_curves.fade_in,
            fade_out_curve: effects.fade_curves.fade_out,
            gain_centibels: effects.gain_centibels,
            low_cut_hertz: effects.low_cut_hertz,
            restoration: effects.restoration,
            de_hum: effects.de_hum,
            de_click: effects.de_click,
            channel_repair: effects.channel_repair,
            equalizer: effects.equalizer,
            compressor,
            reverb,
            space: effects.space,
            creative_vfx: effects.creative_vfx,
            spectral_repair: effects.spectral_repair,
            limiter,
            effect_chain: effects.effect_chain,
            edit_timeline,
            effect_masks: effects.effect_masks,
        })
    }

    /// Creates the identity graph for a known-duration original.
    ///
    /// # Errors
    ///
    /// Returns [`AdjustmentGraphError::InvalidTrimRange`] for an unknown or
    /// zero duration.
    pub fn identity(source_duration_millis: u64) -> Result<Self, AdjustmentGraphError> {
        Self::new(
            source_duration_millis,
            0,
            source_duration_millis,
            0,
            0,
            AdjustmentEffects::default(),
        )
    }

    #[must_use]
    pub const fn trim_start_millis(&self) -> u64 {
        self.trim_start_millis
    }

    #[must_use]
    pub const fn trim_end_millis(&self) -> u64 {
        self.trim_end_millis
    }

    #[must_use]
    pub const fn fade_in_millis(&self) -> u64 {
        self.fade_in_millis
    }

    #[must_use]
    pub const fn fade_out_millis(&self) -> u64 {
        self.fade_out_millis
    }

    #[must_use]
    pub const fn fade_in_curve(&self) -> FadeCurve {
        self.fade_in_curve
    }

    #[must_use]
    pub const fn fade_out_curve(&self) -> FadeCurve {
        self.fade_out_curve
    }

    #[must_use]
    pub const fn gain_centibels(&self) -> i16 {
        self.gain_centibels
    }

    /// High-pass cutoff in hertz, or zero when low-cut is disabled.
    #[must_use]
    pub const fn low_cut_hertz(&self) -> u16 {
        self.low_cut_hertz
    }

    #[must_use]
    pub const fn restoration(&self) -> RestorationSettings {
        self.restoration
    }

    #[must_use]
    pub const fn de_hum(&self) -> DeHumSettings {
        self.de_hum
    }

    #[must_use]
    pub const fn de_click(&self) -> DeClickSettings {
        self.de_click
    }

    #[must_use]
    pub const fn channel_repair(&self) -> ChannelRepairSettings {
        self.channel_repair
    }

    #[must_use]
    pub const fn equalizer(&self) -> ParametricEqualizer {
        self.equalizer
    }

    #[must_use]
    pub const fn compressor(&self) -> CompressorSettings {
        self.compressor
    }

    #[must_use]
    pub const fn reverb(&self) -> ReverbSettings {
        self.reverb
    }

    #[must_use]
    pub const fn space(&self) -> SpaceSettings {
        self.space
    }

    #[must_use]
    pub const fn creative_vfx(&self) -> CreativeVfxSettings {
        self.creative_vfx
    }

    #[must_use]
    pub fn spectral_repair(&self) -> &SpectralRepairSettings {
        &self.spectral_repair
    }

    #[must_use]
    pub const fn limiter(&self) -> LimiterSettings {
        self.limiter
    }

    #[must_use]
    pub const fn effect_chain(&self) -> EffectChain {
        self.effect_chain
    }

    #[must_use]
    pub const fn edit_timeline(&self) -> &EditTimeline {
        &self.edit_timeline
    }

    #[must_use]
    pub fn effect_masks(&self) -> &[EffectMask] {
        &self.effect_masks
    }
}

fn validate_freeze_anchor(
    trim_start_millis: u64,
    trim_end_millis: u64,
    creative_vfx: CreativeVfxSettings,
) -> Result<(), AdjustmentGraphError> {
    let freeze = creative_vfx.freeze;
    if freeze.enabled
        && (freeze.capture_source_millis
            < trim_start_millis.saturating_add(FREEZE_CAPTURE_PRE_ROLL_MILLIS)
            || freeze.capture_source_millis >= trim_end_millis)
    {
        return Err(AdjustmentGraphError::FreezeAnchorOutOfRange);
    }
    Ok(())
}

fn validated_asset_regions(
    trim_start_millis: u64,
    trim_end_millis: u64,
    effects: &AdjustmentEffects,
) -> Result<EditTimeline, AdjustmentGraphError> {
    let edit_timeline = effects.edit_timeline.clone().unwrap_or(
        EditTimeline::identity(trim_start_millis, trim_end_millis)
            .map_err(|_| AdjustmentGraphError::InvalidEditTimeline)?,
    );
    if edit_timeline.trim_start_millis() != trim_start_millis
        || edit_timeline.trim_end_millis() != trim_end_millis
    {
        return Err(AdjustmentGraphError::InvalidEditTimeline);
    }
    if effects.effect_masks.len() > MAX_EFFECT_MASKS
        || effects.effect_masks.iter().any(|mask| {
            mask.start_millis() < trim_start_millis
                || mask.end_millis() > trim_end_millis
                || mask.effect_nodes().iter().any(|node| {
                    !effects.effect_chain.nodes().contains(node)
                        || matches!(
                            node,
                            EffectNodeKind::Master
                                | EffectNodeKind::DeClick
                                | EffectNodeKind::TransformVfx
                                | EffectNodeKind::DriveVfx
                                | EffectNodeKind::RotaryVfx
                                | EffectNodeKind::FreezeVfx
                                | EffectNodeKind::GranularVfx
                                | EffectNodeKind::PitchVfx
                                | EffectNodeKind::BeatRepeatVfx
                        )
                })
        })
    {
        return Err(AdjustmentGraphError::InvalidEffectMasks);
    }
    Ok(edit_timeline)
}

fn validate_restorative_effects(effects: &AdjustmentEffects) -> Result<(), AdjustmentGraphError> {
    let de_plosive = effects.restoration.de_plosive;
    if !(MIN_DE_PLOSIVE_FREQUENCY_HERTZ..=MAX_DE_PLOSIVE_FREQUENCY_HERTZ)
        .contains(&de_plosive.frequency_hertz)
        || de_plosive.sensitivity_percent > MAX_DE_PLOSIVE_SENSITIVITY_PERCENT
        || de_plosive.reduction_centibels > MAX_DE_PLOSIVE_REDUCTION_CENTIBELS
        || !(MIN_DE_PLOSIVE_RELEASE_MILLIS..=MAX_DE_PLOSIVE_RELEASE_MILLIS)
            .contains(&de_plosive.release_millis)
    {
        return Err(AdjustmentGraphError::DePlosiveOutOfRange);
    }
    let de_hum = effects.de_hum;
    if !matches!(de_hum.fundamental_hertz, 50 | 60)
        || !(MIN_DE_HUM_HARMONIC_COUNT..=MAX_DE_HUM_HARMONIC_COUNT).contains(&de_hum.harmonic_count)
        || !(MIN_DE_HUM_QUALITY_TENTHS..=MAX_DE_HUM_QUALITY_TENTHS).contains(&de_hum.quality_tenths)
        || de_hum.depth_centibels > MAX_DE_HUM_DEPTH_CENTIBELS
    {
        return Err(AdjustmentGraphError::DeHumOutOfRange);
    }
    let de_click = effects.de_click;
    if de_click.sensitivity_percent > MAX_DE_CLICK_SENSITIVITY_PERCENT
        || !(MIN_DE_CLICK_DURATION_MICROSECONDS..=MAX_DE_CLICK_DURATION_MICROSECONDS)
            .contains(&de_click.maximum_click_microseconds)
        || de_click.repair_percent > MAX_DE_CLICK_REPAIR_PERCENT
    {
        return Err(AdjustmentGraphError::DeClickOutOfRange);
    }
    if !(MIN_CHANNEL_BALANCE_PERCENT..=MAX_CHANNEL_BALANCE_PERCENT)
        .contains(&effects.channel_repair.balance_percent)
    {
        return Err(AdjustmentGraphError::ChannelRepairOutOfRange);
    }
    let noise_reduction = effects.restoration.noise_reduction;
    if noise_reduction.reduction_centibels > MAX_NOISE_REDUCTION_CENTIBELS
        || noise_reduction.sensitivity_percent > MAX_NOISE_REDUCTION_SENSITIVITY_PERCENT
        || !(MIN_NOISE_REDUCTION_SMOOTHING_MILLIS..=MAX_NOISE_REDUCTION_SMOOTHING_MILLIS)
            .contains(&noise_reduction.smoothing_millis)
    {
        return Err(AdjustmentGraphError::NoiseReductionOutOfRange);
    }
    let de_esser = effects.restoration.de_esser;
    if !(MIN_DE_ESSER_FREQUENCY_HERTZ..=MAX_DE_ESSER_FREQUENCY_HERTZ)
        .contains(&de_esser.frequency_hertz)
        || !(MIN_DE_ESSER_THRESHOLD_CENTIBELS..=MAX_DE_ESSER_THRESHOLD_CENTIBELS)
            .contains(&de_esser.threshold_centibels)
        || de_esser.reduction_centibels > MAX_DE_ESSER_REDUCTION_CENTIBELS
    {
        return Err(AdjustmentGraphError::DeEsserOutOfRange);
    }
    Ok(())
}

/// Stable validation failures for authored adjustment intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjustmentGraphError {
    InvalidTrimRange,
    OverlappingFades,
    GainOutOfRange,
    LowCutOutOfRange,
    NoiseReductionOutOfRange,
    DeEsserOutOfRange,
    DePlosiveOutOfRange,
    DeHumOutOfRange,
    DeClickOutOfRange,
    ChannelRepairOutOfRange,
    EqualizerBandOutOfRange,
    CompressorOutOfRange,
    ReverbOutOfRange,
    SpaceOutOfRange,
    CreativeVfxOutOfRange,
    SpectralRepairOutOfRange,
    FreezeAnchorOutOfRange,
    LimiterOutOfRange,
    InvalidEffectChain,
    InvalidEditTimeline,
    InvalidEffectMasks,
}

impl std::fmt::Display for AdjustmentGraphError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidTrimRange => "trim range must be non-empty and inside the source",
            Self::OverlappingFades => "fade durations must fit inside the trim range",
            Self::GainOutOfRange => "gain must be between -24 dB and +12 dB",
            Self::LowCutOutOfRange => "low cut must be off or between 20 Hz and 240 Hz",
            Self::NoiseReductionOutOfRange => {
                "noise reduction parameters are outside the supported range"
            }
            Self::DeEsserOutOfRange => "de-esser parameters are outside the supported range",
            Self::DePlosiveOutOfRange => "de-plosive parameters are outside the supported range",
            Self::DeHumOutOfRange => "de-hum parameters are outside the supported range",
            Self::DeClickOutOfRange => "de-click parameters are outside the supported range",
            Self::ChannelRepairOutOfRange => {
                "channel repair balance must be between -100 and 100 percent"
            }
            Self::EqualizerBandOutOfRange => {
                "equalizer band frequency, Q, or gain is outside the supported range"
            }
            Self::CompressorOutOfRange => "compressor parameters are outside the supported range",
            Self::ReverbOutOfRange => "reverb parameters are outside the supported range",
            Self::SpaceOutOfRange => "space parameters are outside the supported range",
            Self::CreativeVfxOutOfRange => {
                "creative VFX parameters are outside the supported range"
            }
            Self::SpectralRepairOutOfRange => {
                "spectral repair parameters are outside the supported range"
            }
            Self::FreezeAnchorOutOfRange => {
                "freeze capture must be inside the trim with a complete source pre-roll"
            }
            Self::LimiterOutOfRange => "limiter parameters are outside the supported range",
            Self::InvalidEffectChain => {
                "effect chain must contain unique singleton nodes with master last"
            }
            Self::InvalidEditTimeline => {
                "edit timeline must continuously cover the complete trim range"
            }
            Self::InvalidEffectMasks => {
                "effect masks must target active supported inserts inside the trim range"
            }
        })
    }
}

impl std::error::Error for AdjustmentGraphError {}

#[cfg(test)]
mod tests;
