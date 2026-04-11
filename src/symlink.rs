use crate::{expand_path, log, Cli};
use std::fs;
use std::io;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

/// Detect circular symlink scenario:
/// Creating symlink at A/B pointing to C/B, when C is already a symlink to A.
/// This would create: A/B -> C/B -> A/B (circular!)
pub fn detect_circular_symlink_scenario(link_path: &Path, actual: &Path) -> Option<(PathBuf, PathBuf)> {
    let link_absolute = link_path.to_path_buf();
    let actual_parent = actual.parent()?;
    let link_parent = link_absolute.parent()?;
    
    if actual_parent.is_symlink() {
        if let Ok(parent_target) = fs::read_link(actual_parent) {
            let parent_target = if parent_target.is_absolute() {
                parent_target
            } else {
                actual_parent.parent().unwrap_or(actual_parent).join(&parent_target)
            };
            
            if let Ok(parent_target_resolved) = parent_target.canonicalize() {
                if parent_target_resolved == link_parent {
                    return Some((actual_parent.to_path_buf(), link_parent.to_path_buf()));
                }
            }
        }
    }
    
    None
}

/// Check if creating a symlink at link_path pointing to actual would create a loop.
pub fn would_create_symlink_loop(link_path: &Path, actual: &Path) -> bool {
    // If link_path already exists as a symlink to actual, not a loop
    if link_path.is_symlink() {
        if let Ok(existing_target) = fs::read_link(link_path) {
            if let Ok(existing_resolved) = existing_target.canonicalize() {
                if let Ok(actual_resolved) = actual.canonicalize() {
                    if existing_resolved == actual_resolved {
                        return false;
                    }
                }
            }
        }
    }
    
    let link_absolute = link_path.to_path_buf();
    let actual_resolved = match actual.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    
    // Check: is link_path inside a symlinked directory?
    let mut symlink_parent = None;
    let mut current = match link_absolute.parent() {
        Some(p) => p.to_path_buf(),
        None => return false,
    };
    
    while current.parent() != Some(&current) {
        if current.is_symlink() {
            if let Ok(target) = fs::read_link(&current) {
                let target = if target.is_absolute() {
                    target
                } else {
                    current.parent().unwrap_or(&current).join(&target)
                };
                
                if let Ok(target_resolved) = target.canonicalize() {
                    symlink_parent = Some((current.clone(), target_resolved));
                    break;
                }
            }
        }
        
        current = match current.parent() {
            Some(p) => p.to_path_buf(),
            None => break,
        };
    }
    
    let (symlink_source, symlink_target) = match symlink_parent {
        Some(pair) => pair,
        None => return false,
    };
    
    // Check if actual is inside the symlink target directory
    if let Ok(rel_from_target) = actual_resolved.strip_prefix(&symlink_target) {
        // Get relative path from symlink source to link_path
        if let Ok(rel_from_source) = link_absolute.strip_prefix(&symlink_source) {
            if rel_from_source == rel_from_target {
                return true;
            }
        }
    }
    
    false
}

/// Check if link_path and actual would conflict (one inside the other).
pub fn paths_would_conflict(link_path: &Path, actual: &Path) -> bool {
    let link_absolute = link_path.to_path_buf();
    let actual_resolved = match actual.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    
    // Same path
    if link_absolute == actual_resolved {
        return true;
    }
    
    // Check if link_path is inside actual's directory tree
    if let Ok(_) = link_absolute.strip_prefix(&actual_resolved) {
        return true;
    }
    
    false
}

pub fn create_symlink(actual_path: &str, link: &str, cli: &Cli) -> Result<(), String> {
    let actual = expand_path(actual_path).canonicalize()
        .map_err(|e| format!("Source path does not exist: {}: {}", actual_path, e))?;
    
    let link_path = expand_path(link);
    
    // Check if parent directory is a symlink
    if let Some(link_parent) = link_path.parent() {
        if link_parent.is_symlink() && !actual.is_dir() {
            if let Ok(parent_target) = fs::read_link(link_parent) {
                let parent_target = if parent_target.is_absolute() {
                    parent_target
                } else {
                    link_parent.parent().unwrap_or(link_parent).join(&parent_target)
                };
                
                if let Ok(parent_target_resolved) = parent_target.canonicalize() {
                    if actual.starts_with(&parent_target_resolved) {
                        // Parent symlink issue detected
                        if cli.force {
                            log(cli, "info", &format!("Removing parent symlink {}", link_parent.display()));
                            fs::remove_file(link_parent).map_err(|e| e.to_string())?;
                            fs::create_dir_all(link_parent).map_err(|e| e.to_string())?;
                        } else if cli.interactive {
                            log(cli, "warning", &format!("Parent directory {} is a symlink to {}", 
                                link_parent.display(), parent_target_resolved.display()));
                            print!("Remove {} and create real directory? [y/n] ", link_parent.display());
                            io::Write::flush(&mut io::stdout()).ok();
                            let mut input = String::new();
                            io::stdin().read_line(&mut input).ok();
                            if input.trim().to_lowercase() != "y" {
                                return Err("Would overwrite actual file (parent is symlink)".to_string());
                            }
                            fs::remove_file(link_parent).map_err(|e| e.to_string())?;
                            fs::create_dir_all(link_parent).map_err(|e| e.to_string())?;
                        } else {
                            log(cli, "warning", &format!("Parent directory {} is a symlink to {}", 
                                link_parent.display(), parent_target_resolved.display()));
                            return Err("Would overwrite actual file (parent is symlink)".to_string());
                        }
                    }
                }
            }
        }
    }
    
    // Check if link already exists
    if link_path.exists() || link_path.is_symlink() {
        if link_path.is_symlink() {
            if let Ok(existing_target) = fs::read_link(&link_path) {
                let existing_resolved = expand_path(&existing_target.to_string_lossy());
                if let Ok(existing_canon) = existing_resolved.canonicalize() {
                    if existing_canon == actual {
                        log(cli, "debug", "Symlink already exists, skipping");
                        return Ok(());
                    }
                }
            }
        }
        
        // Handle existing file/link
        if cli.interactive {
            print!("Link {} exists, remove it? [y/n] ", link_path.display());
            io::Write::flush(&mut io::stdout()).ok();
            let mut input = String::new();
            io::stdin().read_line(&mut input).ok();
            if input.trim().to_lowercase() != "y" {
                return Ok(());
            }
        } else if !cli.force {
            return Err(format!("Path exists, use --force or --interactive to overwrite: {}", link_path.display()));
        }
        
        if cli.dry_run {
            log(cli, "debug", &format!("Would remove {}", link_path.display()));
        } else {
            log(cli, "debug", &format!("Removing {}", link_path.display()));
            if link_path.is_dir() && !link_path.is_symlink() {
                fs::remove_dir_all(&link_path).map_err(|e| e.to_string())?;
            } else {
                fs::remove_file(&link_path).map_err(|e| e.to_string())?;
            }
        }
    }
    
    // Check for path conflict
    if paths_would_conflict(&link_path, &actual) {
        log(cli, "warning", &format!("Path conflict: {} and {} would conflict!", link_path.display(), actual.display()));
        return Err("Path conflict detected".to_string());
    }
    
    // Check for symlink loop
    if would_create_symlink_loop(&link_path, &actual) {
        log(cli, "warning", &format!("Creating symlink {} -> {} would create a loop!", link_path.display(), actual.display()));
        
        // For directories, offer to create real directory instead
        if actual.is_dir() && cli.interactive {
            print!("Create real directory at {} instead? [y/n] ", link_path.display());
            io::Write::flush(&mut io::stdout()).ok();
            let mut input = String::new();
            io::stdin().read_line(&mut input).ok();
            if input.trim().to_lowercase() == "y" {
                if cli.dry_run {
                    log(cli, "debug", &format!("Would create directory {}", link_path.display()));
                } else {
                    fs::create_dir_all(&link_path).map_err(|e| e.to_string())?;
                }
                return Ok(());
            }
        }
        
        return Err("Symlink loop detected, skipped".to_string());
    }
    
    // Check for circular symlink scenario
    if let Some((circular_symlink, link_parent)) = detect_circular_symlink_scenario(&link_path, &actual) {
        log(cli, "warning", "Circular symlink scenario detected!");
        log(cli, "warning", &format!("Creating {} -> {} when {} -> {}", 
            link_path.display(), actual.display(), circular_symlink.display(), link_parent.display()));
        
        if cli.interactive {
            print!("Remove {} and create real directory? [y/n] ", circular_symlink.display());
            io::Write::flush(&mut io::stdout()).ok();
            let mut input = String::new();
            io::stdin().read_line(&mut input).ok();
            if input.trim().to_lowercase() != "y" {
                return Err("Circular symlink scenario detected, skipped".to_string());
            }
            
            if cli.dry_run {
                log(cli, "info", &format!("Would remove symlink {}", circular_symlink.display()));
            } else {
                fs::remove_file(&circular_symlink).map_err(|e| e.to_string())?;
                fs::create_dir_all(&circular_symlink).map_err(|e| e.to_string())?;
            }
        } else {
            log(cli, "warning", "Skipping this link to prevent circular reference (use -i to fix interactively)");
            return Err("Circular symlink scenario detected, skipped".to_string());
        }
    }
    
    // Create symlink
    if cli.dry_run {
        log(cli, "info", &format!("Would create symlink: {} -> {}", link_path.display(), actual.display()));
    } else {
        // Create parent directories if needed
        if let Some(parent) = link_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| format!("Failed to create parent directory: {}", e))?;
            }
        }
        
        unix_fs::symlink(&actual, &link_path)
            .map_err(|e| format!("Failed to create symlink: {}", e))?;
        log(cli, "debug", &format!("Created symlink: {} -> {}", link_path.display(), actual.display()));
    }
    
    Ok(())
}
