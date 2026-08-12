#pragma once

#include <cstdint>

namespace echo::audio {

enum class SceneVfxCharacter : std::uint8_t {
    Telephone = 0,
    Radio = 1,
    Intercom = 2,
    BehindWall = 3,
    Underwater = 4,
};

struct SceneVfxAdjustment {
    SceneVfxCharacter character = SceneVfxCharacter::Telephone;
    bool enabled = false;
    std::uint8_t mix_percent = 100;
    std::uint8_t intensity_percent = 50;
};

enum class DelayVfxCharacter : std::uint8_t {
    Slapback = 0,
    Echo = 1,
};

struct SlapbackAdjustment {
    std::uint16_t delay_millis = 90;
    std::uint8_t mix_percent = 22;
    std::uint16_t high_cut_hertz = 7000;
};

struct EchoAdjustment {
    std::uint16_t delay_millis = 375;
    std::uint8_t feedback_percent = 36;
    std::uint8_t mix_percent = 28;
    std::uint16_t high_cut_hertz = 6500;
    std::uint8_t stereo_crossfeed_percent = 70;
};

struct DelayVfxAdjustment {
    DelayVfxCharacter character = DelayVfxCharacter::Slapback;
    bool enabled = false;
    SlapbackAdjustment slapback;
    EchoAdjustment echo;
};

enum class ModulationVfxCharacter : std::uint8_t {
    Chorus = 0,
    Flanger = 1,
    Phaser = 2,
    Tremolo = 3,
};

struct ChorusAdjustment {
    std::uint8_t mix_percent = 35;
    std::uint16_t rate_millihertz = 800;
    std::uint16_t minimum_delay_microseconds = 8000;
    std::uint16_t sweep_microseconds = 10000;
    std::uint16_t stereo_phase_degrees = 90;
};

struct FlangerAdjustment {
    std::uint8_t mix_percent = 50;
    std::uint16_t rate_millihertz = 250;
    std::uint16_t minimum_delay_microseconds = 200;
    std::uint16_t sweep_microseconds = 3500;
    std::int8_t feedback_percent = 35;
    std::uint16_t stereo_phase_degrees = 180;
};

struct PhaserAdjustment {
    std::uint8_t mix_percent = 50;
    std::uint16_t rate_millihertz = 350;
    std::uint16_t sweep_low_hertz = 300;
    std::uint16_t sweep_high_hertz = 2500;
    std::int8_t feedback_percent = 25;
    std::uint16_t stereo_phase_degrees = 90;
};

struct TremoloAdjustment {
    std::uint16_t rate_millihertz = 4000;
    std::uint8_t depth_percent = 60;
    std::uint16_t stereo_phase_degrees = 0;
};

struct ModulationVfxAdjustment {
    ModulationVfxCharacter character = ModulationVfxCharacter::Chorus;
    bool enabled = false;
    ChorusAdjustment chorus;
    FlangerAdjustment flanger;
    PhaserAdjustment phaser;
    TremoloAdjustment tremolo;
};

enum class TransformVfxCharacter : std::uint8_t {
    Robot = 0,
    Monster = 1,
    Tiny = 2,
    Giant = 3,
    Ghost = 4,
};

struct TransformVfxAdjustment {
    TransformVfxCharacter character = TransformVfxCharacter::Robot;
    bool enabled = false;
    std::uint8_t mix_percent = 100;
    std::uint8_t amount_percent = 50;
};

enum class DigitalDegradeVfxCharacter : std::uint8_t {
    Bitcrusher = 0,
    SampleRateReduction = 1,
    LoFi = 2,
};

struct BitcrusherAdjustment {
    std::uint8_t bit_depth = 8;
};

struct SampleRateReductionAdjustment {
    std::uint16_t target_rate_hertz = 8000;
};

struct DigitalDegradeVfxAdjustment {
    DigitalDegradeVfxCharacter character = DigitalDegradeVfxCharacter::Bitcrusher;
    bool enabled = false;
    std::uint8_t mix_percent = 100;
    BitcrusherAdjustment bitcrusher;
    SampleRateReductionAdjustment sample_rate_reduction;
};

enum class DriveVfxCharacter : std::uint8_t {
    SoftClip = 0,
    Overdrive = 1,
    Fuzz = 2,
};

/// Authored input for a deliberately stylized, non-physical drive effect.
struct DriveVfxAdjustment {
    DriveVfxCharacter character = DriveVfxCharacter::SoftClip;
    bool enabled = false;
    std::uint8_t mix_percent = 100;
    std::uint16_t drive_centibels = 1200;
    std::uint16_t tone_hertz = 8000;
    std::int16_t output_gain_centibels = -300;
};

enum class RotaryVfxSpeed : std::uint8_t {
    Slow = 0,
    Fast = 1,
    Brake = 2,
};

/// Authored input for a generic rotary-speaker-inspired creative effect.
struct RotaryVfxAdjustment {
    RotaryVfxSpeed speed = RotaryVfxSpeed::Slow;
    bool enabled = false;
    std::uint8_t mix_percent = 55;
    std::uint8_t motion_percent = 65;
    std::uint8_t stereo_width_percent = 80;
};

/// Complete prepared input state for the independent Creative VFX nodes.
struct CreativeVfxAdjustment {
    SceneVfxAdjustment scene;
    DelayVfxAdjustment delay;
    ModulationVfxAdjustment modulation;
    TransformVfxAdjustment transform;
    DigitalDegradeVfxAdjustment digital_degrade;
    DriveVfxAdjustment drive;
    RotaryVfxAdjustment rotary;
};

} // namespace echo::audio
