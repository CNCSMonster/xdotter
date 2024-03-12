use clap::ArgAction;
use indoc::indoc;
use maplit::hashmap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::{collections::HashMap, path};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Config {
    /// 子成员路径'
    #[serde(default = "HashMap::new", skip_serializing_if = "HashMap::is_empty")]
    links: HashMap<String, String>,
    ///子成员路径,设置成可选
    #[serde(default = "Vec::new", skip_serializing_if = "Vec::is_empty")]
    subs: Vec<String>,
}
// 使用子路径之后能够在子路径中创建配置文件

#[derive(Debug)]
struct LinkAction {
    /// the file that will be linked to
    path: String,
    /// the path where the link will be created
    link: String,
}
impl LinkAction {
    fn from(path: &str, link: &str) -> Result<LinkAction, String> {
        // path内容应该为相对路径,所以需要转换为绝对路径
        let path = match fs::canonicalize(path) {
            Ok(path) => path.to_string_lossy().into_owned(),
            Err(_) => {
                return Err(format!("Path {} does not exist", &path));
            }
        };
        // 判断link是否开头是~,如果是,则解析替换成home路径
        let link = match link.starts_with('~') {
            true => {
                let home = std::env::var("HOME").unwrap();
                link.replace('~', &home)
            }
            _ => link.to_string(),
        };
        // 判断link处的文件是否已经存在,是否是链接?如果是链接,打印链接到的文件
        // 首先判断是否存在文件
        let link = match fs::symlink_metadata(&link) {
            Err(_) => link.to_owned(),
            Ok(metadata) => {
                println!("{}", link);
                let err = if metadata.file_type().is_symlink() {
                    format!("symlink {} already exists", &link)
                } else if metadata.file_type().is_file() {
                    format!("file {} already exists", &link)
                } else if metadata.file_type().is_dir() {
                    format!("dir {} already exists", &link)
                } else {
                    format!("??? {} already exists", &link)
                };
                return Err(err);
            }
        };
        Ok(LinkAction { path, link })
    }
}

fn new() {
    let home = std::env::var("HOME").unwrap();
    let config = Config {
        links: hashmap! {
            "vimrc".to_string() => format!("{}/.vimrc", home),
            "zshrc".to_string() => format!("{}/.zshrc", home),
            "go".to_string() => format!("{}/.config/go", home),
            "yazi".to_string() => format!("{}/.config/yazi", home),
        },
        subs: vec!["sub1".to_string(), "sub2".to_string()],
    };
    let serialized = toml::to_string(&config).unwrap();
    fs::write("xdotter.toml", serialized).expect("Unable to write file");
}
fn deploy(dry: bool) {
    if dry {
        println!("Dry run");
    }
    // First, collect linkActions
    let mut link_actions = Vec::new();
    collect_link_actions(".", &mut link_actions);
    // Then, execute linkActions
    for link_action in link_actions {
        println!("Linking {} to {}", link_action.path, link_action.link);
        if dry {
            continue;
        }
        // 首先创建前置路径,如果路径不存在
        let link_dir = path::Path::new(&link_action.link).parent().unwrap();
        if !link_dir.exists() {
            fs::create_dir_all(link_dir).unwrap();
        }
        // 然后创建路径中的链接
        use std::process::Command;
        let mut cmd = Command::new("ln");
        cmd.arg("-s").arg(link_action.path).arg(link_action.link);
        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    println!("Link created successfully");
                } else {
                    println!("Error creating link");
                }
            }
            Err(e) => {
                println!("Error creating link: {}", e);
            }
        }
    }
}

fn collect_link_actions(dir: &str, link_actions: &mut Vec<LinkAction>) {
    let contents = fs::read_to_string(dir.to_string() + "/xdotter.toml");
    let abs_dir = fs::canonicalize(dir)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    if let Ok(contents) = contents {
        let config: Config = toml::from_str(&contents).unwrap();
        // 判断子路径是否是绝对路径
        for (path, link) in &config.links {
            let path = format!("{}/{}", &abs_dir, path);
            match LinkAction::from(&path, link) {
                Ok(link_action) => link_actions.push(link_action),
                Err(e) => println!("Error: {}", e),
            }
        }
        for sub in &config.subs {
            let sub = format!("{}/{}", &abs_dir, sub);
            collect_link_actions(&sub, link_actions);
        }
    } else {
        println!("No xdotter.toml file in {}", dir)
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
        Some(("deploy", args)) => deploy(args.get_flag("dry-run")),
        _ => println!("No subcommand was used"),
    }
}
