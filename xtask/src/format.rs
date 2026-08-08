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
            command.arg("-i");
        }
        let status = command.args(native_sources()).status()?;
        anyhow::ensure!(status.success(), "clang-format reported differences");
    } else {
        println!("clang-format not found; skipping native formatting");
    }
    Ok(())
}

fn native_sources() -> Vec<std::path::PathBuf> {
    let mut sources = Vec::new();
    for directory in ["apps/desktop/src", "cpp"] {
        let mut stack = vec![std::path::PathBuf::from(directory)];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| {
                        matches!(extension, "cpp" | "hpp" | "h" | "cc" | "mm" | "m")
                    })
                {
                    sources.push(path);
                }
            }
        }
    }
    sources
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
