//! Argument routing for the Echo operator CLI.

use clap::{Parser, Subcommand};

use super::{catalog, import, list, models, probe, transcribe, waveform};

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
    /// Reports cataloged model presence in the model root.
    Models {
        /// Model root (HF hub cache layout); defaults to the standard cache.
        #[arg(long)]
        root: Option<std::path::PathBuf>,
    },
    /// Transcribes a recording with the local ASR model and records evidence.
    Transcribe {
        /// Catalog database path.
        catalog: std::path::PathBuf,
        /// Recording file to transcribe.
        source: std::path::PathBuf,
        /// Model root; defaults to the standard HF cache.
        #[arg(long)]
        model_root: Option<std::path::PathBuf>,
        /// MLX interpreter (default: `ECHO_MLX_PYTHON` or `python3`).
        #[arg(long)]
        python: Option<std::path::PathBuf>,
        /// ASR worker script (default: `tools/asr/transcribe.py`).
        #[arg(long)]
        worker: Option<std::path::PathBuf>,
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
        Command::Models { root } => {
            let root = root.unwrap_or_else(models::default_model_root);
            models::run_models(&root)
        }
        Command::Transcribe {
            catalog,
            source,
            model_root,
            python,
            worker,
        } => {
            let model_root = model_root.unwrap_or_else(models::default_model_root);
            let python = python.unwrap_or_else(transcribe::default_python);
            let worker =
                worker.unwrap_or_else(|| std::path::PathBuf::from("tools/asr/transcribe.py"));
            transcribe::run_transcribe(&catalog, &source, &model_root, &python, &worker)
        }
    }
}
