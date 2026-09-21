#include "echo/audio/decode.hpp"
#include "echo/audio/export_metadata.hpp"
#include "echo/audio/offline_flac_renderer.hpp"
#include "echo/audio/offline_wav_renderer.hpp"
#include "echo/audio/playback.hpp"

#include <cassert>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <memory>
#include <span>
#include <string>
#include <thread>
#include <vector>

namespace {

class MemorySink final : public echo::audio::RenderByteSink {
  public:
    void write(std::span<const std::byte> bytes) override {
        if (cursor_ + bytes.size() > bytes_.size()) {
            bytes_.resize(cursor_ + bytes.size());
        }
        std::memcpy(bytes_.data() + cursor_, bytes.data(), bytes.size());
        cursor_ += bytes.size();
    }

    void seek(std::uint64_t offset) override {
        cursor_ = static_cast<std::size_t>(offset);
    }

    [[nodiscard]] const std::vector<std::byte>& bytes() const {
        return bytes_;
    }

  private:
    std::vector<std::byte> bytes_;
    std::size_t cursor_ = 0;
};

void append(std::string& target, const void* bytes, std::size_t size) {
    target.append(static_cast<const char*>(bytes), size);
}

std::string sine_wav(std::uint32_t frames = 24000) {
    constexpr std::uint32_t sample_rate = 24000;
    constexpr std::uint16_t channels = 1;
    constexpr std::uint16_t bits = 16;
    const std::uint32_t data_bytes = frames * channels * bits / 8;
    std::string wav;
    append(wav, "RIFF", 4);
    const std::uint32_t riff_size = 36 + data_bytes;
    append(wav, &riff_size, 4);
    append(wav, "WAVEfmt ", 8);
    const std::uint32_t fmt_size = 16;
    const std::uint16_t format = 1;
    const std::uint32_t byte_rate = sample_rate * channels * bits / 8;
    const std::uint16_t block_align = channels * bits / 8;
    append(wav, &fmt_size, 4);
    append(wav, &format, 2);
    append(wav, &channels, 2);
    append(wav, &sample_rate, 4);
    append(wav, &byte_rate, 4);
    append(wav, &block_align, 2);
    append(wav, &bits, 2);
    append(wav, "data", 4);
    append(wav, &data_bytes, 4);
    for (std::uint32_t frame = 0; frame < frames; ++frame) {
        const double phase =
            2.0 * 3.14159265358979323846 * 440.0 * static_cast<double>(frame) / sample_rate;
        double plosive = 0.0;
        if (frame >= 8'000 && frame < 10'400) {
            const double progress = static_cast<double>(frame - 8'000) / 2'400.0;
            const double burst_phase =
                2.0 * 3.14159265358979323846 * 85.0 * static_cast<double>(frame) / sample_rate;
            plosive = std::exp(-3.2 * progress) * std::sin(burst_phase) * 20'000.0;
        }
        const auto sample = static_cast<std::int16_t>(
            std::clamp(std::sin(phase) * 3'000.0 + plosive, -32'767.0, 32'767.0)
        );
        append(wav, &sample, 2);
    }
    return wav;
}

std::uint32_t u32(const std::vector<std::byte>& bytes, std::size_t offset) {
    return std::to_integer<std::uint32_t>(bytes[offset])
           | (std::to_integer<std::uint32_t>(bytes[offset + 1]) << 8U)
           | (std::to_integer<std::uint32_t>(bytes[offset + 2]) << 16U)
           | (std::to_integer<std::uint32_t>(bytes[offset + 3]) << 24U);
}

float pcm24(const std::vector<std::byte>& bytes, std::size_t sample_index) {
    const std::size_t offset = 44 + sample_index * 3;
    std::uint32_t raw = std::to_integer<std::uint32_t>(bytes[offset])
                        | (std::to_integer<std::uint32_t>(bytes[offset + 1]) << 8U)
                        | (std::to_integer<std::uint32_t>(bytes[offset + 2]) << 16U);
    if ((raw & 0x0080'0000U) != 0U) {
        raw |= 0xff00'0000U;
    }
    return static_cast<float>(static_cast<std::int32_t>(raw)) / 8'388'607.0F;
}

float rms_channel(
    const std::vector<float>& samples,
    std::size_t channel_count,
    std::size_t first_frame,
    std::size_t last_frame
) {
    float sum = 0.0F;
    for (std::size_t frame = first_frame; frame < last_frame; ++frame) {
        const float sample = samples[frame * channel_count];
        sum += sample * sample;
    }
    return std::sqrt(sum / static_cast<float>(last_frame - first_frame));
}

std::vector<float> render_playback(
    const std::filesystem::path& source,
    const echo::audio::PlaybackAdjustment& adjustment
) {
    echo::audio::PlaybackSession session(
        source.string(),
        adjustment,
        {.apply_output_guard = false, .collect_metering = false}
    );
    std::vector<float> chunk(4096 * session.channel_count());
    std::vector<float> rendered;
    rendered.reserve(session.output_frame_count() * session.channel_count());
    while (true) {
        const std::size_t frames = session.read(chunk.data(), 4096);
        if (frames != 0) {
            rendered.insert(
                rendered.end(),
                chunk.begin(),
                chunk.begin() + static_cast<std::ptrdiff_t>(frames * session.channel_count())
            );
            continue;
        }
        if (session.is_ended() && session.buffered_frames() == 0) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }
    assert(rendered.size() == session.output_frame_count() * session.channel_count());
    return rendered;
}

} // namespace

int main() {
    const auto source =
        std::filesystem::temp_directory_path()
        / ("echo-offline-render-"
           + std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()) + ".wav");
    {
        std::ofstream output(source, std::ios::binary);
        const std::string wav = sine_wav();
        output.write(wav.data(), static_cast<std::streamsize>(wav.size()));
    }

    const echo::audio::PlaybackAdjustment source_adjustment{.trim_end_millis = 1000};
    const auto baseline = render_playback(source, source_adjustment);
    // Frozen working copies use an identity adjustment with an unspecified end.
    MemorySink whole_source, explicit_source, whole_flac;
    const auto whole_result =
        echo::audio::OfflineWavRenderer::render(source.string(), {}, whole_source);
    (void)echo::audio::OfflineWavRenderer::render(
        source.string(),
        source_adjustment,
        explicit_source
    );
    assert(whole_result.frame_count == 48000);
    assert(whole_source.bytes() == explicit_source.bytes());
    assert(
        echo::audio::OfflineFlacRenderer::render(source.string(), {}, whole_flac).frame_count
        == 48000
    );
    for (const echo::audio::PlaybackAdjustment& invalid :
         {echo::audio::PlaybackAdjustment{.trim_start_millis = 1000},
          echo::audio::PlaybackAdjustment{.trim_start_millis = 500, .trim_end_millis = 500}}) {
        bool rejected = false;
        try {
            MemorySink output;
            (void)echo::audio::OfflineWavRenderer::render(source.string(), invalid, output);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);
    }
    const echo::audio::PlaybackAdjustment repaired{
        .trim_end_millis = 1000,
        .spectral_repair = {
            {.start_millis = 100,
             .end_millis = 900,
             .low_hertz = 360,
             .high_hertz = 520,
             .attenuation_centibels = 4800}
        },
    };
    const auto spectral_repaired = render_playback(source, repaired);
    assert(spectral_repaired.size() == baseline.size());
    const float baseline_rms = rms_channel(baseline, 2, 26'400, 36'000);
    const float repaired_rms = rms_channel(spectral_repaired, 2, 26'400, 36'000);
    if (!(repaired_rms < baseline_rms * 0.02F)) {
        std::fprintf(
            stderr,
            "spectral repair export RMS %.6f was not below baseline %.6f\n",
            repaired_rms,
            baseline_rms
        );
        return 1;
    }

    MemorySink sink;
    double progress = 0.0;
    const echo::audio::PlaybackAdjustment adjustment{
        .trim_start_millis = 250,
        .trim_end_millis = 750,
        .fade_in_millis = 50,
        .fade_out_millis = 50,
        .gain_centibels = -300,
    };
    const auto result = echo::audio::OfflineWavRenderer::render(
        source.string(),
        adjustment,
        sink,
        {.progress = [&progress](double value) { progress = value; }}
    );
    assert(result.sample_rate == 48000);
    assert(result.channel_count == 2);
    assert(result.bit_depth == 24);
    assert(result.frame_count == 24000);
    assert(result.size_bytes == sink.bytes().size());
    assert(progress == 1.0);
    assert(std::memcmp(sink.bytes().data(), "RIFF", 4) == 0);
    assert(std::memcmp(sink.bytes().data() + 8, "WAVE", 4) == 0);
    assert(std::memcmp(sink.bytes().data() + 36, "data", 4) == 0);
    assert(u32(sink.bytes(), 40) == sink.bytes().size() - 44);
    assert(u32(sink.bytes(), 24) == 48000);

    MemorySink pcm16_sink;
    const auto pcm16 = echo::audio::OfflineWavRenderer::render(
        source.string(),
        adjustment,
        pcm16_sink,
        {},
        echo::audio::WavPcmDepth::Pcm16
    );
    assert(pcm16.bit_depth == 16);
    assert(pcm16.size_bytes == pcm16_sink.bytes().size());
    assert(std::to_integer<std::uint8_t>(pcm16_sink.bytes()[34]) == 16);
    assert(pcm16_sink.bytes().size() < sink.bytes().size());

    MemorySink flac_sink;
    const auto flac =
        echo::audio::OfflineFlacRenderer::render(source.string(), adjustment, flac_sink);
    assert(flac.sample_rate == 48000);
    assert(flac.channel_count == 2);
    assert(flac.bit_depth == 24);
    assert(flac.frame_count == 24000);
    assert(flac.size_bytes == flac_sink.bytes().size());
    assert(std::memcmp(flac_sink.bytes().data(), "fLaC", 4) == 0);

    // Container disclosure must round-trip through the real probe and leave PCM unchanged.
    const std::string comment =
        R"(Echo source disclosure: {"schema":"echo.source-disclosure.v1","scope":"referenced_sources","kinds":["ai_generated"]})";
    auto save_and_probe = [&](const MemorySink& rendered, const std::string& suffix) {
        const auto path = source.string() + suffix;
        {
            std::ofstream file(path, std::ios::binary);
            file.write(
                reinterpret_cast<const char*>(rendered.bytes().data()),
                static_cast<std::streamsize>(rendered.bytes().size())
            );
        }
        const auto probe = echo::audio::probe(path);
        bool found = false;
        for (const auto& entry : probe.metadata) {
            if (entry.key == "comment") {
                assert(entry.value == comment);
                found = true;
            }
        }
        assert(found);
        assert(probe.duration_millis == 500);
        const auto decoded = render_playback(path, {.trim_end_millis = 500});
        std::filesystem::remove(path);
        return decoded;
    };
    for (const auto depth : {echo::audio::WavPcmDepth::Pcm16, echo::audio::WavPcmDepth::Pcm24}) {
        MemorySink marked;
        const auto marked_result = echo::audio::OfflineWavRenderer::render(
            source.string(),
            adjustment,
            marked,
            {},
            depth,
            comment
        );
        const auto& original =
            depth == echo::audio::WavPcmDepth::Pcm16 ? pcm16_sink.bytes() : sink.bytes();
        assert(marked_result.size_bytes == marked.bytes().size());
        assert(u32(marked.bytes(), 4) == marked.bytes().size() - 8);
        assert(u32(marked.bytes(), 40) == original.size() - 44);
        assert(std::equal(original.begin() + 44, original.end(), marked.bytes().begin() + 44));
        (void)save_and_probe(marked, ".marked.wav");
    }
    MemorySink marked_flac;
    const auto marked_flac_result = echo::audio::OfflineFlacRenderer::render(
        source.string(),
        adjustment,
        marked_flac,
        {},
        comment
    );
    assert(marked_flac_result.size_bytes == marked_flac.bytes().size());
    const auto marked_pcm = save_and_probe(marked_flac, ".marked.flac");
    const auto plain_flac_path = source.string() + ".plain.flac";
    {
        std::ofstream file(plain_flac_path, std::ios::binary);
        file.write(
            reinterpret_cast<const char*>(flac_sink.bytes().data()),
            static_cast<std::streamsize>(flac_sink.bytes().size())
        );
    }
    assert(marked_pcm == render_playback(plain_flac_path, {.trim_end_millis = 500}));
    std::filesystem::remove(plain_flac_path);
    // RIFF even-byte padding and early bounds reject malformed metadata before a write.
    for (const std::string& boundary :
         {std::string("x"), std::string("xy"), std::string(1024, 'x')}) {
        const auto chunk = echo::audio::wav_comment_chunk(boundary);
        assert(chunk.size() % 2 == 0);
        assert(u32(chunk, 4) == chunk.size() - 8);
        assert(u32(chunk, 16) == boundary.size() + 1);
    }
    for (const std::string& invalid : {std::string(1025, 'x'), std::string("x\0y", 3)}) {
        bool rejected = false;
        try {
            (void)echo::audio::wav_comment_chunk(invalid);
        } catch (const std::invalid_argument&) {
            rejected = true;
        }
        assert(rejected);
    }

    MemorySink latency_compensated_sink;
    auto latency_compensated = adjustment;
    latency_compensated.effect_chain = {
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
        echo::audio::EffectNodeKind::DigitalDegradeVfx,
        echo::audio::EffectNodeKind::DriveVfx,
        echo::audio::EffectNodeKind::RotaryVfx,
        echo::audio::EffectNodeKind::FreezeVfx,
        echo::audio::EffectNodeKind::GranularVfx,
    };
    latency_compensated.effect_chain_count = 3;
    const auto compensated = echo::audio::OfflineWavRenderer::render(
        source.string(),
        latency_compensated,
        latency_compensated_sink
    );
    assert(compensated.frame_count == 24000);
    assert(compensated.size_bytes == latency_compensated_sink.bytes().size());

    MemorySink full_source_sink;
    auto full_source = latency_compensated;
    full_source.trim_start_millis = 0;
    full_source.trim_end_millis = 1000;
    const auto full_source_result =
        echo::audio::OfflineWavRenderer::render(source.string(), full_source, full_source_sink);
    assert(full_source_result.frame_count == 48'000);
    assert(full_source_result.size_bytes == full_source_sink.bytes().size());

    auto de_plosive = full_source;
    de_plosive.restoration.de_plosive = {
        .enabled = true,
        .frequency_hertz = 160,
        .sensitivity_percent = 80,
        .reduction_centibels = 1800,
        .release_millis = 140,
    };
    const auto live_de_plosive = render_playback(source, de_plosive);
    MemorySink de_plosive_sink;
    const auto de_plosive_result =
        echo::audio::OfflineWavRenderer::render(source.string(), de_plosive, de_plosive_sink);
    assert(
        de_plosive_result.frame_count * de_plosive_result.channel_count == live_de_plosive.size()
    );
    for (std::size_t index = 0; index < live_de_plosive.size(); ++index) {
        assert(std::abs(live_de_plosive[index] - pcm24(de_plosive_sink.bytes(), index)) < 2.0E-6F);
    }

    auto channel_repaired = full_source;
    channel_repaired.channel_repair = {
        .enabled = true,
        .invert_left = true,
        .swap_channels = true,
        .balance_percent = 25,
    };
    channel_repaired.effect_chain = {
        echo::audio::EffectNodeKind::Restoration,
        echo::audio::EffectNodeKind::DeClick,
        echo::audio::EffectNodeKind::ChannelRepair,
        echo::audio::EffectNodeKind::Master,
        echo::audio::EffectNodeKind::Equalizer,
        echo::audio::EffectNodeKind::Dynamics,
        echo::audio::EffectNodeKind::Space,
        echo::audio::EffectNodeKind::DeHum,
        echo::audio::EffectNodeKind::SceneVfx,
        echo::audio::EffectNodeKind::DelayVfx,
        echo::audio::EffectNodeKind::ModulationVfx,
        echo::audio::EffectNodeKind::TransformVfx,
        echo::audio::EffectNodeKind::DigitalDegradeVfx,
        echo::audio::EffectNodeKind::DriveVfx,
        echo::audio::EffectNodeKind::RotaryVfx,
        echo::audio::EffectNodeKind::FreezeVfx,
        echo::audio::EffectNodeKind::GranularVfx,
    };
    channel_repaired.effect_chain_count = 4;
    const auto live_channel_repaired = render_playback(source, channel_repaired);
    MemorySink channel_repaired_sink;
    const auto channel_repaired_result = echo::audio::OfflineWavRenderer::render(
        source.string(),
        channel_repaired,
        channel_repaired_sink
    );
    assert(
        channel_repaired_result.frame_count * channel_repaired_result.channel_count
        == live_channel_repaired.size()
    );
    for (std::size_t index = 0; index < live_channel_repaired.size(); ++index) {
        assert(
            std::abs(live_channel_repaired[index] - pcm24(channel_repaired_sink.bytes(), index))
            < 2.0E-6F
        );
    }

    auto spring_space = full_source;
    spring_space.reverb = {
        .character = echo::audio::ReverbCharacter::Spring,
        .enabled = true,
        .mix_percent = 55,
        .pre_delay_millis = 8,
        .decay_millis = 2'600,
        .size_percent = 70,
        .damping_percent = 35,
        .low_cut_hertz = 120,
        .high_cut_hertz = 10'000,
    };
    spring_space.effect_chain = {
        echo::audio::EffectNodeKind::Restoration,
        echo::audio::EffectNodeKind::Space,
        echo::audio::EffectNodeKind::Master,
        echo::audio::EffectNodeKind::Equalizer,
        echo::audio::EffectNodeKind::Dynamics,
        echo::audio::EffectNodeKind::DeHum,
        echo::audio::EffectNodeKind::DeClick,
        echo::audio::EffectNodeKind::ChannelRepair,
        echo::audio::EffectNodeKind::SceneVfx,
        echo::audio::EffectNodeKind::DelayVfx,
        echo::audio::EffectNodeKind::ModulationVfx,
        echo::audio::EffectNodeKind::TransformVfx,
        echo::audio::EffectNodeKind::DigitalDegradeVfx,
        echo::audio::EffectNodeKind::DriveVfx,
        echo::audio::EffectNodeKind::RotaryVfx,
        echo::audio::EffectNodeKind::FreezeVfx,
        echo::audio::EffectNodeKind::GranularVfx,
    };
    spring_space.effect_chain_count = 3;
    const auto live_spring_space = render_playback(source, spring_space);
    MemorySink spring_space_sink;
    const auto spring_space_result =
        echo::audio::OfflineWavRenderer::render(source.string(), spring_space, spring_space_sink);
    assert(
        spring_space_result.frame_count * spring_space_result.channel_count
        == live_spring_space.size()
    );
    for (std::size_t index = 0; index < live_spring_space.size(); ++index) {
        assert(
            std::abs(live_spring_space[index] - pcm24(spring_space_sink.bytes(), index)) < 2.0E-6F
        );
    }

    auto convolution_space = full_source;
    auto impulse = std::make_shared<echo::audio::LoadedPreparedImpulseResponse>();
    impulse->preparation_version = 1;
    impulse->left.assign(1'200, 0.0F);
    impulse->right.assign(1'200, 0.0F);
    impulse->left[0] = 0.85F;
    impulse->left[731] = 0.2F;
    impulse->right[0] = 0.8F;
    impulse->right[947] = -0.15F;
    convolution_space.space = {
        .mode = echo::audio::SpaceMode::Convolution,
        .convolution = {
            .import_id = "018f5f1a-ff90-7c71-9ec4-66d36516664c",
            .source_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            .prepared_hash = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            .adjustment = {.enabled = true, .mix_percent = 48, .wet_gain_centibels = -175},
            .impulse = std::move(impulse),
        },
    };
    convolution_space.effect_chain = spring_space.effect_chain;
    convolution_space.effect_chain_count = spring_space.effect_chain_count;
    const auto live_convolution_space = render_playback(source, convolution_space);
    MemorySink convolution_space_sink;
    const auto convolution_space_result = echo::audio::OfflineWavRenderer::render(
        source.string(),
        convolution_space,
        convolution_space_sink
    );
    assert(
        convolution_space_result.frame_count * convolution_space_result.channel_count
        == live_convolution_space.size()
    );
    for (std::size_t index = 0; index < live_convolution_space.size(); ++index) {
        assert(
            std::abs(live_convolution_space[index] - pcm24(convolution_space_sink.bytes(), index))
            < 2.0E-6F
        );
    }

    auto creative_vfx = full_source;
    creative_vfx.creative_vfx.scene.enabled = true;
    creative_vfx.creative_vfx.scene.character = echo::audio::SceneVfxCharacter::Radio;
    creative_vfx.creative_vfx.delay.enabled = true;
    creative_vfx.creative_vfx.delay.character = echo::audio::DelayVfxCharacter::Slapback;
    creative_vfx.creative_vfx.modulation.enabled = true;
    creative_vfx.creative_vfx.modulation.character = echo::audio::ModulationVfxCharacter::Chorus;
    creative_vfx.creative_vfx.transform.enabled = true;
    creative_vfx.creative_vfx.transform.character = echo::audio::TransformVfxCharacter::Robot;
    creative_vfx.creative_vfx.digital_degrade.enabled = true;
    creative_vfx.creative_vfx.digital_degrade.character =
        echo::audio::DigitalDegradeVfxCharacter::LoFi;
    creative_vfx.creative_vfx.drive.enabled = true;
    creative_vfx.creative_vfx.drive.character = echo::audio::DriveVfxCharacter::Overdrive;
    creative_vfx.creative_vfx.rotary.enabled = true;
    creative_vfx.creative_vfx.rotary.speed = echo::audio::RotaryVfxSpeed::Fast;
    creative_vfx.effect_chain = {
        echo::audio::EffectNodeKind::SceneVfx,
        echo::audio::EffectNodeKind::DelayVfx,
        echo::audio::EffectNodeKind::ModulationVfx,
        echo::audio::EffectNodeKind::TransformVfx,
        echo::audio::EffectNodeKind::DigitalDegradeVfx,
        echo::audio::EffectNodeKind::DriveVfx,
        echo::audio::EffectNodeKind::RotaryVfx,
        echo::audio::EffectNodeKind::Master,
        echo::audio::EffectNodeKind::Restoration,
        echo::audio::EffectNodeKind::Equalizer,
        echo::audio::EffectNodeKind::Dynamics,
        echo::audio::EffectNodeKind::Space,
        echo::audio::EffectNodeKind::DeHum,
        echo::audio::EffectNodeKind::DeClick,
        echo::audio::EffectNodeKind::ChannelRepair,
        echo::audio::EffectNodeKind::FreezeVfx,
        echo::audio::EffectNodeKind::GranularVfx,
    };
    creative_vfx.effect_chain_count = 8;
    const auto live_creative_vfx = render_playback(source, creative_vfx);
    MemorySink creative_vfx_sink;
    const auto creative_vfx_result =
        echo::audio::OfflineWavRenderer::render(source.string(), creative_vfx, creative_vfx_sink);
    assert(creative_vfx_result.frame_count == 48'000);
    assert(
        creative_vfx_result.frame_count * creative_vfx_result.channel_count
        == live_creative_vfx.size()
    );
    for (std::size_t index = 0; index < live_creative_vfx.size(); ++index) {
        assert(
            std::abs(live_creative_vfx[index] - pcm24(creative_vfx_sink.bytes(), index)) < 2.0E-6F
        );
    }

    auto freeze_granular = full_source;
    freeze_granular.creative_vfx.freeze = {
        .enabled = true,
        .mix_percent = 65,
        .capture_source_millis = 100,
    };
    freeze_granular.creative_vfx.granular = {
        .enabled = true,
        .mix_percent = 58,
        .grain_millis = 70,
        .density_tenths_hertz = 180,
        .lookback_millis = 180,
        .scatter_millis = 60,
        .pitch_cents = -250,
        .stereo_spread_percent = 70,
        .random_seed = 0x13579BDFU,
    };
    freeze_granular.effect_chain = {
        echo::audio::EffectNodeKind::FreezeVfx,
        echo::audio::EffectNodeKind::GranularVfx,
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
        echo::audio::EffectNodeKind::DigitalDegradeVfx,
        echo::audio::EffectNodeKind::DriveVfx,
        echo::audio::EffectNodeKind::RotaryVfx,
    };
    freeze_granular.effect_chain_count = 3;
    const auto live_freeze_granular = render_playback(source, freeze_granular);
    MemorySink freeze_granular_sink;
    const auto freeze_granular_result = echo::audio::OfflineWavRenderer::render(
        source.string(),
        freeze_granular,
        freeze_granular_sink
    );
    assert(freeze_granular_result.frame_count == 48'000);
    assert(
        freeze_granular_result.frame_count * freeze_granular_result.channel_count
        == live_freeze_granular.size()
    );
    bool freeze_granular_changed = false;
    const auto unprocessed = render_playback(source, full_source);
    for (std::size_t index = 0; index < live_freeze_granular.size(); ++index) {
        assert(
            std::abs(live_freeze_granular[index] - pcm24(freeze_granular_sink.bytes(), index))
            < 2.0E-6F
        );
        freeze_granular_changed =
            freeze_granular_changed
            || std::abs(live_freeze_granular[index] - unprocessed[index]) > 1.0E-5F;
    }
    assert(freeze_granular_changed);

    auto source_edited = full_source;
    source_edited.edit_segments = {
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
    MemorySink edited_wav_sink;
    const auto edited_wav =
        echo::audio::OfflineWavRenderer::render(source.string(), source_edited, edited_wav_sink);
    assert(edited_wav.frame_count == 40'800);
    assert(edited_wav.size_bytes == edited_wav_sink.bytes().size());
    MemorySink edited_flac_sink;
    const auto edited_flac =
        echo::audio::OfflineFlacRenderer::render(source.string(), source_edited, edited_flac_sink);
    assert(edited_flac.frame_count == edited_wav.frame_count);
    assert(edited_flac.size_bytes == edited_flac_sink.bytes().size());

    // 999.75 ms rounds to a 1000 ms authored endpoint. The sub-millisecond
    // tail still passes through segment gaps and fixed-latency processing.
    const auto fractional_source = source.string() + ".fraction.wav";
    {
        std::ofstream output(fractional_source, std::ios::binary);
        const std::string wav = sine_wav(23'994);
        output.write(wav.data(), static_cast<std::streamsize>(wav.size()));
    }
    auto fractional_adjustment = full_source;
    fractional_adjustment.edit_segments = {{
        .source_start_millis = 0,
        .source_end_millis = 1000,
        .state = echo::audio::EditSegmentState::Audible,
        .gap_after_millis = 100,
    }};
    MemorySink fractional_wav_sink;
    const auto fractional_wav = echo::audio::OfflineWavRenderer::render(
        fractional_source,
        fractional_adjustment,
        fractional_wav_sink
    );
    assert(fractional_wav.frame_count == 52'800);
    MemorySink fractional_flac_sink;
    const auto fractional_flac = echo::audio::OfflineFlacRenderer::render(
        fractional_source,
        fractional_adjustment,
        fractional_flac_sink
    );
    assert(fractional_flac.frame_count == fractional_wav.frame_count);
    const auto fractional_playback = render_playback(fractional_source, fractional_adjustment);
    assert(fractional_playback.size() == 52'800 * 2);
    std::filesystem::remove(fractional_source);

    MemorySink cancelled_sink;
    bool did_cancel = false;
    try {
        [[maybe_unused]] const auto cancelled_result = echo::audio::OfflineWavRenderer::render(
            source.string(),
            adjustment,
            cancelled_sink,
            {.cancelled = [] { return true; }}
        );
    } catch (const echo::audio::OfflineRenderCancelled&) {
        did_cancel = true;
    }
    assert(did_cancel);
    std::filesystem::remove(source);
}
