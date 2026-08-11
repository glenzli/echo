#include "echo/audio/channel_repair_processor.hpp"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <stdexcept>
#include <vector>

namespace {

constexpr std::uint32_t kSampleRate = 48000;

void expect(bool condition, const char* message) {
    if (!condition) {
        throw std::runtime_error(message);
    }
}

void expect_near(float actual, float expected, float tolerance, const char* message) {
    expect(std::abs(actual - expected) <= tolerance, message);
}

} // namespace

int main() {
    try {
        echo::audio::ChannelRepairProcessor repair(
            {
                .enabled = true,
                .invert_left = true,
                .invert_right = false,
                .swap_channels = true,
                .mono_fold_down = false,
                .balance_percent = 50,
            },
            kSampleRate,
            2
        );
        float repaired[] = {0.8F, -0.2F};
        repair.process_interleaved(repaired, 1, 2);
        expect_near(repaired[0], -0.1F, 1.0e-6F, "swap and balance repair left channel");
        expect_near(repaired[1], -0.8F, 1.0e-6F, "polarity repair reaches right channel");

        echo::audio::ChannelRepairProcessor mono_fold(
            {
                .enabled = true,
                .mono_fold_down = true,
            },
            kSampleRate,
            2
        );
        float folded[] = {1.0F, 0.0F};
        mono_fold.process_interleaved(folded, 1, 2);
        expect_near(folded[0], 0.5F, 1.0e-6F, "mono fold uses a bounded average");
        expect_near(folded[1], 0.5F, 1.0e-6F, "mono fold produces centered dual mono");

        constexpr std::size_t kFrames = 1800;
        std::vector<float> contiguous(kFrames * 2);
        for (std::size_t frame = 0; frame < kFrames; ++frame) {
            contiguous[frame * 2] = 1.0F;
            contiguous[frame * 2 + 1] = -1.0F;
        }
        auto chunked = contiguous;
        echo::audio::ChannelRepairProcessor contiguous_processor({}, kSampleRate, 2);
        echo::audio::ChannelRepairProcessor chunked_processor({}, kSampleRate, 2);
        const echo::audio::ChannelRepairAdjustment target{
            .enabled = true,
            .invert_left = true,
            .invert_right = false,
            .swap_channels = true,
            .mono_fold_down = true,
            .balance_percent = -35,
        };
        contiguous_processor.update(target);
        chunked_processor.update(target);
        contiguous_processor.process_interleaved(contiguous.data(), kFrames, 2);
        std::size_t processed = 0;
        while (processed < kFrames) {
            const std::size_t count = std::min<std::size_t>(137, kFrames - processed);
            chunked_processor.process_interleaved(chunked.data() + processed * 2, count, 2);
            processed += count;
        }
        for (std::size_t sample = 0; sample < contiguous.size(); ++sample) {
            expect(
                contiguous[sample] == chunked[sample],
                "matrix transition is invariant across decode chunk boundaries"
            );
        }
        for (std::size_t frame = 1; frame < 960; ++frame) {
            expect(
                std::abs(contiguous[frame * 2] - contiguous[(frame - 1) * 2]) < 0.01F,
                "parameter transition does not introduce a full-scale click"
            );
        }

        echo::audio::ChannelRepairProcessor mono_polarity({}, kSampleRate, 1);
        mono_polarity.update({.enabled = true, .invert_left = true});
        mono_polarity.reset();
        float mono_sample = 0.25F;
        mono_polarity.process_interleaved(&mono_sample, 1, 1);
        expect_near(mono_sample, -0.25F, 1.0e-6F, "mono fallback applies left polarity only");

        bool invalid_balance_rejected = false;
        try {
            echo::audio::ChannelRepairProcessor invalid(
                {.enabled = true, .balance_percent = 101},
                kSampleRate,
                2
            );
        } catch (const std::invalid_argument&) {
            invalid_balance_rejected = true;
        }
        expect(invalid_balance_rejected, "out-of-range balance is rejected");

        std::printf("channel repair processor test: ok\n");
        return 0;
    } catch (const std::exception& error) {
        std::fprintf(stderr, "channel repair processor test failed: %s\n", error.what());
        return 1;
    }
}
