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
    // GPG
    ("~/.gnupg", 0o700, "GPG directory"),
    ("~/.gnupg/gpg.conf", 0o600, "GPG config"),
    // Shell configs
    ("~/.bashrc", 0o644, "Bash config"),
    ("~/.zshrc", 0o644, "Zsh config"),
    ("~/.bash_profile", 0o644, "Bash login profile"),
    ("~/.profile", 0o644, "Shell profile"),
    // Other sensitive
    ("~/.netrc", 0o600, "Netrc password file"),
    ("~/.pgpass", 0o600, "PostgreSQL password file"),
];

// Glob patterns for sensitive files (matched against filename only)
// Format: (glob_pattern, required_mode, description)
static SENSITIVE_PATTERNS: &[(&str, u32, &str)] = &[
    ("id_rsa*", 0o600, "SSH RSA private key"),
    ("id_ed25519*", 0o600, "SSH Ed25519 private key"),
    ("id_ecdsa*", 0o600, "SSH ECDSA private key"),
    ("id_dsa*", 0o600, "SSH DSA private key"),
    ("*.pem", 0o600, "PEM private key"),
    ("*.key", 0o600, "Private key file"),
    ("*.gpg", 0o600, "GPG file"),
    ("*.asc", 0o600, "ASCII armored key"),
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
    let filename = path.file_name()
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
        true  // Can't check, assume OK
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
}
