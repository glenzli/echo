#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <vector>

#include "echo/audio/k_weighting_filter.hpp"

namespace echo::audio {

struct OfflineLoudnessResult {
    float integrated_lufs = -70.0F;
    float true_peak_dbtp = -70.0F;
};

/// Bounded-memory whole-program loudness analysis.
///
/// Integrated loudness uses 400 ms K-weighted blocks at 100 ms steps with
/// the BS.1770 absolute and relative gates. True peak is a four-phase cubic
/// reconstruction estimate; it is intentionally reported as an estimate,
/// not a conformance certificate for a rendered deliverable.
class OfflineLoudnessAnalyzer {
  public:
    OfflineLoudnessAnalyzer(std::uint32_t sample_rate, std::size_t channel_count);

    void
    process_interleaved(const float* samples, std::size_t frame_count, std::size_t channel_count);
    [[nodiscard]] OfflineLoudnessResult result() const;

  private:
    static constexpr std::size_t kTruePeakHistory = 4;

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    KWeightingFilter weighting_;
    std::vector<double> energy_window_;
    std::size_t energy_cursor_ = 0;
    std::size_t energy_count_ = 0;
    double energy_sum_ = 0.0;
    double total_energy_ = 0.0;
    std::uint64_t total_frames_ = 0;
    std::size_t frames_since_block_ = 0;
    std::size_t block_step_frames_ = 0;
    std::vector<double> block_energies_;
    std::vector<std::array<float, kTruePeakHistory>> true_peak_history_;
    std::size_t true_peak_history_count_ = 0;
    double true_peak_amplitude_ = 0.0;
};

} // namespace echo::audio
