//! Echo's operator CLI composition boundary.
//!
//! [`commands`] owns argument routing; each child module owns one durable
//! command family, including saved independent-editor inspection in [`edit`].

mod catalog;
mod commands;
mod credentials;
mod edit;
mod import;
mod library;
mod list;
mod models;
mod probe;
mod search;
mod semantic_search;
mod transcribe;
mod waveform;

fn main() -> anyhow::Result<()> {
    commands::run(std::env::args().skip(1))
}
