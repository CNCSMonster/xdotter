mod cli;
mod config;
mod permissions;
mod symlink;

use clap::Parser;
use clap::CommandFactory;
use clap_complete::{generate, Shell as ClapShell};
use cli::{Cli, Command};
use config::{Config, detect_format, validate_toml, validate_json};
use permissions::{get_required_permission, check_permission, fix_permission};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const VERSION: &str = "0.4.0";

fn main() {
    let cli = Cli::parse();
    
    let result = match cli.command.as_ref().unwrap_or(&Command::Deploy) {
        Command::Deploy => cmd_deploy(&cli),
        Command::Undeploy => cmd_undeploy(&cli),
        Command::CheckPermissions => cmd_check_permissions(&cli),
        Command::Validate { files } => cmd_validate(&cli, files),
        Command::New => cmd_new(&cli),
        Command::Completion { shell } => cmd_completion(&cli, shell),
        Command::Version => cmd_version(&cli),
    };
    
    match result {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            log(&cli, "error", &e);
            std::process::exit(1);
        }
    }
}

fn log(cli: &Cli, level: &str, msg: &str) {
    if cli.quiet {
        return;
    }
    
    match level {
        "info" => {
            if cli.verbose || !cli.quiet {
                println!("{}", msg);
            }
        }
        "debug" => {
            if cli.verbose {
                println!("[DEBUG] {}", msg);
            }
        }
        "warning" => println!("\x1b[1;33m[WARNING] {}\x1b[0m", msg),
        "error" => eprintln!("\x1b[0;31m[ERROR] {}\x1b[0m", msg),
        _ => println!("{}", msg),
    }
}

fn cmd_version(_cli: &Cli) -> Result<(), String> {
    println!("{}", VERSION);
    Ok(())
}

fn cmd_completion(_cli: &Cli, shell: &str) -> Result<(), String> {
    let mut cmd = Cli::command();
    let clap_shell = match shell.to_lowercase().as_str() {
        "bash" => ClapShell::Bash,
        "zsh" => ClapShell::Zsh,
        "fish" => ClapShell::Fish,
        _ => return Err(format!("Unsupported shell: {}. Supported: bash, zsh, fish", shell)),
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

fn validate_config_file(filepath: &Path) -> Result<(), String> {
    let content = fs::read_to_string(filepath)
        .map_err(|e| format!("Cannot read file: {}", e))?;
    
    match detect_format(filepath) {
        Some("toml") => validate_toml(&content),
        Some("json") => validate_json(&content),
        _ => Err(format!("Unknown file format: {}", filepath.display())),
    }
}

fn cmd_validate(cli: &Cli, files: &[PathBuf]) -> Result<(), String> {
    if files.is_empty() {
        let defaults = ["xdotter.toml", "xdotter.json"];
        let mut found = false;
        for f in &defaults {
            let path = Path::new(f);
            if path.exists() {
                if let Err(e) = validate_config_file(path) {
                    eprintln!("{}", e);
                    return Err("Validation failed".to_string());
                }
                log(cli, "info", &format!("✓ {} is valid", f));
                found = true;
            }
        }
        if !found {
            return Err("No default config file found (xdotter.toml or xdotter.json)".to_string());
        }
    } else {
        for filepath in files {
            if !filepath.exists() {
                log(cli, "error", &format!("File not found: {}", filepath.display()));
                return Err("Validation failed".to_string());
            }
            if let Err(e) = validate_config_file(filepath) {
                eprintln!("{}", e);
                return Err("Validation failed".to_string());
            }
            log(cli, "info", &format!("✓ {} is valid", filepath.display()));
        }
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
        if let Err(e) = validate_config_file(config_path) {
            log(cli, "error", &e);
            return Err("Config validation failed".to_string());
        }
        log(cli, "debug", "Config validation passed");
    }
    
    let content = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let fmt = detect_format(config_path).unwrap_or("toml");
    let config = if fmt == "json" {
        Config::from_json(&content)?
    } else {
        Config::from_toml(&content)?
    };
    
    log(cli, "debug", &format!("Deploying from {}", config_path.display()));
    
    let mut success = true;
    
    for (actual_path, link) in &config.links {
        log(cli, "info", &format!("deploy: {} -> {}", actual_path, link));
        if let Err(e) = symlink::create_symlink(actual_path, link, cli) {
            log(cli, "error", &format!("failed to create link: {}", e));
            success = false;
            continue;
        }

        // Permission check/fix after successful symlink creation
        let link_path = expand_path(link);
        if link_path.is_symlink() {
            if let Ok(resolved) = link_path.canonicalize() {
                if let Some((required_mode, description)) = get_required_permission(&link_path) {
                    let is_correct = check_permission(&resolved, required_mode);
                    if !is_correct {
                        if cli.fix_permissions {
                            if cli.dry_run {
                                log(cli, "info", &format!("Would fix permission for {} to {:03o}",
                                    link_path.display(), required_mode));
                            } else {
                                if fix_permission(&resolved, required_mode) {
                                    log(cli, "info", &format!("Fixed permission for {} to {:03o}",
                                        link_path.display(), required_mode));
                                } else {
                                    log(cli, "error", &format!("Failed to fix permission for {}",
                                        link_path.display()));
                                    success = false;
                                }
                            }
                        } else if cli.check_permissions {
                            log(cli, "warning", &format!("{}: {} has wrong permission (expected {:03o})",
                                description, link_path.display(), required_mode));
                        }
                    }
                }
            }
        }
    }
    
    // Process dependencies
    let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
    for (dep_name, dep_path) in &config.dependencies {
        log(cli, "debug", &format!("dependency: {}, path: {}", dep_name, dep_path));
        let dep_dir = current_dir.join(dep_path);
        let dep_config = dep_dir.join("xdotter.toml");
        if dep_config.exists() {
            log(cli, "debug", &format!("entering {}", dep_dir.display()));
            // Save and restore current directory
            let prev_dir = current_dir.clone();
            if let Err(e) = std::env::set_current_dir(&dep_dir) {
                log(cli, "error", &format!("Cannot enter dependency dir: {}", e));
                success = false;
                continue;
            }
            if let Err(e) = cmd_deploy(cli) {
                log(cli, "error", &format!("Dependency deploy failed: {}", e));
                success = false;
            }
            let _ = std::env::set_current_dir(&prev_dir);
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
    
    let fmt = detect_format(config_path).unwrap_or("toml");
    let config = if fmt == "json" {
        Config::from_json(&content)?
    } else {
        Config::from_toml(&content)?
    };
    
    let mut success = true;
    
    for (_actual_path, link) in &config.links {
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
                        log(cli, "debug", "Skipping");
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
            if !cli.force {
                success = false;
            }
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
    
    let config_path = Path::new("xdotter.toml");
    if !config_path.exists() {
        return Err(format!("Config file not found: {}", config_path.display()));
    }
    
    let content = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read config: {}", e))?;
    
    let fmt = detect_format(config_path).unwrap_or("toml");
    let config = if fmt == "json" {
        Config::from_json(&content)?
    } else {
        Config::from_toml(&content)?
    };
    
    let mut has_issues = false;
    let fix_mode = cli.fix_permissions;
    
    for (_actual_path, link) in &config.links {
        let link_path = expand_path(link);
        if !link_path.is_symlink() {
            continue;
        }
        
        // Resolve symlink to get actual file
        if let Ok(resolved) = link_path.canonicalize() {
            if let Some((required_mode, description)) = get_required_permission(&link_path) {
                let is_correct = check_permission(&resolved, required_mode);
                if is_correct {
                    if !cli.quiet {
                        println!("\x1b[0;32m✓\x1b[0m {}: {} (permission: {:03o})", 
                            description, link_path.display(), required_mode);
                    }
                } else {
                    if fix_mode {
                        if cli.dry_run {
                            log(cli, "info", &format!("Would fix permission for {} to {:03o}", 
                                link_path.display(), required_mode));
                        } else {
                            if fix_permission(&resolved, required_mode) {
                                log(cli, "info", &format!("Fixed permission for {} to {:03o}", 
                                    link_path.display(), required_mode));
                            } else {
                                log(cli, "error", &format!("Failed to fix permission for {}", 
                                    link_path.display()));
                                has_issues = true;
                            }
                        }
                    } else {
                        log(cli, "warning", &format!("{}: {} has wrong permission", 
                            description, link_path.display()));
                        has_issues = true;
                    }
                }
            }
        }
    }
    
    if has_issues && !fix_mode {
        Err("Permission issues found. Use --fix-permissions to fix them.".to_string())
    } else {
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_path_tilde() {
        // Test with HOME set
        std::env::set_var("HOME", "/home/testuser");
        let result = expand_path("~/.config/file.txt");
        assert_eq!(result, PathBuf::from("/home/testuser/.config/file.txt"));
    }

    #[test]
    fn test_expand_path_tilde_only() {
        std::env::set_var("HOME", "/home/testuser");
        let result = expand_path("~/");
        assert_eq!(result, PathBuf::from("/home/testuser/"));
    }

    #[test]
    fn test_expand_path_absolute() {
        // Absolute path should not be changed
        std::env::set_var("HOME", "/home/testuser");
        let result = expand_path("/etc/config.txt");
        assert_eq!(result, PathBuf::from("/etc/config.txt"));
    }

    #[test]
    fn test_expand_path_relative() {
        // Relative path should not be changed
        std::env::set_var("HOME", "/home/testuser");
        let result = expand_path("relative/path.txt");
        assert_eq!(result, PathBuf::from("relative/path.txt"));
    }

    #[test]
    fn test_expand_path_unicode_home() {
        std::env::set_var("HOME", "/home/ユーザー");
        let result = expand_path("~/.config/設定.txt");
        assert_eq!(result, PathBuf::from("/home/ユーザー/.config/設定.txt"));
    }

    #[test]
    fn test_expand_path_no_home() {
        // When HOME is not set, should fallback to dirs::home_dir or empty
        let old_home = std::env::var("HOME").ok();
        std::env::remove_var("HOME");

        let result = expand_path("~/.config/file.txt");
        // Result depends on dirs::home_dir, but should not panic
        assert!(result.to_str().is_some());

        if let Some(h) = old_home {
            std::env::set_var("HOME", &h);
        }
    }
}
