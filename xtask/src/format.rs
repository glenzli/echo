//! Formatting application: rustfmt for the workspace, clang-format for
//! changed C/C++/Objective-C sources when clang-format is available.

use std::process::Command;

pub(crate) fn run_format(check: bool) -> anyhow::Result<()> {
    println!("== rustfmt ==");
    let mut arguments = vec!["fmt", "--all"];
    if check {
        arguments.push("--check");
    }
    let status = Command::new("cargo").args(arguments).status()?;
    anyhow::ensure!(status.success(), "rustfmt failed");

    if let Some(clang_format) = which_clang_format() {
        println!("== clang-format ==");
        let mut command = Command::new(&clang_format);
        if check {
            command.arg("--dry-run").arg("--Werror");
        } else {
            command.arg("--i");
        }
        let status = command
            .args(["--files-regex", r"\.(cpp|hpp|h|cc|mm|m)$"])
            .arg(".")
            .status()?;
        anyhow::ensure!(status.success(), "clang-format reported differences");
    } else {
        println!("clang-format not found; skipping native formatting");
    }
    Ok(())
}

fn which_clang_format() -> Option<std::path::PathBuf> {
    for candidate in ["clang-format", "clang-format-19", "clang-format-18"] {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
        {
            return Some(std::path::PathBuf::from(candidate));
        }
    }
    None
}
