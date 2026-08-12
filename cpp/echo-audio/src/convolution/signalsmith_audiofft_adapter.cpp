#include "convolution/signalsmith_audiofft_adapter.hpp"

#include "signalsmith-dsp/fft.h"

#include <algorithm>
#include <stdexcept>

namespace audiofft {

AudioFFT::AudioFFT() = default;
AudioFFT::~AudioFFT() = default;

void AudioFFT::init(std::size_t size) {
    if (size == 0) {
        size_ = 0;
        fft_.reset();
        packed_.clear();
        return;
    }
    if (size < 2 || (size & (size - 1)) != 0) {
        throw std::invalid_argument("convolution FFT size must be a power of two");
    }
    size_ = size;
    fft_ = std::make_unique<RealFft>(size);
    packed_.resize(size / 2);
}

void AudioFFT::fft(const float* data, float* real, float* imaginary) {
    fft_->fft(data, packed_.data());
    const std::size_t nyquist = size_ / 2;
    real[0] = packed_[0].real();
    imaginary[0] = 0.0F;
    for (std::size_t index = 1; index < nyquist; ++index) {
        real[index] = packed_[index].real();
        imaginary[index] = packed_[index].imag();
    }
    real[nyquist] = packed_[0].imag();
    imaginary[nyquist] = 0.0F;
}

void AudioFFT::ifft(float* data, const float* real, const float* imaginary) {
    const std::size_t nyquist = size_ / 2;
    packed_[0] = {real[0], real[nyquist]};
    for (std::size_t index = 1; index < nyquist; ++index) {
        packed_[index] = {real[index], imaginary[index]};
    }
    fft_->ifft(packed_.data(), data);
    const float scale = 1.0F / static_cast<float>(size_);
    std::for_each(data, data + size_, [scale](float& sample) { sample *= scale; });
}

} // namespace audiofft
