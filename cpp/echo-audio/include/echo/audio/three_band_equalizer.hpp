#pragma once

#include <cstddef>
#include <cstdint>
#include <optional>
#include <vector>

#include "echo/audio/adjustment.hpp"

namespace echo::audio {

/// Prepared three-band restoration equalizer with click-free target changes.
///
/// Coefficients and per-channel state are allocated before decoding starts;
/// target banks are prepared on the decode producer and linearly crossfaded
/// over a short, fixed interval. The realtime callback never touches this
/// owner.
class ThreeBandEqualizer {
  public:
    ThreeBandEqualizer(
        ThreeBandEqualizerAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );

    /// Queues a new authored target. Rapid superseding changes coalesce to
    /// the newest target while the active transition finishes.
    void transition_to(ThreeBandEqualizerAdjustment adjustment);
    [[nodiscard]] float process_sample(float sample, std::size_t channel);
    void reset();
    [[nodiscard]] bool is_bypassed() const;

  private:
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
        std::vector<State> states;

        [[nodiscard]] float process(float sample, std::size_t channel);
        void reset();
    };

    struct Bank {
        ThreeBandEqualizerAdjustment adjustment;
        Section low;
        Section mid;
        Section high;
        bool bypassed = true;

        [[nodiscard]] float process(float sample, std::size_t channel);
        void reset();
    };

    struct Coefficients {
        double b0;
        double b1;
        double b2;
        double a0;
        double a1;
        double a2;
    };

    static Section normalized(Coefficients coefficients, std::size_t channel_count);
    [[nodiscard]] static Section
    low_shelf(std::int16_t gain_centibels, std::uint32_t sample_rate, std::size_t channel_count);
    [[nodiscard]] static Section
    peaking(std::int16_t gain_centibels, std::uint32_t sample_rate, std::size_t channel_count);
    [[nodiscard]] static Section
    high_shelf(std::int16_t gain_centibels, std::uint32_t sample_rate, std::size_t channel_count);
    [[nodiscard]] static Bank prepare(
        ThreeBandEqualizerAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    static void validate(ThreeBandEqualizerAdjustment adjustment);
    static bool same(ThreeBandEqualizerAdjustment left, ThreeBandEqualizerAdjustment right);
    void begin_transition(ThreeBandEqualizerAdjustment adjustment);

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    std::size_t transition_total_frames_ = 0;
    std::size_t transition_frame_ = 0;
    Bank current_;
    std::optional<Bank> next_;
    std::optional<ThreeBandEqualizerAdjustment> pending_;
};

} // namespace echo::audio
