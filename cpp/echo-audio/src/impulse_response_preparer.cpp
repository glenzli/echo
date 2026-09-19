#include "echo/audio/impulse_response_preparer.hpp"
#include "ffmpeg_input.hpp"

#include "echo/audio/ffmpeg_include.hpp"

#include <algorithm>
#include <array>
#include <bit>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <limits>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

namespace echo::audio {
namespace {

constexpr std::uint64_t kMaximumSourceBytes = 32U * 1024U * 1024U;
constexpr std::uint64_t kMaximumPreparedFrames =
    5U * static_cast<std::uint64_t>(kPreparedImpulseResponseSampleRate);
constexpr std::uint32_t kMinimumSourceSampleRate = 8000;
constexpr std::uint32_t kMaximumSourceSampleRate = 192000;
constexpr std::uint16_t kWaveFormatPcm = 0x0001;
constexpr std::uint16_t kWaveFormatIeeeFloat = 0x0003;
constexpr std::uint16_t kWaveFormatExtensible = 0xfffe;
constexpr std::uint32_t kMonoChannelMask = 0x00000004;
constexpr std::uint32_t kStereoChannelMask = 0x00000003;
constexpr std::uint32_t kQuadChannelMask = 0x00000033;

[[noreturn]] void fail(const std::string& message) {
    throw std::runtime_error(message);
}

std::string av_error_text(int code) {
    char buffer[AV_ERROR_MAX_STRING_SIZE] = {0};
    av_strerror(code, buffer, sizeof(buffer));
    return buffer;
}

void require_av(int result, const std::string& operation) {
    if (result < 0) {
        fail(operation + ": " + av_error_text(result));
    }
}

std::uint16_t read_u16(const std::uint8_t* bytes) {
    return static_cast<std::uint16_t>(bytes[0])
           | static_cast<std::uint16_t>(static_cast<std::uint16_t>(bytes[1]) << 8U);
}

std::uint32_t read_u32(const std::uint8_t* bytes) {
    return static_cast<std::uint32_t>(bytes[0]) | (static_cast<std::uint32_t>(bytes[1]) << 8U)
           | (static_cast<std::uint32_t>(bytes[2]) << 16U)
           | (static_cast<std::uint32_t>(bytes[3]) << 24U);
}

void read_exact(std::ifstream& input, void* destination, std::size_t size) {
    input.read(static_cast<char*>(destination), static_cast<std::streamsize>(size));
    if (input.gcount() != static_cast<std::streamsize>(size)) {
        fail("truncated RIFF/WAVE source");
    }
}

struct WavContract {
    bool floating_point = false;
    std::uint16_t channel_count = 0;
    std::uint16_t container_bits = 0;
    std::uint16_t valid_bits = 0;
    std::uint32_t sample_rate = 0;
    std::uint32_t channel_mask = 0;
};

bool has_wave_subtype_suffix(const std::array<std::uint8_t, 40>& bytes) {
    constexpr std::array<std::uint8_t, 12> suffix{
        0x00,
        0x00,
        0x10,
        0x00,
        0x80,
        0x00,
        0x00,
        0xaa,
        0x00,
        0x38,
        0x9b,
        0x71,
    };
    return std::equal(suffix.begin(), suffix.end(), bytes.begin() + 28);
}

WavContract parse_format_chunk(
    const std::array<std::uint8_t, 40>& bytes,
    std::uint32_t size,
    ImpulseResponsePreparationLayout layout
) {
    if (size < 16U) {
        fail("WAV format chunk is truncated");
    }
    const std::uint16_t format_tag = read_u16(bytes.data());
    WavContract contract{
        .floating_point = format_tag == kWaveFormatIeeeFloat,
        .channel_count = read_u16(bytes.data() + 2),
        .container_bits = read_u16(bytes.data() + 14),
        .valid_bits = read_u16(bytes.data() + 14),
        .sample_rate = read_u32(bytes.data() + 4),
    };
    const std::uint32_t byte_rate = read_u32(bytes.data() + 8);
    const std::uint16_t block_align = read_u16(bytes.data() + 12);

    const bool true_stereo = layout == ImpulseResponsePreparationLayout::TrueStereoLlLrRlRr;
    if ((!true_stereo && contract.channel_count != 1U && contract.channel_count != 2U)
        || (true_stereo && contract.channel_count != 4U)) {
        fail(
            true_stereo ? "true-stereo impulse response WAV must have four channels"
                        : "impulse response WAV must be mono or stereo"
        );
    }
    if (contract.sample_rate < kMinimumSourceSampleRate
        || contract.sample_rate > kMaximumSourceSampleRate) {
        fail("impulse response WAV sample rate must be between 8 and 192 kHz");
    }

    if (format_tag == kWaveFormatExtensible) {
        if (size < 40U || read_u16(bytes.data() + 16) < 22U || !has_wave_subtype_suffix(bytes)) {
            fail("invalid WAVE_FORMAT_EXTENSIBLE header");
        }
        contract.valid_bits = read_u16(bytes.data() + 18);
        contract.channel_mask = read_u32(bytes.data() + 20);
        const std::uint32_t subtype = read_u32(bytes.data() + 24);
        if (subtype == kWaveFormatPcm) {
            contract.floating_point = false;
        } else if (subtype == kWaveFormatIeeeFloat) {
            contract.floating_point = true;
        } else {
            fail("compressed WAVE_FORMAT_EXTENSIBLE sources are not supported");
        }
        const std::uint32_t expected_mask = contract.channel_count == 1U   ? kMonoChannelMask
                                            : contract.channel_count == 2U ? kStereoChannelMask
                                                                           : kQuadChannelMask;
        if (contract.channel_mask != expected_mask) {
            fail("WAVE_FORMAT_EXTENSIBLE channel mask is ambiguous or unsupported");
        }
    } else if (format_tag != kWaveFormatPcm && format_tag != kWaveFormatIeeeFloat) {
        fail("impulse response WAV must contain uncompressed PCM or IEEE float samples");
    } else if (true_stereo) {
        fail("true-stereo impulse response WAV must use WAVE_FORMAT_EXTENSIBLE");
    }

    const bool accepted_integer_bits =
        contract.valid_bits == 16U || contract.valid_bits == 24U || contract.valid_bits == 32U;
    if ((contract.floating_point && (contract.container_bits != 32U || contract.valid_bits != 32U))
        || (!contract.floating_point
            && (!accepted_integer_bits || contract.valid_bits > contract.container_bits
                || (contract.container_bits != 16U && contract.container_bits != 24U
                    && contract.container_bits != 32U)))) {
        fail("impulse response WAV sample format must be PCM16/24/32 or float32");
    }

    const std::uint32_t expected_block_align =
        static_cast<std::uint32_t>(contract.channel_count) * contract.container_bits / 8U;
    const std::uint64_t expected_byte_rate =
        static_cast<std::uint64_t>(contract.sample_rate) * expected_block_align;
    if (block_align != expected_block_align || byte_rate != expected_byte_rate) {
        fail("impulse response WAV byte layout is inconsistent");
    }
    return contract;
}

WavContract inspect_wav_contract(const std::string& path, ImpulseResponsePreparationLayout layout) {
    const std::uint64_t file_size = std::filesystem::file_size(path);
    if (file_size < 12U || file_size > kMaximumSourceBytes) {
        fail("impulse response source must be a non-empty RIFF/WAVE no larger than 32 MiB");
    }
    std::ifstream input(path, std::ios::binary);
    if (!input) {
        fail("cannot open impulse response source " + path);
    }
    std::array<std::uint8_t, 12> riff{};
    read_exact(input, riff.data(), riff.size());
    if (!std::equal(riff.begin(), riff.begin() + 4, reinterpret_cast<const std::uint8_t*>("RIFF"))
        || !std::equal(
            riff.begin() + 8,
            riff.end(),
            reinterpret_cast<const std::uint8_t*>("WAVE")
        )) {
        fail("impulse response source must be classic RIFF/WAVE; RF64 and Wave64 are unsupported");
    }

    std::uint64_t cursor = 12;
    while (cursor + 8U <= file_size) {
        input.seekg(static_cast<std::streamoff>(cursor));
        std::array<std::uint8_t, 8> header{};
        read_exact(input, header.data(), header.size());
        const std::uint32_t chunk_size = read_u32(header.data() + 4);
        const std::uint64_t data_start = cursor + 8U;
        const std::uint64_t padded_size =
            static_cast<std::uint64_t>(chunk_size) + static_cast<std::uint64_t>(chunk_size & 1U);
        if (padded_size > file_size - data_start) {
            fail("RIFF/WAVE chunk exceeds the source file");
        }
        if (std::equal(
                header.begin(),
                header.begin() + 4,
                reinterpret_cast<const std::uint8_t*>("fmt ")
            )) {
            std::array<std::uint8_t, 40> format{};
            const std::size_t bytes_to_read =
                std::min<std::size_t>(format.size(), static_cast<std::size_t>(chunk_size));
            read_exact(input, format.data(), bytes_to_read);
            return parse_format_chunk(format, chunk_size, layout);
        }
        cursor = data_start + padded_size;
    }
    fail("RIFF/WAVE source has no format chunk");
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

void validate_decoder_contract(const AVCodecParameters& parameters, const WavContract& contract) {
    if (parameters.sample_rate != static_cast<int>(contract.sample_rate)
        || parameters.ch_layout.nb_channels != static_cast<int>(contract.channel_count)) {
        fail("decoded WAV stream does not match its RIFF format contract");
    }
    if (contract.channel_count == 4U
        && (parameters.ch_layout.order != AV_CHANNEL_ORDER_NATIVE
            || parameters.ch_layout.u.mask != kQuadChannelMask)) {
        fail("decoded true-stereo WAV does not preserve its quad transport layout");
    }
    if (contract.floating_point) {
        if (parameters.codec_id != AV_CODEC_ID_PCM_F32LE) {
            fail("WAV float32 contract decoded as an unexpected codec");
        }
        return;
    }
    const bool codec_matches =
        (contract.valid_bits == 16U && parameters.codec_id == AV_CODEC_ID_PCM_S16LE)
        || (contract.valid_bits == 24U
            && (parameters.codec_id == AV_CODEC_ID_PCM_S24LE
                || (contract.container_bits == 32U
                    && parameters.codec_id == AV_CODEC_ID_PCM_S32LE)))
        || (contract.valid_bits == 32U && parameters.codec_id == AV_CODEC_ID_PCM_S32LE);
    if (!codec_matches) {
        fail("WAV PCM contract decoded as an unexpected codec");
    }
}

struct DecodedImpulseResponse {
    std::array<std::vector<float>, 4> channels;
    std::uint64_t source_frame_count = 0;
    bool has_nonzero_sample = false;
};

void append_resampled(
    DecodedImpulseResponse& decoded,
    const std::array<std::vector<float>, 4>& output,
    int converted,
    std::uint16_t channel_count
) {
    if (converted < 0) {
        fail("negative converted sample count");
    }
    const std::uint64_t converted_frames = static_cast<std::uint64_t>(converted);
    const std::uint64_t current_frames = decoded.channels[0].size();
    if (converted_frames > kMaximumPreparedFrames - current_frames) {
        fail("prepared impulse response exceeds five seconds");
    }
    for (std::size_t channel = 0; channel < channel_count; ++channel) {
        const auto end = output[channel].begin() + converted;
        for (auto iterator = output[channel].begin(); iterator != end; ++iterator) {
            if (!std::isfinite(*iterator)) {
                fail("impulse response contains a non-finite sample");
            }
            decoded.has_nonzero_sample = decoded.has_nonzero_sample || *iterator != 0.0F;
        }
        decoded.channels[channel]
            .insert(decoded.channels[channel].end(), output[channel].begin(), end);
    }
}

void convert_frame(
    SwrContext* resampler,
    const AVFrame* frame,
    std::uint16_t channel_count,
    DecodedImpulseResponse& decoded
) {
    const int capacity = swr_get_out_samples(resampler, frame->nb_samples);
    if (capacity < 0) {
        fail("cannot determine IR resampler output capacity");
    }
    std::array<std::vector<float>, 4> output;
    std::array<std::uint8_t*, 4> planes{};
    for (std::size_t channel = 0; channel < channel_count; ++channel) {
        output[channel].resize(static_cast<std::size_t>(capacity));
        planes[channel] = reinterpret_cast<std::uint8_t*>(output[channel].data());
    }
    const int converted = swr_convert(
        resampler,
        planes.data(),
        capacity,
        const_cast<const std::uint8_t**>(frame->extended_data),
        frame->nb_samples
    );
    require_av(converted, "cannot resample impulse response");
    append_resampled(decoded, output, converted, channel_count);
}

void flush_resampler(
    SwrContext* resampler,
    std::uint16_t channel_count,
    DecodedImpulseResponse& decoded
) {
    while (true) {
        const int capacity = swr_get_out_samples(resampler, 0);
        if (capacity <= 0) {
            break;
        }
        std::array<std::vector<float>, 4> output;
        std::array<std::uint8_t*, 4> planes{};
        for (std::size_t channel = 0; channel < channel_count; ++channel) {
            output[channel].resize(static_cast<std::size_t>(capacity));
            planes[channel] = reinterpret_cast<std::uint8_t*>(output[channel].data());
        }
        const int converted = swr_convert(resampler, planes.data(), capacity, nullptr, 0);
        require_av(converted, "cannot flush impulse response resampler");
        if (converted == 0) {
            break;
        }
        append_resampled(decoded, output, converted, channel_count);
    }
}

DecodedImpulseResponse
decode_impulse_response(const std::string& path, const WavContract& contract) {
    FormatContext format;
    require_av(open_audio_input(format.slot(), path), "cannot open IR");
    require_av(avformat_find_stream_info(format.get(), nullptr), "cannot read IR stream info");
    AVStream* stream = nullptr;
    for (unsigned int index = 0; index < format.get()->nb_streams; ++index) {
        if (format.get()->streams[index]->codecpar->codec_type == AVMEDIA_TYPE_AUDIO) {
            stream = format.get()->streams[index];
            break;
        }
    }
    if (stream == nullptr) {
        fail("impulse response WAV contains no audio stream");
    }
    validate_decoder_contract(*stream->codecpar, contract);

    const AVCodec* decoder = avcodec_find_decoder(stream->codecpar->codec_id);
    if (decoder == nullptr) {
        fail("no decoder for impulse response WAV");
    }
    std::unique_ptr<AVCodecContext, CodecDeleter> codec(avcodec_alloc_context3(decoder));
    if (codec == nullptr) {
        fail("cannot allocate impulse response decoder");
    }
    require_av(
        avcodec_parameters_to_context(codec.get(), stream->codecpar),
        "cannot initialize impulse response decoder"
    );
    require_av(
        avcodec_open2(codec.get(), decoder, nullptr),
        "cannot open impulse response decoder"
    );

    AVChannelLayout output_layout{};
    if (contract.channel_count == 4U) {
        require_av(
            av_channel_layout_from_mask(&output_layout, kQuadChannelMask),
            "cannot construct true-stereo transport layout"
        );
    } else {
        av_channel_layout_default(&output_layout, static_cast<int>(contract.channel_count));
    }
    SwrContext* raw_resampler = nullptr;
    const int allocation_result = swr_alloc_set_opts2(
        &raw_resampler,
        &output_layout,
        AV_SAMPLE_FMT_FLTP,
        static_cast<int>(kPreparedImpulseResponseSampleRate),
        &codec->ch_layout,
        codec->sample_fmt,
        codec->sample_rate,
        0,
        nullptr
    );
    av_channel_layout_uninit(&output_layout);
    require_av(allocation_result, "cannot allocate impulse response resampler");
    if (raw_resampler == nullptr) {
        fail("cannot allocate impulse response resampler");
    }
    std::unique_ptr<SwrContext, SwrDeleter> resampler(raw_resampler);
    require_av(av_opt_set_int(raw_resampler, "filter_size", 32, 0), "cannot set IR filter size");
    require_av(av_opt_set_int(raw_resampler, "phase_shift", 10, 0), "cannot set IR phase shift");
    require_av(av_opt_set_int(raw_resampler, "exact_rational", 1, 0), "cannot set exact ratio");
    require_av(
        av_opt_set_int(raw_resampler, "dither_method", SWR_DITHER_NONE, 0),
        "cannot disable IR dither"
    );
    require_av(
        av_opt_set_int(raw_resampler, "filter_type", SWR_FILTER_TYPE_BLACKMAN_NUTTALL, 0),
        "cannot set IR resampling filter"
    );
    require_av(swr_init(raw_resampler), "cannot initialize impulse response resampler");

    std::unique_ptr<AVPacket, PacketDeleter> packet(av_packet_alloc());
    std::unique_ptr<AVFrame, FrameDeleter> frame(av_frame_alloc());
    if (packet == nullptr || frame == nullptr) {
        fail("cannot allocate impulse response decode buffers");
    }
    DecodedImpulseResponse decoded;
    const auto receive_frames = [&]() {
        while (true) {
            const int receive_result = avcodec_receive_frame(codec.get(), frame.get());
            if (receive_result == AVERROR(EAGAIN) || receive_result == AVERROR_EOF) {
                break;
            }
            require_av(receive_result, "cannot decode impulse response frame");
            if (frame->nb_samples < 0
                || static_cast<std::uint64_t>(frame->nb_samples)
                       > std::numeric_limits<std::uint64_t>::max() - decoded.source_frame_count) {
                fail("impulse response source frame count overflow");
            }
            decoded.source_frame_count += static_cast<std::uint64_t>(frame->nb_samples);
            convert_frame(raw_resampler, frame.get(), contract.channel_count, decoded);
            av_frame_unref(frame.get());
        }
    };

    int read_result = 0;
    while ((read_result = av_read_frame(format.get(), packet.get())) >= 0) {
        if (packet->stream_index == stream->index) {
            require_av(
                avcodec_send_packet(codec.get(), packet.get()),
                "cannot submit impulse response packet"
            );
            receive_frames();
        }
        av_packet_unref(packet.get());
    }
    if (read_result != AVERROR_EOF) {
        fail("cannot read impulse response packets: " + av_error_text(read_result));
    }
    require_av(avcodec_send_packet(codec.get(), nullptr), "cannot flush impulse response decoder");
    receive_frames();
    flush_resampler(raw_resampler, contract.channel_count, decoded);
    if (decoded.source_frame_count == 0 || decoded.channels[0].empty()) {
        fail("impulse response WAV decoded no samples");
    }
    if (!decoded.has_nonzero_sample) {
        fail("impulse response WAV is digital silence");
    }
    for (std::size_t channel = 1; channel < contract.channel_count; ++channel) {
        if (decoded.channels[0].size() != decoded.channels[channel].size()) {
            fail("prepared impulse response channel lengths differ");
        }
    }
    return decoded;
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

void write_u64(std::ostream& output, std::uint64_t value) {
    write_u32(output, static_cast<std::uint32_t>(value & 0xffffffffU));
    write_u32(output, static_cast<std::uint32_t>(value >> 32U));
}

void write_float(std::ostream& output, float value) {
    write_u32(output, std::bit_cast<std::uint32_t>(value));
}

std::uint64_t write_prepared_artifact(
    const std::string& output_path,
    const WavContract& contract,
    const DecodedImpulseResponse& decoded,
    std::uint32_t preparation_version
) {
    const std::uint64_t frame_count = decoded.channels[0].size();
    const std::uint64_t sample_count = frame_count * contract.channel_count;
    const std::uint64_t data_bytes = sample_count * sizeof(float);
    const std::uint64_t total_bytes = kPreparedImpulseResponseHeaderBytes + data_bytes;
    std::ofstream output(output_path, std::ios::binary | std::ios::trunc);
    if (!output) {
        fail("cannot create prepared impulse response " + output_path);
    }
    output.write("ECHOIR01", 8);
    write_u32(output, kPreparedImpulseResponseHeaderBytes);
    write_u32(output, preparation_version);
    write_u32(output, kPreparedImpulseResponseSampleRate);
    write_u32(output, contract.channel_count);
    write_u64(output, frame_count);
    write_u32(output, contract.sample_rate);
    write_u32(output, contract.channel_count);
    write_u64(output, decoded.source_frame_count);
    write_u32(output, avcodec_version());
    write_u32(output, swresample_version());
    write_u64(output, data_bytes);
    for (std::size_t channel = 0; channel < contract.channel_count; ++channel) {
        for (const float sample : decoded.channels[channel]) {
            write_float(output, sample);
        }
    }
    output.flush();
    if (!output || static_cast<std::uint64_t>(output.tellp()) != total_bytes) {
        fail("cannot finalize prepared impulse response");
    }
    return total_bytes;
}

} // namespace

PreparedImpulseResponseResult prepare_impulse_response(
    const std::string& source_path,
    const std::string& output_path,
    ImpulseResponsePreparationLayout layout
) {
    if (source_path.empty() || output_path.empty() || source_path == output_path) {
        fail("impulse response preparation requires distinct non-empty paths");
    }
    if (layout != ImpulseResponsePreparationLayout::AutoMonoOrStereo
        && layout != ImpulseResponsePreparationLayout::TrueStereoLlLrRlRr) {
        fail("impulse response preparation layout is unsupported");
    }
    const std::uint32_t preparation_version =
        layout == ImpulseResponsePreparationLayout::TrueStereoLlLrRlRr
            ? kPreparedTrueStereoImpulseResponseVersion
            : kPreparedImpulseResponseVersion;
    const WavContract contract = inspect_wav_contract(source_path, layout);
    const DecodedImpulseResponse decoded = decode_impulse_response(source_path, contract);
    const std::uint64_t size_bytes =
        write_prepared_artifact(output_path, contract, decoded, preparation_version);
    return PreparedImpulseResponseResult{
        .preparation_version = preparation_version,
        .source_sample_rate = contract.sample_rate,
        .channel_count = contract.channel_count,
        .source_frame_count = decoded.source_frame_count,
        .prepared_frame_count = decoded.channels[0].size(),
        .avcodec_version = avcodec_version(),
        .swresample_version = swresample_version(),
        .size_bytes = size_bytes,
    };
}

} // namespace echo::audio
