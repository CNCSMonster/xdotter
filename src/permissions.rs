use std::path::Path;

// Permission requirements for sensitive paths
// Format: (path_pattern, required_mode, description)
static SENSITIVE_PATHS: &[(&str, u32, &str)] = &[
    // SSH
    ("~/.ssh", 0o700, "SSH directory"),
    ("~/.ssh/id_rsa", 0o600, "SSH RSA private key"),
    ("~/.ssh/id_ed25519", 0o600, "SSH Ed25519 private key"),
    ("~/.ssh/id_ecdsa", 0o600, "SSH ECDSA private key"),
    ("~/.ssh/id_dsa", 0o600, "SSH DSA private key"),
    ("~/.ssh/authorized_keys", 0o600, "SSH authorized keys"),
    ("~/.ssh/config", 0o600, "SSH config"),
    ("~/.ssh/known_hosts", 0o644, "SSH known hosts"),
    // GPG
    ("~/.gnupg", 0o700, "GPG directory"),
    ("~/.gnupg/private-keys-v1.d", 0o700, "GPG private keys directory"),
    ("~/.gnupg/gpg.conf", 0o600, "GPG config"),
    // Shell configs
    ("~/.bashrc", 0o644, "Bash config"),
    ("~/.zshrc", 0o644, "Zsh config"),
    ("~/.bash_profile", 0o644, "Bash login profile"),
    ("~/.profile", 0o644, "Shell profile"),
    ("~/.zshenv", 0o644, "Zsh environment"),
    ("~/.zprofile", 0o644, "Zsh login profile"),
    // X11
    ("~/.xinitrc", 0o755, "X11 init script"),
    ("~/.xsession", 0o755, "X11 session script"),
    ("~/.xprofile", 0o644, "X session environment"),
    ("~/.Xauthority", 0o600, "X11 authentication"),
    // Other sensitive
    ("~/.netrc", 0o600, "Netrc password file"),
    ("~/.pgpass", 0o600, "PostgreSQL password file"),
    // Cloud/Service credentials
    ("~/.aws/credentials", 0o600, "AWS credentials"),
    ("~/.docker/config.json", 0o644, "Docker config"),
    ("~/.git-credentials", 0o600, "Git credentials"),
    ("~/.npmrc", 0o600, "NPM config"),
    ("~/.pypirc", 0o600, "PyPI config"),
    // Database
    ("~/.my.cnf", 0o600, "MySQL config"),
    ("~/.psqlrc", 0o644, "PostgreSQL config"),
    // Terminal
    ("~/.ssh", 0o700, "SSH directory"),
];

// Glob patterns for sensitive files (matched against filename only)
// Format: (glob_pattern, required_mode, description)
static SENSITIVE_PATTERNS: &[(&str, u32, &str)] = &[
    ("id_rsa*", 0o600, "SSH RSA private key"),
    ("id_ed25519*", 0o600, "SSH Ed25519 private key"),
    ("id_ecdsa*", 0o600, "SSH ECDSA private key"),
    ("id_dsa*", 0o600, "SSH DSA private key"),
    ("*_rsa", 0o600, "Named SSH RSA key"),
    ("*_ed25519", 0o600, "Named SSH Ed25519 key"),
    ("*_ecdsa", 0o600, "Named SSH ECDSA key"),
    ("*_dsa", 0o600, "Named SSH DSA key"),
    ("*.pem", 0o600, "PEM private key"),
    ("*.key", 0o600, "Private key file"),
    ("*.gpg", 0o600, "GPG file"),
    ("*.asc", 0o600, "ASCII armored key"),
    ("*.bashrc", 0o644, "Bash config backup"),
    ("*.zshrc", 0o644, "Zsh config backup"),
    ("*.profile", 0o644, "Shell profile backup"),
    ("credentials*", 0o600, "Credentials file"),
    ("*.token", 0o600, "Auth token file"),
];

pub fn get_required_permission(path: &Path) -> Option<(u32, &'static str)> {
    // Convert to ~ format for matching
    let home = std::env::var("HOME")
        .ok()
        .or_else(|| dirs::home_dir().map(|d| d.to_string_lossy().to_string()));

    let tilde_path = if let Some(ref home) = home {
        let path_str = path.to_string_lossy();
        if path_str.starts_with(home.as_str()) {
            format!("~{}", &path_str[home.len()..])
        } else {
            path.to_string_lossy().to_string()
        }
    } else {
        path.to_string_lossy().to_string()
    };

    // Direct match
    for (pattern, mode, desc) in SENSITIVE_PATHS {
        if tilde_path == *pattern {
            return Some((*mode, *desc));
        }
    }

    // Pattern matching against filename
    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    for (pattern, mode, desc) in SENSITIVE_PATTERNS {
        if glob_match(pattern, &filename) {
            return Some((*mode, *desc));
        }
    }

    None
}

pub fn check_permission(path: &Path, required_mode: u32) -> bool {
    use std::os::unix::fs::PermissionsExt;

    if let Ok(metadata) = std::fs::metadata(path) {
        let current_mode = metadata.permissions().mode() & 0o7777;
        // Check if any extra bits are set
        (current_mode & !required_mode) == 0
    } else {
        true // Can't check, assume OK
    }
}

pub fn fix_permission(path: &Path, required_mode: u32) -> bool {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = match std::fs::metadata(path) {
        Ok(m) => m.permissions(),
        Err(_) => return false,
    };

    perms.set_mode(required_mode);
    std::fs::set_permissions(path, perms).is_ok()
}

/// Simple glob matching (supports only * suffix)
fn glob_match(pattern: &str, text: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix('*') {
        text.starts_with(prefix)
    } else if let Some(suffix) = pattern.strip_prefix('*') {
        text.ends_with(suffix)
    } else {
        pattern == text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match_prefix() {
        assert!(glob_match("id_rsa*", "id_rsa"));
        assert!(glob_match("id_rsa*", "id_rsa.pub"));
        assert!(!glob_match("id_rsa*", "other"));
    }

    #[test]
    fn test_glob_match_suffix() {
        assert!(glob_match("*.pem", "key.pem"));
        assert!(glob_match("*.pem", "cert.pem"));
        assert!(!glob_match("*.pem", "key.txt"));
    }

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("test", "test"));
        assert!(!glob_match("test", "testing"));
    }

    #[test]
    fn test_get_required_permission_ssh_rsa() {
        std::env::set_var("HOME", "/home/user");
        let path = Path::new("/home/user/.ssh/id_rsa");
        let result = get_required_permission(path);
        assert!(result.is_some());
        let (mode, desc) = result.unwrap();
        assert_eq!(mode, 0o600);
        assert_eq!(desc, "SSH RSA private key");
    }

    #[test]
    fn test_get_required_permission_ssh_ed25519() {
        std::env::set_var("HOME", "/home/user");
        let path = Path::new("/home/user/.ssh/id_ed25519");
        let result = get_required_permission(path);
        assert!(result.is_some());
        let (mode, desc) = result.unwrap();
        assert_eq!(mode, 0o600);
        assert!(desc.contains("Ed25519"));
    }

    #[test]
    fn test_get_required_permission_ssh_authorized_keys() {
        std::env::set_var("HOME", "/home/user");
        let path = Path::new("/home/user/.ssh/authorized_keys");
        let result = get_required_permission(path);
        assert!(result.is_some());
        let (mode, _) = result.unwrap();
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn test_get_required_permission_gnupg() {
        std::env::set_var("HOME", "/home/user");
        let path = Path::new("/home/user/.gnupg");
        let result = get_required_permission(path);
        assert!(result.is_some());
        let (mode, _) = result.unwrap();
        assert_eq!(mode, 0o700);
    }

    #[test]
    fn test_get_required_permission_shell_config() {
        std::env::set_var("HOME", "/home/user");
        let path = Path::new("/home/user/.bashrc");
        let result = get_required_permission(path);
        assert!(result.is_some());
        let (mode, _) = result.unwrap();
        assert_eq!(mode, 0o644);
    }

    #[test]
    fn test_get_required_permission_tilde_path() {
        std::env::set_var("HOME", "/home/testuser");
        let path = Path::new("/home/testuser/.ssh/id_ed25519");
        let result = get_required_permission(path);
        assert!(result.is_some());
        let (mode, _) = result.unwrap();
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn test_get_required_permission_pattern_pem() {
        let path = Path::new("/some/path/server.pem");
        let result = get_required_permission(path);
        assert!(result.is_some());
        let (mode, desc) = result.unwrap();
        assert_eq!(mode, 0o600);
        assert!(desc.contains("PEM"));
    }

    #[test]
    fn test_get_required_permission_pattern_key() {
        let path = Path::new("/some/path/mykey.key");
        let result = get_required_permission(path);
        assert!(result.is_some());
        let (mode, _) = result.unwrap();
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn test_get_required_permission_pattern_id_rsa() {
        // id_rsa_custom should match id_rsa* pattern
        let path = Path::new("/some/path/id_rsa_custom");
        let result = get_required_permission(path);
        assert!(result.is_some());
        let (mode, _) = result.unwrap();
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn test_get_required_permission_not_sensitive() {
        let path = Path::new("/home/user/.config/regular_app/config.txt");
        let result = get_required_permission(path);
        assert!(result.is_none());
    }

    #[test]
    fn test_glob_match_id_rsa_prefix() {
        assert!(glob_match("id_rsa*", "id_rsa"));
        assert!(glob_match("id_rsa*", "id_rsa.pub"));
        assert!(glob_match("id_rsa*", "id_rsa_custom_key"));
        assert!(!glob_match("id_rsa*", "other_key"));
    }

    #[test]
    fn test_glob_match_pem_suffix() {
        assert!(glob_match("*.pem", "server.pem"));
        assert!(glob_match("*.pem", "cert.pem"));
        assert!(!glob_match("*.pem", "server.key"));
    }

    #[test]
    fn test_get_required_permission_aws_credentials() {
        std::env::set_var("HOME", "/home/user");
        let path = Path::new("/home/user/.aws/credentials");
        let result = get_required_permission(path);
        assert!(result.is_some());
        let (mode, desc) = result.unwrap();
        assert_eq!(mode, 0o600);
        assert!(desc.contains("AWS"));
    }

    #[test]
    fn test_get_required_permission_git_credentials() {
        std::env::set_var("HOME", "/home/user");
        let path = Path::new("/home/user/.git-credentials");
        let result = get_required_permission(path);
        assert!(result.is_some());
        let (mode, _) = result.unwrap();
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn test_get_required_permission_xauthority() {
        std::env::set_var("HOME", "/home/user");
        let path = Path::new("/home/user/.Xauthority");
        let result = get_required_permission(path);
        assert!(result.is_some());
        let (mode, _) = result.unwrap();
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn test_get_required_permission_named_ssh_key() {
        // *_ed25519 pattern
        let path = Path::new("/some/path/github_ed25519");
        let result = get_required_permission(path);
        assert!(result.is_some());
        let (mode, _) = result.unwrap();
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn test_get_required_permission_token_file() {
        // *.token pattern
        let path = Path::new("/some/path/auth.token");
        let result = get_required_permission(path);
        assert!(result.is_some());
        let (mode, _) = result.unwrap();
        assert_eq!(mode, 0o600);
    }
}
