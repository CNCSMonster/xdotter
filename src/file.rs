use super::*;
use anyhow::Error;
use std::io;
use std::{fs, os::unix::fs::symlink, path::Path};

// 创建路径为link的软链接到actual_path
pub fn create_symlink(actual_path: &str, link: &str) -> Result<(), Error> {
    // 获取actual_path的绝对路径
    let actual_path = std::fs::canonicalize(actual_path)?;
    // 获取link的绝对路径
    let home_dir = if dirs::home_dir().is_none() {
        return Err(anyhow::anyhow!("home dir not found"));
    } else {
        dirs::home_dir().unwrap()
    };
    let link = link.replace('~', home_dir.to_str().unwrap());
    info!("link: {}", link);
    // 化简路径
    let link = Path::new(&link);
    // 获取link的目录,保证link的目录存在
    let link_dir = link.parent().unwrap();
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

        if on_interactive_mod() {
            // 分别针对link是软链接/文件/目录的情况进行处理
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
            info!("link {} already exists, remove it? [y/n]", link.display());
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            if input.trim() == "y" {
                info!("removing link {}", link.display());
                if metadata.is_dir() {
                    fs::remove_dir_all(link).unwrap_or_else(|e| {
                        error!("failed to remove link {}: {}", link.display(), e);
                    });
                } else {
                    fs::remove_file(link).unwrap_or_else(|e| {
                        error!("failed to remove link {}: {}", link.display(), e);
                    });
                }
            } else {
                info!("skipping link {}", link.display());
                return Ok(());
            }
        } else {
            info!("link {} already exists, removing", link.display());
            fs::remove_file(link).unwrap_or_else(|e| {
                error!("failed to remove link {}: {}", link.display(), e);
            });
        }
    }
    symlink(actual_path, link)?;
    Ok(())
}

pub fn delete_symlink(link: &str) -> Result<(), Error> {
    let home_dir = if dirs::home_dir().is_none() {
        return Err(anyhow::anyhow!("home dir not found"));
    } else {
        dirs::home_dir().unwrap()
    };
    let link = link.replace('~', home_dir.to_str().unwrap());
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
