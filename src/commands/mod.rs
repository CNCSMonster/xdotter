use crate::cli::{Cli, Command};
use crate::config::{detect_format, validate_toml, Config};
use crate::expand_path;
use crate::permissions::{check_permission, fix_permission, get_required_permission};
use crate::symlink;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const VERSION: &str = env!("CARGO_PKG_VERSION");

// Completion scripts are generated at build time into OUT_DIR
const BASH_COMPLETION: &str = include_str!(concat!(env!("OUT_DIR"), "/xd.bash"));
const ZSH_COMPLETION: &str = include_str!(concat!(env!("OUT_DIR"), "/_xd"));
const FISH_COMPLETION: &str = include_str!(concat!(env!("OUT_DIR"), "/xd.fish"));

pub fn log(cli: &Cli, level: &str, msg: &str) {
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

pub fn dispatch(cli: &Cli) -> Result<(), String> {
    match cli.command.as_ref().unwrap_or(&Command::Deploy) {
        Command::Deploy => cmd_deploy(cli, &mut HashSet::new()),
        Command::Undeploy => cmd_undeploy(cli),
        Command::Status => cmd_status(cli),
        Command::Validate { files } => cmd_validate(cli, files),
        Command::New => cmd_new(cli),
        Command::Completion { shell } => cmd_completion(shell),
        Command::Version => cmd_version(cli),
    }
}

fn cmd_version(_cli: &Cli) -> Result<(), String> {
    println!("xdotter {}", VERSION);
    Ok(())
}

fn cmd_completion(shell: &str) -> Result<(), String> {
    let script = match shell.to_lowercase().as_str() {
        "bash" => BASH_COMPLETION,
        "zsh" => ZSH_COMPLETION,
        "fish" => FISH_COMPLETION,
        _ => {
            return Err(format!(
                "Unsupported shell: {}. Supported: bash, zsh, fish",
                shell
            ))
        }
    };
    print!("{}", script);
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
    let content = fs::read_to_string(filepath).map_err(|e| format!("Cannot read file: {}", e))?;

    match detect_format(filepath) {
        Some("toml") => validate_toml(&content),
        _ => Err(format!(
            "Unknown file format: {}. Only TOML is supported.",
            filepath.display()
        )),
    }
}

fn cmd_status(cli: &Cli) -> Result<(), String> {
    let config_path = Path::new("xdotter.toml");
    if !config_path.exists() {
        return Err(format!("Config file not found: {}", config_path.display()));
    }

    let content =
        fs::read_to_string(config_path).map_err(|e| format!("Failed to read config: {}", e))?;

    let config = Config::from_toml(&content)?;

    let mut total = 0;
    let mut valid = 0;
    let mut broken = 0;
    let mut perm_issues = 0;

    for link in config.links.values() {
        total += 1;
        let link_path = expand_path(link);

        if link_path.is_symlink() {
            if let Ok(resolved) = fs::read_link(&link_path) {
                // Check if target exists
                let target_exists = expand_path(&resolved.to_string_lossy()).exists();
                if target_exists {
                    valid += 1;
                    // Check permissions
                    if let Ok(canonical) = link_path.canonicalize() {
                        if let Some((required_mode, description)) =
                            get_required_permission(&link_path)
                        {
                            if !check_permission(&canonical, required_mode) {
                                perm_issues += 1;
                                if !cli.quiet {
                                    log(
                                        cli,
                                        "warning",
                                        &format!(
                                            "{} -> {} ({}: expected {:03o})",
                                            link_path.display(),
                                            canonical.display(),
                                            description,
                                            required_mode
                                        ),
                                    );
                                }
                            } else if cli.verbose {
                                log(
                                    cli,
                                    "info",
                                    &format!(
                                        "✓ {} -> {} ({:03o})",
                                        link_path.display(),
                                        canonical.display(),
                                        required_mode
                                    ),
                                );
                            }
                        } else if cli.verbose {
                            log(
                                cli,
                                "info",
                                &format!("✓ {} -> {}", link_path.display(), canonical.display()),
                            );
                        }
                    }
                } else {
                    broken += 1;
                    log(
                        cli,
                        "warning",
                        &format!(
                            "✗ {} -> {} (broken: target missing)",
                            link_path.display(),
                            resolved.display()
                        ),
                    );
                }
            }
        } else if link_path.exists() {
            broken += 1;
            log(
                cli,
                "warning",
                &format!(
                    "✗ {} (not a symlink, regular file exists)",
                    link_path.display()
                ),
            );
        }
        // else: link doesn't exist, not deployed yet (not counted as broken)
    }

    if !cli.quiet {
        println!();
        println!("Status: {}/{} deployed", valid, total);
        if broken > 0 {
            println!("Broken links: {}", broken);
        }
        if perm_issues > 0 {
            println!("Permission issues: {}", perm_issues);
        }
    }

    if perm_issues > 0 {
        Err("Permission issues found. Use --fix-permissions to fix.".to_string())
    } else {
        Ok(())
    }
}

fn cmd_validate(cli: &Cli, files: &[PathBuf]) -> Result<(), String> {
    if files.is_empty() {
        let defaults = ["xdotter.toml"];
        let mut found = false;
        for f in &defaults {
            let path = Path::new(f);
            if path.exists() {
                if let Err(e) = validate_config_file(path) {
                    eprintln!("{}", e);
                    return Err("Validation failed".to_string());
                }
                log(cli, "info", &format!("✓ {} is Valid", f));
                found = true;
            }
        }
        if !found {
            return Err("No default config file found (xdotter.toml)".to_string());
        }
    } else {
        let mut all_valid = true;
        for filepath in files {
            if !filepath.exists() {
                log(
                    cli,
                    "error",
                    &format!("File not found: {}", filepath.display()),
                );
                all_valid = false;
                continue;
            }
            if let Err(e) = validate_config_file(filepath) {
                eprintln!("{}: {}", filepath.display(), e);
                all_valid = false;
            } else {
                log(cli, "info", &format!("✓ {} is Valid", filepath.display()));
            }
        }
        if !all_valid {
            return Err("Validation failed".to_string());
        }
    }

    Ok(())
}

fn cmd_deploy(cli: &Cli, visited: &mut HashSet<PathBuf>) -> Result<(), String> {
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

    let content =
        fs::read_to_string(config_path).map_err(|e| format!("Failed to read config: {}", e))?;

    let config = Config::from_toml(&content)?;

    log(
        cli,
        "debug",
        &format!("Deploying from {}", config_path.display()),
    );

    // P0: Detect dependency cycle using canonical path
    let current_dir = std::env::current_dir().map_err(|e| e.to_string())?;
    let canon_dir = current_dir.canonicalize().map_err(|e| e.to_string())?;
    if !visited.insert(canon_dir.clone()) {
        return Err(format!(
            "Dependency cycle detected: {} is already being processed",
            canon_dir.display()
        ));
    }

    let mut success = true;

    for (actual_path, link) in &config.links {
        log(cli, "info", &format!("deploy: {} -> {}", actual_path, link));

        // P1: Check if source file is a symlink and refuse to fix permissions on it
        let src_path = expand_path(actual_path);
        // Handle both absolute and relative paths
        let src_check = if src_path.is_relative() {
            // For relative paths, resolve from current directory
            std::env::current_dir()
                .ok()
                .map(|cwd| cwd.join(&src_path))
                .unwrap_or_else(|| src_path.clone())
        } else {
            src_path.clone()
        };
        if src_check.is_symlink() {
            return Err(format!(
                "Source file {} is a symlink. Permission fix refused to avoid modifying unintended files. Please resolve the symlink in your dotfile repository.",
                actual_path
            ));
        }

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
                                log(
                                    cli,
                                    "info",
                                    &format!(
                                        "Would fix permission for {} to {:03o}",
                                        link_path.display(),
                                        required_mode
                                    ),
                                );
                            } else {
                                if fix_permission(&resolved, required_mode) {
                                    log(
                                        cli,
                                        "info",
                                        &format!(
                                            "Fixed permission for {} to {:03o}",
                                            link_path.display(),
                                            required_mode
                                        ),
                                    );
                                } else {
                                    log(
                                        cli,
                                        "error",
                                        &format!(
                                            "Failed to fix permission for {}",
                                            link_path.display()
                                        ),
                                    );
                                    success = false;
                                }
                            }
                        } else if cli.check_permissions {
                            log(
                                cli,
                                "warning",
                                &format!(
                                    "{}: {} has wrong permission (expected {:03o})",
                                    description,
                                    link_path.display(),
                                    required_mode
                                ),
                            );
                        }
                    }
                }
            }
        }
    }

    // Process dependencies
    for (dep_name, dep_path) in &config.dependencies {
        log(
            cli,
            "debug",
            &format!("dependency: {}, path: {}", dep_name, dep_path),
        );
        let dep_dir = canon_dir.join(dep_path);
        let dep_config = dep_dir.join("xdotter.toml");
        if dep_config.exists() {
            log(cli, "debug", &format!("entering {}", dep_dir.display()));
            // Save and restore current directory
            if let Err(e) = std::env::set_current_dir(&dep_dir) {
                log(cli, "error", &format!("Cannot enter dependency dir: {}", e));
                success = false;
                continue;
            }
            if let Err(e) = cmd_deploy(cli, visited) {
                let _ = std::env::set_current_dir(&canon_dir);
                // If the error is a cycle, propagate it up immediately
                if e.contains("Dependency cycle detected") {
                    return Err(e);
                }
                log(cli, "error", &format!("Dependency deploy failed: {}", e));
                success = false;
            } else {
                let _ = std::env::set_current_dir(&canon_dir);
            }
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

    let content =
        fs::read_to_string(config_path).map_err(|e| format!("Failed to read config: {}", e))?;

    let config = Config::from_toml(&content)?;

    let mut success = true;

    for link in config.links.values() {
        let link_path = expand_path(link);
        if link_path.is_symlink() {
            if cli.dry_run {
                log(
                    cli,
                    "info",
                    &format!("Would remove symlink: {}", link_path.display()),
                );
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
                    log(
                        cli,
                        "error",
                        &format!("Failed to remove {}: {}", link_path.display(), e),
                    );
                    success = false;
                }
            }
        } else if link_path.exists() {
            log(
                cli,
                "warning",
                &format!("Target is not a symlink: {}", link_path.display()),
            );
            if !cli.force {
                success = false;
            }
        } else {
            log(
                cli,
                "debug",
                &format!("Link does not exist: {}", link_path.display()),
            );
        }
    }

    if success {
        Ok(())
    } else {
        Err("Some links failed to undeploy".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Cli;
    #[cfg(unix)]
    use std::os::unix::fs as unix_fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn test_dir(name: &str) -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("xd_cmd_{}_{}_{}", name, std::process::id(), id));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    /// Set a unique HOME for this test to avoid env race conditions.
    /// Returns the previous HOME value (if any) so it can be restored.
    fn set_unique_home() -> Option<String> {
        let old = std::env::var("HOME").ok();
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let home = std::env::temp_dir().join(format!("xd_home_{}_{}", std::process::id(), id));
        let _ = fs::create_dir_all(&home);
        std::env::set_var("HOME", &home);
        old
    }

    fn restore_home(old: Option<String>) {
        if let Some(v) = old {
            std::env::set_var("HOME", &v);
        } else {
            std::env::remove_var("HOME");
        }
    }

    fn make_cli() -> Cli {
        Cli {
            command: None,
            verbose: false,
            quiet: true,
            dry_run: false,
            interactive: false,
            force: false,
            check_permissions: false,
            fix_permissions: false,
            no_validate: true,
        }
    }

    // ============================================================
    // P0: Dependency cycle detection tests
    // ============================================================

    #[test]
    fn test_deploy_detects_cycle() {
        let dir = test_dir("cycle");
        let a = dir.join("a");
        let b = dir.join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();

        // a depends on b, b depends on a → cycle
        fs::write(
            a.join("xdotter.toml"),
            r#"
[links]

[dependencies]
"b" = "../b"
"#,
        )
        .unwrap();
        fs::write(
            b.join("xdotter.toml"),
            r#"
[links]

[dependencies]
"a" = "../a"
"#,
        )
        .unwrap();

        let cli = make_cli();
        let prev = std::env::current_dir().unwrap();
        let _old_home = set_unique_home();
        std::env::set_current_dir(&a).unwrap();
        let mut visited = HashSet::new();
        let result = cmd_deploy(&cli, &mut visited);
        std::env::set_current_dir(&prev).unwrap();
        restore_home(_old_home);

        let err_msg = result.expect_err("Should detect cycle");
        eprintln!("Actual error: {}", err_msg);
        assert!(
            err_msg.contains("Dependency cycle detected") || err_msg.contains("cycle"),
            "Error should mention cycle: {}",
            err_msg
        );

        cleanup(&dir);
    }

    #[test]
    fn test_deploy_no_cycle_linear() {
        let dir = test_dir("linear");
        let a = dir.join("a");
        let b = dir.join("b");
        fs::create_dir_all(&a).unwrap();
        fs::create_dir_all(&b).unwrap();

        // a depends on b, but b does NOT depend on a → no cycle
        fs::write(
            a.join("xdotter.toml"),
            r#"
[links]

[dependencies]
"b" = "../b"
"#,
        )
        .unwrap();
        fs::write(
            b.join("xdotter.toml"),
            r#"
[links]
"#,
        )
        .unwrap();

        let cli = make_cli();
        let prev = std::env::current_dir().unwrap();
        let _old_home = set_unique_home();
        std::env::set_current_dir(&a).unwrap();
        let mut visited = HashSet::new();
        let result = cmd_deploy(&cli, &mut visited);
        std::env::set_current_dir(&prev).unwrap();
        restore_home(_old_home);

        // Should succeed (no cycle)
        assert!(
            result.is_ok(),
            "Linear dependency should work: {:?}",
            result
        );

        cleanup(&dir);
    }

    // ============================================================
    // P1: Symlink source file rejection tests
    // ============================================================

    #[test]
    #[cfg(unix)]
    fn test_deploy_rejects_symlink_source() {
        let dir = test_dir("symlink_src");
        let real_file = dir.join("real.txt");
        let link_file = dir.join("link.txt");
        fs::write(&real_file, "content").unwrap();
        unix_fs::symlink(&real_file, &link_file).unwrap();

        // Config uses symlink as source
        let target = format!("/tmp/xd_test_target_{}.txt", std::process::id());
        fs::write(
            dir.join("xdotter.toml"),
            format!(
                r#"
[links]
"link.txt" = "{}"
"#,
                target
            ),
        )
        .unwrap();

        let cli = Cli {
            quiet: false,
            no_validate: true,
            ..make_cli()
        };
        let prev = std::env::current_dir().unwrap();
        let _old_home = set_unique_home();
        std::env::set_current_dir(&dir).unwrap();
        let mut visited = HashSet::new();
        let result = cmd_deploy(&cli, &mut visited);
        std::env::set_current_dir(&prev).unwrap();
        restore_home(_old_home);

        let err_msg = result.expect_err("Should reject symlink source");
        eprintln!("Actual error: {}", err_msg);
        assert!(
            err_msg.contains("Source file") && err_msg.contains("symlink"),
            "Error should mention source symlink: {}",
            err_msg
        );

        // Cleanup
        let _ = fs::remove_file(&target);
        cleanup(&dir);
    }
}
