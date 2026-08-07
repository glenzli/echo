# Echo desktop

The first macOS shell is a native Qt Quick application backed by the Rust memory
engine and the C++ audio engine:

```text
startup Audio Space / catalog session
  → DesktopBackend (Qt facade over the CXX ABI)
  → echo-desktop-bridge (long-lived LibrarySession)
  → echo-catalog single writer
  → registered assets projected as bounded summaries
```

Selecting a recording plays it through `PlaybackController` (Qt 6.11 callback
API over the C++ engine's SPSC ring) and renders its cached waveform pyramid
(`WaveformView`). One `QAudioSink` is reused for the controller lifetime —
disposing CoreAudio units on replay crashes Qt 6.11.1 — so the engine always
drives a stereo 48 kHz float device format.

QML never opens SQLite, calls FFmpeg, or interprets cache paths. Theme tokens
live in [`qml/Theme.qml`](qml/Theme.qml); components follow the series naming
convention (`EchoButton`, ...) shared with Shadow.

## Build and run

```sh
cmake --preset desktop-dev
cmake --build --preset desktop-dev
./.echo-local-build/desktop-dev/apps/desktop/echo-desktop ./catalogs/demo.sqlite ./cache
```

The CMake graph builds `echo-desktop-bridge` with Cargo into its own target
directory and syncs the generated CXX headers into a stable include root
([`cmake/sync_cxxbridge_headers.cmake`](cmake/sync_cxxbridge_headers.cmake)).
Headless smoke: `ECHO_DEBUG_SCREENSHOT=/tmp/echo.png ./echo-desktop <catalog> <cache>`
captures the first window and exits; add `ECHO_DEBUG_AUTOPLAY=/path/file` to
start playback first.
