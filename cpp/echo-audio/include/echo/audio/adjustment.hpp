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

/// Fixed-band equalizer gain intent in hundredths of one decibel.
struct ThreeBandEqualizerAdjustment {
    std::int16_t low_gain_centibels = 0;
    std::int16_t mid_gain_centibels = 0;
    std::int16_t high_gain_centibels = 0;
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
    ThreeBandEqualizerAdjustment equalizer;
    CompressorAdjustment compressor;
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
    [[nodiscard]] ThreeBandEqualizerAdjustment equalizer() const;
    [[nodiscard]] CompressorAdjustment compressor() const;
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
    ThreeBandEqualizerAdjustment equalizer_;
    CompressorAdjustment compressor_;
};

} // namespace echo::audio
