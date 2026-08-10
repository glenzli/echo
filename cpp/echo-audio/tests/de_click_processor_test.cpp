#include "echo/audio/de_click_processor.hpp"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <numbers>
#include <stdexcept>
#include <vector>

namespace {

constexpr std::uint32_t kSampleRate = 48000;

void expect(bool condition, const char* message) {
    if (!condition) {
        throw std::runtime_error(message);
    }
}

std::vector<float> sine(float frequency_hertz, float amplitude, std::size_t frame_count) {
    std::vector<float> samples(frame_count);
    for (std::size_t frame = 0; frame < frame_count; ++frame) {
        samples[frame] = amplitude
                         * std::sin(
                             2.0F * std::numbers::pi_v<float> * frequency_hertz
                             * static_cast<float>(frame) / static_cast<float>(kSampleRate)
                         );
    }
    return samples;
}

echo::audio::DeClickParameters enabled_parameters() {
    return {
        .enabled = true,
        .sensitivity_percent = 85,
        .maximum_click_microseconds = 1000,
        .repair_percent = 100,
    };
}

} // namespace

int main() {
    try {
        constexpr std::size_t frame_count = 12000;
        constexpr std::size_t click_frame = 5000;
        const auto clean = sine(220.0F, 0.15F, frame_count);

        auto clicked = clean;
        clicked[click_frame] += 1.0F;
        echo::audio::DeClickProcessor single(enabled_parameters(), kSampleRate, 1);
        const std::size_t latency = single.latency_frames();
        single.process_interleaved(clicked.data(), clicked.size(), 1);
        expect(
            std::abs(clicked[click_frame + latency] - clean[click_frame]) < 0.02F,
            "single-sample click is locally reconstructed"
        );

        auto burst = clean;
        burst[click_frame] += 0.9F;
        burst[click_frame + 1] += 0.85F;
        burst[click_frame + 2] += 0.8F;
        echo::audio::DeClickProcessor multi(enabled_parameters(), kSampleRate, 1);
        multi.process_interleaved(burst.data(), burst.size(), 1);
        for (std::size_t offset = 0; offset < 3; ++offset) {
            expect(
                std::abs(burst[click_frame + latency + offset] - clean[click_frame + offset])
                    < 0.03F,
                "short multi-frame click is interpolated as one span"
            );
        }

        std::vector<float> step(frame_count, 0.0F);
        std::fill(step.begin() + click_frame, step.end(), 0.5F);
        echo::audio::DeClickProcessor step_filter(enabled_parameters(), kSampleRate, 1);
        step_filter.process_interleaved(step.data(), step.size(), 1);
        expect(
            step[click_frame + latency] > 0.45F && step[click_frame + latency + 64] > 0.45F,
            "sustained steps are not mistaken for clicks"
        );

        const auto right_clean = sine(733.0F, 0.12F, frame_count);
        std::vector<float> stereo(frame_count * 2);
        for (std::size_t frame = 0; frame < frame_count; ++frame) {
            stereo[frame * 2] = clean[frame];
            stereo[frame * 2 + 1] = right_clean[frame];
        }
        stereo[click_frame * 2] += 1.0F;
        echo::audio::DeClickProcessor stereo_filter(enabled_parameters(), kSampleRate, 2);
        stereo_filter.process_interleaved(stereo.data(), frame_count, 2);
        expect(
            std::abs(stereo[(click_frame + latency) * 2] - clean[click_frame]) < 0.02F,
            "left-channel click is repaired"
        );
        float maximum_right_error = 0.0F;
        for (std::size_t frame = latency; frame < frame_count; ++frame) {
            maximum_right_error = std::max(
                maximum_right_error,
                std::abs(stereo[frame * 2 + 1] - right_clean[frame - latency])
            );
        }
        expect(maximum_right_error < 1.0E-6F, "unaffected channel remains sample-exact");

        auto bypass_source = sine(440.0F, 0.1F, 4096);
        const auto bypass_reference = bypass_source;
        echo::audio::DeClickProcessor bypassed({}, kSampleRate, 1);
        bypassed.process_interleaved(bypass_source.data(), bypass_source.size(), 1);
        for (std::size_t frame = bypassed.latency_frames(); frame < bypass_source.size(); ++frame) {
            expect(
                bypass_source[frame] == bypass_reference[frame - bypassed.latency_frames()],
                "bypass preserves delayed samples exactly"
            );
        }

        bypassed.update(enabled_parameters());
        auto smooth = sine(330.0F, 0.1F, 4096);
        bypassed.process_interleaved(smooth.data(), smooth.size(), 1);
        float largest_step = 0.0F;
        for (std::size_t frame = 1; frame < smooth.size(); ++frame) {
            largest_step = std::max(largest_step, std::abs(smooth[frame] - smooth[frame - 1]));
        }
        expect(largest_step < 0.03F, "live bypass changes remain click-free");

        bypassed.reset();
        std::vector<float> silence(bypassed.latency_frames(), 0.0F);
        bypassed.process_interleaved(silence.data(), silence.size(), 1);
        expect(
            std::all_of(
                silence.begin(),
                silence.end(),
                [](float sample) { return sample == 0.0F; }
            ),
            "reset clears delay and repair state"
        );

        bool rejected = false;
        try {
            bypassed.update({.maximum_click_microseconds = 2500});
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        expect(rejected, "non-click duration is rejected");

        auto contiguous = clean;
        contiguous[click_frame] += 1.0F;
        contiguous[click_frame + 1] += 0.8F;
        auto chunked = contiguous;
        echo::audio::DeClickProcessor contiguous_filter(enabled_parameters(), kSampleRate, 1);
        echo::audio::DeClickProcessor chunked_filter(enabled_parameters(), kSampleRate, 1);
        contiguous_filter.process_interleaved(contiguous.data(), contiguous.size(), 1);
        std::size_t processed = 0;
        while (processed < chunked.size()) {
            const std::size_t count = std::min<std::size_t>(113, chunked.size() - processed);
            chunked_filter.process_interleaved(chunked.data() + processed, count, 1);
            processed += count;
        }
        for (std::size_t index = 0; index < contiguous.size(); ++index) {
            expect(
                contiguous[index] == chunked[index],
                "look-ahead and repair state are invariant across callback boundaries"
            );
        }
        std::printf("de-click processor test: ok\n");
        return 0;
    } catch (const std::exception& error) {
        std::fprintf(stderr, "de-click processor test failed: %s\n", error.what());
        return 1;
    }
}
