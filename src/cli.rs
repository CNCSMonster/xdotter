use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "xd", version = "0.4.0", about = "A simple dotfile manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Show more information
    #[arg(short, long)]
    pub verbose: bool,

    /// Do not print any output
    #[arg(short, long)]
    pub quiet: bool,

    /// Show what would be done without making changes
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Ask for confirmation when unsure
    #[arg(short, long)]
    pub interactive: bool,

    /// Force overwrite existing files
    #[arg(short, long)]
    pub force: bool,

    /// Check permissions for sensitive files
    #[arg(long)]
    pub check_permissions: bool,

    /// Fix permissions for sensitive files
    #[arg(long)]
    pub fix_permissions: bool,

    /// Skip config syntax validation during deploy
    #[arg(long)]
    pub no_validate: bool,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Deploy dotfiles (default)
    Deploy,
    /// Remove deployed dotfiles
    Undeploy,
    /// Check/fix permissions for deployed files
    CheckPermissions,
    /// Validate configuration file syntax
    Validate {
        /// Files to validate
        files: Vec<PathBuf>,
    },
    /// Create a new xdotter.toml template
    New,
    /// Print help message
    Help,
    /// Print version
    Version,
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}
