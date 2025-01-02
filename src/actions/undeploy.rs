use std::fs;

use log::*;

use crate::{delete_symlink, init_run_mode, mlog::init_logger, on_dry_run_mode, Config, RunArgs};

pub fn undeploy(args: &RunArgs) -> anyhow::Result<()> {
    init_logger(args)?;
    init_run_mode(args)?;

    info!("undeploying...");
    undeploy_on(&args.config).unwrap_or_else(|e| {
        error!("{e}");
    });
    Ok(())
}

fn undeploy_on(conf: &str) -> anyhow::Result<()> {
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
