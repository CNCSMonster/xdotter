mod cli;
mod config;

use clap::Parser;
use clap::CommandFactory;
use clap_complete::{generate, Shell as ClapShell};
use cli::{Cli, Command};
use config::{Config, detect_format, validate_toml, validate_json};
use std::fs;
use std::io;
use std::os::unix::fs as unix_fs;
use std::path::{Path, PathBuf};

fn main() {
    let cli = Cli::parse();
    
    let result = match cli.command.as_ref().unwrap_or(&Command::Deploy) {
        Command::Deploy => cmd_deploy(&cli),
        Command::Undeploy => cmd_undeploy(&cli),
        Command::CheckPermissions => cmd_check_permissions(&cli),
        Command::Validate { files } => cmd_validate(&cli, &files.iter().map(|p| p.to_path_buf()).collect::<Vec<_>>()),
        Command::New => cmd_new(&cli),
        Command::Completion { shell } => cmd_completion(&cli, shell),
        Command::Version => cmd_version(&cli),
    };
    
    match result {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn log(cli: &Cli, level: &str, msg: &str) {
    if cli.quiet {
        return;
    }
    
    match level {
        "info" => if cli.verbose || !cli.quiet { println!("{}", msg); },
        "debug" => if cli.verbose { println!("[DEBUG] {}", msg); },
        "warning" => println!("\x1b[1;33m[WARNING] {}\x1b[0m", msg),
        "error" => eprintln!("\x1b[0;31m[ERROR] {}\x1b[0m", msg),
        _ => println!("{}", msg),
    }
}

fn cmd_help(_cli: &Cli) -> Result<(), String> {
    let mut cmd = Cli::command();
    let _ = cmd.print_help().map_err(|e| e.to_string())?;
    println!();
    Ok(())
}

fn cmd_version(cli: &Cli) -> Result<(), String> {
    log(cli, "info", "0.4.0");
    Ok(())
}

fn cmd_completion(_cli: &Cli, shell: &str) -> Result<(), String> {
    let mut cmd = Cli::command();
    let clap_shell = match shell.to_lowercase().as_str() {
        "bash" => ClapShell::Bash,
        "zsh" => ClapShell::Zsh,
        "fish" => ClapShell::Fish,
        _ => return Err(format!("Unsupported shell: {}", shell)),
    };
    
    generate(clap_shell, &mut cmd, "xd", &mut io::stdout());
    Ok(())
}

fn cmd_new(cli: &Cli) -> Result<(), String> {
    let config_path = Path::new("xdotter.toml");
    if config_path.exists() {
        return Err("xdotter.toml already exists".to_string());
    }
    
    let content = r#"# xdotter configuration file

[links]
# Format: "source_path" = "target_link"
# The source is your actual dotfile in the repo
# The target is where you want it symlinked (~ expands to home directory)

# ".config/nvim/init.lua" = "~/.config/nvim/init.lua"
# ".zshrc" = "~/.zshrc"
# ".gitconfig" = "~/.gitconfig"

[dependencies]
# Format: "name" = "relative_path"
# Subdirectories with their own xdotter.toml
# "go" = "testdata/go"
# "nvim" = "config/nvim"
"#;
    
    if cli.dry_run {
        log(cli, "info", "Would create xdotter.toml");
    } else {
        fs::write(config_path, content).map_err(|e| format!("Failed to write config: {}", e))?;
        log(cli, "info", "Created xdotter.toml");
    }
    
    Ok(())
}

fn cmd_deploy(cli: &Cli) -> Result<(), String> {
    log(cli, "info", "Deploying...");
    
    let config_path = Path::new("xdotter.toml");
    if !config_path.exists() {
        return Err(format!("Config file not found: {}", config_path.display()));
    }
    
    // Auto-validate unless --no-validate
    if !cli.no_validate {
        if let Err(e) = validate_config(config_path) {
            log(cli, "error", &e);
            return Err("Config validation failed".to_string());
        }
    }
    
    let content = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let format = detect_format(config_path).unwrap_or("toml");
    let config = if format == "json" {
        Config::from_json(&content)?
    } else {
        Config::from_toml(&content)?
    };
    
    log(cli, "debug", &format!("Deploying from {}", config_path.display()));
    
    let mut success = true;
    
    for (actual_path, link) in &config.links {
        log(cli, "info", &format!("deploy: {} -> {}", actual_path, link));
        if let Err(e) = create_symlink(actual_path, link, cli) {
            log(cli, "error", &format!("failed to create link: {}", e));
            success = false;
        }
    }
    
    for (dep_name, dep_path) in &config.dependencies {
        log(cli, "debug", &format!("dependency: {}, path: {}", dep_name, dep_path));
        let dep_dir = std::env::current_dir().map_err(|e| e.to_string())?.join(dep_path);
        let dep_config = dep_dir.join("xdotter.toml");
        if dep_config.exists() {
            // Recursive deploy (simplified)
            log(cli, "debug", &format!("entering {}", dep_dir.display()));
        }
    }
    
    if success {
        Ok(())
    } else {
        Err("Some links failed to deploy".to_string())
    }
}

fn cmd_undeploy(cli: &Cli) -> Result<(), String> {
    log(cli, "info", "Undeploying...");
    
    let config_path = Path::new("xdotter.toml");
    if !config_path.exists() {
        return Err(format!("Config file not found: {}", config_path.display()));
    }
    
    let content = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let format = detect_format(config_path).unwrap_or("toml");
    let config = if format == "json" {
        Config::from_json(&content)?
    } else {
        Config::from_toml(&content)?
    };
    
    let mut success = true;
    
    for (link, _actual_path) in &config.links {
        let link_path = expand_path(link);
        if link_path.is_symlink() {
            if cli.dry_run {
                log(cli, "info", &format!("Would remove symlink: {}", link_path.display()));
            } else {
                if cli.interactive {
                    print!("Remove symlink {}? [y/n] ", link_path.display());
                    io::Write::flush(&mut io::stdout()).ok();
                    let mut input = String::new();
                    io::stdin().read_line(&mut input).ok();
                    if input.trim().to_lowercase() != "y" {
                        continue;
                    }
                }
                log(cli, "debug", &format!("Removing {}", link_path.display()));
                if let Err(e) = fs::remove_file(&link_path) {
                    log(cli, "error", &format!("Failed to remove {}: {}", link_path.display(), e));
                    success = false;
                }
            }
        } else if link_path.exists() {
            log(cli, "warning", &format!("Target is not a symlink: {}", link_path.display()));
            success = false;
        } else {
            log(cli, "debug", &format!("Link does not exist: {}", link_path.display()));
        }
    }
    
    if success {
        Ok(())
    } else {
        Err("Some links failed to undeploy".to_string())
    }
}

fn cmd_check_permissions(cli: &Cli) -> Result<(), String> {
    log(cli, "info", "Checking permissions...");
    // Simplified - full implementation would check all sensitive paths
    Ok(())
}

fn cmd_validate(cli: &Cli, files: &[PathBuf]) -> Result<(), String> {
    if files.is_empty() {
        // Validate default files
        let defaults = ["xdotter.toml", "xdotter.json"];
        let mut found = false;
        for f in &defaults {
            let path = Path::new(f);
            if path.exists() {
                if let Err(e) = validate_config(path) {
                    log(cli, "error", &e);
                    return Err("Validation failed".to_string());
                }
                log(cli, "info", &format!("✓ {} is valid", f));
                found = true;
            }
        }
        if !found {
            return Err("No default config file found".to_string());
        }
    } else {
        for filepath in files {
            if !filepath.exists() {
                log(cli, "error", &format!("File not found: {}", filepath.display()));
                return Err("Validation failed".to_string());
            }
            if let Err(e) = validate_config(filepath) {
                log(cli, "error", &e);
                return Err("Validation failed".to_string());
            }
            log(cli, "info", &format!("✓ {} is valid", filepath.display()));
        }
    }
    
    Ok(())
}

fn validate_config(filepath: &Path) -> Result<(), String> {
    let content = fs::read_to_string(filepath)
        .map_err(|e| format!("Cannot read file: {}", e))?;
    
    match detect_format(filepath) {
        Some("toml") => validate_toml(&content),
        Some("json") => validate_json(&content),
        _ => Err(format!("Unknown file format: {}", filepath.display())),
    }
}

fn create_symlink(actual_path: &str, link: &str, cli: &Cli) -> Result<(), String> {
    let actual = expand_path(actual_path).canonicalize()
        .map_err(|e| format!("Source path does not exist: {}: {}", actual_path, e))?;
    
    let link_path = expand_path(link);
    
    // Check if parent directory is a symlink
    let link_parent = link_path.parent().ok_or("Invalid link path")?;
    if link_parent.is_symlink() && !actual.is_dir() {
        if let Some(parent_target) = read_symlink_target(link_parent) {
            let parent_target_resolved = expand_path(&parent_target);
            if actual.starts_with(&parent_target_resolved) {
                // Parent symlink issue - fix with --force or warn
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
                    if input.trim().to_lowercase() == "y" {
                        fs::remove_file(link_parent).map_err(|e| e.to_string())?;
                        fs::create_dir_all(link_parent).map_err(|e| e.to_string())?;
                    } else {
                        return Err("Would overwrite actual file (parent is symlink)".to_string());
                    }
                } else {
                    log(cli, "warning", &format!("Parent directory {} is a symlink to {}", 
                        link_parent.display(), parent_target_resolved.display()));
                    return Err("Would overwrite actual file (parent is symlink)".to_string());
                }
            }
        }
    }
    
    // Check if link already exists
    if link_path.exists() || link_path.is_symlink() {
        if link_path.is_symlink() {
            if let Some(existing) = read_symlink_target(&link_path) {
                let existing_resolved = expand_path(&existing);
                if existing_resolved == actual {
                    log(cli, "debug", "Symlink already exists, skipping");
                    return Ok(());
                }
            }
        }
        
        if cli.interactive {
            print!("Link {} exists, remove it? [y/n] ", link_path.display());
            io::Write::flush(&mut io::stdout()).ok();
            let mut input = String::new();
            io::stdin().read_line(&mut input).ok();
            if input.trim().to_lowercase() != "y" {
                return Ok(());
            }
        }
        
        if !cli.force && !cli.interactive {
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

fn expand_path(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/") {
        let home = std::env::var("HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(dirs::home_dir)
            .unwrap_or_default();
        return home.join(stripped);
    }
    PathBuf::from(path)
}

fn read_symlink_target(path: &Path) -> Option<String> {
    std::fs::read_link(path).ok().map(|p| p.to_string_lossy().to_string())
}
