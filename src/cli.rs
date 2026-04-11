use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "xd", version = "0.4.0", about = "A simple dotfile manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Show more information
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Do not print any output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Show what would be done without making changes
    #[arg(short = 'n', long, global = true)]
    pub dry_run: bool,

    /// Ask for confirmation when unsure
    #[arg(short, long, global = true)]
    pub interactive: bool,

    /// Force overwrite existing files
    #[arg(short, long, global = true)]
    pub force: bool,

    /// Check permissions for sensitive files
    #[arg(long, global = true)]
    pub check_permissions: bool,

    /// Fix permissions for sensitive files
    #[arg(long, global = true)]
    pub fix_permissions: bool,

    /// Skip config syntax validation during deploy
    #[arg(long, global = true)]
    pub no_validate: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Deploy dotfiles (default)
    Deploy,
    /// Remove deployed dotfiles
    Undeploy,
    /// Show deployment status
    Status,
    /// Validate configuration file syntax
    Validate {
        /// Files to validate
        files: Vec<PathBuf>,
    },
    /// Create a new xdotter.toml template
    New,
    /// Generate shell completion scripts
    Completion {
        /// Shell to generate completion for
        shell: String,
    },
    /// Print version
    Version,
}
