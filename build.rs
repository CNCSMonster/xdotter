use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate_to, Shell};
use std::env;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "xd", version = env!("CARGO_PKG_VERSION"), about = "A simple dotfile manager")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(short, long, global = true)]
    verbose: bool,

    #[arg(short, long, global = true)]
    quiet: bool,

    #[arg(short = 'n', long, global = true)]
    dry_run: bool,

    #[arg(short, long, global = true)]
    interactive: bool,

    #[arg(short, long, global = true)]
    force: bool,

    #[arg(long, global = true)]
    check_permissions: bool,

    #[arg(long, global = true)]
    fix_permissions: bool,

    #[arg(long, global = true)]
    no_validate: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    Deploy,
    Undeploy,
    Status,
    Validate { files: Vec<PathBuf> },
    New,
    Completion { shell: String },
    Version,
}

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut cmd = Cli::command();

    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish] {
        generate_to(shell, &mut cmd, "xd", &out_dir).unwrap();
    }

    println!("cargo:rerun-if-changed=src/cli.rs");
    println!("cargo:rerun-if-changed=build.rs");
}
