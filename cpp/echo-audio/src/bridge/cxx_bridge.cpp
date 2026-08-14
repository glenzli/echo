#include "cxx_bridge.hpp"

#include <echo/audio/analysis_proxy.hpp>
#include <echo/audio/decode.hpp>
#include <echo/audio/impulse_response_preparer.hpp>
#include <echo/audio/waveform.hpp>

#include <stdexcept>
#include <string>

namespace echo::bridge {

// The default rust::behavior::trycatch converts any std::exception into the
// Rust error message, so engine failures propagate unchanged.

FfiAudioProbe probe_audio(rust::Str path) {
    const audio::AudioProbe result = audio::probe(std::string(path));
    FfiAudioProbe wire;
    wire.has_audio = result.has_audio;
    wire.codec_name = result.codec_name;
    wire.container_format = result.container_format;
    wire.sample_rate = result.sample_rate;
    wire.channel_count = result.channel_count;
    wire.duration_millis = result.duration_millis;
    wire.recorded_at_millis = result.recorded_at_millis;
    wire.metadata.reserve(result.metadata.size());
    for (const audio::AudioMetadataEntry& entry : result.metadata) {
        FfiAudioMetadataEntry wire_entry;
        wire_entry.key = entry.key;
        wire_entry.value = entry.value;
        wire.metadata.push_back(wire_entry);
    }
    return wire;
}

FfiWaveform build_waveform_bridge(rust::Str path, uint32_t max_levels) {
    const audio::Waveform result = audio::build_waveform(std::string(path), max_levels);
    FfiWaveform wire;
    wire.canonical_sample_rate = result.canonical_sample_rate;
    wire.levels.reserve(result.levels.size());
    for (const audio::WaveformLevel& level : result.levels) {
        FfiWaveformLevel wire_level;
        wire_level.mins.reserve(level.mins.size());
        for (const float sample : level.mins) {
            wire_level.mins.push_back(sample);
        }
        wire_level.maxs.reserve(level.maxs.size());
        for (const float sample : level.maxs) {
            wire_level.maxs.push_back(sample);
        }
        wire_level.samples_per_bucket = level.samples_per_bucket;
        wire.levels.push_back(wire_level);
    }
    return wire;
}

FfiAnalysisProxy build_analysis_proxy_bridge(
    rust::Str source_path,
    rust::Str output_path,
    uint64_t start_millis,
    uint64_t end_millis
) {
    const audio::AnalysisProxyResult result = audio::build_analysis_proxy(
        std::string(source_path),
        std::string(output_path),
        start_millis,
        end_millis
    );
    return FfiAnalysisProxy{
        .sample_rate = result.sample_rate,
        .channel_count = result.channel_count,
        .frame_count = result.frame_count,
        .size_bytes = result.size_bytes,
    };
}

FfiPreparedImpulseResponse
prepare_impulse_response_bridge(rust::Str source_path, rust::Str output_path, uint8_t layout) {
    audio::ImpulseResponsePreparationLayout preparation_layout;
    switch (layout) {
    case 0:
        preparation_layout = audio::ImpulseResponsePreparationLayout::AutoMonoOrStereo;
        break;
    case 1:
        preparation_layout = audio::ImpulseResponsePreparationLayout::TrueStereoLlLrRlRr;
        break;
    default:
        throw std::invalid_argument("impulse response preparation layout is unsupported");
    }
    const audio::PreparedImpulseResponseResult result = audio::prepare_impulse_response(
        std::string(source_path),
        std::string(output_path),
        preparation_layout
    );
    return FfiPreparedImpulseResponse{
        .preparation_version = result.preparation_version,
        .source_sample_rate = result.source_sample_rate,
        .channel_count = result.channel_count,
        .source_frame_count = result.source_frame_count,
        .prepared_frame_count = result.prepared_frame_count,
        .avcodec_version = result.avcodec_version,
        .swresample_version = result.swresample_version,
        .size_bytes = result.size_bytes,
    };
}

} // namespace echo::bridge
