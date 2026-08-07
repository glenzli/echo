# echo-bridge

`echo-bridge` is the coarse-grained CXX boundary between Rust application
state and the C++20 audio engine (`cpp/echo-audio`).

## Source index

The generated CXX wire declaration stays centralized and auditable in
[`src/lib.rs`](src/lib.rs). Safe callers use the responsibility-named entry
points:

- [`src/waveform.rs`](src/waveform.rs) owns waveform pyramid extraction over
  the canonical mono mixdown;
- `probe` (in the facade) owns container/stream metadata inspection.

On the native side, the public shim lives in
[`cxx_bridge.hpp`](../../cpp/echo-audio/src/bridge/cxx_bridge.hpp) and its
composition shim; the real engine behavior lives in the
[`decode.hpp`](../../cpp/echo-audio/include/echo/audio/decode.hpp) and
[`waveform.hpp`](../../cpp/echo-audio/include/echo/audio/waveform.hpp)
headers.

FFmpeg 8.x public headers carry no `extern "C"` guards; engine sources include
them only through
[`ffmpeg_include.hpp`](../../cpp/echo-audio/include/echo/audio/ffmpeg_include.hpp).
