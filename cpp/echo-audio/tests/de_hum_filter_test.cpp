#include "echo/audio/de_hum_filter.hpp"

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

float rms(const std::vector<float>& samples, std::size_t first_frame, std::size_t stride = 1) {
    double energy = 0.0;
    std::size_t count = 0;
    for (std::size_t index = first_frame * stride; index < samples.size(); index += stride) {
        energy += static_cast<double>(samples[index]) * static_cast<double>(samples[index]);
        ++count;
    }
    return count == 0 ? 0.0F : static_cast<float>(std::sqrt(energy / static_cast<double>(count)));
}

} // namespace

int main() {
    try {
        const echo::audio::DeHumParameters enabled{
            .enabled = true,
            .fundamental_hertz = 50,
            .harmonic_count = 4,
            .quality_tenths = 300,
            .depth_centibels = 3000,
        };

        auto hum = sine(50.0F, 0.5F, kSampleRate * 3);
        const float dry_hum = rms(hum, kSampleRate * 2);
        echo::audio::DeHumFilter hum_filter(enabled, kSampleRate, 1);
        hum_filter.process_interleaved(hum.data(), hum.size(), 1);
        expect(rms(hum, kSampleRate * 2) < dry_hum * 0.08F, "50 Hz hum is deeply rejected");

        auto harmonic = sine(100.0F, 0.5F, kSampleRate * 3);
        echo::audio::DeHumFilter harmonic_filter(enabled, kSampleRate, 1);
        harmonic_filter.process_interleaved(harmonic.data(), harmonic.size(), 1);
        expect(
            rms(harmonic, kSampleRate * 2) < dry_hum * 0.1F,
            "configured hum harmonics are rejected"
        );

        auto nearby = sine(83.0F, 0.5F, kSampleRate * 3);
        const float dry_nearby = rms(nearby, kSampleRate * 2);
        echo::audio::DeHumFilter nearby_filter(enabled, kSampleRate, 1);
        nearby_filter.process_interleaved(nearby.data(), nearby.size(), 1);
        expect(
            rms(nearby, kSampleRate * 2) > dry_nearby * 0.97F,
            "narrow cuts preserve nearby programme material"
        );

        const auto right_source = sine(733.0F, 0.25F, kSampleRate * 2);
        const auto left_source = sine(50.0F, 0.5F, kSampleRate * 2);
        std::vector<float> stereo(kSampleRate * 2 * 2);
        for (std::size_t frame = 0; frame < left_source.size(); ++frame) {
            stereo[frame * 2] = left_source[frame];
            stereo[frame * 2 + 1] = right_source[frame];
        }
        echo::audio::DeHumFilter stereo_filter(enabled, kSampleRate, 2);
        stereo_filter.process_interleaved(stereo.data(), stereo.size() / 2, 2);
        expect(
            rms(stereo, kSampleRate, 2) < 0.05F,
            "per-channel state removes hum from the affected channel"
        );
        auto right_reference = right_source;
        echo::audio::DeHumFilter right_filter(enabled, kSampleRate, 1);
        right_filter.process_interleaved(right_reference.data(), right_reference.size(), 1);
        double right_error = 0.0;
        for (std::size_t frame = kSampleRate; frame < right_source.size(); ++frame) {
            const double difference =
                static_cast<double>(stereo[frame * 2 + 1] - right_reference[frame]);
            right_error += difference * difference;
        }
        right_error = std::sqrt(right_error / static_cast<double>(kSampleRate));
        expect(right_error < 0.002, "per-channel state does not leak between channels");

        echo::audio::DeHumFilter live({}, kSampleRate, 1);
        auto transition = sine(50.0F, 0.4F, kSampleRate / 2);
        live.process_interleaved(transition.data(), kSampleRate / 4, 1);
        live.update(enabled);
        live.process_interleaved(
            transition.data() + kSampleRate / 4,
            transition.size() - kSampleRate / 4,
            1
        );
        float largest_added_step = 0.0F;
        const auto reference = sine(50.0F, 0.4F, transition.size());
        for (std::size_t frame = kSampleRate / 4 + 1; frame < transition.size(); ++frame) {
            const float processed_step = transition[frame] - transition[frame - 1];
            const float original_step = reference[frame] - reference[frame - 1];
            largest_added_step =
                std::max(largest_added_step, std::abs(processed_step - original_step));
        }
        expect(largest_added_step < 0.01F, "live parameter changes remain click-free");

        live.reset();
        std::vector<float> silence(256, 0.0F);
        live.process_interleaved(silence.data(), silence.size(), 1);
        expect(
            std::all_of(
                silence.begin(),
                silence.end(),
                [](float sample) { return sample == 0.0F; }
            ),
            "reset clears all filter state"
        );

        bool rejected = false;
        try {
            live.update({.fundamental_hertz = 55});
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        expect(rejected, "unsupported mains frequency is rejected");

        auto contiguous = sine(60.0F, 0.4F, 16000);
        auto chunked = contiguous;
        auto sixty_hertz = enabled;
        sixty_hertz.fundamental_hertz = 60;
        echo::audio::DeHumFilter contiguous_filter(sixty_hertz, kSampleRate, 1);
        echo::audio::DeHumFilter chunked_filter(sixty_hertz, kSampleRate, 1);
        contiguous_filter.process_interleaved(contiguous.data(), contiguous.size(), 1);
        std::size_t processed = 0;
        while (processed < chunked.size()) {
            const std::size_t count = std::min<std::size_t>(137, chunked.size() - processed);
            chunked_filter.process_interleaved(chunked.data() + processed, count, 1);
            processed += count;
        }
        for (std::size_t index = 0; index < contiguous.size(); ++index) {
            expect(
                contiguous[index] == chunked[index],
                "filter state is invariant across decode chunk boundaries"
            );
        }
        std::printf("de-hum filter test: ok\n");
        return 0;
    } catch (const std::exception& error) {
        std::fprintf(stderr, "de-hum filter test failed: %s\n", error.what());
        return 1;
    }
}
