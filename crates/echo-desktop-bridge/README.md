# echo-desktop-bridge

`echo-desktop-bridge` owns the long-lived services that Qt controllers consume
over the generated CXX ABI.

Start in [`src/lib.rs`](src/lib.rs) for the centralized wire declaration and
the `extern "Rust"` surface; [`src/session.rs`](src/session.rs) owns the
`LibrarySession` lifecycle (one catalog attachment per process). QML never
opens SQLite; every Library read flows through this crate.

User-authored album lists and mutations cross the ABI as one explicit contract;
the Catalog keeps them separate from rebuildable smart-album candidates.

Sound Assembly uses the same boundary without leaking QML into SQLite. The
session creates Library-driven sequence or layered documents, validates and
appends immutable revisions, resolves every clip's exact Original and
adjustment revision for native preparation, rejects destinations that would
replace an Original, and records a verified mixdown plus source provenance
only after the output exists.
