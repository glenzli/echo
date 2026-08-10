#include "echo/audio/offline_loudness_analyzer.hpp"

#include <cassert>
#include <cmath>
#include <vector>

int main() {
    constexpr std::uint32_t sample_rate = 48'000;
    constexpr std::size_t channels = 2;
    std::vector<float> tone(sample_rate * 2 * channels);
    for (std::size_t frame = 0; frame < tone.size() / channels; ++frame) {
        const float sample = 0.1F
                             * std::sin(
                                 2.0F * 3.14159265358979323846F * 1'000.0F
                                 * static_cast<float>(frame) / static_cast<float>(sample_rate)
                             );
        tone[frame * channels] = sample;
        tone[frame * channels + 1] = sample;
    }
    echo::audio::OfflineLoudnessAnalyzer analyzer(sample_rate, channels);
    analyzer.process_interleaved(tone.data(), tone.size() / channels, channels);
    const echo::audio::OfflineLoudnessResult result = analyzer.result();
    assert(result.integrated_lufs > -21.0F && result.integrated_lufs < -19.0F);
    assert(result.true_peak_dbtp > -20.1F && result.true_peak_dbtp < -19.8F);

    std::vector<float> silence(sample_rate * channels, 0.0F);
    echo::audio::OfflineLoudnessAnalyzer silent(sample_rate, channels);
    silent.process_interleaved(silence.data(), silence.size() / channels, channels);
    assert(silent.result().integrated_lufs == -70.0F);
}
