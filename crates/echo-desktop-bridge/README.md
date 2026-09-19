# echo-desktop-bridge

`echo-desktop-bridge` owns the long-lived services that Qt controllers consume
over the generated CXX ABI.

Start in [`src/lib.rs`](src/lib.rs) for the centralized wire declaration and
the `extern "Rust"` surface; [`src/session.rs`](src/session.rs) owns the
`LibrarySession` lifecycle (one catalog attachment per process). QML never
opens SQLite; every Library read flows through this crate.

`editor_session.rs` owns explicit audio intake into an isolated editing session.
It reuses the editing services while disabling scan roots and background Library
jobs. `editor_project.rs` owns portable `.echo` snapshots: project-owned resources
are streamed into SQLite, verified on reopen, and restored under relative paths.
Neither owner attaches the user's Library catalog.

User-authored album lists and mutations cross the ABI as one explicit contract;
the Catalog keeps them separate from rebuildable smart-album candidates.

Sound Assembly uses the same boundary without leaking QML into SQLite. The
session creates Library-driven sequence or layered documents, validates and
appends immutable revisions, resolves every clip's exact Original and
adjustment revision for native preparation, rejects destinations that would
replace an Original, and records a verified mixdown plus source provenance
only after the output exists.

`session/sound_library.rs` owns collection membership, global/project material
intake, durable memory destinations, and explicit acceptance of completed mix
editions. `session/sound_assembly.rs` also owns project-scoped precision saves:
a clip processing revision and its parent project revision commit atomically,
without advancing the original recording's listening adjustment. Summary wires
identify recordings and mix memories explicitly; originals remain assets, while
accepted mix waveforms are indexed by export identity.
