#pragma once

#include "rust/cxx.h"

#include <cstdint>
#include <string>

#include "echo-bridge/src/lib.rs.h"

/// Coarse CXX boundary between Rust application state and the C++ audio
/// engine. Exception-converting shims only; real behavior lives in the
/// engine headers.
namespace echo::bridge {

[[nodiscard]] FfiAudioProbe probe_audio(rust::Str path);
[[nodiscard]] FfiWaveform build_waveform_bridge(rust::Str path, uint32_t max_levels);
[[nodiscard]] FfiAnalysisProxy build_analysis_proxy_bridge(
    rust::Str source_path,
    rust::Str output_path,
    uint64_t start_millis,
    uint64_t end_millis
);
[[nodiscard]] FfiPreparedImpulseResponse
prepare_impulse_response_bridge(rust::Str source_path, rust::Str output_path);

} // namespace echo::bridge
