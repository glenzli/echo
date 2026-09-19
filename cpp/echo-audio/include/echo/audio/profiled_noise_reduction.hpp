#pragma once

#include <cstdint>
#include <span>
#include <stop_token>
#include <string>
#include <vector>

namespace echo::audio {

inline constexpr std::size_t kNoiseProfileWindowFrames = 2'048;
inline constexpr std::size_t kNoiseProfileBins = kNoiseProfileWindowFrames / 2 + 1;

/// Version 1 uses mean Hann-windowed power, normalized by (4 / 2048)^2,
/// at the canonical 48 kHz rate. Centibels store 10 log10(power).
struct ProfiledNoiseReduction {
    bool enabled = false;
    std::uint16_t algorithm_version = 1;
    std::uint64_t capture_start_millis = 0;
    std::uint64_t capture_end_millis = 0;
    std::vector<std::int16_t> power_centibels;
    std::int16_t reduction_centibels = 1'200;
    std::int16_t sensitivity_centibels = 600;
    std::uint16_t smoothing_bins = 3;
    /// Diagnostic only: never serialized in an authored adjustment.
    bool residue = false;
};

void validate_noise_profile(const ProfiledNoiseReduction& settings, std::uint64_t duration_millis);

/// Explicit Original-only analysis; bounded to 0.1–30 seconds and cancellable.
/// The returned profile is disabled until the user enables its processing.
[[nodiscard]] ProfiledNoiseReduction learn_noise_profile(
    const std::string& path,
    std::uint64_t start_millis,
    std::uint64_t end_millis,
    std::stop_token cancellation = {}
);

/// Producer-thread spectral gain estimation. The caller supplies the maximum
/// power across channels and applies one common gain to preserve the stereo image.
class ProfiledNoiseReducer {
  public:
    explicit ProfiledNoiseReducer(ProfiledNoiseReduction settings);
    void reset();
    [[nodiscard]] std::vector<float> gains(std::span<const float> powers);

  private:
    ProfiledNoiseReduction settings_;
    std::vector<float> noise_power_;
    std::vector<float> previous_power_;
    std::vector<float> previous_gain_;
    bool started_ = false;
};

} // namespace echo::audio
