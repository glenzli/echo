//! Real CLI consumer of the saved-state editor facade.

use std::path::PathBuf;

use clap::Subcommand;
use echo_desktop_bridge::editor_commands::{self, InspectRequest};

#[derive(Debug, Subcommand)]
pub(crate) enum EditCommand {
    /// Reports the versioned contract and currently supported editor operations.
    Capabilities,
    /// Emits saved identities and timing from one existing independent session.
    Inspect {
        /// Explicit session directory containing catalog.sqlite; never created or migrated.
        #[arg(long)]
        session: PathBuf,
    },
}

pub(crate) fn run(command: EditCommand) -> anyhow::Result<()> {
    match command {
        EditCommand::Capabilities => {
            println!("{}", editor_commands::capabilities().to_json());
            Ok(())
        }
        EditCommand::Inspect { session } => {
            match editor_commands::inspect(&InspectRequest {
                session_root: session,
            }) {
                Ok(response) => {
                    println!("{}", response.to_json());
                    Ok(())
                }
                Err(error) => {
                    println!("{}", error.to_json());
                    Err(error.into())
                }
            }
        }
    }
}
