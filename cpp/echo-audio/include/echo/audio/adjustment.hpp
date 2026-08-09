#pragma once

#include <cstdint>

namespace echo::audio {

/// Authored fade interpolation. Numeric values match Echo's stable Catalog
/// representation and are validated before a playback plan is accepted.
enum class FadeCurve : std::uint8_t {
    Linear = 0,
    Smooth = 1,
    EqualPower = 2,
};

/// Authored non-destructive playback adjustments. Milliseconds and
/// centibels are explicit so the cross-language boundary never relies on
/// floating-point UI units. A zero `trim_end_millis` means source end.
struct PlaybackAdjustment {
    std::uint64_t trim_start_millis = 0;
    std::uint64_t trim_end_millis = 0;
    std::uint64_t fade_in_millis = 0;
    std::uint64_t fade_out_millis = 0;
    FadeCurve fade_in_curve = FadeCurve::Linear;
    FadeCurve fade_out_curve = FadeCurve::Linear;
    std::int16_t gain_centibels = 0;
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
    [[nodiscard]] std::uint64_t clamp_seek_millis(std::uint64_t millis) const;
    [[nodiscard]] float amplitude_at(std::uint64_t source_frame) const;

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
};

} // namespace echo::audio
