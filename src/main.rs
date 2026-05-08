mod apply;
mod cli;
mod commands;
mod config;
mod discover;
mod error;
mod log;
mod path;
mod permissions;
mod plan;

use clap::error::ErrorKind;
use clap::Parser;

fn main() {
    // Manually parse so we can wrap CLI argument errors with the SPEC
    // [CLI 参数错误] classification label. clap's --help/--version exits
    // are not errors and are passed through.
    let cli = match cli::Cli::try_parse() {
        Ok(c) => c,
        Err(e) => match e.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                // clap prints the help/version to stdout and exits 0.
                e.exit();
            }
            _ => {
                let body = e.to_string();
                eprintln!("{}", error::XdError::cli(body));
                std::process::exit(1);
            }
        },
    };

    match commands::dispatch(&cli) {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }
}
