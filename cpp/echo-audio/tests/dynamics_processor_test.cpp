#include "echo/audio/dynamics_processor.hpp"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <stdexcept>
#include <vector>

namespace {

void expect(bool condition, const char* message) {
    if (!condition) {
        throw std::runtime_error(message);
    }
}

} // namespace

int main() {
    try {
        echo::audio::DynamicsProcessor compressor(
            {
                .enabled = true,
                .threshold_centibels = -1200,
                .ratio_tenths = 40,
                .attack_millis = 1,
                .release_millis = 80,
                .makeup_centibels = 0,
            },
            48'000
        );
        std::vector<float> loud(48'000 * 2, 1.0F);
        compressor.process_interleaved(loud.data(), 48'000, 2);
        expect(std::abs(loud.back()) < 0.5F, "steady signal receives meaningful compression");
        expect(
            std::abs(loud[loud.size() - 1] - loud[loud.size() - 2]) < 1.0E-6F,
            "stereo-linked detector applies identical gain"
        );

        const float before_update = compressor.current_gain();
        compressor.update({
            .enabled = true,
            .threshold_centibels = -2400,
            .ratio_tenths = 80,
            .attack_millis = 20,
            .release_millis = 200,
            .makeup_centibels = 300,
        });
        expect(
            std::abs(compressor.current_gain() - before_update) < 1.0E-6F,
            "live update preserves the detector envelope"
        );

        std::vector<float> transition(512 * 2, 0.8F);
        compressor.process_interleaved(transition.data(), 512, 2);
        float largest_step = 0.0F;
        for (std::size_t index = 2; index < transition.size(); index += 2) {
            largest_step =
                std::max(largest_step, std::abs(transition[index] - transition[index - 2]));
        }
        expect(largest_step < 0.02F, "live parameter changes stay click-free");

        compressor.update({});
        std::vector<float> bypass(96'000 * 2, 0.25F);
        compressor.process_interleaved(bypass.data(), 96'000, 2);
        expect(
            std::abs(bypass.back() - 0.25F) < 0.01F,
            "disabled compressor releases smoothly back to unity"
        );

        bool rejected = false;
        try {
            compressor.update({.ratio_tenths = 201});
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        expect(rejected, "invalid ratio is rejected");
        std::printf("dynamics processor test: ok\n");
        return 0;
    } catch (const std::exception& error) {
        std::fprintf(stderr, "dynamics processor test failed: %s\n", error.what());
        return 1;
    }
}
