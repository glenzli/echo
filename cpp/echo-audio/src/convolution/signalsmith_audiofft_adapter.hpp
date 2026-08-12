#pragma once

#include <complex>
#include <cstddef>
#include <memory>
#include <vector>

namespace signalsmith::fft {
template <typename V, int option_flags> class RealFFT;
}

namespace audiofft {

/// MIT-only adapter between FFTConvolver's split-complex API and Signalsmith's
/// packed real FFT. `init()` is a preparation operation; `fft()` and `ifft()`
/// use only storage allocated by `init()`.
class AudioFFT {
  public:
    AudioFFT();
    ~AudioFFT();

    AudioFFT(const AudioFFT&) = delete;
    AudioFFT& operator=(const AudioFFT&) = delete;

    void init(std::size_t size);
    void fft(const float* data, float* real, float* imaginary);
    void ifft(float* data, const float* real, const float* imaginary);

    [[nodiscard]] static constexpr std::size_t ComplexSize(std::size_t size) noexcept {
        return size / 2 + 1;
    }

  private:
    using RealFft = signalsmith::fft::RealFFT<float, 0>;

    std::size_t size_ = 0;
    std::unique_ptr<RealFft> fft_;
    std::vector<std::complex<float>> packed_;
};

} // namespace audiofft
