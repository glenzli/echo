#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <vector>

#include "echo/audio/adjustment.hpp"

namespace echo::audio {

/// Prepared six-band parametric equalizer with click-free target changes.
class ParametricEqualizer {
  public:
    ParametricEqualizer(
        ParametricEqualizerAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );

    void transition_to(ParametricEqualizerAdjustment adjustment);
    [[nodiscard]] float process_sample(float sample, std::size_t channel);
    void reset();
    [[nodiscard]] bool is_bypassed() const;
    [[nodiscard]] static double response_decibels(
        ParametricEqualizerAdjustment adjustment,
        std::uint32_t sample_rate,
        double frequency_hertz
    );

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
        ParametricEqualizerAdjustment adjustment;
        std::array<Section, kParametricEqualizerBandCount> sections;
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
    [[nodiscard]] static Section prepare_section(
        ParametricEqualizerBand band,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    [[nodiscard]] static Bank prepare(
        ParametricEqualizerAdjustment adjustment,
        std::uint32_t sample_rate,
        std::size_t channel_count
    );
    static void validate(ParametricEqualizerAdjustment adjustment, std::uint32_t sample_rate);
    static bool same(ParametricEqualizerAdjustment left, ParametricEqualizerAdjustment right);
    void begin_transition(ParametricEqualizerAdjustment adjustment);

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    std::size_t transition_total_frames_ = 0;
    std::size_t transition_frame_ = 0;
    Bank current_;
    std::optional<Bank> next_;
    std::optional<ParametricEqualizerAdjustment> pending_;
};

} // namespace echo::audio
