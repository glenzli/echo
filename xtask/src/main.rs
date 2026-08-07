//! Repository-level validation commands for Echo.

mod commands;
mod doctor;
mod format;

fn main() -> anyhow::Result<()> {
    commands::run(std::env::args().skip(1))
}
