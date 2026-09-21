#pragma once
#include <string_view>

#include <string>

#include "echo/audio/adjustment.hpp"
#include "echo/audio/offline_render.hpp"

namespace echo::audio {

enum class WavPcmDepth : std::uint8_t { Pcm16 = 16, Pcm24 = 24 };

/// Streams the authored adjustment graph into a canonical 48 kHz stereo,
/// 24-bit PCM WAV. Memory use is constant in source duration.
class OfflineWavRenderer {
  public:
    [[nodiscard]] static OfflineRenderResult render(
        const std::string& sourcePath,
        const PlaybackAdjustment& adjustment,
        RenderByteSink& sink,
        const OfflineRenderCallbacks& callbacks = {},
        WavPcmDepth depth = WavPcmDepth::Pcm24,
        std::string_view comment = {}
    );
};

} // namespace echo::audio
