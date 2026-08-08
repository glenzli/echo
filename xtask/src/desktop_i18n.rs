//! Desktop translation contract gate: exact Chinese coverage, valid
//! placeholders, and a compilable QM catalog.

use std::process::Command;

pub(crate) fn run() -> anyhow::Result<()> {
    let repository_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives directly below the repository root")
        .to_path_buf();
    let checker = repository_root.join("scripts/check_qt_translations.py");
    let python = std::env::var_os("PYTHON").unwrap_or_else(|| "python3".into());
    let status = Command::new(&python)
        .current_dir(&repository_root)
        .arg(&checker)
        .status()
        .map_err(|error| {
            anyhow::anyhow!(
                "launch desktop translation checker {} with {:?}: {error}",
                checker.display(),
                std::path::Path::new(&python).display()
            )
        })?;
    anyhow::ensure!(status.success(), "desktop translation contract failed");
    Ok(())
}
