//! Command implementations. Each command corresponds to a SPEC §"命令"
//! subcommand. Output stream conventions per SPEC §"输出语义":
//! - Command results -> stdout
//! - Warnings / errors / diagnostics -> stderr

mod completion;
mod deploy;
mod new;
mod status;
mod undeploy;
mod version;

use crate::cli::{Cli, Command};
use crate::error::XdError;

pub fn dispatch(cli: &Cli) -> Result<(), XdError> {
    let cmd = cli.command.as_ref();
    match cmd {
        None => deploy::run(cli, &Default::default()),
        Some(Command::Deploy(args)) => deploy::run(cli, args),
        Some(Command::Undeploy(args)) => undeploy::run(cli, args),
        Some(Command::Status) => status::run(cli),
        Some(Command::New(args)) => new::run(cli, args),
        Some(Command::Completion { shell }) => completion::run(shell),
        Some(Command::Version) => version::run(),
    }
}
