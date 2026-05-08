//! Tiny verbosity-aware diagnostic helper per SPEC §"输出语义".
//!
//! `-v` (level 1) prints normal operation info; `-vv` (2) prints debug
//! info; `-vvv` (3) prints trace info. Any value > 3 is treated as 3.
//! All diagnostics go to stderr; warnings/errors are printed
//! unconditionally elsewhere.

use crate::cli::Cli;

#[derive(Copy, Clone, Debug)]
pub enum Level {
    Info = 1,
    Debug = 2,
    Trace = 3,
}

fn current_level(cli: &Cli) -> u8 {
    cli.verbose.min(3)
}

/// Emit a diagnostic line to stderr if `cli`'s verbosity is at least `lvl`.
pub fn log(cli: &Cli, lvl: Level, msg: impl AsRef<str>) {
    if current_level(cli) >= lvl as u8 {
        eprintln!("{}", msg.as_ref());
    }
}

pub fn info(cli: &Cli, msg: impl AsRef<str>) {
    log(cli, Level::Info, msg);
}

pub fn debug(cli: &Cli, msg: impl AsRef<str>) {
    log(cli, Level::Debug, msg);
}

#[allow(dead_code)]
pub fn trace(cli: &Cli, msg: impl AsRef<str>) {
    log(cli, Level::Trace, msg);
}
