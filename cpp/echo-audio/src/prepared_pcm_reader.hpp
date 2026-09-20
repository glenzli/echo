#pragma once

#include <array>
#include <cstdint>
#include <fstream>
#include <memory>
#include <span>
#include <string>

namespace echo::audio {
// Direct, bounded reads of the exact PCM WAV layout emitted by Echo's renderer.
// Other layouts return nullptr and retain the general decoder path.
class PreparedPcmReader {
  public:
    static std::unique_ptr<PreparedPcmReader> open(const std::string& path);
    void seek(std::uint64_t frame);
    std::size_t read(std::span<float> stereo);

  private:
    PreparedPcmReader(std::ifstream input, std::uint32_t bytes, unsigned sample_bytes);
    std::ifstream input_;
    std::uint64_t frames_;
    std::uint64_t cursor_ = 0;
    unsigned sample_bytes_;
    std::array<unsigned char, 4096 * 2 * 3> bytes_{};
};
} // namespace echo::audio
