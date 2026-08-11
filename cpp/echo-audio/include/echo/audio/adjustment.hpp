#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <vector>

namespace echo::audio {

/// Authored fade interpolation. Numeric values match Echo's stable Catalog
/// representation and are validated before a playback plan is accepted.
enum class FadeCurve : std::uint8_t {
    Linear = 0,
    Smooth = 1,
    EqualPower = 2,
};

inline constexpr std::size_t kParametricEqualizerBandCount = 6;
inline constexpr std::size_t kEffectNodeCount = 7;

enum class EffectNodeKind : std::uint8_t {
    Restoration = 0,
    Equalizer = 1,
    Dynamics = 2,
    Space = 3,
    Master = 4,
    DeHum = 5,
    DeClick = 6,
};

enum class EditSegmentState : std::uint8_t {
    Audible = 0,
    Muted = 1,
    Hidden = 2,
};

/// One original-time interval in the non-destructive source edit list.
struct EditSegment {
    std::uint64_t source_start_millis = 0;
    std::uint64_t source_end_millis = 0;
    EditSegmentState state = EditSegmentState::Audible;
    std::int16_t gain_centibels = 0;
    std::uint64_t fade_in_millis = 0;
    std::uint64_t fade_out_millis = 0;
    FadeCurve fade_in_curve = FadeCurve::Linear;
    FadeCurve fade_out_curve = FadeCurve::Linear;
    std::uint64_t gap_after_millis = 0;
};

/// Original-time activation region for one or more insert effects.
struct EffectMask {
    std::uint64_t start_millis = 0;
    std::uint64_t end_millis = 0;
    std::uint64_t feather_millis = 0;
    std::vector<EffectNodeKind> nodes;
};

enum class EqualizerFilterKind : std::uint8_t {
    Bell = 0,
    LowShelf = 1,
    HighShelf = 2,
    Notch = 3,
};

/// One authored parametric band in stable integer units.
struct ParametricEqualizerBand {
    bool enabled = false;
    EqualizerFilterKind filter_kind = EqualizerFilterKind::Bell;
    std::uint16_t frequency_hertz = 1000;
    std::uint16_t q_hundredths = 100;
    std::int16_t gain_centibels = 0;
};

struct ParametricEqualizerAdjustment {
    bool enabled = true;
    std::array<ParametricEqualizerBand, kParametricEqualizerBandCount> bands{{
        {true, EqualizerFilterKind::LowShelf, 120, 71, 0},
        {false, EqualizerFilterKind::Bell, 250, 100, 0},
        {true, EqualizerFilterKind::Bell, 1000, 100, 0},
        {false, EqualizerFilterKind::Bell, 3000, 100, 0},
        {false, EqualizerFilterKind::Bell, 5000, 100, 0},
        {true, EqualizerFilterKind::HighShelf, 8000, 71, 0},
    }};
};

/// Stereo-linked soft-knee compressor intent. Time values are milliseconds;
/// ratio is stored in tenths so the cross-language contract stays exact.
struct CompressorAdjustment {
    bool enabled = false;
    std::int16_t threshold_centibels = -1800;
    std::uint16_t ratio_tenths = 30;
    std::uint16_t attack_millis = 10;
    std::uint16_t release_millis = 120;
    std::int16_t makeup_centibels = 0;
};

struct NoiseReductionAdjustment {
    bool enabled = false;
    std::uint16_t reduction_centibels = 900;
    std::uint8_t sensitivity_percent = 50;
    std::uint16_t smoothing_millis = 240;
};

struct DeEsserAdjustment {
    bool enabled = false;
    std::uint16_t frequency_hertz = 6500;
    std::int16_t threshold_centibels = -2400;
    std::uint16_t reduction_centibels = 600;
};

struct RestorationAdjustment {
    bool enabled = true;
    NoiseReductionAdjustment noise_reduction;
    DeEsserAdjustment de_esser;
};

/// Finite power-line hum rejection in stable authored units.
struct DeHumAdjustment {
    bool enabled = false;
    std::uint16_t fundamental_hertz = 50;
    std::uint8_t harmonic_count = 4;
    std::uint16_t quality_tenths = 300;
    std::uint16_t depth_centibels = 2400;
};

/// Conservative short-transient repair in stable authored units.
struct DeClickAdjustment {
    bool enabled = false;
    std::uint8_t sensitivity_percent = 50;
    std::uint16_t maximum_click_microseconds = 1000;
    std::uint8_t repair_percent = 100;
};

/// Stereo-linked final-output peak limiter intent.
struct LimiterAdjustment {
    bool enabled = false;
    std::int16_t ceiling_centibels = -100;
    std::uint16_t release_millis = 100;
};

/// Algorithmic room controls in stable integer units.
struct ReverbAdjustment {
    bool enabled = false;
    std::uint8_t mix_percent = 18;
    std::uint16_t pre_delay_millis = 20;
    std::uint16_t decay_millis = 1800;
    std::uint8_t size_percent = 55;
    std::uint8_t damping_percent = 45;
    std::uint16_t low_cut_hertz = 120;
    std::uint16_t high_cut_hertz = 10000;
};

/// Authored non-destructive playback adjustments. Milliseconds, centibels,
/// and hertz are explicit so the cross-language boundary never relies on
/// floating-point UI units. A zero `trim_end_millis` means source end.
struct PlaybackAdjustment {
    std::uint64_t trim_start_millis = 0;
    std::uint64_t trim_end_millis = 0;
    std::uint64_t fade_in_millis = 0;
    std::uint64_t fade_out_millis = 0;
    FadeCurve fade_in_curve = FadeCurve::Linear;
    FadeCurve fade_out_curve = FadeCurve::Linear;
    std::int16_t gain_centibels = 0;
    /// High-pass cutoff in hertz, or zero when disabled.
    std::uint16_t low_cut_hertz = 0;
    RestorationAdjustment restoration;
    DeHumAdjustment de_hum;
    DeClickAdjustment de_click;
    ParametricEqualizerAdjustment equalizer;
    CompressorAdjustment compressor;
    ReverbAdjustment reverb;
    LimiterAdjustment limiter;
    std::array<EffectNodeKind, kEffectNodeCount> effect_chain{{
        EffectNodeKind::Restoration,
        EffectNodeKind::Equalizer,
        EffectNodeKind::Dynamics,
        EffectNodeKind::Space,
        EffectNodeKind::Master,
        EffectNodeKind::DeHum,
        EffectNodeKind::DeClick,
    }};
    std::uint8_t effect_chain_count = 5;
    std::vector<EditSegment> edit_segments;
    std::vector<EffectMask> effect_masks;
};

/// Playback-ready adjustment compiled once before decoding begins.
///
/// The producer thread asks this value for one multiplication coefficient
/// per source frame. The realtime callback never evaluates the graph.
class PreparedAdjustment {
  public:
    PreparedAdjustment(
        PlaybackAdjustment authored,
        std::uint64_t source_duration_millis,
        std::uint32_t sample_rate
    );

    [[nodiscard]] std::uint64_t start_frame() const;
    [[nodiscard]] std::uint64_t end_frame() const;
    [[nodiscard]] std::uint64_t trim_start_millis() const;
    [[nodiscard]] std::uint64_t trim_end_millis() const;
    [[nodiscard]] std::uint16_t low_cut_hertz() const;
    [[nodiscard]] RestorationAdjustment restoration() const;
    [[nodiscard]] DeHumAdjustment de_hum() const;
    [[nodiscard]] DeClickAdjustment de_click() const;
    [[nodiscard]] ParametricEqualizerAdjustment equalizer() const;
    [[nodiscard]] CompressorAdjustment compressor() const;
    [[nodiscard]] ReverbAdjustment reverb() const;
    [[nodiscard]] LimiterAdjustment limiter() const;
    [[nodiscard]] std::array<EffectNodeKind, kEffectNodeCount> effect_chain() const;
    [[nodiscard]] std::size_t effect_chain_count() const;
    [[nodiscard]] std::uint64_t clamp_seek_millis(std::uint64_t millis) const;
    [[nodiscard]] float amplitude_at(std::uint64_t source_frame) const;
    [[nodiscard]] float gain_amplitude() const;
    [[nodiscard]] float envelope_at(std::uint64_t source_frame) const;

  private:
    std::uint64_t start_frame_ = 0;
    std::uint64_t end_frame_ = 0;
    std::uint64_t fade_in_frames_ = 0;
    std::uint64_t fade_out_frames_ = 0;
    std::uint64_t trim_start_millis_ = 0;
    std::uint64_t trim_end_millis_ = 0;
    FadeCurve fade_in_curve_ = FadeCurve::Linear;
    FadeCurve fade_out_curve_ = FadeCurve::Linear;
    float gain_amplitude_ = 1.0F;
    std::uint16_t low_cut_hertz_ = 0;
    RestorationAdjustment restoration_;
    DeHumAdjustment de_hum_;
    DeClickAdjustment de_click_;
    ParametricEqualizerAdjustment equalizer_;
    CompressorAdjustment compressor_;
    ReverbAdjustment reverb_;
    LimiterAdjustment limiter_;
    std::array<EffectNodeKind, kEffectNodeCount> effect_chain_;
    std::size_t effect_chain_count_ = 5;
};

} // namespace echo::audio
