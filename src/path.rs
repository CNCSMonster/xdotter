//! Path semantics per SPEC §"路径语义".
//!
//! Three path forms exist:
//! - **Absolute**: `/...` on Unix; drive-letter absolute on Windows.
//! - **Home-relative**: starts with `~/`. The leading `~` character is
//!   replaced by the current user's home directory; the trailing slash
//!   and remainder are kept literally.
//! - **Normal-relative**: anything else, non-empty and not `.`. Resolved
//!   against the declaring config's directory.

use std::path::{Component, Path, PathBuf};

use crate::error::XdError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathForm {
    Absolute,
    HomeRelative,
    NormalRelative,
}

/// Classify a raw path string into one of the three SPEC forms.
/// Empty and `.` are rejected by callers via [`validate_*`] helpers,
/// so this function returns `Err` for them.
pub fn classify(raw: &str) -> Result<PathForm, &'static str> {
    if raw.is_empty() {
        return Err("路径不得为空");
    }
    if raw == "." {
        return Err("路径不得是 \".\"");
    }
    if raw == "~" {
        // bare `~` is not a supported form per SPEC.
        return Err("不支持单独的 `~`，请使用 `~/...`");
    }
    if raw.starts_with("~/") {
        return Ok(PathForm::HomeRelative);
    }
    #[cfg(windows)]
    {
        if is_windows_drive_absolute(raw) {
            return Ok(PathForm::Absolute);
        }
        if raw.starts_with('/') || raw.starts_with('\\') {
            // SPEC: Windows rejects POSIX-style root, UNC, long-path,
            // and drive-relative forms.
            return Err("Windows 上链接路径必须是 `~/...` 或盘符绝对路径");
        }
        return Ok(PathForm::NormalRelative);
    }
    #[cfg(not(windows))]
    {
        if raw.starts_with('/') {
            return Ok(PathForm::Absolute);
        }
        Ok(PathForm::NormalRelative)
    }
}

#[cfg(windows)]
fn is_windows_drive_absolute(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    if bytes.len() < 3 {
        return false;
    }
    let c0 = bytes[0];
    let drive = (c0 as char).is_ascii_alphabetic();
    let colon = bytes[1] == b':';
    let sep = bytes[2] == b'/' || bytes[2] == b'\\';
    drive && colon && sep
}

/// Expand `~/` prefix using the current user's home directory.
/// Returns an error if the path begins with `~/` but home cannot be
/// resolved. Non-`~/` paths are returned unchanged as `PathBuf`.
pub fn expand_tilde(raw: &str) -> Result<PathBuf, XdError> {
    if let Some(rest) = raw.strip_prefix("~/") {
        let home = home_dir().ok_or_else(|| {
            XdError::planning(format!(
                "无法确定当前用户的 HOME 目录，无法展开 `~/`：{}",
                raw
            ))
        })?;
        // Preserve the slash so that `~/` becomes `<home>/`.
        let mut p = home;
        if !rest.is_empty() {
            p.push(rest);
        }
        return Ok(p);
    }
    Ok(PathBuf::from(raw))
}

fn home_dir() -> Option<PathBuf> {
    if let Ok(h) = std::env::var("HOME") {
        if !h.is_empty() {
            return Some(PathBuf::from(h));
        }
    }
    dirs::home_dir()
}

/// Lexically normalize a path: collapse `.` components and repeated
/// separators. `..` components are preserved (caller decides whether to
/// reject them). Trailing separators carry no meaning.
pub fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in p.components() {
        match comp {
            Component::CurDir => {}
            Component::Normal(_) | Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                out.push(comp.as_os_str());
            }
        }
    }
    if out.as_os_str().is_empty() {
        // Pure "." normalizes to ".".
        out.push(".");
    }
    out
}

/// Check if a path contains any `..` (parent-dir traversal) component.
pub fn has_parent_traversal(p: &Path) -> bool {
    p.components().any(|c| c == Component::ParentDir)
}

// -----------------------------------------------------------------------------
// Static validators (config-error category)
// -----------------------------------------------------------------------------

/// Validate a source path string per SPEC §"源路径".
///
/// - must be normal-relative
/// - must not be absolute, home-relative, empty, or `.`
/// - must not contain `..` components
///
/// Filesystem-existence and symlink-component checks belong to the
/// planning stage, not this function.
pub fn validate_source_path(raw: &str) -> Result<(), XdError> {
    let form = classify(raw).map_err(|m| {
        XdError::config(format!("源路径 \"{}\" 非法: {}", raw, m))
    })?;
    if form != PathForm::NormalRelative {
        return Err(XdError::config(format!(
            "源路径必须是普通相对路径，不得是绝对或 home 相对路径: \"{}\"",
            raw
        )));
    }
    if has_parent_traversal(Path::new(raw)) {
        return Err(XdError::config(format!(
            "源路径不得包含父目录跳转 `..`: \"{}\"",
            raw
        )));
    }
    Ok(())
}

/// Validate a link path string per SPEC §"链接路径".
///
/// - must be absolute or home-relative; normal-relative is rejected
/// - must not be empty
/// - must not, after `~/` expansion, contain `..` components
/// - must not statically resolve to filesystem root, the home directory
///   itself, `~`, `~/`, `/`, or any string consisting solely of path
///   separators (e.g. `//`)
pub fn validate_link_path(raw: &str) -> Result<(), XdError> {
    if raw.is_empty() {
        return Err(XdError::config("链接路径不得为空".to_string()));
    }
    // Reject pure-separator strings statically (SPEC: "~", "~/", "/",
    // "//" are rejected without home expansion).
    if raw == "~" || raw == "~/" {
        return Err(XdError::config(format!(
            "链接路径不得解析为 home 目录本身: \"{}\"",
            raw
        )));
    }
    if is_pure_separators(raw) {
        return Err(XdError::config(format!(
            "链接路径不得解析为文件系统根目录: \"{}\"",
            raw
        )));
    }

    let form = classify(raw).map_err(|m| {
        XdError::config(format!("链接路径 \"{}\" 非法: {}", raw, m))
    })?;

    match form {
        PathForm::NormalRelative => {
            return Err(XdError::config(format!(
                "链接路径必须是绝对路径或 home 相对路径，不得是普通相对路径: \"{}\"",
                raw
            )));
        }
        PathForm::Absolute | PathForm::HomeRelative => {}
    }

    // After `~/` is conceptually replaced by `<home>/`, no `..`
    // components may appear. We only need to inspect components after
    // the `~/` prefix or the absolute root, both of which translate
    // into the structural components of the raw string minus the
    // prefix; checking the raw string for any `..` component covers it
    // because `~/` itself contains no `..`.
    if has_parent_traversal(Path::new(raw)) {
        return Err(XdError::config(format!(
            "链接路径不得包含父目录跳转 `..`: \"{}\"",
            raw
        )));
    }

    // Reject `~/` followed only by separators / dots that normalize to
    // the home directory itself, e.g. "~/", "~/.", "~/./", "~/.//".
    if let Some(rest) = raw.strip_prefix("~/") {
        if rest_is_empty_or_curdir_only(rest) {
            return Err(XdError::config(format!(
                "链接路径不得解析为 home 目录本身: \"{}\"",
                raw
            )));
        }
    }

    // Reject `/` followed only by separators / dots, e.g. "/", "/.",
    // "/.//". `is_pure_separators` already covered all-`/`; this also
    // catches "/." etc.
    #[cfg(not(windows))]
    {
        if let Some(rest) = raw.strip_prefix('/') {
            if rest_is_empty_or_curdir_only(rest) {
                return Err(XdError::config(format!(
                    "链接路径不得解析为文件系统根目录: \"{}\"",
                    raw
                )));
            }
        }
    }

    Ok(())
}

fn is_pure_separators(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c == '/' || c == '\\')
}

fn rest_is_empty_or_curdir_only(rest: &str) -> bool {
    if rest.is_empty() {
        return true;
    }
    // Tokenize on `/` and `\` — accept Windows mixed seps. Anything that
    // is not "" or "." means the path goes somewhere meaningful.
    rest.split(|c| c == '/' || c == '\\')
        .all(|seg| seg.is_empty() || seg == ".")
}

/// Validate a dependency path string per SPEC §"依赖路径".
pub fn validate_dependency_path(raw: &str) -> Result<(), XdError> {
    let form = classify(raw).map_err(|m| {
        XdError::config(format!("依赖路径 \"{}\" 非法: {}", raw, m))
    })?;
    if form != PathForm::NormalRelative {
        return Err(XdError::config(format!(
            "依赖路径必须是相对路径，不得是绝对或 home 相对路径: \"{}\"",
            raw
        )));
    }
    if has_parent_traversal(Path::new(raw)) {
        return Err(XdError::config(format!(
            "依赖路径不得包含父目录跳转 `..`: \"{}\"",
            raw
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- classify ----------
    #[test]
    fn classify_basic() {
        assert_eq!(classify("foo/bar").unwrap(), PathForm::NormalRelative);
        assert_eq!(classify("~/x").unwrap(), PathForm::HomeRelative);
        assert!(classify("").is_err());
        assert!(classify(".").is_err());
        assert!(classify("~").is_err());
    }

    #[cfg(not(windows))]
    #[test]
    fn classify_unix_absolute() {
        assert_eq!(classify("/etc").unwrap(), PathForm::Absolute);
    }

    // ---------- expand_tilde ----------
    #[test]
    fn expand_tilde_replaces_leading() {
        std::env::set_var("HOME", "/home/u");
        assert_eq!(
            expand_tilde("~/.zshrc").unwrap(),
            PathBuf::from("/home/u/.zshrc")
        );
    }

    #[test]
    fn expand_tilde_passthrough() {
        assert_eq!(expand_tilde("foo").unwrap(), PathBuf::from("foo"));
    }

    // ---------- normalize ----------
    #[test]
    fn normalize_collapses_dots() {
        assert_eq!(normalize(Path::new("a/./b")), PathBuf::from("a/b"));
        assert_eq!(normalize(Path::new("./a")), PathBuf::from("a"));
    }

    #[test]
    fn normalize_keeps_parent_dir() {
        assert_eq!(normalize(Path::new("a/../b")), PathBuf::from("a/../b"));
    }

    // ---------- validate_source_path ----------
    #[test]
    fn source_must_be_normal_relative() {
        assert!(validate_source_path("a/b").is_ok());
        assert!(validate_source_path("").is_err());
        assert!(validate_source_path(".").is_err());
        assert!(validate_source_path("~/.zshrc").is_err());
        assert!(validate_source_path("../escape").is_err());
        assert!(validate_source_path("a/../b").is_err());
        #[cfg(not(windows))]
        assert!(validate_source_path("/abs").is_err());
    }

    // ---------- validate_link_path ----------
    #[test]
    fn link_rejects_normal_relative() {
        assert!(validate_link_path("foo/bar").is_err());
    }

    #[test]
    fn link_rejects_empty_and_specials() {
        for s in ["", "~", "~/", "/", "//", "///"] {
            assert!(validate_link_path(s).is_err(), "{s} should be rejected");
        }
    }

    #[test]
    fn link_rejects_dotdot() {
        assert!(validate_link_path("~/a/../b").is_err());
        #[cfg(not(windows))]
        assert!(validate_link_path("/a/../b").is_err());
    }

    #[test]
    fn link_rejects_curdir_only_after_home() {
        assert!(validate_link_path("~/.").is_err());
        assert!(validate_link_path("~/./").is_err());
        assert!(validate_link_path("~/./.").is_err());
    }

    #[cfg(not(windows))]
    #[test]
    fn link_rejects_curdir_only_after_root() {
        assert!(validate_link_path("/.").is_err());
        assert!(validate_link_path("/./").is_err());
    }

    #[test]
    fn link_accepts_normal_targets() {
        assert!(validate_link_path("~/.zshrc").is_ok());
        #[cfg(not(windows))]
        assert!(validate_link_path("/etc/foo").is_ok());
    }

    // ---------- validate_dependency_path ----------
    #[test]
    fn dependency_must_be_normal_relative() {
        assert!(validate_dependency_path("config/nvim").is_ok());
        assert!(validate_dependency_path("").is_err());
        assert!(validate_dependency_path(".").is_err());
        assert!(validate_dependency_path("../escape").is_err());
        assert!(validate_dependency_path("~/x").is_err());
        #[cfg(not(windows))]
        assert!(validate_dependency_path("/abs").is_err());
    }
}
