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

std::string sine_wav() {
    constexpr std::uint32_t sample_rate = 24000;
    constexpr std::uint16_t channels = 1;
    constexpr std::uint16_t bits = 16;
    constexpr std::uint32_t frames = sample_rate;
    constexpr std::uint32_t data_bytes = frames * channels * bits / 8;
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

    auto plate_space = full_source;
    plate_space.reverb = {
        .character = echo::audio::ReverbCharacter::Plate,
        .enabled = true,
        .mix_percent = 55,
        .pre_delay_millis = 8,
        .decay_millis = 2'600,
        .size_percent = 70,
        .damping_percent = 35,
        .low_cut_hertz = 120,
        .high_cut_hertz = 10'000,
    };
    plate_space.effect_chain = {
        echo::audio::EffectNodeKind::Restoration,
        echo::audio::EffectNodeKind::Space,
        echo::audio::EffectNodeKind::Master,
        echo::audio::EffectNodeKind::Equalizer,
        echo::audio::EffectNodeKind::Dynamics,
        echo::audio::EffectNodeKind::DeHum,
        echo::audio::EffectNodeKind::DeClick,
        echo::audio::EffectNodeKind::ChannelRepair,
    };
    plate_space.effect_chain_count = 3;
    const auto live_plate_space = render_playback(source, plate_space);
    MemorySink plate_space_sink;
    const auto plate_space_result =
        echo::audio::OfflineWavRenderer::render(source.string(), plate_space, plate_space_sink);
    assert(
        plate_space_result.frame_count * plate_space_result.channel_count == live_plate_space.size()
    );
    for (std::size_t index = 0; index < live_plate_space.size(); ++index) {
        assert(
            std::abs(live_plate_space[index] - pcm24(plate_space_sink.bytes(), index)) < 2.0E-6F
        );
    }

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
