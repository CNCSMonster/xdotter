//! Error classification per SPEC §"错误分类".
//!
//! Every user-facing error message must carry a recognizable label
//! identifying one of the four classes. The exact wording is an
//! implementation choice; the labels chosen here are documented below
//! and in README.

use std::fmt;

/// SPEC error class. Each variant maps to a stable label prefix that
/// appears at the start of `Display` output so users and scripts can
/// classify failures without parsing free-form text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XdError {
    /// CLI 参数错误 — invalid command-line arguments. Detected before
    /// any config is read.
    Cli(String),
    /// 配置错误 — static configuration violation (TOML, types, path
    /// rules judgable from config text alone).
    Config(String),
    /// 规划阻塞错误 — planning-stage error that prevents safe planning
    /// or apply (filesystem-state-dependent).
    Planning(String),
    /// 应用阶段错误 — error during apply-stage execution.
    Apply(String),
}

impl XdError {
    pub fn cli<S: Into<String>>(msg: S) -> Self {
        XdError::Cli(msg.into())
    }
    pub fn config<S: Into<String>>(msg: S) -> Self {
        XdError::Config(msg.into())
    }
    pub fn planning<S: Into<String>>(msg: S) -> Self {
        XdError::Planning(msg.into())
    }
    pub fn apply<S: Into<String>>(msg: S) -> Self {
        XdError::Apply(msg.into())
    }

    #[allow(dead_code)]
    pub fn is_cli(&self) -> bool {
        matches!(self, XdError::Cli(_))
    }
    #[allow(dead_code)]
    pub fn is_config(&self) -> bool {
        matches!(self, XdError::Config(_))
    }
    #[allow(dead_code)]
    pub fn is_planning(&self) -> bool {
        matches!(self, XdError::Planning(_))
    }
    #[allow(dead_code)]
    pub fn is_apply(&self) -> bool {
        matches!(self, XdError::Apply(_))
    }

    pub fn label(&self) -> &'static str {
        match self {
            XdError::Cli(_) => "[CLI 参数错误]",
            XdError::Config(_) => "[配置错误]",
            XdError::Planning(_) => "[规划阻塞错误]",
            XdError::Apply(_) => "[应用阶段错误]",
        }
    }

    pub fn body(&self) -> &str {
        match self {
            XdError::Cli(s) | XdError::Config(s) | XdError::Planning(s) | XdError::Apply(s) => s,
        }
    }
}

/// True when `s` starts with one of the four SPEC classification labels.
fn starts_with_label(s: &str) -> bool {
    s.starts_with("[CLI 参数错误]")
        || s.starts_with("[配置错误]")
        || s.starts_with("[规划阻塞错误]")
        || s.starts_with("[应用阶段错误]")
}

impl fmt::Display for XdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let body = self.body();
        if starts_with_label(body) {
            // Joined error bag: each line already carries its own
            // classification label per SPEC §"输出语义".
            write!(f, "{}", body)
        } else {
            write!(f, "{} {}", self.label(), body)
        }
    }
}

impl std::error::Error for XdError {}

/// Convenience: collect multiple errors into one Display-able blob,
/// preserving each error's label so the aggregate still satisfies the
/// "every error is classifiable" contract.
#[derive(Debug, Default)]
pub struct ErrorBag {
    items: Vec<XdError>,
}

impl ErrorBag {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push(&mut self, e: XdError) {
        self.items.push(e);
    }
    #[allow(dead_code)]
    pub fn extend<I: IntoIterator<Item = XdError>>(&mut self, it: I) {
        self.items.extend(it);
    }
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.items.len()
    }
    #[allow(dead_code)]
    pub fn into_vec(self) -> Vec<XdError> {
        self.items
    }
    pub fn iter(&self) -> std::slice::Iter<'_, XdError> {
        self.items.iter()
    }
    /// Reduce the bag to a single error by joining all messages with
    /// newlines. Each error line preserves its own classification label
    /// per SPEC §"输出语义"; the outer `XdError` variant does not add
    /// another label when the joined text spans multiple lines.
    pub fn into_single(self) -> Option<XdError> {
        if self.items.is_empty() {
            return None;
        }
        if self.items.len() == 1 {
            return self.items.into_iter().next();
        }
        let joined = self
            .items
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        // Variant choice does not affect Display output for multi-line
        // content (see Display impl).
        Some(XdError::Planning(joined))
    }

    /// Convert the bag into a single `XdError` suitable for `Err`
    /// return. An empty bag produces a generic fallback error.
    pub fn into_error(self) -> XdError {
        self.into_single()
            .unwrap_or_else(|| XdError::apply("未知错误".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_prefix_present() {
        assert!(XdError::cli("x").to_string().starts_with("[CLI 参数错误]"));
        assert!(XdError::config("x").to_string().starts_with("[配置错误]"));
        assert!(XdError::planning("x")
            .to_string()
            .starts_with("[规划阻塞错误]"));
        assert!(XdError::apply("x")
            .to_string()
            .starts_with("[应用阶段错误]"));
    }

    #[test]
    fn single_error_with_multiline_body_still_gets_label() {
        // A config error listing collision entries has a multi-line body
        // but is a single error — it must still get the [配置错误] prefix.
        let e = XdError::config("多个链接冲突：\n  - a\n  - b");
        let s = e.to_string();
        assert!(
            s.starts_with("[配置错误] 多个链接冲突："),
            "multi-line single error must carry label: {s}"
        );
        // Each content line should be preserved.
        assert!(s.contains("  - a"), "must contain line a: {s}");
        assert!(s.contains("  - b"), "must contain line b: {s}");
    }

    #[test]
    fn into_single_preserves_per_line_labels() {
        let mut bag = ErrorBag::new();
        bag.push(XdError::planning("p1"));
        bag.push(XdError::apply("a1"));
        let joined = bag.into_single().unwrap().to_string();
        assert!(
            joined.contains("[规划阻塞错误] p1"),
            "must contain planning error with label: {joined}"
        );
        assert!(
            joined.contains("[应用阶段错误] a1"),
            "must contain apply error with label: {joined}"
        );
        // No outer label wrapping — the Display should not prepend
        // a redundant label before multi-line joined text.
        assert!(
            joined.starts_with("[规划阻塞错误] p1"),
            "joined text must start directly with first error's label: {joined}"
        );
    }
}
