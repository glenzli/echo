#pragma once
#include "echo/audio/adjustment.hpp"
#include "echo/audio/assembly.hpp"
#include "echo/audio/offline_render.hpp"
#include <string>
#include <string_view>

namespace echo::audio {
// Delivery conversion follows the 48 kHz authored DSP graph. It does not change originals.
struct AudioExportProfile {
    std::string format = "wav_pcm24";
    std::uint32_t sample_rate = 48000;
    std::uint32_t channels = 2;
    std::uint32_t bitrate_kbps = 192;
    // Filled only by an explicit delivery opt-in, never by preview or source tags.
    std::string memory_notes;
    std::string memory_place;
    std::string memory_time;
    void validate() const;
    std::string extension() const;
    std::uint16_t bit_depth() const;
};
class AudioExporter {
  public:
    static OfflineRenderResult render(
        const std::string& path,
        const PlaybackAdjustment& adjustment,
        RenderByteSink& sink,
        const AudioExportProfile& profile,
        const OfflineRenderCallbacks& callbacks = {},
        std::string_view comment = {}
    );
    static OfflineRenderResult render_assembly(
        const AssemblyMixPlan& plan,
        RenderByteSink& sink,
        const AudioExportProfile& profile,
        const OfflineRenderCallbacks& callbacks = {},
        std::string_view comment = {}
    );
};
} // namespace echo::audio
