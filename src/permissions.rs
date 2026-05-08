//! Built-in permission policy per SPEC §"权限和敏感文件语义".
//!
//! Only the targets enumerated in SPEC are checked. Classification is
//! based on the `~/`-expanded link path string only — never on source
//! file content, name, or type.

use std::path::Path;

/// SPEC permission table:
///
/// | link path                                  | required mode |
/// |--------------------------------------------|---------------|
/// | `~/.ssh`                                   | 0o700         |
/// | `~/.ssh/config`                            | 0o600         |
/// | `~/.ssh/authorized_keys`                   | 0o600         |
/// | `~/.ssh/id_*` (not ending in `.pub`)       | 0o600         |
/// | `~/.ssh/*_{rsa,ed25519,ecdsa,dsa}` (^.pub) | 0o600         |
/// | `~/.pgpass`                                | 0o600         |
/// | `~/.netrc`                                 | 0o600         |
/// | `~/.gnupg`                                 | 0o700         |
///
/// Anything not on this list returns `None`.
pub fn required_permission(home_relative_link: &str) -> Option<(u32, &'static str)> {
    // Direct exact-match entries.
    match home_relative_link {
        "~/.ssh" => return Some((0o700, "SSH 目录")),
        "~/.ssh/config" => return Some((0o600, "SSH config")),
        "~/.ssh/authorized_keys" => return Some((0o600, "SSH authorized_keys")),
        "~/.pgpass" => return Some((0o600, ".pgpass")),
        "~/.netrc" => return Some((0o600, ".netrc")),
        "~/.gnupg" => return Some((0o700, "GPG 目录")),
        _ => {}
    }

    // Pattern entries — only inside `~/.ssh/`.
    if let Some(name) = home_relative_link.strip_prefix("~/.ssh/") {
        // No subdirectories; SPEC patterns target files directly under `~/.ssh`.
        if name.contains('/') {
            return None;
        }
        // Public keys (".pub" suffix) are not in the SPEC table.
        if name.ends_with(".pub") {
            return None;
        }
        if name.starts_with("id_") {
            return Some((0o600, "SSH 私钥 (id_*)"));
        }
        for suffix in ["_rsa", "_ed25519", "_ecdsa", "_dsa"] {
            if name.ends_with(suffix) && name.len() > suffix.len() {
                return Some((0o600, "SSH 私钥 (*_{rsa,ed25519,ecdsa,dsa})"));
            }
        }
    }

    None
}

/// On Unix, returns true iff the file's mode bits are no more permissive
/// than `required_mode` (i.e. `(mode & !required_mode) == 0`). On Windows
/// SPEC says no Unix-mode check is performed; we return true.
pub fn check_permission(path: &Path, required_mode: u32) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match std::fs::metadata(path) {
            Ok(m) => {
                let cur = m.permissions().mode() & 0o7777;
                (cur & !required_mode) == 0
            }
            Err(_) => false,
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (path, required_mode);
        true
    }
}

/// Set the file's mode bits exactly to `required_mode` (Unix only).
/// Returns true on success.
pub fn fix_permission(path: &Path, required_mode: u32) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = match std::fs::metadata(path) {
            Ok(m) => m.permissions(),
            Err(_) => return false,
        };
        perms.set_mode(required_mode);
        std::fs::set_permissions(path, perms).is_ok()
    }
    #[cfg(not(unix))]
    {
        let _ = (path, required_mode);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_targets_are_recognized() {
        for (k, mode) in [
            ("~/.ssh", 0o700),
            ("~/.ssh/config", 0o600),
            ("~/.ssh/authorized_keys", 0o600),
            ("~/.pgpass", 0o600),
            ("~/.netrc", 0o600),
            ("~/.gnupg", 0o700),
            ("~/.ssh/id_rsa", 0o600),
            ("~/.ssh/id_ed25519", 0o600),
            ("~/.ssh/id_ecdsa_custom", 0o600),
            ("~/.ssh/github_rsa", 0o600),
            ("~/.ssh/work_ed25519", 0o600),
        ] {
            let r = required_permission(k);
            assert!(r.is_some(), "{k} should match");
            assert_eq!(r.unwrap().0, mode, "{k} mode");
        }
    }

    #[test]
    fn pub_keys_are_not_in_spec_table() {
        assert!(required_permission("~/.ssh/id_rsa.pub").is_none());
        assert!(required_permission("~/.ssh/github_ed25519.pub").is_none());
    }

    #[test]
    fn out_of_table_paths_return_none() {
        assert!(required_permission("~/.bashrc").is_none());
        assert!(required_permission("~/.zshrc").is_none());
        assert!(required_permission("~/.aws/credentials").is_none());
        assert!(required_permission("~/.git-credentials").is_none());
        assert!(required_permission("~/.config/foo").is_none());
        assert!(required_permission("~/.ssh/known_hosts").is_none());
        assert!(required_permission("~/.ssh/sub/id_rsa").is_none());
    }

    #[test]
    fn underscore_suffix_requires_prefix() {
        // "_rsa" alone shouldn't match (length must exceed suffix length).
        assert!(required_permission("~/.ssh/_rsa").is_none());
    }
}
