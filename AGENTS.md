# Echo development conventions

Read `ROADMAP.md` before changing architecture, data models, or milestone boundaries; it is the
durable design record. This file owns engineering conventions only.

## Working style

- Follow the `maintain-source-cohesion` skill: keep large owners whole when they are one cohesive
  domain; put new responsibilities in their own semantic owner from the start; update the nearest
  index when adding/renaming/extracting a responsibility.
- Do not build speculative machinery before a real consumer exists (no empty crates, no
  scheduler with no jobs).
- Commit at milestone boundaries with a coherent message; every committed state must build.

## Test topology

- A production owner ends with `#[cfg(test)] mod tests;`; its test implementations start in
  `<owner>/tests.rs`, never inline in the production file.
- Crate-level `src/tests/` is reserved for facade contracts spanning sibling modules, named
  `<responsibility>_contract.rs`.
- Crate-level `tests/` is reserved for black-box contracts consuming the public crate API.
- Keep focused tests adjacent to their semantic owner; move them with an extraction.

## Formatting

- Rust: repository `rustfmt.toml`, run `cargo xtask format`.
- C++/Objective-C++: repository `.clang-format`.
- No hand-edited formatting decisions.

## Localization

English `tr()`/`qsTr()` source text is the canonical message identity. Simplified Chinese must
remain a complete product presentation: every user-visible message needs exactly one finished,
non-empty Simplified Chinese entry (technical tokens stay language-neutral).

## Series consistency

Echo and Shadow are one series. Follow Shadow's conventions where they exist and apply: CXX
bridge boundaries, content-addressed cache, immutable original, `Cargo.toml` workspace lint
policy, `.clang-format`, build outputs outside the worktree (`.echo-local-*` siblings), and the
`Shadow*` → `Echo*` QML component naming.

## Platform

macOS (Apple Silicon) is the first-class platform. Windows must not require architecture changes:
no macOS-only APIs outside the audio engine's platform shim, no MLX assumptions below the
`InferenceBackend` routing layer.

## Local build retention

- Use `scripts/build_and_promote_debug.sh` for the reusable runnable Debug app.
- Use `scripts/build_desktop_scratch.sh` for focused temporary desktop validation; it cleans its
  managed candidate by default. Do not create persistent ad-hoc `/private/tmp/echo-*` CMake
  directories.
- `scripts/prune_local_builds.sh` is report-only unless `--apply` is explicit. It must only remove
  Echo-marked scratch builds or verified inactive Debug releases, never audio libraries, catalogs,
  caches, models, credentials, or unmarked historical directories.
