#pragma once

#include <string>

#include "echo/audio/adjustment.hpp"
#include "echo/audio/offline_render.hpp"

namespace echo::audio {

/// Streams the authored adjustment graph into 48 kHz stereo, 24-bit FLAC.
class OfflineFlacRenderer {
  public:
    [[nodiscard]] static OfflineRenderResult render(
        const std::string& sourcePath,
        const PlaybackAdjustment& adjustment,
        RenderByteSink& sink,
        const OfflineRenderCallbacks& callbacks = {}
    );
};

} // namespace echo::audio
