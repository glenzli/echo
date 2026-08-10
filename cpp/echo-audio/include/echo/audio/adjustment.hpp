#pragma once

#include <array>
#include <cstddef>
#include <cstdint>

namespace echo::audio {

/// Authored fade interpolation. Numeric values match Echo's stable Catalog
/// representation and are validated before a playback plan is accepted.
enum class FadeCurve : std::uint8_t {
    Linear = 0,
    Smooth = 1,
    EqualPower = 2,
};

inline constexpr std::size_t kParametricEqualizerBandCount = 6;

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

/// Stereo-linked final-output peak limiter intent.
struct LimiterAdjustment {
    bool enabled = false;
    std::int16_t ceiling_centibels = -100;
    std::uint16_t release_millis = 100;
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
    ParametricEqualizerAdjustment equalizer;
    CompressorAdjustment compressor;
    LimiterAdjustment limiter;
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
    [[nodiscard]] ParametricEqualizerAdjustment equalizer() const;
    [[nodiscard]] CompressorAdjustment compressor() const;
    [[nodiscard]] LimiterAdjustment limiter() const;
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
    ParametricEqualizerAdjustment equalizer_;
    CompressorAdjustment compressor_;
    LimiterAdjustment limiter_;
};

} // namespace echo::audio
