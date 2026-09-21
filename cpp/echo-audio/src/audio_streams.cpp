#include "echo/audio/audio_streams.hpp"
#include "ffmpeg_input.hpp"
#include "ffmpeg_output.hpp"
#include <array>
#include <memory>
#include <stdexcept>
namespace echo::audio {
namespace {
void check(int status, const char* action) {
    if (status >= 0)
        return;
    std::array<char, AV_ERROR_MAX_STRING_SIZE> text{};
    av_strerror(status, text.data(), text.size());
    throw std::runtime_error(std::string(action) + ": " + text.data());
}
struct InputDeleter {
    void operator()(AVFormatContext* value) const {
        avformat_close_input(&value);
    }
};
std::unique_ptr<AVFormatContext, InputDeleter> open(const std::string& path) {
    AVFormatContext* input = nullptr;
    check(open_audio_input(&input, path), "open audio container");
    std::unique_ptr<AVFormatContext, InputDeleter> value(input);
    check(avformat_find_stream_info(input, nullptr), "inspect audio tracks");
    return value;
}
std::string tag(const AVDictionary* metadata, const char* key) {
    const auto* value = av_dict_get(metadata, key, nullptr, 0);
    return value ? value->value : "";
}
} // namespace
std::vector<AudioStreamInfo> audio_streams(const std::string& path) {
    const auto input = open(path);
    std::vector<AudioStreamInfo> result;
    for (unsigned i = 0; i < input->nb_streams; ++i) {
        const auto* stream = input->streams[i];
        const auto* codec = stream->codecpar;
        if (codec->codec_type != AVMEDIA_TYPE_AUDIO)
            continue;
        const auto duration = audio_duration(input.get(), stream, {1, 1000});
        result.push_back(
            {.index = static_cast<int>(i),
             .codec = avcodec_get_name(codec->codec_id),
             .title = tag(stream->metadata, "title"),
             .language = tag(stream->metadata, "language"),
             .sample_rate = codec->sample_rate,
             .channels = codec->ch_layout.nb_channels,
             .duration_millis = static_cast<std::uint64_t>(std::max<int64_t>(0, duration))}
        );
        if (result.size() > 64)
            throw std::runtime_error("too many audio streams in this container");
    }
    return result;
}
void copy_audio_stream(
    const std::string& path,
    int index,
    RenderByteSink& sink,
    const std::string& source_sha256,
    const std::string& source_filename,
    const OfflineRenderCallbacks& callbacks
) {
    const auto input = open(path);
    if (index < 0 || static_cast<unsigned>(index) >= input->nb_streams
        || input->streams[index]->codecpar->codec_type != AVMEDIA_TYPE_AUDIO)
        throw std::invalid_argument("selected audio stream is unavailable");
    auto* source = input->streams[index];
    AVFormatContext* raw = nullptr;
    check(
        avformat_alloc_output_context2(&raw, nullptr, "matroska", nullptr),
        "create selected audio container"
    );
    std::unique_ptr<AVFormatContext, detail::FormatDeleter> output(raw);
    if (!output)
        throw std::bad_alloc();
    AVStream* target = avformat_new_stream(output.get(), nullptr);
    if (!target)
        throw std::bad_alloc();
    check(avcodec_parameters_copy(target->codecpar, source->codecpar), "copy selected audio codec");
    target->codecpar->codec_tag = 0;
    target->time_base = source->time_base;
    check(av_dict_copy(&output->metadata, input->metadata, 0), "preserve container metadata");
    check(av_dict_copy(&target->metadata, source->metadata, 0), "preserve track metadata");
    const auto title = tag(source->metadata, "title");
    check(
        av_dict_set(
            &output->metadata,
            "title",
            (title.empty() ? source_filename + " · #" + std::to_string(index + 1) : title).c_str(),
            0
        ),
        "name selected audio"
    );
    check(
        av_dict_set(&output->metadata, "echo_source_sha256", source_sha256.c_str(), 0),
        "record original identity"
    );
    check(
        av_dict_set(
            &output->metadata,
            "echo_source_stream_index",
            std::to_string(index).c_str(),
            0
        ),
        "record selected stream"
    );
    check(
        av_dict_set(&output->metadata, "echo_source_filename", source_filename.c_str(), 0),
        "record original name"
    );
    detail::AvioBridge bridge{.sink = &sink};
    auto* buffer = static_cast<unsigned char*>(av_malloc(32768));
    if (!buffer)
        throw std::bad_alloc();
    std::unique_ptr<AVIOContext, detail::AvioDeleter> avio(avio_alloc_context(
        buffer,
        32768,
        1,
        &bridge,
        nullptr,
        detail::write_packet,
        detail::seek_packet
    ));
    if (!avio) {
        av_free(buffer);
        throw std::bad_alloc();
    }
    output->pb = avio.get();
    output->flags |= AVFMT_FLAG_CUSTOM_IO;
    check(avformat_write_header(output.get(), nullptr), "write selected audio header");
    std::unique_ptr<AVPacket, detail::PacketDeleter> packet(av_packet_alloc());
    if (!packet)
        throw std::bad_alloc();
    bool wrote = false;
    int status = 0;
    const auto start = source->start_time == AV_NOPTS_VALUE ? 0 : source->start_time;
    while ((status = av_read_frame(input.get(), packet.get())) >= 0) {
        if (callbacks.cancelled && callbacks.cancelled())
            throw OfflineRenderCancelled();
        if (packet->stream_index == index) {
            if (packet->pts != AV_NOPTS_VALUE)
                packet->pts -= start;
            if (packet->dts != AV_NOPTS_VALUE)
                packet->dts -= start;
            av_packet_rescale_ts(packet.get(), source->time_base, target->time_base);
            packet->stream_index = target->index;
            packet->pos = -1;
            check(
                av_interleaved_write_frame(output.get(), packet.get()),
                "copy selected audio packet"
            );
            wrote = true;
        }
        av_packet_unref(packet.get());
    }
    if (status != AVERROR_EOF)
        check(status, "read selected audio packet");
    if (!wrote)
        throw std::runtime_error("selected audio stream is empty");
    check(av_write_trailer(output.get()), "finish selected audio container");
    avio_flush(avio.get());
    check(avio->error, "flush selected audio");
}
} // namespace echo::audio
