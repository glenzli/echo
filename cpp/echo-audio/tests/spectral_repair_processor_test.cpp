#include "echo/audio/spectral_repair_processor.hpp"

#include <cmath>
#include <cstddef>
#include <cstdint>
#include <iostream>
#include <numbers>
#include <vector>

namespace {

float rms(const std::vector<float>& samples, std::size_t first, std::size_t last) {
    float sum = 0.0F;
    for (std::size_t index = first; index < last; ++index) {
        sum += samples[index] * samples[index];
    }
    return std::sqrt(sum / static_cast<float>(last - first));
}

} // namespace

int main() {
    constexpr std::uint32_t sample_rate = 48'000;
    const std::vector<echo::audio::SpectralAttenuationRegion> regions{{
        .start_millis = 200,
        .end_millis = 1'800,
        .low_hertz = 920,
        .high_hertz = 1'080,
        .attenuation_centibels = 4'800,
        .time_feather_millis = 0,
        .frequency_feather_hertz = 0,
    }};
    std::vector<float> samples(sample_rate * 2);
    for (std::size_t frame = 0; frame < samples.size(); ++frame) {
        samples[frame] = 0.5F
                         * std::sin(
                             2.0F * std::numbers::pi_v<float>
                             * 1'000.0F * static_cast<float>(frame) / sample_rate
                         );
    }
    const float before = rms(samples, sample_rate / 4, sample_rate / 2);
    echo::audio::SpectralRepairProcessor::process_interleaved(
        samples.data(),
        samples.size(),
        1,
        sample_rate,
        0,
        regions
    );
    const float attenuated = rms(samples, sample_rate / 2, sample_rate);
    const float untouched = rms(samples, sample_rate * 18 / 10, sample_rate * 19 / 10);
    if (!(attenuated < before * 0.02F && untouched > before * 0.7F)) {
        std::cerr << "spectral attenuation did not remain source-anchored\n";
        return 1;
    }

    std::vector<float> stream_input(sample_rate * 2);
    for (std::size_t frame = 0; frame < stream_input.size(); ++frame) {
        stream_input[frame] = 0.5F
                              * std::sin(
                                  2.0F * std::numbers::pi_v<float>
                                  * 1'000.0F * static_cast<float>(frame) / sample_rate
                              );
    }
    echo::audio::SpectralRepairStream stream(sample_rate, 1, regions);
    std::vector<float> streamed;
    std::vector<float> drained(701);
    std::vector<std::uint64_t> source_frames(701);
    for (std::size_t first = 0; first < stream_input.size();) {
        const std::size_t count = std::min<std::size_t>(317, stream_input.size() - first);
        stream.push_interleaved(stream_input.data() + first, count, first);
        while (true) {
            const std::size_t produced =
                stream.drain_interleaved(drained.data(), source_frames.data(), drained.size());
            if (produced == 0) {
                break;
            }
            if (source_frames[0] != streamed.size()) {
                std::cerr << "streamed source frames are not contiguous\n";
                return 1;
            }
            streamed.insert(streamed.end(), drained.begin(), drained.begin() + produced);
        }
        first += count;
    }
    stream.finish();
    while (true) {
        const std::size_t produced =
            stream.drain_interleaved(drained.data(), source_frames.data(), drained.size());
        if (produced == 0) {
            break;
        }
        if (source_frames[0] != streamed.size()) {
            std::cerr << "flushed source frames are not contiguous\n";
            return 1;
        }
        streamed.insert(streamed.end(), drained.begin(), drained.begin() + produced);
    }
    if (streamed.size() != stream_input.size()) {
        std::cerr << "stream did not preserve source duration\n";
        return 1;
    }
    const float stream_attenuated = rms(streamed, sample_rate / 2, sample_rate);
    const float stream_untouched = rms(streamed, sample_rate * 18 / 10, sample_rate * 19 / 10);
    if (!(stream_attenuated < before * 0.02F && stream_untouched > before * 0.7F)) {
        std::cerr << "streaming attenuation did not remain source-anchored\n";
        return 1;
    }
    // The stream must reconstruct even a one-frame source or a seek boundary.
    for (const std::size_t count : {1U, 511U, 512U, 513U, 1536U, 2049U}) {
        echo::audio::SpectralRepairStream identity(sample_rate, 1, {});
        for (const std::uint64_t origin : {0U, 48'000U}) {
            identity.reset();
            std::vector<float> input(count, 0.125F), output(count);
            std::vector<std::uint64_t> positions(count);
            identity.push_interleaved(input.data(), count, origin);
            identity.finish();
            if (identity.drain_interleaved(output.data(), positions.data(), count) != count) {
                std::cerr << "short stream changed duration after reset\n";
                return 1;
            }
            for (std::size_t frame = 0; frame < count; ++frame) {
                if (std::abs(output[frame] - input[frame]) > 0.000002F
                    || positions[frame] != origin + frame) {
                    std::cerr << "boundary reconstruction lost samples or source positions\n";
                    return 1;
                }
            }
        }
    }
    return 0;
}
