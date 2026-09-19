#pragma once
#include <array>
#include <cstdint>

namespace SpectrogramPalette {
using Color = std::array<std::uint8_t, 4>;
// Ordered luminance: quiet navy, teal detail, warm high energy. Identical in
// light and dark appearance, so a given dB value always means the same color.
inline Color color(std::uint8_t magnitude) {
    constexpr std::array<Color, 8> stops{
        {{11, 20, 32, 255},
         {20, 43, 67, 255},
         {34, 83, 109, 255},
         {50, 125, 134, 255},
         {103, 165, 141, 255},
         {180, 195, 122, 255},
         {239, 209, 116, 255},
         {255, 243, 176, 255}}
    };
    const unsigned scaled = static_cast<unsigned>(magnitude) * 7;
    const unsigned first = scaled / 255, fraction = scaled % 255;
    if (first == 7)
        return stops.back();
    Color result{};
    for (unsigned channel = 0; channel < 4; ++channel)
        result[channel] = static_cast<std::uint8_t>(
            (stops[first][channel] * (255 - fraction) + stops[first + 1][channel] * fraction + 127)
            / 255
        );
    return result;
}
} // namespace SpectrogramPalette
