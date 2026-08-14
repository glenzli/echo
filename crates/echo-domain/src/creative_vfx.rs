//! Authored deterministic Creative VFX intent for an immutable original.
//!
//! These values are deliberately separate from restoration. They describe
//! explicit, bypassable transformations of selected source audio; execution
//! state, coefficients, delay buffers, and generated wet samples are never
//! persisted as user intent.

use serde::{Deserialize, Serialize};

use crate::{freeze_vfx::FreezeVfxSettings, granular_vfx::GranularVfxSettings};

/// Stable scene-filter identity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum SceneVfxCharacter {
    #[default]
    Telephone = 0,
    Radio = 1,
    Intercom = 2,
    BehindWall = 3,
    Underwater = 4,
}

impl SceneVfxCharacter {
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        self as u8
    }

    /// Restores the stable desktop ABI value.
    ///
    /// # Errors
    ///
    /// Returns [`CreativeVfxValueError`] for an unknown value.
    pub const fn from_wire_value(value: u8) -> Result<Self, CreativeVfxValueError> {
        match value {
            0 => Ok(Self::Telephone),
            1 => Ok(Self::Radio),
            2 => Ok(Self::Intercom),
            3 => Ok(Self::BehindWall),
            4 => Ok(Self::Underwater),
            _ => Err(CreativeVfxValueError),
        }
    }
}

/// Stable repeat-effect identity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum DelayVfxCharacter {
    #[default]
    Slapback = 0,
    Echo = 1,
}

impl DelayVfxCharacter {
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        self as u8
    }

    /// Restores the stable desktop ABI value.
    ///
    /// # Errors
    ///
    /// Returns [`CreativeVfxValueError`] for an unknown value.
    pub const fn from_wire_value(value: u8) -> Result<Self, CreativeVfxValueError> {
        match value {
            0 => Ok(Self::Slapback),
            1 => Ok(Self::Echo),
            _ => Err(CreativeVfxValueError),
        }
    }
}

/// Stable modulation-effect identity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum ModulationVfxCharacter {
    #[default]
    Chorus = 0,
    Flanger = 1,
    Phaser = 2,
    Tremolo = 3,
}

impl ModulationVfxCharacter {
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        self as u8
    }

    /// Restores the stable desktop ABI value.
    ///
    /// # Errors
    ///
    /// Returns [`CreativeVfxValueError`] for an unknown value.
    pub const fn from_wire_value(value: u8) -> Result<Self, CreativeVfxValueError> {
        match value {
            0 => Ok(Self::Chorus),
            1 => Ok(Self::Flanger),
            2 => Ok(Self::Phaser),
            3 => Ok(Self::Tremolo),
            _ => Err(CreativeVfxValueError),
        }
    }
}

/// Stable non-cloning voice-role identity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum TransformVfxCharacter {
    #[default]
    Robot = 0,
    Monster = 1,
    Tiny = 2,
    Giant = 3,
    Ghost = 4,
}

/// Stable digital-resolution effect identity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum DigitalDegradeVfxCharacter {
    #[default]
    Bitcrusher = 0,
    SampleRateReduction = 1,
    LoFi = 2,
}

/// Stable antialiased saturation identity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum DriveVfxCharacter {
    #[default]
    SoftClip = 0,
    Overdrive = 1,
    Fuzz = 2,
}

impl DriveVfxCharacter {
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        self as u8
    }

    /// Restores the stable desktop ABI value.
    ///
    /// # Errors
    ///
    /// Returns [`CreativeVfxValueError`] for an unknown value.
    pub const fn from_wire_value(value: u8) -> Result<Self, CreativeVfxValueError> {
        match value {
            0 => Ok(Self::SoftClip),
            1 => Ok(Self::Overdrive),
            2 => Ok(Self::Fuzz),
            _ => Err(CreativeVfxValueError),
        }
    }
}

/// Stable rotary motion role.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum RotaryVfxSpeed {
    #[default]
    Slow = 0,
    Fast = 1,
    Brake = 2,
}

impl RotaryVfxSpeed {
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        self as u8
    }

    /// Restores the stable desktop ABI value.
    ///
    /// # Errors
    ///
    /// Returns [`CreativeVfxValueError`] for an unknown value.
    pub const fn from_wire_value(value: u8) -> Result<Self, CreativeVfxValueError> {
        match value {
            0 => Ok(Self::Slow),
            1 => Ok(Self::Fast),
            2 => Ok(Self::Brake),
            _ => Err(CreativeVfxValueError),
        }
    }
}

impl DigitalDegradeVfxCharacter {
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        self as u8
    }

    /// Restores the stable desktop ABI value.
    ///
    /// # Errors
    ///
    /// Returns [`CreativeVfxValueError`] for an unknown value.
    pub const fn from_wire_value(value: u8) -> Result<Self, CreativeVfxValueError> {
        match value {
            0 => Ok(Self::Bitcrusher),
            1 => Ok(Self::SampleRateReduction),
            2 => Ok(Self::LoFi),
            _ => Err(CreativeVfxValueError),
        }
    }
}

impl TransformVfxCharacter {
    #[must_use]
    pub const fn wire_value(self) -> u8 {
        self as u8
    }

    /// Restores the stable desktop ABI value.
    ///
    /// # Errors
    ///
    /// Returns [`CreativeVfxValueError`] for an unknown value.
    pub const fn from_wire_value(value: u8) -> Result<Self, CreativeVfxValueError> {
        match value {
            0 => Ok(Self::Robot),
            1 => Ok(Self::Monster),
            2 => Ok(Self::Tiny),
            3 => Ok(Self::Giant),
            4 => Ok(Self::Ghost),
            _ => Err(CreativeVfxValueError),
        }
    }
}

/// An unknown stable Creative VFX enum value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreativeVfxValueError;

impl std::fmt::Display for CreativeVfxValueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("creative VFX character is outside the stable contract")
    }
}

impl std::error::Error for CreativeVfxValueError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneVfxSettings {
    pub character: SceneVfxCharacter,
    pub enabled: bool,
    pub mix_percent: u8,
    pub intensity_percent: u8,
}

impl Default for SceneVfxSettings {
    fn default() -> Self {
        Self {
            character: SceneVfxCharacter::Telephone,
            enabled: false,
            mix_percent: 100,
            intensity_percent: 50,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlapbackSettings {
    pub delay_millis: u16,
    pub mix_percent: u8,
    pub high_cut_hertz: u16,
}

impl Default for SlapbackSettings {
    fn default() -> Self {
        Self {
            delay_millis: 90,
            mix_percent: 22,
            high_cut_hertz: 7_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EchoSettings {
    pub delay_millis: u16,
    pub feedback_percent: u8,
    pub mix_percent: u8,
    pub high_cut_hertz: u16,
    pub stereo_crossfeed_percent: u8,
}

impl Default for EchoSettings {
    fn default() -> Self {
        Self {
            delay_millis: 375,
            feedback_percent: 36,
            mix_percent: 28,
            high_cut_hertz: 6_500,
            stereo_crossfeed_percent: 70,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DelayVfxSettings {
    pub character: DelayVfxCharacter,
    pub enabled: bool,
    pub slapback: SlapbackSettings,
    pub echo: EchoSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChorusSettings {
    pub mix_percent: u8,
    pub rate_millihertz: u16,
    pub minimum_delay_microseconds: u16,
    pub sweep_microseconds: u16,
    pub stereo_phase_degrees: u16,
}

impl Default for ChorusSettings {
    fn default() -> Self {
        Self {
            mix_percent: 35,
            rate_millihertz: 800,
            minimum_delay_microseconds: 8_000,
            sweep_microseconds: 10_000,
            stereo_phase_degrees: 90,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlangerSettings {
    pub mix_percent: u8,
    pub rate_millihertz: u16,
    pub minimum_delay_microseconds: u16,
    pub sweep_microseconds: u16,
    pub feedback_percent: i8,
    pub stereo_phase_degrees: u16,
}

impl Default for FlangerSettings {
    fn default() -> Self {
        Self {
            mix_percent: 50,
            rate_millihertz: 250,
            minimum_delay_microseconds: 200,
            sweep_microseconds: 3_500,
            feedback_percent: 35,
            stereo_phase_degrees: 180,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaserSettings {
    pub mix_percent: u8,
    pub rate_millihertz: u16,
    pub sweep_low_hertz: u16,
    pub sweep_high_hertz: u16,
    pub feedback_percent: i8,
    pub stereo_phase_degrees: u16,
}

impl Default for PhaserSettings {
    fn default() -> Self {
        Self {
            mix_percent: 50,
            rate_millihertz: 350,
            sweep_low_hertz: 300,
            sweep_high_hertz: 2_500,
            feedback_percent: 25,
            stereo_phase_degrees: 90,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TremoloSettings {
    pub rate_millihertz: u16,
    pub depth_percent: u8,
    pub stereo_phase_degrees: u16,
}

impl Default for TremoloSettings {
    fn default() -> Self {
        Self {
            rate_millihertz: 4_000,
            depth_percent: 60,
            stereo_phase_degrees: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ModulationVfxSettings {
    pub character: ModulationVfxCharacter,
    pub enabled: bool,
    pub chorus: ChorusSettings,
    pub flanger: FlangerSettings,
    pub phaser: PhaserSettings,
    pub tremolo: TremoloSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransformVfxSettings {
    pub character: TransformVfxCharacter,
    pub enabled: bool,
    pub mix_percent: u8,
    pub amount_percent: u8,
}

impl Default for TransformVfxSettings {
    fn default() -> Self {
        Self {
            character: TransformVfxCharacter::Robot,
            enabled: false,
            mix_percent: 100,
            amount_percent: 50,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitcrusherSettings {
    pub bit_depth: u8,
}

impl Default for BitcrusherSettings {
    fn default() -> Self {
        Self { bit_depth: 8 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SampleRateReductionSettings {
    pub target_rate_hertz: u16,
}

impl Default for SampleRateReductionSettings {
    fn default() -> Self {
        Self {
            target_rate_hertz: 8_000,
        }
    }
}

/// Input-driven digital resolution degradation. No dither or independent
/// noise source is part of this authored contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DigitalDegradeVfxSettings {
    pub character: DigitalDegradeVfxCharacter,
    pub enabled: bool,
    pub mix_percent: u8,
    pub bitcrusher: BitcrusherSettings,
    pub sample_rate_reduction: SampleRateReductionSettings,
}

impl Default for DigitalDegradeVfxSettings {
    fn default() -> Self {
        Self {
            character: DigitalDegradeVfxCharacter::Bitcrusher,
            enabled: false,
            mix_percent: 100,
            bitcrusher: BitcrusherSettings::default(),
            sample_rate_reduction: SampleRateReductionSettings::default(),
        }
    }
}

/// Input-driven, antialiased drive. Characters are listening roles rather
/// than physical models of a named circuit or device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriveVfxSettings {
    pub character: DriveVfxCharacter,
    pub enabled: bool,
    pub mix_percent: u8,
    pub drive_centibels: u16,
    pub tone_hertz: u16,
    pub output_gain_centibels: i16,
}

impl Default for DriveVfxSettings {
    fn default() -> Self {
        Self {
            character: DriveVfxCharacter::SoftClip,
            enabled: false,
            mix_percent: 100,
            drive_centibels: 1_200,
            tone_hertz: 8_000,
            output_gain_centibels: -300,
        }
    }
}

/// Generic rotary-speaker-inspired motion without a brand or cabinet claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RotaryVfxSettings {
    pub speed: RotaryVfxSpeed,
    pub enabled: bool,
    pub mix_percent: u8,
    pub motion_percent: u8,
    pub stereo_width_percent: u8,
}

impl Default for RotaryVfxSettings {
    fn default() -> Self {
        Self {
            speed: RotaryVfxSpeed::Slow,
            enabled: false,
            mix_percent: 55,
            motion_percent: 65,
            stereo_width_percent: 80,
        }
    }
}

/// Complete bounded Creative VFX state persisted with one adjustment revision.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreativeVfxSettings {
    #[serde(default)]
    pub scene: SceneVfxSettings,
    #[serde(default)]
    pub delay: DelayVfxSettings,
    #[serde(default)]
    pub modulation: ModulationVfxSettings,
    #[serde(default)]
    pub transform: TransformVfxSettings,
    #[serde(default)]
    pub digital_degrade: DigitalDegradeVfxSettings,
    #[serde(default)]
    pub drive: DriveVfxSettings,
    #[serde(default)]
    pub rotary: RotaryVfxSettings,
    #[serde(default)]
    pub freeze: FreezeVfxSettings,
    #[serde(default)]
    pub granular: GranularVfxSettings,
}

impl CreativeVfxSettings {
    #[must_use]
    pub const fn standard() -> Self {
        Self {
            scene: SceneVfxSettings {
                character: SceneVfxCharacter::Telephone,
                enabled: false,
                mix_percent: 100,
                intensity_percent: 50,
            },
            delay: DelayVfxSettings {
                character: DelayVfxCharacter::Slapback,
                enabled: false,
                slapback: SlapbackSettings {
                    delay_millis: 90,
                    mix_percent: 22,
                    high_cut_hertz: 7_000,
                },
                echo: EchoSettings {
                    delay_millis: 375,
                    feedback_percent: 36,
                    mix_percent: 28,
                    high_cut_hertz: 6_500,
                    stereo_crossfeed_percent: 70,
                },
            },
            modulation: ModulationVfxSettings {
                character: ModulationVfxCharacter::Chorus,
                enabled: false,
                chorus: ChorusSettings {
                    mix_percent: 35,
                    rate_millihertz: 800,
                    minimum_delay_microseconds: 8_000,
                    sweep_microseconds: 10_000,
                    stereo_phase_degrees: 90,
                },
                flanger: FlangerSettings {
                    mix_percent: 50,
                    rate_millihertz: 250,
                    minimum_delay_microseconds: 200,
                    sweep_microseconds: 3_500,
                    feedback_percent: 35,
                    stereo_phase_degrees: 180,
                },
                phaser: PhaserSettings {
                    mix_percent: 50,
                    rate_millihertz: 350,
                    sweep_low_hertz: 300,
                    sweep_high_hertz: 2_500,
                    feedback_percent: 25,
                    stereo_phase_degrees: 90,
                },
                tremolo: TremoloSettings {
                    rate_millihertz: 4_000,
                    depth_percent: 60,
                    stereo_phase_degrees: 0,
                },
            },
            transform: TransformVfxSettings {
                character: TransformVfxCharacter::Robot,
                enabled: false,
                mix_percent: 100,
                amount_percent: 50,
            },
            digital_degrade: DigitalDegradeVfxSettings {
                character: DigitalDegradeVfxCharacter::Bitcrusher,
                enabled: false,
                mix_percent: 100,
                bitcrusher: BitcrusherSettings { bit_depth: 8 },
                sample_rate_reduction: SampleRateReductionSettings {
                    target_rate_hertz: 8_000,
                },
            },
            drive: DriveVfxSettings {
                character: DriveVfxCharacter::SoftClip,
                enabled: false,
                mix_percent: 100,
                drive_centibels: 1_200,
                tone_hertz: 8_000,
                output_gain_centibels: -300,
            },
            rotary: RotaryVfxSettings {
                speed: RotaryVfxSpeed::Slow,
                enabled: false,
                mix_percent: 55,
                motion_percent: 65,
                stereo_width_percent: 80,
            },
            freeze: FreezeVfxSettings::standard(),
            granular: GranularVfxSettings::standard(),
        }
    }

    /// Validates stable authored ranges without preparing DSP state.
    ///
    /// # Errors
    ///
    /// Returns [`CreativeVfxSettingsError`] when any family is outside its
    /// bounded public contract.
    pub fn validate(self) -> Result<(), CreativeVfxSettingsError> {
        if self.scene.mix_percent > 100 || self.scene.intensity_percent > 100 {
            return Err(CreativeVfxSettingsError::Scene);
        }
        if self.delay.slapback.delay_millis < 30
            || self.delay.slapback.delay_millis > 180
            || self.delay.slapback.mix_percent > 100
            || self.delay.slapback.high_cut_hertz < 1_000
            || self.delay.slapback.high_cut_hertz > 20_000
            || self.delay.echo.delay_millis < 80
            || self.delay.echo.delay_millis > 2_000
            || self.delay.echo.feedback_percent > 90
            || self.delay.echo.mix_percent > 100
            || self.delay.echo.high_cut_hertz < 1_000
            || self.delay.echo.high_cut_hertz > 20_000
            || self.delay.echo.stereo_crossfeed_percent > 100
        {
            return Err(CreativeVfxSettingsError::Delay);
        }
        if self.modulation.chorus.mix_percent > 100
            || self.modulation.chorus.rate_millihertz < 50
            || self.modulation.chorus.rate_millihertz > 5_000
            || self.modulation.chorus.minimum_delay_microseconds < 5_000
            || self.modulation.chorus.minimum_delay_microseconds > 25_000
            || self.modulation.chorus.sweep_microseconds < 500
            || self.modulation.chorus.sweep_microseconds > 20_000
            || self.modulation.chorus.minimum_delay_microseconds
                + self.modulation.chorus.sweep_microseconds
                > 45_000
            || self.modulation.chorus.stereo_phase_degrees > 180
            || self.modulation.flanger.mix_percent > 100
            || self.modulation.flanger.rate_millihertz < 50
            || self.modulation.flanger.rate_millihertz > 10_000
            || self.modulation.flanger.minimum_delay_microseconds < 100
            || self.modulation.flanger.minimum_delay_microseconds > 5_000
            || self.modulation.flanger.sweep_microseconds < 100
            || self.modulation.flanger.sweep_microseconds > 10_000
            || self.modulation.flanger.minimum_delay_microseconds
                + self.modulation.flanger.sweep_microseconds
                > 15_000
            || self.modulation.flanger.feedback_percent < -90
            || self.modulation.flanger.feedback_percent > 90
            || self.modulation.flanger.stereo_phase_degrees > 180
            || self.modulation.phaser.mix_percent > 100
            || self.modulation.phaser.rate_millihertz < 50
            || self.modulation.phaser.rate_millihertz > 10_000
            || self.modulation.phaser.sweep_low_hertz < 20
            || self.modulation.phaser.sweep_high_hertz > 20_000
            || self.modulation.phaser.sweep_low_hertz >= self.modulation.phaser.sweep_high_hertz
            || self.modulation.phaser.feedback_percent < -90
            || self.modulation.phaser.feedback_percent > 90
            || self.modulation.phaser.stereo_phase_degrees > 180
            || self.modulation.tremolo.rate_millihertz < 100
            || self.modulation.tremolo.rate_millihertz > 20_000
            || self.modulation.tremolo.depth_percent > 100
            || self.modulation.tremolo.stereo_phase_degrees > 180
        {
            return Err(CreativeVfxSettingsError::Modulation);
        }
        if self.transform.mix_percent > 100 || self.transform.amount_percent > 100 {
            return Err(CreativeVfxSettingsError::Transform);
        }
        if self.digital_degrade.mix_percent > 100
            || self.digital_degrade.bitcrusher.bit_depth < 2
            || self.digital_degrade.bitcrusher.bit_depth > 16
            || self.digital_degrade.sample_rate_reduction.target_rate_hertz < 1_000
            || self.digital_degrade.sample_rate_reduction.target_rate_hertz > 24_000
        {
            return Err(CreativeVfxSettingsError::DigitalDegrade);
        }
        if self.drive.mix_percent > 100
            || self.drive.drive_centibels > 3_600
            || self.drive.tone_hertz < 500
            || self.drive.tone_hertz > 16_000
            || self.drive.output_gain_centibels < -2_400
            || self.drive.output_gain_centibels > 600
        {
            return Err(CreativeVfxSettingsError::Drive);
        }
        if self.rotary.mix_percent > 100
            || self.rotary.motion_percent > 100
            || self.rotary.stereo_width_percent > 100
        {
            return Err(CreativeVfxSettingsError::Rotary);
        }
        if !self.freeze.is_valid() {
            return Err(CreativeVfxSettingsError::Freeze);
        }
        if !self.granular.is_valid() {
            return Err(CreativeVfxSettingsError::Granular);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreativeVfxSettingsError {
    Scene,
    Delay,
    Modulation,
    Transform,
    DigitalDegrade,
    Drive,
    Rotary,
    Freeze,
    Granular,
}

impl std::fmt::Display for CreativeVfxSettingsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let family = match self {
            Self::Scene => "scene",
            Self::Delay => "delay",
            Self::Modulation => "modulation",
            Self::Transform => "transform",
            Self::DigitalDegrade => "digital degrade",
            Self::Drive => "drive",
            Self::Rotary => "rotary",
            Self::Freeze => "freeze",
            Self::Granular => "granular",
        };
        write!(
            formatter,
            "creative VFX {family} settings are outside the supported range"
        )
    }
}

impl std::error::Error for CreativeVfxSettingsError {}

#[cfg(test)]
mod tests;
