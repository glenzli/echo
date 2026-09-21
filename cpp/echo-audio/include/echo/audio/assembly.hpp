#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>

namespace echo::audio {

enum class AssemblyFadeCurve : std::uint8_t { Linear = 0, Smooth = 1, EqualPower = 2 };

struct AssemblyGainPoint {
    std::uint64_t source_millis = 0;
    std::int16_t gain_centibels = 0;
};

/// One clip over a prepared, canonical 48 kHz stereo linear asset revision.
struct AssemblyClipSource {
    std::string path;
    std::uint64_t source_start_millis = 0;
    std::uint64_t source_end_millis = 0;
    std::uint64_t timeline_start_millis = 0;
    std::int16_t gain_centibels = 0;
    std::int16_t pan_percent = 0;
    std::uint64_t fade_in_millis = 0;
    std::uint64_t fade_out_millis = 0;
    AssemblyFadeCurve fade_in_curve = AssemblyFadeCurve::Linear;
    AssemblyFadeCurve fade_out_curve = AssemblyFadeCurve::Linear;
    bool muted = false;
    bool gain_envelope_enabled = false;
    std::vector<AssemblyGainPoint> gain_envelope;
};

struct AssemblyTrackMix {
    std::int16_t gain_centibels = 0;
    std::int16_t pan_percent = 0;
    bool muted = false;
    bool solo = false;
    std::vector<AssemblyClipSource> clips;
};

/// Complete deterministic mix plan shared by preview preparation and export.
struct AssemblyMixPlan {
    std::vector<AssemblyTrackMix> tracks;
    // Ephemeral preview window; zero end renders the full project.
    std::uint64_t render_start_millis = 0;
    std::uint64_t render_end_millis = 0;
    std::int16_t master_gain_centibels = 0;
    bool limiter_enabled = true;
    std::int16_t limiter_ceiling_centibels = -100;
    std::uint16_t limiter_release_millis = 100;
};

// Bounded control-only snapshot; no source, timeline or DSP topology changes.
struct AssemblyTrackControls {
    std::int16_t gain_centibels = 0;
    std::int16_t pan_percent = 0;
    bool muted = false;
    bool solo = false;
};
struct AssemblyMixControls {
    std::size_t track_count = 0;
    std::array<AssemblyTrackControls, 8> tracks{};
    std::int16_t master_gain_centibels = 0;
};
struct AssemblyTrackPeak {
    float left_dbfs = -70;
    float right_dbfs = -70;
};
using AssemblyTrackPeaks = std::array<AssemblyTrackPeak, 8>;
AssemblyMixControls assembly_mix_controls(const AssemblyMixPlan& plan);
bool valid_assembly_mix_controls(const AssemblyMixControls& controls, std::size_t track_count);

} // namespace echo::audio
