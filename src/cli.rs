use clap::{Args, Parser, Subcommand};
use clap_complete::Shell;

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct XdotterCli {
    #[command(subcommand)]
    pub subcommand: Option<Action>,
}

#[derive(Debug, Subcommand)]
pub enum Action {
    Deploy(RunArgs),
    Undeploy(RunArgs),
    New,
    Complete {
        #[arg(short, long)]
        shell: Shell,
    },
}

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Specify the configuration file
    #[arg(short, long, default_value_t = String::from("xdotter.toml"))]
    pub(crate) config: String,
    /// Show more information in execution
    #[arg(short, long)]
    pub(crate) verbose: bool,
    /// Do not actually work,but show you what will happen
    #[arg(short, long)]
    pub(crate) dry_run: bool,
    /// Ask for confirmation while unsure,in case like conflict with existing file entry
    #[arg(short, long, conflicts_with = "force", conflicts_with = "quiet")]
    pub(crate) interactive: bool,
    /// If conflict with existed file entry,just remove it
    #[arg(short, long)]
    pub(crate) force: bool,
    /// Do not print any output
    #[arg(short, long)]
    pub(crate) quiet: bool,
}

impl Default for RunArgs {
    fn default() -> Self {
        Self {
            config: "xdotter.toml".to_string(),
            verbose: true,
            dry_run: Default::default(),
            interactive: Default::default(),
            force: Default::default(),
            quiet: Default::default(),
        }
    }
}
