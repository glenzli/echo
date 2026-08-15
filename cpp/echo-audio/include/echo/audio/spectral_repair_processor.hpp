#pragma once

#include <cstddef>
#include <cstdint>
#include <vector>

namespace echo::audio {

/// Source-anchored soft rectangular attenuation intent for the deterministic
/// STFT repair kernel. Values use the same integer units as the domain model.
struct SpectralAttenuationRegion {
    std::uint64_t start_millis = 0;
    std::uint64_t end_millis = 0;
    std::uint16_t low_hertz = 20;
    std::uint16_t high_hertz = 24'000;
    std::int16_t attenuation_centibels = 0;
    std::uint16_t time_feather_millis = 0;
    std::uint16_t frequency_feather_hertz = 0;
};

/// Stateless offline STFT attenuation kernel. It modifies interleaved PCM in
/// place and is intentionally separate from display-tile generation.
class SpectralRepairProcessor {
  public:
    static constexpr std::size_t kWindowFrames = 2'048;
    static constexpr std::size_t kHopFrames = 512;

    /// Validates bounded source-time attenuation regions for one canonical
    /// source duration. Throws `std::invalid_argument` when invalid.
    static void validate_regions(
        const std::vector<SpectralAttenuationRegion>& regions,
        std::uint64_t source_duration_millis,
        std::uint32_t sample_rate
    );

    /// Applies the selected attenuations to interleaved PCM. The first frame
    /// names its Original source position, so identical regions have identical
    /// semantics in preview and offline render.
    static void process_interleaved(
        float* samples,
        std::size_t frame_count,
        std::size_t channel_count,
        std::uint32_t sample_rate,
        std::uint64_t first_source_frame,
        const std::vector<SpectralAttenuationRegion>& regions
    );
};

/// Stateful producer-thread companion for `SpectralRepairProcessor`. It keeps
/// the 75%-overlapped STFT analysis history across decoder blocks and tags
/// every drained frame with its Original source position. It is deliberately
/// not an audio-callback type: callers drain its prepared PCM into their
/// existing realtime-safe ring.
class SpectralRepairStream {
  public:
    SpectralRepairStream(
        std::uint32_t sample_rate,
        std::size_t channel_count,
        std::vector<SpectralAttenuationRegion> regions
    );

    /// Drops analysis and overlap-add history, as required on a source seek.
    void reset();

    /// Appends contiguous source PCM. `first_source_frame` is checked so an
    /// accidental discontinuity cannot silently move an authored repair.
    void push_interleaved(
        const float* samples,
        std::size_t frame_count,
        std::uint64_t first_source_frame
    );

    /// Zero-pads the final analysis windows and makes every received source
    /// frame available to `drain_interleaved` without extending duration.
    void finish();

    /// Drains up to `capacity_frames` repaired frames. Returns zero when no
    /// prepared frame remains. `source_frames` may be null when provenance is
    /// not needed by the caller.
    std::size_t
    drain_interleaved(float* output, std::uint64_t* source_frames, std::size_t capacity_frames);

  private:
    void process_window();
    void emit_hop();
    void append_zero_frame();

    std::uint32_t sample_rate_ = 0;
    std::size_t channel_count_ = 0;
    std::vector<SpectralAttenuationRegion> regions_;
    std::vector<float> window_;
    std::vector<float> input_;
    std::size_t input_frames_ = 0;
    std::vector<float> overlap_add_;
    std::vector<float> normalization_;
    std::vector<float> ready_;
    std::size_t ready_offset_frames_ = 0;
    std::uint64_t ready_first_source_frame_ = 0;
    std::uint64_t next_input_source_frame_ = 0;
    std::uint64_t next_output_source_frame_ = 0;
    std::uint64_t received_end_source_frame_ = 0;
    bool started_ = false;
    bool finished_ = false;
};

} // namespace echo::audio
