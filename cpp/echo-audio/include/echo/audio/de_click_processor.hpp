#pragma once

#include <cstddef>
#include <cstdint>
#include <vector>

namespace echo::audio {

struct DeClickParameters {
    bool enabled = false;
    std::uint8_t sensitivity_percent = 50;
    std::uint16_t maximum_click_microseconds = 1000;
    std::uint8_t repair_percent = 100;
};

/// Conservative short-transient repair with bounded fixed look-ahead.
///
/// A candidate must jump away from the preceding local trajectory and return
/// through an opposite edge within the authored maximum duration. Sustained
/// steps and broad transients therefore remain untouched. Confirmed spans are
/// softened with endpoint interpolation, while repair amount and bypass are
/// sample-smoothed. The fixed latency never changes during live edits and the
/// processing path allocates nothing.
class DeClickProcessor {
  public:
    DeClickProcessor(
        DeClickParameters parameters,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );

    void update(DeClickParameters parameters);
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);
    void reset();

    [[nodiscard]] DeClickParameters parameters() const;
    [[nodiscard]] std::size_t latency_frames() const;
    [[nodiscard]] bool is_bypassed() const;

  private:
    struct ChannelState {
        std::size_t repair_total_frames = 0;
        std::size_t repair_frame = 0;
        float repair_start = 0.0F;
        float repair_end = 0.0F;
    };

    static void
    validate(DeClickParameters parameters, std::uint32_t sample_rate, std::size_t channel_count);
    [[nodiscard]] float history(std::size_t channel, std::size_t frames_ago) const;
    [[nodiscard]] std::size_t maximum_click_frames() const;
    [[nodiscard]] std::size_t detect_click(std::size_t channel) const;
    [[nodiscard]] float repaired_sample(std::size_t channel, float delayed_input);

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    std::size_t maximum_supported_click_frames_ = 0;
    std::size_t latency_frames_ = 0;
    std::size_t history_capacity_frames_ = 0;
    std::size_t history_cursor_ = 0;
    std::size_t frames_seen_ = 0;
    std::vector<float> history_;
    std::vector<ChannelState> channel_states_;
    DeClickParameters target_;
    float sensitivity_percent_ = 50.0F;
    float repair_mix_ = 0.0F;
    float parameter_coefficient_ = 1.0F;
};

} // namespace echo::audio
