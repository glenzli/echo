//! Focused engine test: the playback session's realtime read path, seek, and
//! pause semantics over a synthesized WAV (generated in-memory).

#include <echo/audio/playback.hpp>

#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <filesystem>
#include <fstream>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>

namespace {

echo::audio::ParametricEqualizerAdjustment
legacyEqualizer(std::int16_t low, std::int16_t mid, std::int16_t high) {
    echo::audio::ParametricEqualizerAdjustment result;
    result.bands[0].gain_centibels = low;
    result.bands[2].gain_centibels = mid;
    result.bands[5].gain_centibels = high;
    return result;
}

std::string synthesize_sine_wav(
    std::uint32_t sample_rate,
    double seconds,
    double frequency = 440.0,
    double amplitude = 12000.0
) {
    const std::uint16_t channels = 1;
    const std::uint16_t bits = 16;
    const std::uint32_t sample_count =
        static_cast<std::uint32_t>(static_cast<double>(sample_rate) * seconds);
    const std::uint32_t data_bytes = sample_count * channels * bits / 8;

    std::string wav;
    wav.reserve(44 + data_bytes);
    const auto append = [&wav](const void* bytes, std::size_t size) {
        wav.append(static_cast<const char*>(bytes), size);
    };
    append("RIFF", 4);
    const std::uint32_t riff_size = 36 + data_bytes;
    append(&riff_size, 4);
    append("WAVE", 4);
    append("fmt ", 4);
    const std::uint32_t fmt_size = 16;
    append(&fmt_size, 4);
    const std::uint16_t format = 1;
    append(&format, 2);
    append(&channels, 2);
    append(&sample_rate, 4);
    const std::uint32_t byte_rate = sample_rate * channels * bits / 8;
    append(&byte_rate, 4);
    const std::uint16_t block_align = channels * bits / 8;
    append(&block_align, 2);
    append(&bits, 2);
    append("data", 4);
    append(&data_bytes, 4);
    for (std::uint32_t index = 0; index < sample_count; ++index) {
        const double phase = 2.0 * 3.14159265358979323846 * frequency * static_cast<double>(index)
                             / static_cast<double>(sample_rate);
        const std::int16_t sample = static_cast<std::int16_t>(std::sin(phase) * amplitude);
        append(&sample, 2);
    }
    return wav;
}

int failures = 0;

void expect(bool condition, const char* message) {
    if (!condition) {
        std::fprintf(stderr, "FAIL: %s\n", message);
        ++failures;
    }
}

/// Pulls until at least `target` frames arrive or the timeout expires.
std::size_t pull_until(
    echo::audio::PlaybackSession& session,
    float* buffer,
    std::size_t target,
    std::size_t chunk
) {
    std::size_t total = 0;
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(4);
    while (total < target && std::chrono::steady_clock::now() < deadline) {
        const std::size_t pulled = session.read(buffer + total * session.channel_count(), chunk);
        total += pulled;
        if (pulled == 0) {
            std::this_thread::sleep_for(std::chrono::milliseconds(2));
        }
    }
    return total;
}

std::size_t drain_until_ended(echo::audio::PlaybackSession& session, std::size_t chunk) {
    std::vector<float> buffer(chunk * session.channel_count(), 0.0F);
    std::size_t total = 0;
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(3);
    while (std::chrono::steady_clock::now() < deadline) {
        total += session.read(buffer.data(), chunk);
        if (session.is_ended() && session.buffered_frames() == 0) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
    return total;
}

} // namespace

int main(int argc, char* argv[]) {
    // With one argument, smoke-test playback of that exact file (useful for
    // engine-level reproduction without Qt); otherwise synthesize a WAV.
    const std::filesystem::path path =
        argc > 1
            ? std::filesystem::path(argv[1])
            : std::filesystem::temp_directory_path()
                  / ("echo-playback-test-"
                     + std::to_string(std::chrono::system_clock::now().time_since_epoch().count())
                     + ".wav");
    if (argc <= 1) {
        std::ofstream file(path, std::ios::binary);
        const std::string wav = synthesize_sine_wav(44100, 2.0);
        file.write(wav.data(), static_cast<std::streamsize>(wav.size()));
    }

    echo::audio::PlaybackSession session(path.string());
    if (argc > 1) {
        // Standalone smoke: pull for up to two seconds and report.
        std::vector<float> buffer(9600, 0.0F);
        std::size_t total = 0;
        std::uint64_t last_position = 0;
        bool nonzero = false;
        std::vector<float> dumped;
        const bool want_dump = std::getenv("ECHO_PLAYBACK_DUMP") != nullptr;
        if (want_dump) {
            dumped.reserve(480000);
        }
        for (int attempt = 0; attempt < 40; ++attempt) {
            const std::size_t pulled = session.read(buffer.data(), 4800);
            for (std::size_t index = 0; index < pulled * session.channel_count(); ++index) {
                nonzero = nonzero || buffer[index] != 0.0F;
                if (want_dump) {
                    dumped.push_back(buffer[index]);
                }
            }
            total += pulled;
            last_position = session.position_millis();
            if (session.is_ended() || last_position >= session.duration_millis()) {
                break;
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(50));
        }
        if (want_dump && !dumped.empty()) {
            if (const char* dump_path = std::getenv("ECHO_PLAYBACK_DUMP")) {
                std::FILE* dump = std::fopen(dump_path, "wb");
                if (dump != nullptr) {
                    std::fwrite(dumped.data(), sizeof(float), dumped.size(), dump);
                    std::fclose(dump);
                }
            }
        }
        std::printf(
            "standalone: pulled %zu frames (%s), position %llu/%llu ms, buffered %zu, "
            "ended %d\n",
            total,
            nonzero ? "nonzero" : "ALL SILENCE",
            last_position,
            session.duration_millis(),
            session.buffered_frames(),
            session.is_ended() ? 1 : 0
        );
        return total > 0 && nonzero ? 0 : 1;
    }
    expect(session.sample_rate() == 48000, "canonical 48 kHz playback");
    expect(session.channel_count() == 2, "playback always drives a stereo sink");
    expect(
        session.duration_millis() >= 1900 && session.duration_millis() <= 2100,
        "duration is about two seconds"
    );
    expect(!session.is_ended(), "session not ended before reads");

    // Realtime path: pull a bounded chunk and verify nonzero audio arrives.
    std::vector<float> buffer(9600, 0.0F);
    const std::size_t first = pull_until(session, buffer.data(), 1000, 100);
    expect(first >= 100, "first reads return frames");
    {
        bool nonzero = false;
        for (std::size_t index = 0; index < first * session.channel_count(); ++index) {
            nonzero = nonzero || buffer[index] != 0.0F;
        }
        expect(nonzero, "decoded audio is not silence");
    }

    // Position advances as the consumer pulls.
    const std::uint64_t position_before = session.position_millis();
    pull_until(session, buffer.data(), 4800, 4800);
    const std::uint64_t position_after = session.position_millis();
    expect(position_after > position_before, "position advances with consumption");

    // Pause: reads drain and then return zero.
    session.pause();
    expect(session.is_paused(), "pause is observable");
    std::this_thread::sleep_for(std::chrono::milliseconds(30));
    const std::uint64_t position_paused = session.position_millis();
    session.read(buffer.data(), 100);
    std::this_thread::sleep_for(std::chrono::milliseconds(30));
    expect(session.position_millis() >= position_paused, "position freezes while paused");
    session.resume();
    expect(!session.is_paused(), "resume clears pause");

    // Seek: reposition and continue reading.
    session.seek(1000);
    pull_until(session, buffer.data(), 100, 100);
    expect(session.position_millis() >= 900, "seek lands near the target");

    // Regression: stop() while the producer is blocked on a full ring (a
    // paused or stalled consumer) must return promptly instead of joining a
    // thread that never exits.
    {
        const std::filesystem::path second_path =
            std::filesystem::temp_directory_path()
            / ("echo-playback-fullring-"
               + std::to_string(std::chrono::system_clock::now().time_since_epoch().count())
               + ".wav");
        {
            std::ofstream file(second_path, std::ios::binary);
            const std::string wav = synthesize_sine_wav(44100, 2.0);
            file.write(wav.data(), static_cast<std::streamsize>(wav.size()));
        }
        echo::audio::PlaybackSession second(second_path.string());
        std::this_thread::sleep_for(std::chrono::milliseconds(400));
        const auto stop_begin = std::chrono::steady_clock::now();
        second.stop();
        const auto stop_elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(
            std::chrono::steady_clock::now() - stop_begin
        );
        expect(
            stop_elapsed < std::chrono::milliseconds(500),
            "stop returns while the ring is full"
        );
        std::remove(second_path.c_str());
    }

    // Regression: 24 kHz sources (TTS output) upsample 2x to the canonical
    // 48 kHz; the resampler output buffer must be sized by the exact output
    // count or decode overflows the heap and distorts playback.
    {
        const std::filesystem::path upsample_path =
            std::filesystem::temp_directory_path()
            / ("echo-playback-upsample-"
               + std::to_string(std::chrono::system_clock::now().time_since_epoch().count())
               + ".wav");
        {
            std::ofstream file(upsample_path, std::ios::binary);
            const std::string wav = synthesize_sine_wav(24000, 1.0);
            file.write(wav.data(), static_cast<std::streamsize>(wav.size()));
        }
        echo::audio::PlaybackSession upsample(upsample_path.string());
        std::vector<float> check(9600, 0.0F);
        const std::size_t pulled = pull_until(upsample, check.data(), 1000, 100);
        expect(pulled >= 100, "24 kHz source decodes");
        bool sane = true;
        for (std::size_t index = 0; index < pulled * upsample.channel_count(); ++index) {
            const float value = check[index];
            sane = sane && !std::isnan(value) && value >= -1.0F && value <= 1.0F;
        }
        expect(sane, "upsampled samples stay finite and bounded");
        upsample.stop();
        std::remove(upsample_path.c_str());
    }

    // The authored graph is applied by the producer: playback begins at the
    // selected range, stops at its end, and gain is audible without adding
    // work to the realtime read callback.
    {
        const echo::audio::PlaybackAdjustment adjustment{
            .trim_start_millis = 500,
            .trim_end_millis = 1000,
            .fade_in_millis = 100,
            .fade_out_millis = 100,
            .gain_centibels = -600,
            .low_cut_hertz = 80,
            .equalizer = legacyEqualizer(200, -100, 150),
            .compressor =
                {
                    .enabled = true,
                    .threshold_centibels = -1800,
                    .ratio_tenths = 30,
                    .attack_millis = 10,
                    .release_millis = 120,
                    .makeup_centibels = 0,
                },
            .limiter = {.enabled = true, .ceiling_centibels = -600, .release_millis = 100},
            .effect_chain =
                {
                    echo::audio::EffectNodeKind::Restoration,
                    echo::audio::EffectNodeKind::Equalizer,
                    echo::audio::EffectNodeKind::Dynamics,
                    echo::audio::EffectNodeKind::Space,
                    echo::audio::EffectNodeKind::ChannelRepair,
                    echo::audio::EffectNodeKind::SceneVfx,
                    echo::audio::EffectNodeKind::DelayVfx,
                    echo::audio::EffectNodeKind::ModulationVfx,
                    echo::audio::EffectNodeKind::TransformVfx,
                    echo::audio::EffectNodeKind::Master,
                    echo::audio::EffectNodeKind::DeHum,
                    echo::audio::EffectNodeKind::DeClick,
                },
            .effect_chain_count = 10,
        };
        echo::audio::PlaybackSession adjusted(path.string(), adjustment);
        std::vector<float> adjusted_buffer(48'000, 0.0F);
        const std::size_t pulled = pull_until(adjusted, adjusted_buffer.data(), 12'000, 1'000);
        expect(pulled > 1'000, "adjusted range returns audio");
        expect(adjusted.position_millis() >= 500, "adjusted playback begins at trim start");
        float peak = 0.0F;
        for (std::size_t index = 0; index < pulled * adjusted.channel_count(); ++index) {
            peak = std::max(peak, std::abs(adjusted_buffer[index]));
        }
        expect(peak > 0.05F && peak < 0.22F, "adjusted gain changes decoded amplitude");
        const echo::audio::PlaybackMeterSnapshot meter = adjusted.meter_snapshot();
        expect(meter.momentary_lufs > -70.0F, "prepared playback publishes real loudness");
        expect(meter.output_peak_dbfs > -70.0F, "prepared playback publishes output peak");
        expect(
            meter.gain_reduction_decibels >= 0.0F,
            "prepared playback publishes bounded gain reduction"
        );
        expect(
            meter.limiter_reduction_decibels >= 0.0F,
            "prepared playback publishes bounded limiter reduction"
        );
        const std::uint64_t position_before_equalizer_update = adjusted.position_millis();
        adjusted.update_equalizer(legacyEqualizer(-600, 600, -600));
        std::vector<float> updated_buffer(8'192, 0.0F);
        const std::size_t updated = pull_until(adjusted, updated_buffer.data(), 2'048, 200);
        expect(updated > 0, "live equalizer update keeps returning audio");
        expect(
            adjusted.position_millis() > position_before_equalizer_update,
            "live equalizer update does not restart the playback timeline"
        );
        expect(
            std::all_of(
                updated_buffer.begin(),
                updated_buffer.begin()
                    + static_cast<std::ptrdiff_t>(updated * adjusted.channel_count()),
                [](float sample) {
                    return std::isfinite(sample) && sample >= -1.0F && sample <= 1.0F;
                }
            ),
            "live equalizer output stays finite and device-bounded"
        );
        echo::audio::CreativeVfxAdjustment first_creative_update;
        first_creative_update.scene = {
            .character = echo::audio::SceneVfxCharacter::Radio,
            .enabled = true,
            .mix_percent = 100,
            .intensity_percent = 65,
        };
        first_creative_update.delay = {
            .character = echo::audio::DelayVfxCharacter::Slapback,
            .enabled = true,
        };
        first_creative_update.modulation = {
            .character = echo::audio::ModulationVfxCharacter::Chorus,
            .enabled = true,
        };
        first_creative_update.transform = {
            .character = echo::audio::TransformVfxCharacter::Robot,
            .enabled = true,
            .mix_percent = 80,
            .amount_percent = 55,
        };
        auto latest_creative_update = first_creative_update;
        latest_creative_update.scene.character = echo::audio::SceneVfxCharacter::Underwater;
        latest_creative_update.delay.character = echo::audio::DelayVfxCharacter::Echo;
        latest_creative_update.modulation.character = echo::audio::ModulationVfxCharacter::Phaser;
        latest_creative_update.transform.character = echo::audio::TransformVfxCharacter::Ghost;
        const std::uint64_t position_before_creative_update = adjusted.position_millis();
        adjusted.update_creative_vfx(first_creative_update);
        adjusted.update_creative_vfx(latest_creative_update);
        std::vector<float> creative_buffer(8'192, 0.0F);
        const std::size_t creative_updated =
            pull_until(adjusted, creative_buffer.data(), 4'096, 256);
        expect(creative_updated > 0, "live Creative VFX update keeps returning audio");
        expect(
            adjusted.position_millis() > position_before_creative_update,
            "latest-wins Creative VFX update does not restart the playback timeline"
        );
        expect(
            std::all_of(
                creative_buffer.begin(),
                creative_buffer.begin()
                    + static_cast<std::ptrdiff_t>(creative_updated * adjusted.channel_count()),
                [](float sample) {
                    return std::isfinite(sample) && sample >= -1.0F && sample <= 1.0F;
                }
            ),
            "live Creative VFX output stays finite and device-bounded"
        );
        bool creative_rejected_update = false;
        try {
            auto invalid_creative_update = latest_creative_update;
            invalid_creative_update.scene.mix_percent = 101;
            adjusted.update_creative_vfx(invalid_creative_update);
        } catch (const std::invalid_argument&) {
            creative_rejected_update = true;
        }
        expect(creative_rejected_update, "live Creative VFX update preserves every family bound");
        bool rejected_update = false;
        try {
            adjusted.update_equalizer(legacyEqualizer(0, 0, 1'201));
        } catch (const std::invalid_argument&) {
            rejected_update = true;
        }
        expect(rejected_update, "live equalizer update preserves the authored gain bounds");
        const std::uint64_t position_before_compressor_update = adjusted.position_millis();
        adjusted.update_compressor({
            .enabled = true,
            .threshold_centibels = -2400,
            .ratio_tenths = 60,
            .attack_millis = 15,
            .release_millis = 180,
            .makeup_centibels = 200,
        });
        const std::size_t dynamics_updated =
            pull_until(adjusted, updated_buffer.data(), 1'024, 200);
        expect(dynamics_updated > 0, "live compressor update keeps returning audio");
        expect(
            adjusted.position_millis() > position_before_compressor_update,
            "live compressor update does not restart the playback timeline"
        );
        rejected_update = false;
        try {
            adjusted.update_compressor({.ratio_tenths = 201});
        } catch (const std::invalid_argument&) {
            rejected_update = true;
        }
        expect(rejected_update, "live compressor update preserves the authored bounds");
        const std::uint64_t position_before_channel_repair_update = adjusted.position_millis();
        adjusted.update_channel_repair({
            .enabled = true,
            .invert_left = true,
            .swap_channels = true,
            .balance_percent = -30,
        });
        const std::size_t channel_repair_updated =
            pull_until(adjusted, updated_buffer.data(), 512, 200);
        expect(channel_repair_updated > 0, "live channel repair update keeps returning audio");
        expect(
            adjusted.position_millis() > position_before_channel_repair_update,
            "live channel repair update does not restart the playback timeline"
        );
        rejected_update = false;
        try {
            adjusted.update_channel_repair({.enabled = true, .balance_percent = -101});
        } catch (const std::invalid_argument&) {
            rejected_update = true;
        }
        expect(rejected_update, "live channel repair update preserves the authored bounds");
        const std::uint64_t position_before_limiter_update = adjusted.position_millis();
        adjusted.update_limiter(
            {.enabled = true, .ceiling_centibels = -300, .release_millis = 160}
        );
        const std::size_t limiter_updated = pull_until(adjusted, updated_buffer.data(), 512, 200);
        expect(limiter_updated > 0, "live limiter update keeps returning audio");
        expect(
            adjusted.position_millis() > position_before_limiter_update,
            "live limiter update does not restart the playback timeline"
        );
        rejected_update = false;
        try {
            adjusted.update_limiter({.ceiling_centibels = -601});
        } catch (const std::invalid_argument&) {
            rejected_update = true;
        }
        expect(rejected_update, "live limiter update preserves the authored bounds");
        adjusted.seek(0);
        expect(adjusted.position_millis() >= 500, "adjusted seek clamps to trim start");
        adjusted.stop();
    }

    // The authored chain is executable order, not presentation metadata. A
    // nonlinear compressor and time-domain space tank must produce different audio
    // when their order is exchanged while every parameter stays identical.
    {
        const echo::audio::PlaybackAdjustment dynamics_then_space{
            .compressor =
                {.enabled = true,
                 .threshold_centibels = -2400,
                 .ratio_tenths = 60,
                 .attack_millis = 5,
                 .release_millis = 180,
                 .makeup_centibels = 300},
            .reverb =
                {.character = echo::audio::ReverbCharacter::Hall,
                 .enabled = true,
                 .mix_percent = 70,
                 .pre_delay_millis = 0,
                 .decay_millis = 1800,
                 .size_percent = 70,
                 .damping_percent = 35,
                 .low_cut_hertz = 120,
                 .high_cut_hertz = 10000},
            .effect_chain =
                {echo::audio::EffectNodeKind::Restoration,
                 echo::audio::EffectNodeKind::Equalizer,
                 echo::audio::EffectNodeKind::Dynamics,
                 echo::audio::EffectNodeKind::Space,
                 echo::audio::EffectNodeKind::Master,
                 echo::audio::EffectNodeKind::DeHum,
                 echo::audio::EffectNodeKind::DeClick,
                 echo::audio::EffectNodeKind::ChannelRepair,
                 echo::audio::EffectNodeKind::SceneVfx,
                 echo::audio::EffectNodeKind::DelayVfx,
                 echo::audio::EffectNodeKind::ModulationVfx,
                 echo::audio::EffectNodeKind::TransformVfx},
            .effect_chain_count = 5,
        };
        auto space_then_dynamics = dynamics_then_space;
        space_then_dynamics.effect_chain = {
            echo::audio::EffectNodeKind::Restoration,
            echo::audio::EffectNodeKind::Equalizer,
            echo::audio::EffectNodeKind::Space,
            echo::audio::EffectNodeKind::Dynamics,
            echo::audio::EffectNodeKind::Master,
            echo::audio::EffectNodeKind::DeHum,
            echo::audio::EffectNodeKind::DeClick,
            echo::audio::EffectNodeKind::ChannelRepair,
            echo::audio::EffectNodeKind::SceneVfx,
            echo::audio::EffectNodeKind::DelayVfx,
            echo::audio::EffectNodeKind::ModulationVfx,
            echo::audio::EffectNodeKind::TransformVfx,
        };

        echo::audio::PlaybackSession first_order(path.string(), dynamics_then_space);
        echo::audio::PlaybackSession second_order(path.string(), space_then_dynamics);
        std::vector<float> first_samples(32'000, 0.0F);
        std::vector<float> second_samples(32'000, 0.0F);
        const std::size_t first_frames =
            pull_until(first_order, first_samples.data(), 12'000, 1'000);
        const std::size_t second_frames =
            pull_until(second_order, second_samples.data(), 12'000, 1'000);
        // Producer scheduling may make either final read partial; order
        // comparison below already uses their common rendered prefix.
        expect(first_frames >= 12'000 && second_frames >= 12'000, "both chain orders render");
        double absolute_difference = 0.0;
        const std::size_t compared_samples =
            std::min(first_frames, second_frames) * first_order.channel_count();
        for (std::size_t index = 0; index < compared_samples; ++index) {
            absolute_difference += std::abs(
                static_cast<double>(first_samples[index])
                - static_cast<double>(second_samples[index])
            );
        }
        expect(
            compared_samples > 0
                && absolute_difference / static_cast<double>(compared_samples) > 0.0001,
            "changing authored node order changes processed audio"
        );
        first_order.stop();
        second_order.stop();
    }

    // A fixed-look-ahead node keeps its latency while bypassed, but playback
    // compensates it at the session boundary: neither a full trim nor a seek
    // loses source frames or extends the authored duration.
    {
        const echo::audio::PlaybackAdjustment latency_compensated{
            .trim_start_millis = 500,
            .trim_end_millis = 1000,
            .effect_chain =
                {
                    echo::audio::EffectNodeKind::Restoration,
                    echo::audio::EffectNodeKind::DeClick,
                    echo::audio::EffectNodeKind::Master,
                    echo::audio::EffectNodeKind::Equalizer,
                    echo::audio::EffectNodeKind::Dynamics,
                    echo::audio::EffectNodeKind::Space,
                    echo::audio::EffectNodeKind::DeHum,
                    echo::audio::EffectNodeKind::ChannelRepair,
                    echo::audio::EffectNodeKind::SceneVfx,
                    echo::audio::EffectNodeKind::DelayVfx,
                    echo::audio::EffectNodeKind::ModulationVfx,
                    echo::audio::EffectNodeKind::TransformVfx,
                },
            .effect_chain_count = 3,
        };
        echo::audio::PlaybackSession full(path.string(), latency_compensated);
        expect(
            drain_until_ended(full, 173) == 24'000,
            "latency compensation preserves the full selected frame count"
        );
        expect(full.position_millis() == 1000, "latency-compensated playback ends at trim out");
        full.stop();

        echo::audio::PlaybackSession sought(path.string(), latency_compensated);
        sought.seek(750);
        expect(
            drain_until_ended(sought, 113) == 12'000,
            "latency compensation resets after seek without losing frames"
        );
        expect(sought.position_millis() == 1000, "latency-compensated seek ends at trim out");
        sought.stop();
    }

    // Source edits collapse hidden time, preserve muted time, insert explicit
    // gaps, and keep fixed DeClick latency compensation frame-exact.
    {
        echo::audio::PlaybackAdjustment edited;
        edited.trim_end_millis = 1000;
        edited.effect_chain = {
            echo::audio::EffectNodeKind::Equalizer,
            echo::audio::EffectNodeKind::DeClick,
            echo::audio::EffectNodeKind::Master,
            echo::audio::EffectNodeKind::Restoration,
            echo::audio::EffectNodeKind::Dynamics,
            echo::audio::EffectNodeKind::Space,
            echo::audio::EffectNodeKind::DeHum,
            echo::audio::EffectNodeKind::ChannelRepair,
            echo::audio::EffectNodeKind::SceneVfx,
            echo::audio::EffectNodeKind::DelayVfx,
            echo::audio::EffectNodeKind::ModulationVfx,
            echo::audio::EffectNodeKind::TransformVfx,
        };
        edited.effect_chain_count = 3;
        edited.effect_masks = {{
            .start_millis = 0,
            .end_millis = 250,
            .feather_millis = 10,
            .nodes = {echo::audio::EffectNodeKind::Equalizer},
        }};
        edited.edit_segments = {
            {
                .source_start_millis = 0,
                .source_end_millis = 250,
                .state = echo::audio::EditSegmentState::Audible,
            },
            {
                .source_start_millis = 250,
                .source_end_millis = 500,
                .state = echo::audio::EditSegmentState::Hidden,
                .gap_after_millis = 100,
            },
            {
                .source_start_millis = 500,
                .source_end_millis = 750,
                .state = echo::audio::EditSegmentState::Muted,
            },
            {
                .source_start_millis = 750,
                .source_end_millis = 1000,
                .state = echo::audio::EditSegmentState::Audible,
            },
        };
        echo::audio::PlaybackSession source_edited(path.string(), edited);
        expect(
            source_edited.output_frame_count() == 40'800,
            "source edit plan publishes arranged output frame count"
        );
        expect(
            drain_until_ended(source_edited, 127) == 40'800,
            "hidden mute gap and DeClick compensation preserve arranged frame count"
        );
        expect(
            source_edited.position_millis() == 1000,
            "source-edited playback position remains original-time anchored"
        );
        source_edited.stop();
    }

    {
        echo::audio::PlaybackAdjustment hidden_gap;
        hidden_gap.trim_end_millis = 100;
        hidden_gap.effect_chain = {
            echo::audio::EffectNodeKind::Master,
            echo::audio::EffectNodeKind::Restoration,
            echo::audio::EffectNodeKind::Equalizer,
            echo::audio::EffectNodeKind::Dynamics,
            echo::audio::EffectNodeKind::Space,
            echo::audio::EffectNodeKind::DeHum,
            echo::audio::EffectNodeKind::DeClick,
            echo::audio::EffectNodeKind::ChannelRepair,
            echo::audio::EffectNodeKind::SceneVfx,
            echo::audio::EffectNodeKind::DelayVfx,
            echo::audio::EffectNodeKind::ModulationVfx,
            echo::audio::EffectNodeKind::TransformVfx,
        };
        hidden_gap.effect_chain_count = 1;
        hidden_gap.edit_segments = {{
            .source_start_millis = 0,
            .source_end_millis = 100,
            .state = echo::audio::EditSegmentState::Hidden,
            .gap_after_millis = 50,
        }};
        echo::audio::PlaybackSession silence(path.string(), hidden_gap);
        std::vector<float> gap_samples(4'800, 1.0F);
        const std::size_t gap_frames = pull_until(silence, gap_samples.data(), 2'400, 127);
        expect(gap_frames == 2'400, "all-hidden edit still emits its authored gap");
        expect(
            std::all_of(
                gap_samples.begin(),
                gap_samples.begin()
                    + static_cast<std::ptrdiff_t>(gap_frames * silence.channel_count()),
                [](float sample) { return sample == 0.0F; }
            ),
            "all-hidden authored gap is silent"
        );
        silence.stop();
    }

    // Positive EQ on near-full-scale material must not recreate the former
    // hard-clipped plateau at the device boundary.
    {
        const std::filesystem::path overload_path =
            std::filesystem::temp_directory_path()
            / ("echo-playback-overload-"
               + std::to_string(std::chrono::system_clock::now().time_since_epoch().count())
               + ".wav");
        {
            std::ofstream file(overload_path, std::ios::binary);
            const std::string wav = synthesize_sine_wav(48'000, 1.0, 1'000.0, 32'000.0);
            file.write(wav.data(), static_cast<std::streamsize>(wav.size()));
        }
        echo::audio::PlaybackSession protected_playback(
            overload_path.string(),
            {.equalizer = legacyEqualizer(1'200, 1'200, 1'200)}
        );
        std::vector<float> protected_samples(16'384, 0.0F);
        const std::size_t protected_frames =
            pull_until(protected_playback, protected_samples.data(), 4'096, 512);
        float protected_peak = 0.0F;
        std::size_t hard_clipped = 0;
        for (std::size_t index = 0; index < protected_frames * protected_playback.channel_count();
             ++index) {
            protected_peak = std::max(protected_peak, std::abs(protected_samples[index]));
            hard_clipped += std::abs(protected_samples[index]) >= 0.9999F ? 1U : 0U;
        }
        expect(protected_peak < 1.0F, "overloaded EQ preview stays below full scale");
        expect(hard_clipped == 0, "overloaded EQ preview has no hard-clipped plateau");
        protected_playback.stop();
        std::remove(overload_path.c_str());
    }

    // Stop: reads return zero immediately.
    session.stop();
    expect(session.is_stopped(), "stop is observable");
    expect(session.read(buffer.data(), 100) == 0, "read after stop returns zero");
    std::remove(path.c_str());

    if (failures == 0) {
        std::printf("playback engine test: ok\n");
        return 0;
    }
    return 1;
}
