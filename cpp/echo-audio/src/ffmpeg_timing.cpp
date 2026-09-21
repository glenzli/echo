#include "ffmpeg_timing.hpp"
#include "ffmpeg_input.hpp"
#include <algorithm>
#include <filesystem>
#include <memory>
#include <mutex>
#include <stdexcept>
#include <string>
#include <vector>
namespace echo::audio {
namespace {
struct FileIdentity {
    std::string path;
    std::uintmax_t bytes;
    std::filesystem::file_time_type modified;
    int stream;
    bool operator==(const FileIdentity&) const = default;
};
struct Entry {
    FileIdentity identity;
    ExactAudioTiming timing;
};
std::mutex cache_mutex;
std::vector<Entry> cache;
FileIdentity identity(const AVFormatContext* format, const AVStream* stream) {
    const std::filesystem::path path(format->url);
    return {
        path.string(),
        std::filesystem::file_size(path),
        std::filesystem::last_write_time(path),
        stream->index
    };
}
ExactAudioTiming inspect(const FileIdentity& file) {
    AVFormatContext* raw = nullptr;
    if (open_audio_input(&raw, file.path) < 0)
        throw std::runtime_error("cannot inspect source timing");
    const auto close = [](AVFormatContext* value) { avformat_close_input(&value); };
    std::unique_ptr<AVFormatContext, decltype(close)> format(raw, close);
    if (avformat_find_stream_info(raw, nullptr) < 0 || file.stream < 0
        || static_cast<unsigned>(file.stream) >= raw->nb_streams)
        throw std::runtime_error("cannot inspect source timing stream");
    auto* stream = raw->streams[file.stream];
    const auto* decoder = avcodec_find_decoder(stream->codecpar->codec_id);
    if (!decoder)
        throw std::runtime_error("source timing decoder unavailable");
    const auto free_codec = [](AVCodecContext* value) { avcodec_free_context(&value); };
    const auto free_frame = [](AVFrame* value) { av_frame_free(&value); };
    const auto free_packet = [](AVPacket* value) { av_packet_free(&value); };
    std::unique_ptr<AVCodecContext, decltype(free_codec)> codec(
        avcodec_alloc_context3(decoder),
        free_codec
    );
    std::unique_ptr<AVFrame, decltype(free_frame)> frame(av_frame_alloc(), free_frame);
    std::unique_ptr<AVPacket, decltype(free_packet)> packet(av_packet_alloc(), free_packet);
    if (!codec || !frame || !packet)
        throw std::bad_alloc();
    if (avcodec_parameters_to_context(codec.get(), stream->codecpar) < 0
        || avcodec_open2(codec.get(), decoder, nullptr) < 0)
        throw std::runtime_error("cannot open source timing decoder");
    ExactAudioTiming timing{0, codec->sample_rate, 0};
    const auto receive = [&] {
        int result;
        while ((result = avcodec_receive_frame(codec.get(), frame.get())) >= 0) {
            if (timing.samples == 0 && frame->best_effort_timestamp != AV_NOPTS_VALUE)
                timing.first_timestamp = frame->best_effort_timestamp;
            timing.samples += frame->nb_samples;
            av_frame_unref(frame.get());
        }
        if (result != AVERROR_EOF && result != AVERROR(EAGAIN))
            throw std::runtime_error("source timing decode failed");
    };
    int result;
    while ((result = av_read_frame(raw, packet.get())) >= 0) {
        if (packet->stream_index == file.stream) {
            if (avcodec_send_packet(codec.get(), packet.get()) < 0)
                throw std::runtime_error("source timing packet failed");
            receive();
        }
        av_packet_unref(packet.get());
    }
    if (result != AVERROR_EOF)
        throw std::runtime_error("source timing read failed");
    if (avcodec_send_packet(codec.get(), nullptr) < 0)
        throw std::runtime_error("source timing flush failed");
    receive();
    if (timing.samples <= 0 || timing.sample_rate <= 0)
        throw std::runtime_error("source audio is empty");
    return timing;
}
} // namespace
std::optional<ExactAudioTiming>
exact_audio_timing(const AVFormatContext* format, const AVStream* stream) {
    const std::string type = format->iformat ? format->iformat->name : "";
    if (type != "aac" && type != "asf" && stream->codecpar->codec_id != AV_CODEC_ID_OPUS)
        return std::nullopt;
    const auto key = identity(format, stream);
    {
        const std::lock_guard lock(cache_mutex);
        const auto found = std::find_if(cache.begin(), cache.end(), [&](const auto& entry) {
            return entry.identity == key;
        });
        if (found != cache.end())
            return found->timing;
    }
    const auto timing = inspect(key);
    if (identity(format, stream) != key)
        throw std::runtime_error("source changed while inspecting timing");
    const std::lock_guard lock(cache_mutex);
    if (cache.size() >= 32)
        cache.erase(cache.begin());
    cache.push_back({key, timing});
    return timing;
}
} // namespace echo::audio
