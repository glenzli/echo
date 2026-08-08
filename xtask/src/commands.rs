//! Argument routing for repository checks.

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "xtask", about = "Echo repository checks")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Runs formatting, lint, tests, and the translation contract.
    Check,
    /// Applies repository formatting (rustfmt, clang-format).
    Format,
    /// Verifies toolchain prerequisites.
    Doctor,
    /// Verifies the desktop translation catalog contract.
    DesktopI18nCheck,
}

pub(crate) fn run(arguments: impl Iterator<Item = String>) -> anyhow::Result<()> {
    let cli = Cli::parse_from(std::iter::once("xtask".to_owned()).chain(arguments));
    match cli.command {
        Command::Check => {
            crate::format::run_format(true)?;
            run_clippy()?;
            run_tests()?;
            crate::desktop_i18n::run()?;
            Ok(())
        }
        Command::Format => crate::format::run_format(false),
        Command::Doctor => crate::doctor::run_doctor(),
        Command::DesktopI18nCheck => crate::desktop_i18n::run(),
    }
}

fn run_clippy() -> anyhow::Result<()> {
    println!("== clippy ==");
    let status = std::process::Command::new("cargo")
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ])
        .status()?;
    anyhow::ensure!(status.success(), "clippy reported errors");
    Ok(())
}

fn run_tests() -> anyhow::Result<()> {
    println!("== tests ==");
    let status = std::process::Command::new("cargo")
        .args(["test", "--workspace"])
        .status()?;
    anyhow::ensure!(status.success(), "tests failed");
    Ok(())
}
