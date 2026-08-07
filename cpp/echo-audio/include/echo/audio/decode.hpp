#pragma once

#include <cstdint>
#include <string>

namespace echo::audio {

/// Container/stream metadata for one source. A source without any audio
/// stream reports `has_audio == false` and zeroed fields; the caller keeps
/// the asset registered but cannot play or analyze it.
struct AudioProbe {
    bool has_audio = false;
    std::string codec_name;
    std::string container_format;
    uint32_t sample_rate = 0;
    uint32_t channel_count = 0;
    uint64_t duration_millis = 0;
};

/// Probes a source file without decoding audio samples.
///
/// @throws std::runtime_error when the file cannot be opened or its stream
///         information cannot be read.
AudioProbe probe(const std::string& path);

} // namespace echo::audio
