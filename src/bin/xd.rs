use clap::Parser;
use xdotter::{
    complete::complete, deploy::deploy, new::new, undeploy::undeploy, Action, RunArgs, XdotterCli,
};

extern crate xdotter;

fn main() -> anyhow::Result<()> {
    let cli = XdotterCli::parse();
    match &cli.subcommand {
        Some(action) => match action {
            Action::Deploy(args) => deploy(args),
            Action::Undeploy(args) => undeploy(args),
            Action::New => new(),
            Action::Complete { shell } => complete(shell),
        },
        None => deploy(&RunArgs::default()),
    }
}
