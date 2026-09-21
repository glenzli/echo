#pragma once

#include "echo/audio/de_click_processor.hpp"
#include <cstdint>
#include <limits>
#include <optional>
#include <stop_token>
#include <string>
#include <vector>

namespace echo::audio {

struct ClickCandidate {
    std::uint64_t start_frame = 0;
    std::uint64_t end_frame = 0;
    std::uint32_t channel_mask = 0;
    float maximum_difference = 0;
};

struct ClickAnalysisResult {
    std::vector<ClickCandidate> candidates;
    std::uint64_t total_candidates = 0;
    std::uint64_t analyzed_frames = 0;
};

/// Bounded original-vs-repaired comparison at canonical 48 kHz. Frames stay
/// relative to the first input; no synthetic tail is used to invent evidence.
class ClickCandidateAnalyzer {
  public:
    ClickCandidateAnalyzer(
        DeClickParameters parameters,
        std::size_t channels,
        std::uint64_t first_frame = 0,
        std::uint64_t last_frame = std::numeric_limits<std::uint64_t>::max()
    );
    void process_interleaved(const float* samples, std::size_t frames);
    [[nodiscard]] ClickAnalysisResult result() const;

  private:
    void finish_candidate();
    DeClickProcessor processor_;
    std::size_t channels_;
    std::vector<float> work_;
    std::vector<float> history_;
    std::uint64_t input_frames_ = 0;
    std::uint64_t first_frame_;
    std::uint64_t last_frame_;
    std::optional<ClickCandidate> current_;
    ClickAnalysisResult result_;
};

/// Source-only, at most five minutes, 256 displayed candidates, cancellable.
/// Returned frames are absolute original coordinates at 48 kHz. This is an
/// algorithmic suggestion, not a determination that a transient is unwanted.
[[nodiscard]] ClickAnalysisResult analyze_clicks(
    const std::string& path,
    std::uint64_t start_millis,
    std::uint64_t end_millis,
    DeClickParameters parameters,
    std::stop_token cancellation = {}
);

} // namespace echo::audio
