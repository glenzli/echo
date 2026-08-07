//! Echo's operator CLI composition boundary.
//!
//! [`commands`] owns argument routing; each child module owns one durable
//! command family: Catalog initialization, asset import, and Library listing.

mod catalog;
mod commands;
mod import;
mod list;
mod probe;
mod waveform;

fn main() -> anyhow::Result<()> {
    commands::run(std::env::args().skip(1))
}
