use clap::CommandFactory;
use clap_complete::Shell;

use crate::XdotterCli;

pub fn complete(shell: &Shell) -> anyhow::Result<()> {
    let mut cli = XdotterCli::command();
    let bin_name = cli.get_name().to_string();
    clap_complete::generate(*shell, &mut cli, bin_name, &mut std::io::stdout().lock());
    Ok(())
}
