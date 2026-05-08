use clap::{ArgAction, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "xd",
    version = env!("CARGO_PKG_VERSION"),
    about = "A simple dotfile manager"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Increase log verbosity (-v info, -vv debug, -vvv trace).
    /// May be repeated; more than three is treated as -vvv.
    #[arg(short = 'v', long = "verbose", global = true, action = ArgAction::Count)]
    pub verbose: u8,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Deploy dotfiles (default)
    Deploy(DeployArgs),
    /// Remove deployed dotfiles
    Undeploy(UndeployArgs),
    /// Show deployment status
    Status,
    /// Create a new xdotter.toml template
    New(NewArgs),
    /// Generate shell completion scripts
    Completion {
        /// Shell to generate completion for
        shell: String,
    },
    /// Print version
    Version,
}

#[derive(clap::Args, Debug, Default)]
pub struct DeployArgs {
    /// Show planned operations without modifying the filesystem.
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,

    /// Automatically handle recoverable conflicts (cannot bypass safety checks).
    #[arg(short = 'f', long = "force", conflicts_with = "interactive")]
    pub force: bool,

    /// Ask for confirmation before each destructive operation.
    #[arg(short = 'i', long = "interactive", conflicts_with = "force")]
    pub interactive: bool,
}

#[derive(clap::Args, Debug, Default)]
pub struct UndeployArgs {
    /// Show planned operations without modifying the filesystem.
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,

    /// Automatically handle recoverable conflicts (cannot bypass safety checks).
    #[arg(short = 'f', long = "force", conflicts_with = "interactive")]
    pub force: bool,

    /// Ask for confirmation before each destructive operation.
    #[arg(short = 'i', long = "interactive", conflicts_with = "force")]
    pub interactive: bool,
}

#[derive(clap::Args, Debug, Default)]
pub struct NewArgs {
    /// Report what would be created without writing the file.
    #[arg(short = 'n', long = "dry-run")]
    pub dry_run: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ConflictMode {
    Default,
    Force,
    Interactive,
}

impl DeployArgs {
    pub fn conflict_mode(&self) -> ConflictMode {
        if self.force {
            ConflictMode::Force
        } else if self.interactive {
            ConflictMode::Interactive
        } else {
            ConflictMode::Default
        }
    }
}

impl UndeployArgs {
    pub fn conflict_mode(&self) -> ConflictMode {
        if self.force {
            ConflictMode::Force
        } else if self.interactive {
            ConflictMode::Interactive
        } else {
            ConflictMode::Default
        }
    }
}
