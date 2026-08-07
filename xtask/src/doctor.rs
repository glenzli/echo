//! Toolchain prerequisite verification.

use std::process::Command;

pub(crate) fn run_doctor() -> anyhow::Result<()> {
    check_command("cargo", ["--version"])?;
    check_command("rustc", ["--version"])?;
    check_command("cmake", ["--version"])?;
    check_command("ninja", ["--version"])?;
    check_command("ffmpeg", ["-version"])?;
    check_command("qmake6", ["--version"])?;
    println!("doctor: all prerequisites present");
    Ok(())
}

fn check_command(
    program: &str,
    arguments: impl IntoIterator<Item = &'static str>,
) -> anyhow::Result<()> {
    let output = Command::new(program)
        .args(arguments)
        .output()
        .map_err(|error| anyhow::anyhow!("{program} unavailable: {error}"))?;
    anyhow::ensure!(
        output.status.success(),
        "{program} failed with {}",
        output.status
    );
    Ok(())
}
