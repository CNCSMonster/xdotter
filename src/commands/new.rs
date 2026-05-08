use std::fs;
use std::path::Path;

use crate::cli::{Cli, NewArgs};
use crate::error::XdError;
use crate::log;

const TEMPLATE: &str = r#"# xdotter configuration file

[links]
# Format: "source_path" = "link_path"
# - source: path inside this repo (relative to this file)
# - link:   absolute or `~/...` path to create
#
# ".zshrc" = "~/.zshrc"
# ".config/nvim" = "~/.config/nvim"

[dependencies]
# Format: "name" = "relative_subdirectory"
# Each subdirectory must contain its own xdotter.toml.
#
# "nvim" = "config/nvim"
"#;

pub fn run(cli: &Cli, args: &NewArgs) -> Result<(), XdError> {
    let path = Path::new("xdotter.toml");
    log::debug(cli, format!("new: 目标路径 {}", path.display()));
    if path.exists() {
        return Err(XdError::config(format!(
            "{} 已存在，拒绝覆盖",
            path.display()
        )));
    }
    if args.dry_run {
        println!("Would create xdotter.toml");
        return Ok(());
    }
    fs::write(path, TEMPLATE).map_err(|e| {
        XdError::apply(format!("写入 {} 失败: {}", path.display(), e))
    })?;
    log::info(cli, "new: 模板已写入");
    println!("Created xdotter.toml");
    Ok(())
}
