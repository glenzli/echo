//! Library management commands: scan roots, background jobs, and one-shot
//! synchronous processing for scripts.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use echo_catalog::{JobStats, job_stats, list_scan_roots, open_catalog};

pub(crate) fn run_add_root(catalog_path: &Path, root: &Path) -> anyhow::Result<()> {
    let catalog = open_catalog(catalog_path)
        .with_context(|| format!("cannot open catalog at {}", catalog_path.display()))?;
    let now = now_millis();
    echo_core::add_root_and_scan(&catalog, root, now)?;
    println!("scan root added: {}", root.display());
    Ok(())
}

pub(crate) fn run_list_roots(catalog_path: &Path) -> anyhow::Result<()> {
    let catalog = open_catalog(catalog_path)
        .with_context(|| format!("cannot open catalog at {}", catalog_path.display()))?;
    let roots = catalog.with_transaction(list_scan_roots)?;
    if roots.is_empty() {
        println!("no scan roots configured");
    }
    for root in roots {
        println!(
            "[{}] {} (id {})",
            if root.enabled { "on " } else { "off" },
            root.root.display(),
            root.id
        );
    }
    Ok(())
}

pub(crate) fn run_scan(
    catalog_path: &Path,
    cache_root: &Path,
    runtime_endpoint: &str,
    workers: usize,
) -> anyhow::Result<()> {
    let catalog = Arc::new(
        open_catalog(catalog_path)
            .with_context(|| format!("cannot open catalog at {}", catalog_path.display()))?,
    );
    let queued = echo_core::queue_scans_for_enabled_roots(&catalog, now_millis())?;
    println!("queued {queued} scan job(s), processing...");
    let config = echo_core::WorkerConfig {
        cache_root: cache_root.to_owned(),
        infer_runtime: echo_core::InferRuntimeConfig {
            base_url: runtime_endpoint.to_owned(),
            bearer_token: std::env::var("ECHO_INFER_TOKEN").unwrap_or_default(),
        },
    };
    let pool = echo_core::WorkerPool::start(&catalog, &config, workers)?;
    loop {
        let stats: JobStats = catalog.with_transaction(job_stats)?;
        print!(
            "\r  pending {} · running {} · done {} · failed {}",
            stats.pending, stats.running, stats.done, stats.failed
        );
        let _ = std::io::Write::flush(&mut std::io::stdout());
        if stats.pending == 0 && stats.running == 0 {
            println!();
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    pool.stop();
    Ok(())
}

pub(crate) fn run_jobs(catalog_path: &Path) -> anyhow::Result<()> {
    let catalog = open_catalog(catalog_path)
        .with_context(|| format!("cannot open catalog at {}", catalog_path.display()))?;
    let stats = catalog.with_transaction(job_stats)?;
    println!(
        "jobs: {} pending, {} running, {} done, {} failed",
        stats.pending, stats.running, stats.done, stats.failed
    );
    let failed = catalog.with_transaction(echo_catalog::list_failed_jobs)?;
    for job in failed {
        println!(
            "  failed {} [{}]: {}",
            job.kind_label(),
            job.id,
            job.error.as_deref().unwrap_or("unknown error")
        );
    }
    Ok(())
}

pub(crate) fn default_cache_root() -> PathBuf {
    std::env::var_os("ECHO_CACHE").map_or_else(|| PathBuf::from("cache"), PathBuf::from)
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}

trait JobKindLabel {
    fn kind_label(&self) -> &'static str;
}

impl JobKindLabel for echo_catalog::Job {
    fn kind_label(&self) -> &'static str {
        use echo_catalog::JobKind;
        match self.kind {
            JobKind::ScanRoot => "scan",
            JobKind::ImportFile => "import",
            JobKind::ExtractMetadata => "metadata",
            JobKind::AnalyzeWaveform => "waveform",
            JobKind::Transcribe => "transcribe",
            JobKind::Align => "align",
            JobKind::Contextual => "contextual",
        }
    }
}
