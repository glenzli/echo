#include "echo/audio/offline_flac_renderer.hpp"

#include "echo/audio/offline_loudness_analyzer.hpp"
#include "echo/audio/playback.hpp"

#include <algorithm>
#include <array>
#include <cerrno>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <limits>
#include <memory>
#include <span>
#include <stdexcept>
#include <string>
#include <thread>
#include <vector>

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/avutil.h>
#include <libavutil/channel_layout.h>
#include <libavutil/error.h>
#include <libavutil/frame.h>
#include <libavutil/mem.h>
}

namespace echo::audio {
namespace {

constexpr std::uint32_t kSampleRate = 48'000;
constexpr std::uint32_t kChannels = 2;
constexpr std::uint16_t kBitDepth = 24;
constexpr std::size_t kDecodeFrames = 4096;

std::runtime_error ffmpeg_error(const char* operation, int code) {
    std::array<char, AV_ERROR_MAX_STRING_SIZE> message{};
    av_strerror(code, message.data(), message.size());
    return std::runtime_error(std::string(operation) + ": " + message.data());
}

void require_ffmpeg(int code, const char* operation) {
    if (code < 0) {
        throw ffmpeg_error(operation, code);
    }
}

struct AvioBridge {
    RenderByteSink* sink = nullptr;
    std::uint64_t cursor = 0;
    std::uint64_t extent = 0;
};

int write_packet(void* opaque, const std::uint8_t* buffer, int size) {
    auto& bridge = *static_cast<AvioBridge*>(opaque);
    try {
        bridge.sink->write(std::as_bytes(std::span(buffer, static_cast<std::size_t>(size))));
        bridge.cursor += static_cast<std::uint64_t>(size);
        bridge.extent = std::max(bridge.extent, bridge.cursor);
        return size;
    } catch (...) {
        return AVERROR(EIO);
    }
}

std::int64_t seek_packet(void* opaque, std::int64_t offset, int whence) {
    auto& bridge = *static_cast<AvioBridge*>(opaque);
    if ((whence & AVSEEK_SIZE) != 0) {
        return static_cast<std::int64_t>(bridge.extent);
    }
    whence &= ~AVSEEK_FORCE;
    std::int64_t target = offset;
    if (whence == SEEK_CUR) {
        target += static_cast<std::int64_t>(bridge.cursor);
    } else if (whence == SEEK_END) {
        target += static_cast<std::int64_t>(bridge.extent);
    } else if (whence != SEEK_SET) {
        return AVERROR(EINVAL);
    }
    if (target < 0) {
        return AVERROR(EINVAL);
    }
    try {
        bridge.sink->seek(static_cast<std::uint64_t>(target));
        bridge.cursor = static_cast<std::uint64_t>(target);
        return target;
    } catch (...) {
        return AVERROR(EIO);
    }
}

struct FormatDeleter {
    void operator()(AVFormatContext* value) const {
        avformat_free_context(value);
    }
};
struct CodecDeleter {
    void operator()(AVCodecContext* value) const {
        avcodec_free_context(&value);
    }
};
struct FrameDeleter {
    void operator()(AVFrame* value) const {
        av_frame_free(&value);
    }
};
struct PacketDeleter {
    void operator()(AVPacket* value) const {
        av_packet_free(&value);
    }
};
struct AvioDeleter {
    void operator()(AVIOContext* value) const {
        avio_context_free(&value);
    }
};

class Dither {
  public:
    float tpdf() {
        return (uniform() - uniform()) / 8'388'608.0F;
    }

  private:
    float uniform() {
        state_ ^= state_ << 13U;
        state_ ^= state_ >> 17U;
        state_ ^= state_ << 5U;
        return static_cast<float>(state_ & 0x00ff'ffffU) / 16'777'216.0F;
    }
    std::uint32_t state_ = 0x464c'4143U;
};

std::int32_t quantize24(float sample, Dither& dither) {
    const float prepared = std::clamp(sample + dither.tpdf(), -1.0F, 0.99999988F);
    const auto value = static_cast<std::int32_t>(std::lrint(prepared * 8'388'607.0F));
    return std::clamp(value, -8'388'608, 8'388'607);
}

bool cancelled(const OfflineRenderCallbacks& callbacks) {
    return callbacks.cancelled && callbacks.cancelled();
}

void report_progress(const OfflineRenderCallbacks& callbacks, double value) {
    if (callbacks.progress) {
        callbacks.progress(std::clamp(value, 0.0, 1.0));
    }
}

} // namespace

OfflineRenderResult OfflineFlacRenderer::render(
    const std::string& sourcePath,
    const PlaybackAdjustment& adjustment,
    RenderByteSink& sink,
    const OfflineRenderCallbacks& callbacks
) {
    if (adjustment.trim_end_millis <= adjustment.trim_start_millis) {
        throw std::invalid_argument("offline render requires a non-empty selection");
    }

    AVFormatContext* raw_format = nullptr;
    require_ffmpeg(
        avformat_alloc_output_context2(&raw_format, nullptr, "flac", nullptr),
        "create FLAC container"
    );
    std::unique_ptr<AVFormatContext, FormatDeleter> format(raw_format);
    if (!format) {
        throw std::runtime_error("FLAC container is unavailable");
    }
    const AVCodec* encoder = avcodec_find_encoder(AV_CODEC_ID_FLAC);
    if (encoder == nullptr) {
        throw std::runtime_error("FLAC encoder is unavailable");
    }
    std::unique_ptr<AVCodecContext, CodecDeleter> codec(avcodec_alloc_context3(encoder));
    if (!codec) {
        throw std::runtime_error("cannot allocate FLAC encoder");
    }
    codec->sample_fmt = AV_SAMPLE_FMT_S32;
    codec->sample_rate = static_cast<int>(kSampleRate);
    codec->time_base = AVRational{1, static_cast<int>(kSampleRate)};
    codec->bits_per_raw_sample = kBitDepth;
    av_channel_layout_default(&codec->ch_layout, static_cast<int>(kChannels));
    if ((format->oformat->flags & AVFMT_GLOBALHEADER) != 0) {
        codec->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    }
    require_ffmpeg(avcodec_open2(codec.get(), encoder, nullptr), "open FLAC encoder");

    AVStream* stream = avformat_new_stream(format.get(), nullptr);
    if (stream == nullptr) {
        throw std::runtime_error("cannot allocate FLAC stream");
    }
    stream->time_base = codec->time_base;
    require_ffmpeg(
        avcodec_parameters_from_context(stream->codecpar, codec.get()),
        "configure FLAC stream"
    );

    AvioBridge bridge{.sink = &sink};
    constexpr int kAvioBufferSize = 32 * 1024;
    auto* avio_buffer = static_cast<unsigned char*>(av_malloc(kAvioBufferSize));
    if (avio_buffer == nullptr) {
        throw std::runtime_error("cannot allocate FLAC output buffer");
    }
    std::unique_ptr<AVIOContext, AvioDeleter> avio(avio_alloc_context(
        avio_buffer,
        kAvioBufferSize,
        1,
        &bridge,
        nullptr,
        write_packet,
        seek_packet
    ));
    if (!avio) {
        av_free(avio_buffer);
        throw std::runtime_error("cannot create FLAC output stream");
    }
    format->pb = avio.get();
    format->flags |= AVFMT_FLAG_CUSTOM_IO;
    require_ffmpeg(avformat_write_header(format.get(), nullptr), "write FLAC header");

    std::unique_ptr<AVFrame, FrameDeleter> frame(av_frame_alloc());
    std::unique_ptr<AVPacket, PacketDeleter> packet(av_packet_alloc());
    if (!frame || !packet) {
        throw std::runtime_error("cannot allocate FLAC encode buffers");
    }

    PlaybackSession session(
        sourcePath,
        adjustment,
        {.apply_output_guard = false, .collect_metering = false}
    );
    OfflineLoudnessAnalyzer analyzer(session.sample_rate(), session.channel_count());
    const std::size_t encode_frames =
        codec->frame_size > 0 ? static_cast<std::size_t>(codec->frame_size) : kDecodeFrames;
    std::vector<float> decoded(kDecodeFrames * session.channel_count());
    std::vector<float> pending;
    pending.reserve(encode_frames * kChannels * 2U);
    std::vector<float> measured;
    Dither dither;
    std::uint64_t frame_count = 0;
    const std::uint64_t expected_frames =
        (adjustment.trim_end_millis - adjustment.trim_start_millis) * kSampleRate / 1000U;

    const auto drain_packets = [&] {
        while (true) {
            const int status = avcodec_receive_packet(codec.get(), packet.get());
            if (status == AVERROR(EAGAIN) || status == AVERROR_EOF) {
                return;
            }
            require_ffmpeg(status, "encode FLAC packet");
            av_packet_rescale_ts(packet.get(), codec->time_base, stream->time_base);
            packet->stream_index = stream->index;
            require_ffmpeg(
                av_interleaved_write_frame(format.get(), packet.get()),
                "write FLAC packet"
            );
            av_packet_unref(packet.get());
        }
    };
    const auto encode = [&](std::span<const float> samples, std::size_t frames) {
        av_frame_unref(frame.get());
        frame->format = codec->sample_fmt;
        frame->sample_rate = codec->sample_rate;
        frame->nb_samples = static_cast<int>(frames);
        frame->pts = static_cast<std::int64_t>(frame_count);
        require_ffmpeg(
            av_channel_layout_copy(&frame->ch_layout, &codec->ch_layout),
            "configure FLAC frame channels"
        );
        require_ffmpeg(av_frame_get_buffer(frame.get(), 0), "allocate FLAC frame samples");
        require_ffmpeg(av_frame_make_writable(frame.get()), "prepare FLAC frame samples");
        auto* destination = reinterpret_cast<std::int32_t*>(frame->data[0]);
        measured.resize(samples.size());
        for (std::size_t index = 0; index < samples.size(); ++index) {
            const std::int32_t value = quantize24(samples[index], dither);
            destination[index] = value * 256;
            measured[index] = static_cast<float>(value) / 8'388'607.0F;
        }
        analyzer.process_interleaved(measured.data(), frames, kChannels);
        require_ffmpeg(avcodec_send_frame(codec.get(), frame.get()), "submit FLAC frame");
        drain_packets();
        frame_count += frames;
    };

    while (true) {
        if (cancelled(callbacks)) {
            session.stop();
            throw OfflineRenderCancelled();
        }
        const std::size_t frames = session.read(decoded.data(), kDecodeFrames);
        if (frames == 0) {
            if (session.is_ended() && session.buffered_frames() == 0) {
                break;
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(1));
            continue;
        }
        pending.insert(
            pending.end(),
            decoded.begin(),
            decoded.begin() + static_cast<std::ptrdiff_t>(frames * kChannels)
        );
        while (pending.size() >= encode_frames * kChannels) {
            encode(std::span(pending.data(), encode_frames * kChannels), encode_frames);
            pending.erase(
                pending.begin(),
                pending.begin() + static_cast<std::ptrdiff_t>(encode_frames * kChannels)
            );
        }
        report_progress(
            callbacks,
            expected_frames == 0
                ? 0.0
                : static_cast<double>(frame_count) / static_cast<double>(expected_frames)
        );
    }
    session.stop();
    if (!pending.empty()) {
        encode(std::span(pending), pending.size() / kChannels);
    }
    require_ffmpeg(avcodec_send_frame(codec.get(), nullptr), "finish FLAC encoder");
    drain_packets();
    require_ffmpeg(av_write_trailer(format.get()), "finish FLAC container");
    avio_flush(avio.get());
    report_progress(callbacks, 1.0);
    const OfflineLoudnessResult loudness = analyzer.result();
    return {
        .frame_count = frame_count,
        .size_bytes = bridge.extent,
        .sample_rate = kSampleRate,
        .channel_count = kChannels,
        .bit_depth = kBitDepth,
        .integrated_lufs = loudness.integrated_lufs,
        .true_peak_dbtp = loudness.true_peak_dbtp,
    };
}

} // namespace echo::audio
