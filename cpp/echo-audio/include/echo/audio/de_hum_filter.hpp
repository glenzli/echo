#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>

namespace echo::audio {

struct DeHumParameters {
    bool enabled = false;
    std::uint16_t fundamental_hertz = 50;
    std::uint8_t harmonic_count = 4;
    std::uint16_t quality_tenths = 300;
    std::uint16_t depth_centibels = 2400;
};

/// Finite harmonic hum rejection with click-free authored updates.
///
/// Every harmonic blends a narrow notch with the dry signal for bounded depth,
/// preserving nearby programme material. Updates crossfade complete prepared
/// banks, so frequency, Q, depth, harmonic-count, and bypass changes cannot
/// expose coefficient or filter-state discontinuities. Processing allocates
/// nothing and keeps independent state for every prepared channel.
class DeHumFilter {
  public:
    DeHumFilter(DeHumParameters parameters, std::uint32_t sample_rate, std::size_t channel_count);

    void update(DeHumParameters parameters);
    void process_interleaved(float* samples, std::size_t frame_count, std::size_t channel_count);
    void reset();

    [[nodiscard]] DeHumParameters parameters() const;
    [[nodiscard]] bool is_bypassed() const;

  private:
    static constexpr std::size_t kMaximumHarmonics = 8;
    static constexpr std::size_t kMaximumChannels = 8;

    struct State {
        float z1 = 0.0F;
        float z2 = 0.0F;
    };

    struct Section {
        float b0 = 1.0F;
        float b1 = 0.0F;
        float b2 = 0.0F;
        float a1 = 0.0F;
        float a2 = 0.0F;
        float mix = 0.0F;
        std::array<State, kMaximumChannels> states{};

        [[nodiscard]] float process(float sample, std::size_t channel);
        void reset();
    };

    struct Bank {
        DeHumParameters parameters;
        std::array<Section, kMaximumHarmonics> sections{};
        std::size_t section_count = 0;
        bool bypassed = true;

        [[nodiscard]] float process(float sample, std::size_t channel);
        void reset();
    };

    static void
    validate(DeHumParameters parameters, std::uint32_t sample_rate, std::size_t channel_count);
    [[nodiscard]] static Bank prepare(DeHumParameters parameters, std::uint32_t sample_rate);
    [[nodiscard]] static bool same(DeHumParameters left, DeHumParameters right);

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    std::size_t transition_total_frames_ = 1;
    std::size_t transition_frame_ = 0;
    Bank current_;
    std::optional<Bank> next_;
    std::optional<Bank> pending_;
};

} // namespace echo::audio
