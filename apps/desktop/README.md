# Echo desktop

The first macOS shell is a native Qt Quick application backed by the Rust memory
engine and the C++ audio engine:

```text
startup Memory Library / catalog session
  → DesktopBackend (Qt facade over the CXX ABI)
  → echo-desktop-bridge (long-lived LibrarySession)
  → echo-catalog single writer
  → recordings and accepted mix editions projected as bounded summaries
```

Selecting a recording plays it through `PlaybackController` (Qt 6.11 callback
API over the C++ engine's SPSC ring) and renders its cached waveform pyramid
(`WaveformView`). One `QAudioSink` is reused for the controller lifetime —
disposing CoreAudio units on replay crashes Qt 6.11.1 — so the engine always
drives a stereo 48 kHz float device format.

Sound editing is a peer workspace rather than a mode hidden inside the library.
Audio Space owns browsing, listening, and evidence inspection; Sound
Adjustments inherits the current selection and owns the first non-destructive
editing surface. Trim range, fades, and output gain remain a draft until
explicitly saved as an append-only catalog revision. Preview compiles those
authored units into the C++ producer path; the Qt audio callback still only
copies prepared frames. Audio Space playback consumes the saved revision and
never mutates adjustment parameters.

`SoundEditorTimeline.qml` owns the editor's high-frequency interaction state:
time projection, logarithmic zoom, horizontal navigation, direct trim and fade
handles, time selection, the clip-gain dB line, playhead, and transient gesture
readouts. Its `SourceEditTimeline.qml` overlay owns source-anchored segment
discovery and the contextual split, hide, mute, restore, gap, and mask actions;
`EffectMaskEditor.qml` keeps multi-effect selection and soft mask edges with the
authored serial chain rather than presenting a branch graph.
`SoundAdjustmentDraft.qml` owns validation, gesture-coalesced
undo/redo history, saved-state comparison, and explicit publication.
`SoundAdjustmentEditor.qml` composes the bottom precision console. Its
`SoundSignalChain.qml` keeps clip/preamp and master output pinned around an
independently scrolling insert list, while `AdvancedEffectsRack.qml` owns the
resizable horizontal split between that navigation and the selected node's
parameter surface. The workspace owns the outer vertical split with the
timeline, so both ratios can be resized without changing draft semantics.
Meanwhile,
`SoundEditingWorkspace.qml` owns source, transport, selection looping,
adjusted/original audition, backend lifecycle, and the small command projection
consumed by the window chrome. `MainTitleBar.qml` presents editor-wide draft
state and undo/redo beside the workspace navigation; the inspector does not
duplicate global history commands. Pointer movement never
persists or recompiles playback; the prepared graph is rebuilt only when the
user explicitly auditions the changed draft.

Sound Assembly is a third peer workspace. `SoundAssemblyWorkspace.qml` owns
the versioned document, bounded undo/redo history, timeline commands, clip and
master inspectors, preview, and mixdown presentation; `SoundAssemblyTrack.qml`
owns one track's mix controls and direct clip placement, trim, and selection.
Library selection creates a sequence or layered document. Every clip pins an
exact asset adjustment revision. `SoundAssemblyController` first renders each
unique pinned revision through the existing single-sound offline renderer,
then feeds the resulting canonical 48 kHz stereo sources to one shared native
assembly plan for preview and PCM24 WAV mixdown. Catalog publication happens
only after an atomic output commit and records the assembly revision, Original
hashes, pinned adjustment revisions, and output hash.

`SoundSourceBrowser.qml` owns the editor's project, memory and material bins and
is reused for the global material page. `desktop_sound_library.cpp` exposes
collection membership, queued durable imports and memory destinations.
`SoundPlaybackSource.qml` maps saved processing into either the main transport
or the independent material audition transport. `MemorySourceReferences.qml`
shows the exact source snapshot of an accepted listening edition.

“Keep in memories” renders to `media/memories` beside the catalog, commits and
hashes the output, then pins that export as the memory's listening edition.
Saving a project alone leaves the accepted edition unchanged. Imported materials
live in `media/materials`; neither directory is a rebuildable cache. Global
collection removal preserves project references and bytes.

Opening a clip in the existing precision editor carries its exact source
revision and project context. Saving writes an isolated adjustment revision and
an updated project revision together. It leaves the recording's current saved
adjustment unchanged. Returning to the project preserves the selected clip and
playhead. The post-render spectral working-copy route stays in the standalone
sound editor until it has an independent project-scoped lifecycle.

QML never opens SQLite, calls FFmpeg, or interprets cache paths. Theme tokens
live in [`qml/Theme.qml`](qml/Theme.qml); components follow the series naming
convention (`EchoButton`, ...) shared with Shadow.

Sound albums remain split by authority. `SoundAlbumState.qml` owns the live
projection and mutation lifecycle for durable user albums and rebuildable
suggestions; `AudioLibrarySidebar.qml` owns creation, rename, delete, and
suggestion-confirmation presentation; `SoundSelectionToolbar.qml` owns the
selected sound's membership popup. `AudioSpaceWorkspace.qml` only composes
those owners into filtering and selection.

## Build and run

```sh
cmake --preset desktop-dev
cmake --build --preset desktop-dev
../.echo-local-build/desktop-dev/apps/desktop/Echo.app/Contents/MacOS/Echo ./catalogs/demo.sqlite ./cache
```

The CMake graph builds `echo-desktop-bridge` with Cargo into its own target
directory and syncs the generated CXX headers into a stable include root
([`cmake/sync_cxxbridge_headers.cmake`](cmake/sync_cxxbridge_headers.cmake)).
Headless smoke: `ECHO_DEBUG_SCREENSHOT=/tmp/echo.png ../.echo-local-build/desktop-dev/apps/desktop/Echo.app/Contents/MacOS/Echo <catalog> <cache>`

Set `ECHO_DEBUG_OPEN_EDITOR=1` with `ECHO_DEBUG_SCREENSHOT` to capture the
selected sound in the adjustment workspace after the catalog has loaded.
Set `ECHO_DEBUG_OPEN_ASSEMBLY=1` to capture the assembly workspace, or
`ECHO_DEBUG_CREATE_ASSEMBLY=1` to create a single-clip sequence from the
initially selected Library sound before capture.
Add `ECHO_DEBUG_ASSEMBLY_EXPORT=/path/mix.wav` to exercise exact-revision
preparation, assembly mixdown, atomic publication, and Catalog provenance in
one isolated smoke run.
Set `ECHO_DEBUG_REPLAY_EDITOR=1` to start, stop, and restart that adjusted
sound before capture; the transport timestamp proves the replacement session
is consumed by the packaged audio sink.
captures the first window and exits; add `ECHO_DEBUG_AUTOPLAY=/path/file` to
start playback first.

`EchoComboBox.qml` shares themed selectors across source browsing and project inspection; `Main.qml` supplies the matching palette to inherited Qt controls.

`MemoryWorkflowSmoke.qml` is an opt-in packaged integration contract. With an isolated
catalog containing a recording, set `ECHO_DEBUG_MEMORY_MATERIAL=file:///absolute/material.wav`
and `ECHO_DEBUG_MEMORY_REPORT=/absolute/report.json`. It exercises project material intake,
isolated clip precision editing, accepted mix publication, waveform access, draft retention
and project reopening, then exits with a JSON report and numbered window captures. It invokes
the same workspace handlers as the UI; pointer gestures and native file dialogs remain manual QA.
