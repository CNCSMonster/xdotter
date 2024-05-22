extern crate xdotter;
use clap::{ArgAction, ArgMatches};
use indoc::indoc;
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::Path;
use std::{collections::HashMap, path};
use xdotter::{create_link, get_dry_run};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Config {
    /// dependencies,子成员的路径,每个子成员内部应该有配置
    dependencies: HashMap<String, String>,
    /// 子成员路径',如果子成员路径存在,则在子成员路径中创建配置文件,左边为子成员路径,右边为目标链接创建路径
    #[serde(default = "HashMap::new", skip_serializing_if = "HashMap::is_empty")]
    links: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
enum LinkAction {
    /// the path where the link will be created
    Link(String),
}

fn new() {
    let config = Config {
        dependencies: hashmap! {
            "go".to_string() => "testdata/go".to_string(),
        },
        links: hashmap! {
            "testdata/mm".to_string() => "~/.cache/mm".to_string(),
        },
    };
    let config_str = toml::to_string(&config).unwrap();
    fs::write("xdotter.toml", config_str).unwrap();
    println!("Created xdotter.toml");
}

fn deploy(args: &ArgMatches) {
    let dry_run = get_dry_run(args);
    deploy_on(dry_run);
}

fn deploy_on(dry_run: bool) {
    let config_str = fs::read_to_string("xdotter.toml").unwrap();
    let config: Config = toml::from_str(&config_str).unwrap();
    for (actual_path, link) in config.links {
        if !dry_run {
            create_link(&actual_path, &link).unwrap();
        }
    }
    let current_dir = std::env::current_dir().unwrap();
    for (dependency, path) in config.dependencies {
        println!("dependency: {}, path: {}", dependency, path);
        let path = current_dir.join(path);
        // 切换路径
        println!("entering {}", path.display());
        std::env::set_current_dir(&path).unwrap();
        deploy_on(dry_run);
        // 切换回原路径
        println!("leaving {}", path.display());
        std::env::set_current_dir(&current_dir).unwrap();
    }
}

fn main() {
    // 使用clap作为命令行工具框架
    let matches = clap::Command::new("xdotter")
        .version("0.1.0")
        .author("xdotter")
        .about("A simple tool to manage dotfiles")
        .subcommand(clap::Command::new("new").about("Create a new xdotter.toml file"))
        .subcommand(
            clap::Command::new("deploy")
                .about(indoc! {"
                    Create symlinks for all the files and directories specified in the xdotter.toml file,
                    and locate these symlinkes in the paths specified in the xdotter.toml file.
                    The actual file path should be relative to the xdotter.toml file,like ./vimrc
                    the link path can be absolute or relative to the home directory,like ~/.vimrc
                "})
                .arg(
                    clap::Arg::new("dry-run")
                        .long("dry-run")
                        .short('d')
                        .help("Print what would be done without doing it")
                        .action(ArgAction::SetTrue),
                ),
        )
        .get_matches();
    match matches.subcommand() {
        Some(("new", _)) => new(),
        Some(("deploy", args)) => deploy(args),
        _ => println!("No subcommand was used"),
    }
}
