extern crate xdotter;
use anyhow::{anyhow, Error, Ok, Result};
use clap::{arg, Arg, ArgAction, ArgMatches, Command};
use clap_complete::Shell;
use indoc::indoc;
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self};
use xdotter::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Config {
    /// dependencies,子成员的路径,每个子成员内部应该有配置
    dependencies: Option<HashMap<String, String>>,
    /// 子成员路径',如果子成员路径存在,则在子成员路径中创建配置文件,左边为子成员路径,右边为目标链接创建路径
    #[serde(skip_serializing_if = "Option::is_none")]
    links: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
enum LinkAction {
    /// the path where the link will be created
    Link(String),
}

fn new_cmd()->Result<()> {
    let config = Config {
        dependencies: Some(hashmap! {
            "go".to_string() => "testdata/go".to_string(),
        }),
        links: Some(hashmap! {
            "testdata/mm".to_string() => "~/.cache/mm".to_string(),
        }),
    };
    let config_str = toml::to_string(&config).unwrap();
    info!("creating xdotter.toml");
    fs::write("xdotter.toml", config_str).unwrap_or_else(|e| {
        error!("failed to create xdotter.toml: {}", e);
    });
    info!("Created xdotter.toml");
    Ok(())
}

fn deploy_cmd(am: &ArgMatches) ->Result<()>{
    info!("deploying...");
    let dry_run = get_dry_run(am);
    let interactive = get_interactive(am);
    let conf = get_start_config(am);

    if dry_run {
        info!("running in dry-run mode");
    }
    if interactive {
        info!("running in interactive mode");
    }
    set_dry_run_mode(dry_run);
    set_interactive_mode(interactive);
    deploy_on(&conf).unwrap_or_else(|e| {
        error!("{e}");
    });
    Ok(())
}

fn deploy_on(conf: &str) -> Result<()> {
    info!("deploying on {}", conf);
    let dry_run = on_dry_run_mode();
    let config_str = fs::read_to_string(conf)?;
    let config: Config = toml::from_str(&config_str)?;
    if let Some(links) = config.links.as_ref() {
        for (actual_path, link) in links {
            info!("deploy: {} -> {}", link, actual_path);
            if !dry_run {
                create_symlink(actual_path, link).unwrap_or_else(|e| {
                    error!("failed to create link: {}", e);
                });
            }
        }
    }
    let current_dir = std::env::current_dir()?;
    if let Some(deps) = config.dependencies.as_ref() {
        for (dependency, path) in deps {
            info!("dependency: {}, path: {}", dependency, path);
            let path = current_dir.join(path);
            if let Err(e) = std::env::set_current_dir(&path) {
                error!("failed to enter {}: {}", path.display(), e);
                continue;
            }
            info!("entering {}", path.display());
            deploy_on(&format!("{}/xdotter.toml", path.display())).unwrap_or_else(|e| {
                error!("{}", e);
            });
            std::env::set_current_dir(&current_dir).unwrap_or_else(|e| {
                error!("failed to leave {}: {}", path.display(), e);
            });
            info!("leaving {}", path.display());
        }
    }
    Ok(())
}
fn undeploy_cmd(am: &ArgMatches) ->Result<()>{
    info!("undeploying...");
    let dry_run = get_dry_run(am);
    let interactive = get_interactive(am);
    let conf = get_start_config(am);
    if dry_run {
        info!("running in dry-run mode");
    }
    if interactive {
        info!("running in interactive mode");
    }
    set_dry_run_mode(dry_run);
    set_interactive_mode(interactive);
    undeploy_on(&conf).unwrap_or_else(|e| {
        error!("{e}");
    });
    Ok(())
}
fn undeploy_on(conf: &str) -> Result<(), Error> {
    info!("undeploying on {}", conf);
    let dry_run = on_dry_run_mode();
    let config_str = fs::read_to_string(conf)?;
    let config: Config = toml::from_str(&config_str)?;
    if let Some(links) = config.links.as_ref() {
        for (actual_path, link) in links {
            info!("undeploy: {} -> {}", link, actual_path);
            if !dry_run {
                delete_symlink(link).unwrap_or_else(|e| {
                    error!("failed to delete link: {}", e);
                });
            }
        }
    }
    let current_dir = std::env::current_dir()?;
    if let Some(deps) = config.dependencies.as_ref() {
        for (dependency, path) in deps {
            debug!("dependency: {}, path: {}", dependency, path);
            let path = current_dir.join(path);
            if let Err(e) = std::env::set_current_dir(&path) {
                error!("failed to enter {}: {}", path.display(), e);
                continue;
            }
            debug!("entering {}", path.display());
            undeploy_on(&format!("{}/xdotter.toml", path.display())).unwrap_or_else(|e| {
                error!("{}", e);
            });
            std::env::set_current_dir(&current_dir).unwrap_or_else(|e| {
                error!("failed to leave {}: {}", path.display(), e);
            });
            debug!("leaving {}", path.display());
        }
    }
    Ok(())
}
fn completions_cmd(am:&ArgMatches)->Result<()>{
    let shell=am.get_one::<Shell>("shell").ok_or_else(||anyhow!("Shell name missing")).unwrap();
    let mut cli=xdotter_cli();
    let bin_name=cli.get_name().to_string();
    clap_complete::generate(*shell, &mut cli, bin_name, &mut std::io::stdout().lock());
    Ok(())
}

fn xdotter_cli() -> Command {
    let new_cmd = clap::Command::new("new").about("Create a new xdotter.toml file");
    let deploy_cmd = clap::Command::new("deploy").about(indoc! {"
        Deploy the dotfiles according to the configuration file. This is the default subcommand.
    "});
    let undeploy_cmd = clap::Command::new("undeploy").about(indoc! {"
        Delete all the symlinks created by the deploy command.
    "});
    let completions_cmd = clap::Command::new("completions")
        .arg(arg!(-s --shell <shell> "Specify the shell to generate completions for").value_parser(clap::value_parser!(Shell)).required(true))
        .about("Generate shell completions");
    clap::Command::new("xdotter")
        .version(env!("CARGO_PKG_VERSION"))
        .author("xdotter")
        .about("A simple tool to manage dotfiles")
        .arg(
            arg!(-v --verbose "Print test information verbosely")
                .required(false)
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .arg(
            arg!(-q --quiet "Do not print any output")
                .required(false)
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .arg(
            Arg::new("dry-run")
                .long("dry-run")
                .short('d')
                .action(ArgAction::SetTrue)
                .required(false)
                .global(true),
        )
        .arg(
            arg!(-c --config <config_file> "Specify the configuration file")
                .required(false)
                .global(true),
        )
        .arg(
            arg!(-i --interactive "Ask for confirmation while unsure")
                .required(false)
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .subcommands(vec![new_cmd, deploy_cmd, undeploy_cmd, completions_cmd])
}

fn main() {
    let cli = xdotter_cli();
    let am = cli.get_matches();
    let verbose = get_verbose(&am);
    let quiet = get_quiet(&am);
    if verbose && !quiet {
        std::env::set_var("RUST_LOG", "trace");
    } else if quiet {
        std::env::set_var("RUST_LOG", "error");
    } else {
        let level = std::env::var("RUST_LOG");
        if level.is_err() {
            std::env::set_var("RUST_LOG", "info");
        }
    }
    env_logger::init();
    match am.subcommand() {
        Some(("new", _)) => new_cmd(),
        Some(("deploy", sub_m)) => deploy_cmd(sub_m),
        Some(("undeploy", sub_m)) => undeploy_cmd(sub_m),
        Some(("completions",sub_m))=>completions_cmd(sub_m),
        Some((_, sub_m)) => deploy_cmd(sub_m),
        None => deploy_cmd(&am),
    }.unwrap_or_else(|e|{
        panic!("{e}");
    });
}
