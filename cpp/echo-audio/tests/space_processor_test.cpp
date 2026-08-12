#include "echo/audio/space_processor.hpp"

#include <cassert>
#include <cmath>
#include <memory>
#include <vector>

namespace {

std::shared_ptr<const echo::audio::LoadedPreparedImpulseResponse> impulse() {
    auto value = std::make_shared<echo::audio::LoadedPreparedImpulseResponse>();
    value->preparation_version = 1;
    value->left = {1.0F, 0.25F, 0.0F, -0.1F};
    value->right = value->left;
    return value;
}

echo::audio::SpaceAdjustment convolution_space() {
    return {
        .mode = echo::audio::SpaceMode::Convolution,
        .convolution = {
            .import_id = "018f5f1a-ff90-7c71-9ec4-66d36516664c",
            .source_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            .prepared_hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            .adjustment = {.enabled = true, .mix_percent = 35, .wet_gain_centibels = 0},
            .impulse = impulse(),
        },
    };
}

void disabled_algorithmic_space_is_identity() {
    echo::audio::SpaceAdjustment adjustment;
    adjustment.algorithmic.enabled = false;
    echo::audio::SpaceProcessor processor(adjustment, 48000, 2);
    std::vector<float> samples{0.1F, -0.2F, 0.3F, -0.4F};
    const auto original = samples;
    processor.process_interleaved(samples.data(), 2, 2);
    assert(samples == original);
}

void convolution_switch_is_finite_and_silence_preserving() {
    echo::audio::SpaceAdjustment adjustment;
    echo::audio::SpaceProcessor processor(adjustment, 48000, 2);
    processor.update(convolution_space());
    std::vector<float> samples(4096 * 2, 0.0F);
    processor.process_interleaved(samples.data(), 4096, 2);
    for (const float sample : samples) {
        assert(std::isfinite(sample));
        assert(sample == 0.0F);
    }
    processor.reset();
}

} // namespace

int main() {
    disabled_algorithmic_space_is_identity();
    convolution_switch_is_finite_and_silence_preserving();
}
