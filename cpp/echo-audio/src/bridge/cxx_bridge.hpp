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

} // namespace echo::bridge
