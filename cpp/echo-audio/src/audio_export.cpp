#include "echo/audio/audio_export.hpp"
#include "echo/audio/assembly_mixer.hpp"
#include "echo/audio/export_metadata.hpp"
#include "echo/audio/ffmpeg_include.hpp"
#include "echo/audio/offline_loudness_analyzer.hpp"
#include "echo/audio/playback.hpp"
#include "ffmpeg_output.hpp"
#include <algorithm>
#include <array>
#include <cerrno>
#include <chrono>
#include <cmath>
#include <cstdio>
#include <memory>
#include <span>
#include <stdexcept>
#include <thread>
#include <vector>
namespace echo::audio {
void AudioExportProfile::validate() const {
    if (format != "wav_pcm16" && format != "wav_pcm24" && format != "wav_float32"
        && format != "flac24" && format != "mp3" && format != "aac_m4a")
        throw std::invalid_argument("unsupported audio export format");
    if (sample_rate != 44100 && sample_rate != 48000 && sample_rate != 96000)
        throw std::invalid_argument("unsupported delivery sample rate");
    if (channels != 1 && channels != 2)
        throw std::invalid_argument("delivery requires mono or stereo");
    if ((format == "mp3" || format == "aac_m4a")
        && (sample_rate == 96000
            || (bitrate_kbps != 128 && bitrate_kbps != 192 && bitrate_kbps != 256
                && bitrate_kbps != 320)))
        throw std::invalid_argument("unsupported compressed audio profile");
}
std::string AudioExportProfile::extension() const {
    validate();
    return format == "flac24"    ? "flac"
           : format == "mp3"     ? "mp3"
           : format == "aac_m4a" ? "m4a"
                                 : "wav";
}
std::uint16_t AudioExportProfile::bit_depth() const {
    return format == "wav_pcm16"                      ? 16
           : format == "wav_float32"                  ? 32
           : (format == "mp3" || format == "aac_m4a") ? 0
                                                      : 24;
}
namespace {
void require_ffmpeg(int code, const char* operation) {
    if (code >= 0)
        return;
    std::array<char, AV_ERROR_MAX_STRING_SIZE> text{};
    av_strerror(code, text.data(), text.size());
    throw std::runtime_error(std::string(operation) + ": " + text.data());
}
using namespace detail;
struct SwrDeleter {
    void operator()(SwrContext* value) const {
        swr_free(&value);
    }
};
struct FifoDeleter {
    void operator()(AVAudioFifo* value) const {
        av_audio_fifo_free(value);
    }
};
using PcmReader = std::function<std::span<const float>()>;
OfflineRenderResult encode(
    PcmReader read,
    std::uint64_t expected,
    RenderByteSink& sink,
    const AudioExportProfile& profile,
    const OfflineRenderCallbacks& callbacks,
    std::string_view comment
) {
    profile.validate();
    validate_export_comment(comment);
    const bool mp3 = profile.format == "mp3", aac = profile.format == "aac_m4a",
               flac = profile.format == "flac24";
    const AVCodecID id = mp3                               ? AV_CODEC_ID_MP3
                         : aac                             ? AV_CODEC_ID_AAC
                         : flac                            ? AV_CODEC_ID_FLAC
                         : profile.format == "wav_float32" ? AV_CODEC_ID_PCM_F32LE
                         : profile.format == "wav_pcm16"   ? AV_CODEC_ID_PCM_S16LE
                                                           : AV_CODEC_ID_PCM_S24LE;
    const char* muxer = mp3 ? "mp3" : aac ? "ipod" : flac ? "flac" : "wav";
    AVFormatContext* raw = nullptr;
    require_ffmpeg(
        avformat_alloc_output_context2(&raw, nullptr, muxer, nullptr),
        "create delivery container"
    );
    std::unique_ptr<AVFormatContext, FormatDeleter> format(raw);
    const AVCodec* encoder =
        mp3 ? avcodec_find_encoder_by_name("libmp3lame") : avcodec_find_encoder(id);
    if (!format || !encoder)
        throw std::runtime_error("requested audio encoder is unavailable");
    std::unique_ptr<AVCodecContext, CodecDeleter> codec(avcodec_alloc_context3(encoder));
    if (!codec)
        throw std::bad_alloc();
    codec->sample_fmt = (mp3 || aac)                      ? AV_SAMPLE_FMT_FLTP
                        : profile.format == "wav_float32" ? AV_SAMPLE_FMT_FLT
                        : profile.format == "wav_pcm16"   ? AV_SAMPLE_FMT_S16
                                                          : AV_SAMPLE_FMT_S32;
    codec->sample_rate = static_cast<int>(profile.sample_rate);
    codec->time_base = {1, codec->sample_rate};
    codec->bits_per_raw_sample = profile.bit_depth();
    if (mp3 || aac)
        codec->bit_rate = static_cast<int64_t>(profile.bitrate_kbps) * 1000;
    av_channel_layout_default(&codec->ch_layout, static_cast<int>(profile.channels));
    if (format->oformat->flags & AVFMT_GLOBALHEADER)
        codec->flags |= AV_CODEC_FLAG_GLOBAL_HEADER;
    require_ffmpeg(avcodec_open2(codec.get(), encoder, nullptr), "open delivery encoder");
    AVStream* stream = avformat_new_stream(format.get(), nullptr);
    if (!stream)
        throw std::bad_alloc();
    stream->time_base = codec->time_base;
    require_ffmpeg(
        avcodec_parameters_from_context(stream->codecpar, codec.get()),
        "configure delivery stream"
    );
    AvioBridge bridge{.sink = &sink};
    auto* buffer = static_cast<unsigned char*>(av_malloc(32768));
    if (!buffer)
        throw std::bad_alloc();
    std::unique_ptr<AVIOContext, AvioDeleter> avio(
        avio_alloc_context(buffer, 32768, 1, &bridge, nullptr, write_packet, seek_packet)
    );
    if (!avio) {
        av_free(buffer);
        throw std::bad_alloc();
    }
    format->url = av_strdup(("audio." + profile.extension()).c_str());
    format->pb = avio.get();
    format->flags |= AVFMT_FLAG_CUSTOM_IO;
    std::string delivery_comment(comment);
    const auto append_memory = [&](std::string_view label, const std::string& value) {
        if (value.empty())
            return;
        if (value.size() > 8000 || value.find('\0') != std::string::npos)
            throw std::invalid_argument("invalid memory export text");
        delivery_comment += "\n";
        delivery_comment += label;
        delivery_comment += value;
    };
    append_memory("Notes: ", profile.memory_notes);
    append_memory("Place: ", profile.memory_place);
    append_memory("Time: ", profile.memory_time);
    if (flac && !profile.memory_place.empty())
        require_ffmpeg(
            av_dict_set(&format->metadata, "location", profile.memory_place.c_str(), 0),
            "write memory place"
        );
    if (!delivery_comment.empty())
        require_ffmpeg(
            av_dict_set(&format->metadata, "comment", delivery_comment.c_str(), 0),
            "write source disclosure"
        );
    AVDictionary* options = nullptr;
    if (!mp3 && !aac && !flac)
        av_dict_set(&options, "rf64", "auto", 0);
    const int header_status = avformat_write_header(format.get(), &options);
    av_dict_free(&options);
    require_ffmpeg(header_status, "write delivery header");
    SwrContext* raw_swr = nullptr;
    AVChannelLayout input = AV_CHANNEL_LAYOUT_STEREO;
    require_ffmpeg(
        swr_alloc_set_opts2(
            &raw_swr,
            &codec->ch_layout,
            codec->sample_fmt,
            codec->sample_rate,
            &input,
            AV_SAMPLE_FMT_FLT,
            48000,
            0,
            nullptr
        ),
        "configure delivery conversion"
    );
    std::unique_ptr<SwrContext, SwrDeleter> swr(raw_swr);
    if (profile.bit_depth() == 16 || profile.bit_depth() == 24) {
        require_ffmpeg(
            av_opt_set_int(swr.get(), "dither_method", SWR_DITHER_TRIANGULAR, 0),
            "configure dither"
        );
        require_ffmpeg(
            av_opt_set_int(swr.get(), "output_sample_bits", profile.bit_depth(), 0),
            "configure delivery precision"
        );
    }
    require_ffmpeg(swr_init(swr.get()), "start delivery conversion");
    std::unique_ptr<AVAudioFifo, FifoDeleter> fifo(
        av_audio_fifo_alloc(codec->sample_fmt, static_cast<int>(profile.channels), 8192)
    );
    std::unique_ptr<AVFrame, FrameDeleter> frame(av_frame_alloc()), converted(av_frame_alloc());
    std::unique_ptr<AVPacket, PacketDeleter> packet(av_packet_alloc());
    if (!fifo || !frame || !converted || !packet)
        throw std::bad_alloc();
    OfflineLoudnessAnalyzer meter(profile.sample_rate, profile.channels);
    std::uint64_t input_frames = 0, output_frames = 0;
    const auto cancelled = [&] {
        if (callbacks.cancelled && callbacks.cancelled())
            throw OfflineRenderCancelled();
    };
    const auto packets = [&] {
        for (;;) {
            int status = avcodec_receive_packet(codec.get(), packet.get());
            if (status == AVERROR(EAGAIN) || status == AVERROR_EOF)
                return;
            require_ffmpeg(status, "encode delivery packet");
            av_packet_rescale_ts(packet.get(), codec->time_base, stream->time_base);
            packet->stream_index = stream->index;
            require_ffmpeg(
                av_interleaved_write_frame(format.get(), packet.get()),
                "write delivery packet"
            );
            av_packet_unref(packet.get());
        }
    };
    const int block = codec->frame_size > 0 ? codec->frame_size : 4096;
    const auto submit = [&](int count) {
        cancelled();
        av_frame_unref(frame.get());
        frame->format = codec->sample_fmt;
        frame->sample_rate = codec->sample_rate;
        frame->nb_samples = count;
        frame->pts = static_cast<int64_t>(output_frames);
        require_ffmpeg(
            av_channel_layout_copy(&frame->ch_layout, &codec->ch_layout),
            "copy delivery layout"
        );
        require_ffmpeg(av_frame_get_buffer(frame.get(), 0), "allocate delivery samples");
        if (av_audio_fifo_read(fifo.get(), reinterpret_cast<void**>(frame->data), count) != count)
            throw std::runtime_error("incomplete delivery buffer");
        std::vector<float> measured(static_cast<std::size_t>(count) * profile.channels);
        for (int i = 0; i < count; ++i)
            for (std::uint32_t ch = 0; ch < profile.channels; ++ch) {
                const auto offset = static_cast<std::size_t>(i) * profile.channels + ch;
                switch (codec->sample_fmt) {
                case AV_SAMPLE_FMT_FLTP:
                    measured[offset] = reinterpret_cast<float*>(frame->data[ch])[i];
                    break;
                case AV_SAMPLE_FMT_FLT:
                    measured[offset] = reinterpret_cast<float*>(frame->data[0])[offset];
                    break;
                case AV_SAMPLE_FMT_S16:
                    measured[offset] =
                        static_cast<float>(reinterpret_cast<int16_t*>(frame->data[0])[offset])
                        / 32768.0F;
                    break;
                case AV_SAMPLE_FMT_S32:
                    measured[offset] = static_cast<float>(
                                           reinterpret_cast<int32_t*>(frame->data[0])[offset] & ~255
                                       )
                                       / 2147483648.0F;
                    break;
                default:
                    throw std::runtime_error("unsupported delivery sample type");
                }
            }
        meter.process_interleaved(
            measured.data(),
            static_cast<std::size_t>(count),
            profile.channels
        );
        require_ffmpeg(avcodec_send_frame(codec.get(), frame.get()), "submit delivery samples");
        packets();
        output_frames += static_cast<std::uint64_t>(count);
    };
    const auto convert = [&](std::span<const float> samples) {
        const int count = static_cast<int>(samples.size() / 2);
        const int capacity = std::max(1, swr_get_out_samples(swr.get(), count));
        av_frame_unref(converted.get());
        converted->format = codec->sample_fmt;
        converted->sample_rate = codec->sample_rate;
        converted->nb_samples = capacity;
        require_ffmpeg(
            av_channel_layout_copy(&converted->ch_layout, &codec->ch_layout),
            "copy resample layout"
        );
        require_ffmpeg(av_frame_get_buffer(converted.get(), 0), "allocate resample samples");
        const uint8_t* source = reinterpret_cast<const uint8_t*>(samples.data());
        const int produced = swr_convert(
            swr.get(),
            converted->data,
            capacity,
            samples.empty() ? nullptr : &source,
            count
        );
        require_ffmpeg(produced, "convert delivery samples");
        if (av_audio_fifo_write(fifo.get(), reinterpret_cast<void**>(converted->data), produced)
            != produced)
            throw std::runtime_error("cannot queue delivery samples");
        while (av_audio_fifo_size(fifo.get()) >= block)
            submit(block);
        return produced;
    };
    for (;;) {
        cancelled();
        const auto samples = read();
        if (samples.empty())
            break;
        input_frames += samples.size() / 2;
        convert(samples);
        if (callbacks.progress)
            callbacks.progress(
                std::min(
                    0.99,
                    static_cast<double>(input_frames)
                        / static_cast<double>(std::max<std::uint64_t>(expected, 1))
                )
            );
    }
    if (input_frames != expected)
        throw std::runtime_error("decoded duration does not match delivery source");
    while (convert({}) > 0)
        cancelled();
    if (const int remaining = av_audio_fifo_size(fifo.get()); remaining > 0)
        submit(remaining);
    if (output_frames == 0)
        throw std::runtime_error("cannot export empty audio");
    require_ffmpeg(avcodec_send_frame(codec.get(), nullptr), "finish delivery encoder");
    packets();
    cancelled();
    require_ffmpeg(av_write_trailer(format.get()), "finish delivery container");
    avio_flush(avio.get());
    require_ffmpeg(avio->error, "flush delivery bytes");
    if (callbacks.progress)
        callbacks.progress(1.0);
    const auto loudness = meter.result();
    return {
        .frame_count = output_frames,
        .size_bytes = bridge.extent,
        .sample_rate = profile.sample_rate,
        .channel_count = profile.channels,
        .bit_depth = profile.bit_depth(),
        .integrated_lufs = loudness.integrated_lufs,
        .true_peak_dbtp = loudness.true_peak_dbtp
    };
}
} // namespace
OfflineRenderResult AudioExporter::render(
    const std::string& path,
    const PlaybackAdjustment& adjustment,
    RenderByteSink& sink,
    const AudioExportProfile& profile,
    const OfflineRenderCallbacks& callbacks,
    std::string_view comment
) {
    profile.validate();
    PlaybackSession session(
        path,
        adjustment,
        {.apply_output_guard = false, .collect_metering = false}
    );
    std::vector<float> samples(8192);
    return encode(
        [&]() -> std::span<const float> {
            for (;;) {
                if (callbacks.cancelled && callbacks.cancelled())
                    throw OfflineRenderCancelled();
                const auto frames = session.read(samples.data(), 4096);
                if (frames)
                    return {samples.data(), frames * 2};
                if (session.is_ended() && session.buffered_frames() == 0)
                    return {};
                if (session.is_stopped())
                    throw std::runtime_error("audio decoding stopped before delivery completed");
                std::this_thread::sleep_for(std::chrono::milliseconds(1));
            }
        },
        session.output_frame_count(),
        sink,
        profile,
        callbacks,
        comment
    );
}
OfflineRenderResult AudioExporter::render_assembly(
    const AssemblyMixPlan& plan,
    RenderByteSink& sink,
    const AudioExportProfile& profile,
    const OfflineRenderCallbacks& callbacks,
    std::string_view comment
) {
    profile.validate();
    AssemblyMixer mixer(plan);
    return encode(
        [&] { return mixer.next(callbacks); },
        mixer.frame_count(),
        sink,
        profile,
        callbacks,
        comment
    );
}
} // namespace echo::audio
