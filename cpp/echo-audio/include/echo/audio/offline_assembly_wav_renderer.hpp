#pragma once

#include "echo/audio/assembly.hpp"
#include "echo/audio/offline_render.hpp"

namespace echo::audio {

/// Mixes prepared linear asset revisions into a canonical 48 kHz stereo,
/// 24-bit PCM WAV. The same plan is used for private preview preparation and
/// durable publication, so placement, fades, pan, gain, solo, and mute do not
/// fork across those paths.
class OfflineAssemblyWavRenderer {
  public:
    [[nodiscard]] static OfflineRenderResult render(
        const AssemblyMixPlan& plan,
        RenderByteSink& sink,
        const OfflineRenderCallbacks& callbacks = {}
    );
};

} // namespace echo::audio
