#pragma once
#include "echo/audio/ffmpeg_include.hpp"
#include "echo/audio/offline_render.hpp"
#include <algorithm>
#include <cerrno>
#include <cstdio>
#include <memory>
#include <span>
namespace echo::audio::detail {
struct AvioBridge {
    RenderByteSink* sink = nullptr;
    std::uint64_t cursor = 0;
    std::uint64_t extent = 0;
};

inline int write_packet(void* opaque, const std::uint8_t* buffer, int size) {
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

inline std::int64_t seek_packet(void* opaque, std::int64_t offset, int whence) {
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
        av_freep(&value->buffer);
        avio_context_free(&value);
    }
};

} // namespace echo::audio::detail
