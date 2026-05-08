use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

use crate::error::XdError;

/// Parsed `xdotter.toml`. Both `[links]` and `[dependencies]` may be absent;
/// an empty config is legal per SPEC.
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Source path -> link path. Source paths are TOML keys; thus 1 source -> 1 link.
    pub links: BTreeMap<String, String>,
    /// Dependency name -> relative subdirectory containing its own `xdotter.toml`.
    pub dependencies: BTreeMap<String, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(default)]
    links: Option<BTreeMap<String, String>>,
    #[serde(default)]
    dependencies: Option<BTreeMap<String, String>>,
}

impl Config {
    /// Parse a TOML string. Unknown top-level keys/tables and malformed types
    /// are reported as configuration errors per SPEC.
    pub fn from_toml(content: &str, source: &Path) -> Result<Self, XdError> {
        let raw: RawConfig = basic_toml::from_str(content)
            .map_err(|e| XdError::config(format!("{}: TOML 解析失败: {}", source.display(), e)))?;

        Ok(Config {
            links: raw.links.unwrap_or_default(),
            dependencies: raw.dependencies.unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p() -> PathBuf {
        PathBuf::from("xdotter.toml")
    }

    #[test]
    fn empty_config_is_legal() {
        let c = Config::from_toml("", &p()).unwrap();
        assert!(c.links.is_empty());
        assert!(c.dependencies.is_empty());
    }

    #[test]
    fn known_tables_parse() {
        let c = Config::from_toml(
            r#"
[links]
".zshrc" = "~/.zshrc"

[dependencies]
"nvim" = "config/nvim"
"#,
            &p(),
        )
        .unwrap();
        assert_eq!(c.links.get(".zshrc").unwrap(), "~/.zshrc");
        assert_eq!(c.dependencies.get("nvim").unwrap(), "config/nvim");
    }

    #[test]
    fn unknown_top_level_key_is_config_error() {
        let err = Config::from_toml(r#"unknown = "x""#, &p()).unwrap_err();
        assert!(err.is_config(), "got: {err:?}");
    }

    #[test]
    fn unknown_top_level_table_is_config_error() {
        let err = Config::from_toml(
            r#"
[unknown]
x = 1
"#,
            &p(),
        )
        .unwrap_err();
        assert!(err.is_config());
    }

    #[test]
    fn malformed_toml_is_config_error() {
        let err = Config::from_toml("[links", &p()).unwrap_err();
        assert!(err.is_config());
    }

    #[test]
    fn duplicate_top_level_keys_rejected_by_toml() {
        // basic-toml rejects duplicate keys during parsing.
        let err = Config::from_toml(
            r#"
[links]
"a" = "~/a"
"a" = "~/b"
"#,
            &p(),
        )
        .unwrap_err();
        assert!(err.is_config());
    }
}
