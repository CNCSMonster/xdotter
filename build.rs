use clap::CommandFactory;
use clap_complete::{generate_to, Shell};
use std::env;
use std::path::PathBuf;

#[path = "src/cli.rs"]
#[allow(dead_code)]
mod cli;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let mut cmd = cli::Cli::command();

    for shell in [Shell::Bash, Shell::Zsh, Shell::Fish] {
        generate_to(shell, &mut cmd, "xd", &out_dir).unwrap();
    }

    println!("cargo:rerun-if-changed=src/cli.rs");
    println!("cargo:rerun-if-changed=build.rs");
}
