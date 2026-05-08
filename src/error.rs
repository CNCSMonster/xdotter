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
            XdError::Cli(s)
            | XdError::Config(s)
            | XdError::Planning(s)
            | XdError::Apply(s) => s,
        }
    }
}

impl fmt::Display for XdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.label(), self.body())
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
    /// newlines. The first error's class is preserved as the bag's
    /// class; downstream callers should iterate `items` for full detail
    /// when they care about per-item classes.
    pub fn into_single(self) -> Option<XdError> {
        if self.items.is_empty() {
            return None;
        }
        if self.items.len() == 1 {
            return self.items.into_iter().next();
        }
        let first_class = self.items[0].clone();
        let joined = self
            .items
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        Some(match first_class {
            XdError::Cli(_) => XdError::Cli(joined),
            XdError::Config(_) => XdError::Config(joined),
            XdError::Planning(_) => XdError::Planning(joined),
            XdError::Apply(_) => XdError::Apply(joined),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_prefix_present() {
        assert!(XdError::cli("x").to_string().starts_with("[CLI 参数错误]"));
        assert!(XdError::config("x").to_string().starts_with("[配置错误]"));
        assert!(XdError::planning("x").to_string().starts_with("[规划阻塞错误]"));
        assert!(XdError::apply("x").to_string().starts_with("[应用阶段错误]"));
    }
}
