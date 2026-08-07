# Echo

Echo is a local-first, source-anchored sound memory system. It preserves, understands, indexes,
restores, and re-listens to real recorded sound — it does not synthesize new sound works.

Echo is the audio sibling of [Shadow](../shadow), sharing its visual language, memory-engine
architecture, and cross-language boundaries:

> Shadow = Photo Library + RAW Developer + AI Understanding
> Echo   = Audio Library + Audio Restoration + AI Understanding

Read [`ROADMAP.md`](ROADMAP.md) for the product design, data model, progressive analysis levels,
AI model selection, and milestone plan.

## Repository map

| Area | Stable entry | Responsibility |
| --- | --- | --- |
| Domain contracts | [`echo-domain`](crates/echo-domain/src/lib.rs) | Asset identity, immutable Original, analysis contracts, progressive levels |
| Catalog | [`echo-catalog`](crates/echo-catalog/src/lib.rs) | SQLite ownership, asset registration, analysis records |
| Cache | [`echo-cache`](crates/echo-cache/src/lib.rs) | Content-addressed rebuildable blobs (waveform, embeddings, renders) |
| Core workflows | [`echo-core`](crates/echo-core/src/lib.rs) | Import, scanning, background job scheduling |
| AI contracts | [`echo-ai`](crates/echo-ai/src/lib.rs) | Capability routing, inference backend identities |
| Audio bridge | [`echo-bridge`](crates/echo-bridge/README.md) | Safe Rust API over the C++ audio engine |
| Native audio engine | [`cpp/echo-audio`](cpp/echo-audio/CMakeLists.txt) | FFmpeg decode, canonical PCM, waveform pyramid |
| Desktop services | `crates/echo-desktop-bridge` | Long-lived Qt-facing services |
| Qt application | [`apps/desktop`](apps/desktop/README.md) | QML presentation, Qt controllers, Audio Space |
| CLI and validation | [`echo-cli`](apps/echo-cli/src/main.rs), [`xtask`](xtask/src/main.rs) | Operator commands and repository-level checks |

## Developer commands

```sh
cargo xtask check          # fmt + clippy + tests
cargo xtask format         # apply rustfmt and clang-format
cargo xtask doctor         # verify toolchain prerequisites
cargo run --package echo-cli -- init ./catalogs/demo.sqlite
cargo run --package echo-cli -- import ./catalogs/demo.sqlite /path/to/recording.m4a
cargo run --package echo-cli -- probe /path/to/recording.m4a
cargo run --package echo-cli -- waveform ./cache /path/to/recording.m4a
cargo run --package echo-cli -- list ./catalogs/demo.sqlite
```

Native engine checks (independent CMake graph):

```sh
cmake --preset native-dev && cmake --build --preset native-dev && ctest --preset native-dev
```

## License

MIT. See [`LICENSE`](LICENSE).
