#include "echo/audio/offline_render.hpp"

namespace echo::audio {

OfflineRenderCancelled::OfflineRenderCancelled() : std::runtime_error("offline render cancelled") {}

} // namespace echo::audio
