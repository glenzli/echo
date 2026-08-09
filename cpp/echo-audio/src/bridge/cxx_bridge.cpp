#include "cxx_bridge.hpp"

#include <echo/audio/decode.hpp>
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

} // namespace echo::bridge
