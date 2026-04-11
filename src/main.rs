mod cli;
mod commands;
mod config;
mod permissions;
mod symlink;

use clap::Parser;
use cli::Cli;
use std::path::PathBuf;

fn main() {
    let cli = Cli::parse();

    let result = commands::dispatch(&cli);

    match result {
        Ok(()) => std::process::exit(0),
        Err(e) => {
            commands::log(&cli, "error", &e);
            std::process::exit(1);
        }
    }
}

pub fn expand_path(path: &str) -> PathBuf {
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
