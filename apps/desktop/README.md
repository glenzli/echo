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
consumed by the window chrome. `EchoWindowChrome.qml` shares the 44 px titlebar,
native safe areas, surface and drag gesture between both application shells.
`EchoWorkspaceTab.qml` shares icon navigation and the selection marker between both shells.
`IndependentTitleBar.qml` owns the project identity, active-editor history and icon document actions; native
window titles remain available to macOS menus without a second painted titlebar.
`MainTitleBar.qml` presents editor-wide draft
state and undo/redo beside the workspace navigation; the inspector does not
duplicate global history commands. Pointer movement never
persists or recompiles playback; the prepared graph is rebuilt only when the
user explicitly auditions the changed draft.

`PlaybackSessionHandoff` owns the control-thread publication and reclamation of
sessions borrowed by the audio callback. Replaced sessions survive in-flight reads,
then retire on the control thread; a temporary cleanup timer also runs after stop
or pause. Replaying does not accumulate decoder buffers until the window closes,
and the callback never allocates, releases shared ownership, or destroys a session.

Sound Assembly is a third peer workspace. `SoundAssemblyWorkspace.qml` owns
the versioned document, bounded undo/redo history, timeline commands, preview,
and mixdown presentation. `SoundAssemblyInspector.qml` owns compact clip/master parameter presentation,
reusing track colors and typography; it dispatches edits to the workspace
without owning another undo stack. `EchoValueSpinBox.qml` owns themed numeric
input, and `EchoTimeSpinBox.qml` adds the seconds-to-milliseconds projection.
`SoundAssemblyTrack.qml` owns a lane and its fixed mix controls.
`SoundAssemblyMarkers.qml` owns named-point/range navigation and precision fields;
`SoundAssemblyMarkerLane.qml` displays their composition-time positions. `M` adds a
point, `Shift+M` names the selected span, and `Alt+Left/Right` navigates points.
Annotations persist in assembly revisions and portable projects, use their own
UUIDs, and remain fixed when clips move or ripple-delete. Out-of-audio entries
remain editable but cannot start invalid playback. Named-range boundaries bind
the preview window; editing those boundaries invalidates it. Domain validation
lives in `assembly/marker.rs`; omitted/empty annotations preserve legacy JSON.
Track gain, pan, mute, solo and master gain can change during audition, including
undo/redo and paused playback. Drag samples remain temporary; release records
one history step. `SoundAssemblyController` admits only controls over the same
pinned source/topology snapshot. The producer coalesces bounded control updates
and ramps for 10 ms; buffered audio remains intact (up to 16,384 frames ahead).
`EchoStereoMeter.qml` displays stereo peaks after each track fader and before
master processing. These are producer-side readings, ahead of audible output by
the current preview buffer. Clip/source/limiter changes still invalidate preview.
`SoundAssemblyClip.qml` owns the complete
move, trim and fade gesture lifecycle; `SoundAssemblyEditing.js` owns bounded
geometry, magnetic snapping, split/crossfade transforms and source-time mapping.
`SoundAssemblySelection.js` owns atomic batch transforms for transient clip
selections, including shared movement limits and source-preserving ripple cuts.
Command/Control- or Shift-click toggles selection; dragging a selected clip moves
the selection, while Alt-drag slips only the active source inside fixed boundaries.
`AssemblyWaveformController` serializes asynchronous source waveform reads and
retains a bounded min/max pyramid for the active project's sources (at most 8192
buckets in the finest presentation level). Mix-only edits reuse that projection
without republishing it; the view chooses its level to match the visible width. The overview
follows pinned source segments and gaps; it does not claim to show rendered DSP.
Library selection creates a sequence or layered document. Every clip pins an
exact asset adjustment revision. `SoundAssemblyController` first renders each
unique pinned revision through the existing single-sound offline renderer.
`PreparedAssemblySourceCache` reuses canonical sources by source bytes, pinned
adjustment identity, impulse-response bytes and renderer version. Writes are atomic,
cancellation never admits partial outputs, and completed jobs evict unused sources
toward a 4 GiB cache budget. Sources pinned by the active preview remain available
even when they exceed that budget. The cache is private to the editing process and
removed on exit. The controller
then feeds the resulting canonical 48 kHz stereo sources to `AssemblyMixer`,
which executes fixed 4096-frame blocks for both preview and PCM24 WAV mixdown.
`PreparedPcmReader` reads Echo's canonical PCM16/PCM24 WAV blocks directly on that
producer, with no per-clip DSP pipeline or decoder thread. Other source layouts
retain the general decoder path. Reads validate the header and physical data size.
`AssemblyPlaybackSession` mixes on a producer thread into a 16384-frame ring,
and `PlaybackStream` is the device-facing transport for either a source DSP
session or an assembly. The cache pins every source referenced by the prepared
preview, including future clips, while exports may trim other cached sources.
Preview readiness no longer represents a completed mixed WAV. Catalog publication happens
only after an atomic output commit and records the assembly revision, Original
hashes, pinned adjustment revisions, and output hash.

Sources and the inspector can be collapsed independently. The ruler, grid and
playhead share the scrolling timeline's coordinates. Preview reuse is bound to
the authored document and preview range, including exact source versions; saving after an edit
cannot reuse an older preview. Space toggles playback, S splits at the playhead,
Cmd/Ctrl+D duplicates after the clip, arrows nudge 10 ms (Shift: 100 ms), F fits
the project, and +/- zoom. Shift bypasses magnetic snapping while dragging.
Right-click a clip for crossfade and same-track gap-closing deletion. Selected-clip
range preview mixes all tracks over that interval; repeat playback reuses this render.
This is bounded offline preview, not live mixer automation.

`SoundGainEnvelope.qml` owns viewport-sized point gestures; `SoundAutomation.js`
owns source-time dB interpolation and deterministic ducking geometry.
`SoundDuckingPanel.qml` detects original-waveform peak activity in bounded per-clip
steps, maps it through source cuts/gaps, and generates replacement envelopes for
one target track. It reports source activity, not speech detection or post-effect
loudness. Applying a candidate is one undo step. Gain envelopes remain editable,
bypassable, persistent, and stable under move, split and trim; the native mixer uses
the same source-time dB interpolation for preview and export. `echo-domain`'s
`assembly/envelope/remap.rs` re-anchors curves on surviving original audio when
a precision edit changes source topology; restored audio starts at unity gain.
A remapping that exceeds the keyframe limit rejects the revision transaction.

Interaction references: [Audition clip editing](https://helpx.adobe.com/ca/audition/using/arranging-editing-multitrack-clips.html),
[Audacity tracks and clips](https://manual.audacityteam.org/man/audacity_tracks_and_clips.html),
and [Ardour fades](https://manual.ardour.org/editing-and-arranging/create-region-fades-and-crossfades/).

Focused checks: `node --test apps/desktop/tests/assembly_editing_contract.mjs`
and `python3 scripts/test_assembly_ui.py` (Qt Quick Test mouse gestures). For a
native fixture workflow, prepare an external directory with
`scripts/prepare_multitrack_demo.py <directory> --operator-credential <protected-local-operator-file>`.
The tool downloads two hash-pinned CC0 sounds and requests a new descriptive
synthetic voice through Infer Runtime, without changing Echo's consumer grants.
It keeps audio, license/source attribution, request and job evidence outside Git.
On Python installations without configured CA certificates, set `SSL_CERT_FILE`
to the host's trusted CA bundle; do not disable certificate verification.
Import `Rain in the gutter.mp3` into an isolated catalog with `echo-cli import`,
then launch Echo against that catalog with `ECHO_DEBUG_MULTITRACK_ROOT=<directory>`
and `ECHO_DEBUG_MULTITRACK_REPORT=<absolute-report.json>`. The opt-in workflow
checks three source waveforms, crossfade, undo, ripple deletion, preview identity,
seek and mix provenance, and captures the actual native pages.

Set `ECHO_DEBUG_COMPLEX=1` on that workflow to additionally exercise eight
tracks and 32 overlapping clips, exact save/reopen, render cancellation and
retry, UI heartbeat timing, and all 32 source references in the accepted mix.

`SpectrogramPalette` owns both the displayed spectral colors and their energy
legend. Quiet bins are dark blue; increasing intensity passes through teal to
warm highlights. The palette has monotonically increasing measured luminance,
including in grayscale. Spectral selection uses a light edge with a dark rim
and a translucent fill so underlying energy remains visible. `Theme.qml` owns
waveform/background contrast independently in light and dark appearances.
Arrangement clips keep their track hue while adjusting waveform brightness for
each appearance. Fade curves show only authored fade regions, with a narrow
surface-colored rim to separate them from peaks. Native pixel checks cover all
eight track colors in both appearances, including Retina capture scale.

`SemanticSearchController` runs one inference request at a time and keeps only
the latest pending query. Clear, source endpoint changes and obsolete results
cannot restart discarded requests or publish stale hits. Its asynchronous
contract uses a controllable slow/failing boundary; live AI checks remain a
separate validation of Runtime authorization, actual model output and retrieval.

Native `echo-audio-workflow-contract-test` composes noise-profile learning,
spectral attenuation, segments, fades, gain, dynamics, effects and multitrack
mixing, then compares preview PCM against exported PCM24. Audio input owners
share `cpp/echo-audio/src/ffmpeg_input.*`: recognized RIFF/RF64/RIFX WAVE headers
select the WAV demuxer before FFmpeg probing, avoiding periodic PCM being
misidentified as transport streams. Only native DSP code is optimized in Debug;
debug symbols and normal floating-point semantics remain enabled.

`SoundSourceBrowser.qml` owns the editor's project, memory and material bins and
is reused for the global material page. `desktop_sound_library.cpp` exposes
collection membership, queued durable imports and memory destinations.
Editing names use `SoundSemantics.sourceTitle`: explicit user captions, embedded
source titles or filenames remain stable when AI analysis arrives. Model event
labels in the source browser are marked as AI suggestions.
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

Sound collections remain split by authority. `SoundAlbumState.qml` owns the live
projection and mutation lifecycle for durable user collections and rebuildable
suggestions; `AudioLibrarySidebar.qml` owns creation, rename, delete, and
suggestion-confirmation presentation; `SoundSelectionToolbar.qml` owns the
selected sound's membership popup. `AudioSpaceWorkspace.qml` only composes
those owners into filtering and selection.

## Independent editing

`Echo --edit [audio files…]` opens `IndependentEditor.qml` directly. The toolbar's
“Independent editing…” action starts another isolated window; the editor supports
opening audio, drag-and-drop, source selection, single-source processing, multitrack
arrangement, project save/Save As, and the existing audio exporters. `Echo --project
/path/project.echo` opens a saved project. Independent windows do not open the
normal Library catalog or start its worker pool, scans, automatic transcription, or indexes. Explicit range transcription
is available in the shared editor and saves only to the project catalog.

`independent_editor_controller.*` owns process entry, a locked recovery directory,
async source admission and portable file operations. The shared `DesktopBackend`
and editing/rendering components attach only to that directory's project store.
`editor_session.rs` owns explicit source intake and the no-background-analysis
policy. `editor_project.rs` streams project-owned sources and processing files in
bounded chunks into a versioned SQLite `.echo` file; it validates resource paths,
lengths and hashes when reopening, and atomically replaces successful saves.
Private source/cache paths are relative to the editor process working directory;
opening a moved project requires no global asset registration or original path.

Source copies and append-only adjustment/assembly revisions remain separate from
original input files. The private working store also supports recovery after an
unclean exit; available sessions appear on the editor's empty page. A normal close
asks about unsaved project changes. `--resume-editor /absolute/session-directory`
opens an existing unlocked recovery session. Save a `.echo` file for durable,
portable storage; library membership and automatic AI analysis are not part of
this workflow.

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

`SpectrogramView.qml` owns precise source-frequency selection, per-region repair
inspection, harmonic expansion and diagnostic audition. `SpectralRepairSurface.qml`
owns marquee/move/edge-resize gestures and `SpectralEditing.js` owns their bounded
source-time/Hz geometry. `SpectrogramPreviewController` serializes cancellable,
latest-wins native viewport analysis; `spectrogram_detail` performs channel-preserving
STFT analysis into uniform time buckets, with logarithmic/linear frequency projection,
2048/8192-frame resolution and display-floor control. This is source evidence, not a
rendered preview. The cached whole-file overview remains available to other consumers.

Spectral repair settings are explicitly passed to editor, library and source-browser
playback, whole-preview loudness analysis and single-file export. The catalog's library
projection restores the complete saved graph, including bypassed repair regions.
Source-band listening creates temporary diagnostic filters; it never writes an adjustment.
Spectral edits invalidate old audition identity. Source-preserving
attenuation remains distinct from future context healing and
note-level pitch correction. Interaction references: [Audacity spectral selection](https://manual.audacityteam.org/man/spectral_selection_toolbar.html)
and [Audition spectral ranges](https://helpx.adobe.com/ie/audition/desktop/editing-audio-files/selecting-audio.html).

`SpectralWorkflowSmoke.qml` exercises actual packaged playback, undo, save/readback,
reopening, source-band audition, loudness analysis and WAV export. Set
`ECHO_DEBUG_SPECTRAL_ROOT` to a fixture prepared by `scripts/prepare_spectral_demo.py`
and `ECHO_DEBUG_SPECTRAL_REPORT` to a JSON output path. The fixture has an isolated
catalog/cache, an analytically known two-tone source and an optional synthetic narration
with explicitly injected interference. `scripts/verify_spectral_demo.py` checks source
hashes, exported frequency attenuation, untouched audio outside the repair and catalog
integrity. Real mouse gestures and numerical controls are covered separately by
`tests/qml/tst_SpectralRepair.qml`.

`NoiseReductionPanel.qml` owns explicit noise capture, enable/bypass and parameter
controls within the spectral inspector. `NoiseProfileController` owns the cancellable,
bounded Original-only analysis job; `NoiseProfileProjection` and `NoiseProfileEditing.js`
preserve its complete versioned evidence across native playback and draft history.
`profiled_noise_reduction` learns a canonical power spectrum and computes shared stereo
gains in the existing spectral stream. `NoiseProfileSettings` stores the 1025 quantized
bins, source interval and processing parameters; absent profiles leave legacy JSON intact.
The temporary residue monitor never enters the saved graph or export path.
Streaming and in-place spectral processing share the same padded overlap kernel;
short sources, seek boundaries and first/last samples retain their source positions
without dividing edited Hann edges by near-zero weights.

`scripts/prepare_noise_profile_demo.py` prepares analytical noise/tone fixtures and a
noisy copy of a supplied synthetic narration. Set `ECHO_DEBUG_NOISE_ROOT` and
`ECHO_DEBUG_NOISE_REPORT` to run `NoiseWorkflowSmoke.qml` in the packaged app. It checks
stale learning results, capture without applying, native audition, saved readback,
loudness/export parity and diagnostic isolation. `scripts/verify_noise_profile_demo.py`
measures the actual WAV outputs against immutable source hashes and the clean narration.
The interaction follows the explicit sample/preview/residue workflow documented by
[Audacity](https://manual.audacityteam.org/man/noise_reduction.html) and
[Audition](https://helpx.adobe.com/audition/desktop/effects-reference/noise-reduction-restoration-effects.html).


## Explicit editor AI

`SelectionTranscriptionController` owns one background original-range request.
`SoundTranscriptPanel.qml` owns literal phrase search, checked sentences/aligned
units and review of speech-gap candidates. `EchoCheckBox.qml` keeps compact checked
selection and keyboard focus consistent with the editor's surface colors. Original audition includes 250 ms of
context for gaps; that context is excluded from edits. `SoundTranscriptEditing.js`
maps phrase hits to complete timed units and derives bounded gaps from speech
coverage, retaining edge padding and excluding uncertain zero-duration boundaries.
Gaps may contain ambience and require explicit acceptance; they are not silence detection.
Changing search, evidence, granularity, parameters or draft identity clears checks.
`SourceEditRanges.js` owns atomic source-range transforms used by
`SoundAdjustmentDraft.editSourceRanges`: merge overlaps, preserve existing
gain/fades/gaps, reject empty output or more than 128 segments, then publish once
to undo history. Keep selected hides the complement without restoring prior edits.
The core
`editor_transcription` owner creates a bounded proxy (up to five minutes), verifies
source content before and after inference, and retains the accepted Runtime model
and Job identity. Optional forced alignment supplies finer units only when its
text and timing match the transcript; otherwise sentence segments remain available. `SelectionTranscript` records never complete whole-source ASR
or enqueue library analysis. Independent projects retain this evidence in their
private portable catalog. Superseded source, draft or selection keys cannot accept
late results. The current synchronous ASR SDK exposes the Job ID only on completion:
Discard result prevents acceptance but does not promise to stop Runtime computation.

The editor source browser adds asynchronous semantic candidates after literal
matches, using a separate search presentation controller. Collection, project and
category filters remain authoritative. Similarity is labeled as a candidate, and
an unavailable Runtime leaves literal results usable. This reuses existing text and
CLAP indexes; it does not extend CLAP coverage to unindexed long recordings.
