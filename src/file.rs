use anyhow::{anyhow, bail, Result};
use log::{error, info};
use std::fs;
use std::io;
use std::path::Path;

#[cfg(unix)]
use std::os::unix::fs::symlink as create_symlink_impl;

#[cfg(windows)]
use std::os::windows::fs::symlink_file as create_symlink_impl;

#[cfg(windows)]
use std::os::windows::fs::symlink_dir as create_symlink_dir_impl;

use crate::on_force_mode;
use crate::on_interactive_mod;

// 创建路径为link的软链接到actual_path
pub fn create_symlink(actual_path: &str, link: &str) -> Result<()> {
    // 获取actual_path的绝对路径
    let actual_path = std::fs::canonicalize(actual_path)
        .map_err(|e| anyhow!("failed to get absolute path of actual path {actual_path}:{e}"))?;
    // 获取link的绝对路径
    let home_dir = if dirs::home_dir().is_none() {
        return Err(anyhow::anyhow!("home dir not found"));
    } else {
        dirs::home_dir().ok_or(anyhow!("home dir not found"))?
    };
    let link = link.replace('~', home_dir.to_str().ok_or(anyhow!("home dir not found"))?);
    info!("link: {}", link);
    // 化简路径
    let link = Path::new(&link);
    // 获取link的目录,保证link的目录存在
    let link_dir = link.parent().ok_or(anyhow!("link parent not valid"))?;
    if !link_dir.exists() {
        info!("link_dir {} not exists, creating", link_dir.display());
        fs::create_dir_all(link_dir).unwrap_or_else(|e| {
            error!("failed to create link_dir {}: {}", link_dir.display(), e);
        });
    }

    info!(
        "creating link {} to {}",
        link.display(),
        actual_path.display()
    );
    // 判断link是否已经存在了
    if link.exists() {
        // 检查link是否已经是一个指向actual_path的软链接
        if let Ok(path) = fs::read_link(link) {
            if path == actual_path {
                info!(
                    "link {} already exists and points to {}, skipping",
                    link.display(),
                    actual_path.display()
                );
                return Ok(());
            }
        }

        let metadata = link.symlink_metadata()?;
        if metadata.is_symlink() {
            info!("link {} is a symlink", link.display());
        } else if metadata.is_file() {
            info!("link {} is a file", link.display());
        } else if metadata.is_dir() {
            info!("link {} is a directory", link.display());
        } else {
            info!(
                "link {} is not a symlink, file or directory",
                link.display()
            );
            info!("skipping link {}", link.display());
            return Ok(());
        }
        fn rm_entry(metadata: &fs::Metadata, p: &Path) -> Result<()> {
            info!("removing entry {}", p.display());
            if metadata.is_dir() {
                fs::remove_dir_all(p).inspect_err(|e| {
                    error!("failed to remove p {}: {}", p.display(), e);
                })
            } else {
                fs::remove_file(p).inspect_err(|e| {
                    error!("failed to remove p {}: {}", p.display(), e);
                })
            }?;
            Ok(())
        }
        if on_interactive_mod() {
            // 分别针对link是软链接/文件/目录的情况进行处理
            info!("link {} already exists, remove it? [y/n]", link.display());
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if input.trim() == "y" {
                rm_entry(&metadata, link)?;
            } else {
                info!("skipping link {}", link.display());
                return Ok(());
            }
        } else if on_force_mode() {
            info!("link {} already exists, try remove it", link.display());
            rm_entry(&metadata, link)?;
        } else {
            bail!(
                "path {} has been taken,can't create symlink,please use --force / --interactive",
                link.display()
            );
        }
    }

    #[cfg(unix)]
    create_symlink_impl(actual_path, link)?;

    #[cfg(windows)]
    {
        if actual_path.is_file() {
            create_symlink_impl(actual_path, link)?;
        } else if actual_path.is_dir() {
            create_symlink_dir_impl(actual_path, link)?;
        } else {
            bail!("Unsupported file type for symlink creation on Windows");
        }
    }

    Ok(())
}

pub fn delete_symlink(link: &str) -> Result<()> {
    let home_dir = if dirs::home_dir().is_none() {
        return Err(anyhow::anyhow!("home dir not found"));
    } else {
        dirs::home_dir().ok_or(anyhow::anyhow!("home dir not found"))?
    };
    let link = link.replace('~', home_dir.to_str().ok_or(anyhow!("home dir not found"))?);
    let link = Path::new(&link);
    if !link.exists() {
        info!("link {} not exists, skipping", link.display());
        return Ok(());
    }
    if !link.symlink_metadata()?.file_type().is_symlink() {
        info!("link {} is not a symlink, skipping", link.display());
        return Ok(());
    }
    info!("removing link {}", link.display());
    if on_interactive_mod() {
        info!("remove link {}? [y/n]", link.display());
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if input.trim() == "y" {
            fs::remove_file(link)?;
            info!("removed link {}", link.display());
        } else {
            info!("skipping link {}", link.display());
        }
    } else {
        fs::remove_file(link)?;
        info!("removed link {}", link.display());
    }
    Ok(())
}
