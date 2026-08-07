//! Argument routing for the Echo operator CLI.

use clap::{Parser, Subcommand};

use super::{catalog, import, list, probe, waveform};

#[derive(Debug, Parser)]
#[command(name = "echo-cli", about = "Echo operator commands")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Creates (or opens) a catalog at the given `SQLite` path.
    Init {
        /// Catalog database path.
        catalog: std::path::PathBuf,
    },
    /// Imports one recording into the catalog (idempotent by content).
    Import {
        /// Catalog database path.
        catalog: std::path::PathBuf,
        /// Recording file to import.
        source: std::path::PathBuf,
    },
    /// Lists registered assets, newest import first.
    List {
        /// Catalog database path.
        catalog: std::path::PathBuf,
    },
    /// Probes one recording without modifying it.
    Probe {
        /// Recording file to probe.
        source: std::path::PathBuf,
    },
    /// Builds a waveform pyramid and publishes it into the cache.
    Waveform {
        /// Cache root directory.
        cache: std::path::PathBuf,
        /// Recording file to analyze.
        source: std::path::PathBuf,
    },
}

pub(crate) fn run(arguments: impl Iterator<Item = String>) -> anyhow::Result<()> {
    let cli = Cli::parse_from(std::iter::once("echo-cli".to_owned()).chain(arguments));
    match cli.command {
        Command::Init { catalog } => catalog::run_init(&catalog),
        Command::Import { catalog, source } => import::run_import(&catalog, &source),
        Command::List { catalog } => list::run_list(&catalog),
        Command::Probe { source } => probe::run_probe(&source),
        Command::Waveform { cache, source } => waveform::run_waveform(&cache, &source),
    }
}
