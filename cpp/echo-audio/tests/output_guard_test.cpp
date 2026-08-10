#include "echo/audio/output_guard.hpp"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <vector>

int main() {
    int failures = 0;
    const auto expect = [&failures](bool condition, const char* message) {
        if (!condition) {
            std::fprintf(stderr, "FAIL: %s\n", message);
            ++failures;
        }
    };

    echo::audio::OutputGuard transparent(48'000);
    std::vector<float> quiet(512 * 2, 0.25F);
    transparent.process_interleaved(quiet.data(), 512, 2);
    expect(std::abs(quiet.front() - 0.25F) < 1.0e-6F, "quiet audio remains transparent");

    echo::audio::OutputGuard protected_output(48'000);
    std::vector<float> overloaded(4'096 * 2, 1.8F);
    protected_output.process_interleaved(overloaded.data(), 4'096, 2);
    const float peak = *std::max_element(overloaded.begin(), overloaded.end());
    expect(peak < 1.0F, "overloaded preview remains below the device ceiling");
    expect(peak > 0.9F, "the guard does not collapse overloaded preview level");
    expect(
        std::all_of(
            overloaded.begin(),
            overloaded.end(),
            [](float sample) { return std::isfinite(sample) && sample >= -1.0F && sample <= 1.0F; }
        ),
        "guarded samples stay finite and bounded"
    );

    protected_output.reset();
    std::vector<float> reset_signal(64, 0.4F);
    protected_output.process_interleaved(reset_signal.data(), reset_signal.size(), 1);
    expect(std::abs(reset_signal.front() - 0.4F) < 1.0e-6F, "reset clears prior gain reduction");

    if (failures == 0) {
        std::printf("output guard test: ok\n");
        return 0;
    }
    return 1;
}
