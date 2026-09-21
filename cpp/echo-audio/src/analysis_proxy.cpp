#include "echo/audio/analysis_proxy.hpp"
#include "ffmpeg_input.hpp"

#include "echo/audio/ffmpeg_include.hpp"

#include <algorithm>
#include <array>
#include <cstddef>
#include <cstdint>
#include <fstream>
#include <limits>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <vector>

namespace echo::audio {
namespace {

constexpr int kProxySampleRate = 16000;
constexpr std::uint16_t kProxyChannels = 1;
constexpr std::uint16_t kProxyBits = 16;
constexpr std::uint64_t kMaximumRangeMillis = 10 * 60 * 1000;

std::string av_error_text(int code) {
    char buffer[AV_ERROR_MAX_STRING_SIZE] = {0};
    av_strerror(code, buffer, sizeof(buffer));
    return buffer;
}

[[noreturn]] void fail(const std::string& message) {
    throw std::runtime_error(message);
}

class FormatContext {
  public:
    ~FormatContext() {
        if (pointer_ != nullptr) {
            avformat_close_input(&pointer_);
        }
    }

    AVFormatContext** slot() {
        return &pointer_;
    }
    AVFormatContext* get() const {
        return pointer_;
    }

  private:
    AVFormatContext* pointer_ = nullptr;
};

struct CodecDeleter {
    void operator()(AVCodecContext* context) const {
        avcodec_free_context(&context);
    }
};

struct PacketDeleter {
    void operator()(AVPacket* packet) const {
        av_packet_free(&packet);
    }
};

struct FrameDeleter {
    void operator()(AVFrame* frame) const {
        av_frame_free(&frame);
    }
};

struct SwrDeleter {
    void operator()(SwrContext* context) const {
        swr_free(&context);
    }
};

void write_u16(std::ostream& output, std::uint16_t value) {
    const std::array<char, 2> bytes{
        static_cast<char>(value & 0xffU),
        static_cast<char>((value >> 8U) & 0xffU),
    };
    output.write(bytes.data(), static_cast<std::streamsize>(bytes.size()));
}

void write_u32(std::ostream& output, std::uint32_t value) {
    const std::array<char, 4> bytes{
        static_cast<char>(value & 0xffU),
        static_cast<char>((value >> 8U) & 0xffU),
        static_cast<char>((value >> 16U) & 0xffU),
        static_cast<char>((value >> 24U) & 0xffU),
    };
    output.write(bytes.data(), static_cast<std::streamsize>(bytes.size()));
}

class PcmWavWriter {
  public:
    explicit PcmWavWriter(const std::string& path) :
        output_(path, std::ios::binary | std::ios::in | std::ios::out | std::ios::trunc) {
        if (!output_) {
            fail("cannot create analysis proxy " + path);
        }
        output_.write("RIFF", 4);
        write_u32(output_, 0);
        output_.write("WAVEfmt ", 8);
        write_u32(output_, 16);
        write_u16(output_, 1);
        write_u16(output_, kProxyChannels);
        write_u32(output_, kProxySampleRate);
        write_u32(output_, kProxySampleRate * kProxyChannels * kProxyBits / 8U);
        write_u16(output_, kProxyChannels * kProxyBits / 8U);
        write_u16(output_, kProxyBits);
        output_.write("data", 4);
        write_u32(output_, 0);
    }

    void append(const std::int16_t* samples, std::size_t count) {
        output_.write(
            reinterpret_cast<const char*>(samples),
            static_cast<std::streamsize>(count * sizeof(std::int16_t))
        );
        if (!output_) {
            fail("cannot write analysis proxy");
        }
        frame_count_ += count;
    }

    AnalysisProxyResult finish() {
        if (frame_count_ == 0) {
            fail("analysis proxy range decoded no samples");
        }
        const std::uint64_t data_size = frame_count_ * sizeof(std::int16_t);
        if (data_size > std::numeric_limits<std::uint32_t>::max() - 36U) {
            fail("analysis proxy exceeds RIFF size limit");
        }
        output_.seekp(4);
        write_u32(output_, static_cast<std::uint32_t>(36U + data_size));
        output_.seekp(40);
        write_u32(output_, static_cast<std::uint32_t>(data_size));
        output_.flush();
        if (!output_) {
            fail("cannot finalize analysis proxy");
        }
        return AnalysisProxyResult{
            .sample_rate = kProxySampleRate,
            .channel_count = kProxyChannels,
            .frame_count = frame_count_,
            .size_bytes = 44U + data_size,
        };
    }

  private:
    std::fstream output_;
    std::uint64_t frame_count_ = 0;
};

} // namespace

AnalysisProxyResult build_analysis_proxy(
    const std::string& source_path,
    const std::string& output_path,
    std::uint64_t start_millis,
    std::uint64_t end_millis
) {
    if (end_millis <= start_millis || end_millis - start_millis > kMaximumRangeMillis) {
        fail("analysis proxy range must be positive and at most ten minutes");
    }

    FormatContext format;
    int result = open_audio_input(format.slot(), source_path);
    if (result < 0) {
        fail("cannot open " + source_path + ": " + av_error_text(result));
    }
    result = avformat_find_stream_info(format.get(), nullptr);
    if (result < 0) {
        fail("cannot read stream info for " + source_path + ": " + av_error_text(result));
    }
    const AVStream* stream = nullptr;
    for (unsigned int index = 0; index < format.get()->nb_streams; ++index) {
        if (format.get()->streams[index]->codecpar->codec_type == AVMEDIA_TYPE_AUDIO) {
            stream = format.get()->streams[index];
            break;
        }
    }
    if (stream == nullptr) {
        fail("no audio stream in " + source_path);
    }

    const AVCodec* decoder = avcodec_find_decoder(stream->codecpar->codec_id);
    if (decoder == nullptr) {
        fail("no decoder for analysis proxy source");
    }
    std::unique_ptr<AVCodecContext, CodecDeleter> codec(avcodec_alloc_context3(decoder));
    if (codec == nullptr) {
        fail("cannot allocate analysis proxy decoder");
    }
    result = avcodec_parameters_to_context(codec.get(), stream->codecpar);
    if (result < 0 || avcodec_open2(codec.get(), decoder, nullptr) < 0) {
        fail("cannot open analysis proxy decoder");
    }

    AVChannelLayout output_layout;
    av_channel_layout_default(&output_layout, kProxyChannels);
    SwrContext* raw_resampler = nullptr;
    result = swr_alloc_set_opts2(
        &raw_resampler,
        &output_layout,
        AV_SAMPLE_FMT_S16,
        kProxySampleRate,
        &codec->ch_layout,
        codec->sample_fmt,
        codec->sample_rate,
        0,
        nullptr
    );
    av_channel_layout_uninit(&output_layout);
    if (result < 0 || raw_resampler == nullptr || swr_init(raw_resampler) < 0) {
        if (raw_resampler != nullptr) {
            swr_free(&raw_resampler);
        }
        fail("cannot initialize analysis proxy resampler");
    }
    std::unique_ptr<SwrContext, SwrDeleter> resampler(raw_resampler);

    const auto source_start = audio_start_time(format.get(), stream);
    const std::int64_t seek_timestamp = source_start
                                        + av_rescale_q(
                                            static_cast<std::int64_t>(start_millis),
                                            AVRational{1, 1000},
                                            stream->time_base
                                        );
    if (start_millis != 0) {
        const auto preroll = (codec->frame_size > 0 || av_get_bits_per_sample(codec->codec_id) == 0)
                                 ? av_rescale_q(
                                       std::max(4096, codec->frame_size * 4),
                                       AVRational{1, codec->sample_rate},
                                       stream->time_base
                                   )
                                 : 0;
        if (av_seek_frame(
                format.get(),
                stream->index,
                seek_timestamp - preroll,
                AVSEEK_FLAG_BACKWARD
            )
            < 0) {
            fail("cannot seek analysis proxy source");
        }
        avcodec_flush_buffers(codec.get());
    }

    PcmWavWriter writer(output_path);
    std::unique_ptr<AVPacket, PacketDeleter> packet(av_packet_alloc());
    std::unique_ptr<AVFrame, FrameDeleter> frame(av_frame_alloc());
    if (packet == nullptr || frame == nullptr) {
        fail("cannot allocate analysis proxy decode buffers");
    }
    bool reached_end = false;
    std::optional<std::int64_t> output_position;
    const auto first_frame = av_rescale(static_cast<int64_t>(start_millis), kProxySampleRate, 1000);
    const auto last_frame = av_rescale(static_cast<int64_t>(end_millis), kProxySampleRate, 1000);
    const auto append_output = [&](const std::vector<std::int16_t>& output, int count) {
        if (!output_position)
            return;
        const auto first = std::clamp<int64_t>(first_frame - *output_position, 0, count);
        const auto last = std::clamp<int64_t>(last_frame - *output_position, first, count);
        if (last > first)
            writer.append(output.data() + first, static_cast<std::size_t>(last - first));
        *output_position += count;
        reached_end = *output_position >= last_frame;
    };
    const auto receive = [&]() {
        while (!reached_end && avcodec_receive_frame(codec.get(), frame.get()) == 0) {
            const std::int64_t timestamp = frame->best_effort_timestamp;
            if (!output_position) {
                output_position = timestamp == AV_NOPTS_VALUE ? first_frame
                                                              : av_rescale_q(
                                                                    timestamp - source_start,
                                                                    stream->time_base,
                                                                    AVRational{1, kProxySampleRate}
                                                                );
            }

            const int capacity = swr_get_out_samples(resampler.get(), frame->nb_samples);
            std::vector<std::int16_t> output(static_cast<std::size_t>(capacity));
            std::array<std::uint8_t*, 1> output_planes{
                reinterpret_cast<std::uint8_t*>(output.data()),
            };
            const int converted = swr_convert(
                resampler.get(),
                output_planes.data(),
                capacity,
                const_cast<const std::uint8_t**>(frame->extended_data),
                frame->nb_samples
            );
            if (converted < 0) {
                fail("cannot resample analysis proxy: " + av_error_text(converted));
            }
            append_output(output, converted);
            av_frame_unref(frame.get());
        }
    };

    while (!reached_end && av_read_frame(format.get(), packet.get()) >= 0) {
        if (packet->stream_index == stream->index
            && avcodec_send_packet(codec.get(), packet.get()) >= 0) {
            receive();
        }
        av_packet_unref(packet.get());
    }
    if (!reached_end && avcodec_send_packet(codec.get(), nullptr) >= 0) {
        receive();
    }
    while (!reached_end && output_position) {
        const int capacity = std::max(1, swr_get_out_samples(resampler.get(), 0));
        std::vector<std::int16_t> output(static_cast<std::size_t>(capacity));
        std::uint8_t* plane = reinterpret_cast<std::uint8_t*>(output.data());
        const int count = swr_convert(resampler.get(), &plane, capacity, nullptr, 0);
        if (count < 0)
            fail("cannot flush analysis proxy resampler");
        if (count == 0)
            break;
        append_output(output, count);
    }
    return writer.finish();
}

} // namespace echo::audio
