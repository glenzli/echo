#pragma once

#include <cstddef>
#include <cstdint>
#include <vector>

namespace echo::audio {

/// Prepared second-order Butterworth high-pass filter for decoded PCM.
///
/// State is channel-local and owned by the decode producer. A zero cutoff is
/// an exact bypass. The realtime sink only consumes already filtered frames.
class LowCutFilter {
  public:
    LowCutFilter(std::uint16_t cutoff_hertz, std::uint32_t sample_rate, std::size_t channel_count);

    [[nodiscard]] float process_sample(float input, std::size_t channel);
    void reset();
    [[nodiscard]] bool enabled() const;

  private:
    struct ChannelState {
        float delay_one = 0.0F;
        float delay_two = 0.0F;
    };

    float b_zero_ = 1.0F;
    float b_one_ = 0.0F;
    float b_two_ = 0.0F;
    float a_one_ = 0.0F;
    float a_two_ = 0.0F;
    bool enabled_ = false;
    std::vector<ChannelState> states_;
};

} // namespace echo::audio
