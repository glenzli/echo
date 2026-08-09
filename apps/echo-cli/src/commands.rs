//! Argument routing for the Echo operator CLI.

use clap::{Parser, Subcommand};

use super::{catalog, import, library, list, models, probe, search, transcribe, waveform};

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
        /// MLX interpreter (default: `ECHO_MLX_PYTHON` or the local MLX venv).
        #[arg(long)]
        python: Option<std::path::PathBuf>,
        /// Audio worker script (default: `tools/inference/local_audio_worker.py`).
        #[arg(long)]
        worker: Option<std::path::PathBuf>,
    },
    /// Adds a scan root and queues its scan.
    AddRoot {
        /// Catalog database path.
        catalog: std::path::PathBuf,
        /// Directory to watch.
        root: std::path::PathBuf,
    },
    /// Lists configured scan roots.
    Roots {
        /// Catalog database path.
        catalog: std::path::PathBuf,
    },
    /// Scans all enabled roots and processes the job queue to completion.
    Scan {
        /// Catalog database path.
        catalog: std::path::PathBuf,
        /// Cache root (default: `ECHO_CACHE` or `cache`).
        #[arg(long)]
        cache: Option<std::path::PathBuf>,
        /// Model root; defaults to the standard HF cache.
        #[arg(long)]
        model_root: Option<std::path::PathBuf>,
        /// MLX interpreter (default: `ECHO_MLX_PYTHON` or the local MLX venv).
        #[arg(long)]
        python: Option<std::path::PathBuf>,
        /// Audio worker script (default: `tools/inference/local_audio_worker.py`).
        #[arg(long)]
        worker: Option<std::path::PathBuf>,
        /// Worker threads (default 2).
        #[arg(long, default_value_t = 2)]
        workers: usize,
    },
    /// Reports job queue statistics and failures.
    Jobs {
        /// Catalog database path.
        catalog: std::path::PathBuf,
    },
    /// Searches indexed transcripts.
    Search {
        /// Catalog database path.
        catalog: std::path::PathBuf,
        /// Search query.
        query: String,
        /// Maximum hits (default 20).
        #[arg(long, default_value_t = 20)]
        limit: u64,
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
            let worker = worker.unwrap_or_else(|| {
                std::path::PathBuf::from("tools/inference/local_audio_worker.py")
            });
            transcribe::run_transcribe(&catalog, &source, &model_root, &python, &worker)
        }
        Command::AddRoot { catalog, root } => library::run_add_root(&catalog, &root),
        Command::Roots { catalog } => library::run_list_roots(&catalog),
        Command::Scan {
            catalog,
            cache,
            model_root,
            python,
            worker,
            workers,
        } => {
            let cache = cache.unwrap_or_else(library::default_cache_root);
            let model_root = model_root.unwrap_or_else(models::default_model_root);
            let python = python.unwrap_or_else(transcribe::default_python);
            let worker = worker.unwrap_or_else(|| {
                std::path::PathBuf::from("tools/inference/local_audio_worker.py")
            });
            library::run_scan(&catalog, &cache, &model_root, &python, &worker, workers)
        }
        Command::Jobs { catalog } => library::run_jobs(&catalog),
        Command::Search {
            catalog,
            query,
            limit,
        } => search::run_search(&catalog, &query, limit),
    }
}
