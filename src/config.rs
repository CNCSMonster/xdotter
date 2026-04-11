use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Config {
    pub links: HashMap<String, String>,
    pub dependencies: HashMap<String, String>,
}

impl Config {
    pub fn from_toml(content: &str) -> Result<Self, String> {
        let data: TomlData = toml::from_str(content).map_err(|e| format_toml_error(content, &e))?;

        Ok(Config {
            links: data.links.unwrap_or_default(),
            dependencies: data.dependencies.unwrap_or_default(),
        })
    }

    pub fn from_json(content: &str) -> Result<Self, String> {
        let data: JsonData =
            serde_json::from_str(content).map_err(|e| format_json_error(content, &e))?;

        Ok(Config {
            links: data.links.unwrap_or_default(),
            dependencies: data.dependencies.unwrap_or_default(),
        })
    }
}

#[derive(Deserialize)]
struct TomlData {
    links: Option<HashMap<String, String>>,
    dependencies: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
struct JsonData {
    links: Option<HashMap<String, String>>,
    dependencies: Option<HashMap<String, String>>,
}

pub fn validate_toml(content: &str) -> Result<(), String> {
    let _: TomlData = toml::from_str(content).map_err(|e| format_toml_error(content, &e))?;
    Ok(())
}

pub fn validate_json(content: &str) -> Result<(), String> {
    let _: JsonData = serde_json::from_str(content).map_err(|e| format_json_error(content, &e))?;
    Ok(())
}

pub fn detect_format(filepath: &Path) -> Option<&'static str> {
    match filepath.extension().and_then(|e| e.to_str()) {
        Some("toml") => Some("toml"),
        Some("json") => Some("json"),
        _ => None,
    }
}

fn format_toml_error(content: &str, error: &toml::de::Error) -> String {
    let line = error
        .span()
        .map(|s| content[..s.start].lines().count() + 1)
        .unwrap_or(1);
    let lines: Vec<&str> = content.lines().collect();
    let error_line = lines.get(line.saturating_sub(1)).unwrap_or(&"");
    let prev_line = if line > 1 { lines.get(line - 2) } else { None };
    let next_line = lines.get(line);

    let mut msg = format!(
        "❌ TOML 语法错误\n\n错误：{} (第 {} 行)",
        error.message(),
        line
    );

    if let Some(prev) = prev_line {
        msg.push_str(&format!("\n  {} | {}", line - 1, prev));
    }
    msg.push_str(&format!("\n> {} | {}", line, error_line));
    if let Some(next) = next_line {
        msg.push_str(&format!("\n  {} | {}", line + 1, next));
    }

    // Add suggestion
    let error_msg = error.message().to_lowercase();
    if error_msg.contains("expected") || error_msg.contains("invalid") {
        msg.push_str("\n\n提示：检查语法，键名可能需要引号包裹");
    }

    msg
}

fn format_json_error(content: &str, error: &serde_json::Error) -> String {
    let line = error.line();
    let column = error.column();
    let lines: Vec<&str> = content.lines().collect();
    let error_line = lines.get(line.saturating_sub(1)).unwrap_or(&"");
    let prev_line = if line > 1 { lines.get(line - 2) } else { None };
    let next_line = lines.get(line);

    let mut msg = format!(
        "❌ JSON 语法错误\n\n错误：{} (第 {} 行，第 {} 列)",
        error, line, column
    );

    if let Some(prev) = prev_line {
        msg.push_str(&format!("\n  {} | {}", line - 1, prev));
    }
    msg.push_str(&format!("\n> {} | {}", line, error_line));
    if let Some(next) = next_line {
        msg.push_str(&format!("\n  {} | {}", line + 1, next));
    }

    // Add suggestion
    let error_msg = error.to_string().to_lowercase();
    if error_msg.contains("comma") {
        msg.push_str("\n\n提示：JSON 对象属性之间需要用逗号分隔");
    } else if error_msg.contains("quote") {
        msg.push_str("\n\n提示：JSON 键名必须是字符串（用双引号包裹）");
    }

    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_toml() {
        let content = r#"
[links]
"~/.zshrc" = "~/.config/zshrc"

[dependencies]
"nvim" = "config/nvim"
"#;
        let config = Config::from_toml(content).unwrap();
        assert_eq!(config.links.get("~/.zshrc").unwrap(), "~/.config/zshrc");
        assert_eq!(config.dependencies.get("nvim").unwrap(), "config/nvim");
    }

    #[test]
    fn test_parse_invalid_toml() {
        let content = r#"
[links
"~/.zshrc" = "~/.config/zshrc"
"#;
        assert!(Config::from_toml(content).is_err());
    }

    #[test]
    fn test_parse_valid_json() {
        let content = r#"{
  "links": {"~/.zshrc": "~/.config/zshrc"},
  "dependencies": {"nvim": "config/nvim"}
}"#;
        let config = Config::from_json(content).unwrap();
        assert_eq!(config.links.get("~/.zshrc").unwrap(), "~/.config/zshrc");
    }

    #[test]
    fn test_parse_invalid_json() {
        let content = r#"{"links": }"#;
        assert!(Config::from_json(content).is_err());
    }

    #[test]
    fn test_detect_format() {
        use std::path::Path;
        assert_eq!(detect_format(Path::new("config.toml")), Some("toml"));
        assert_eq!(detect_format(Path::new("config.json")), Some("json"));
        assert_eq!(detect_format(Path::new("config.yaml")), None);
    }

    #[test]
    fn test_parse_empty_toml() {
        let content = "";
        let config = Config::from_toml(content).unwrap();
        assert!(config.links.is_empty());
        assert!(config.dependencies.is_empty());
    }

    #[test]
    fn test_parse_empty_json() {
        let content = "{}";
        let config = Config::from_json(content).unwrap();
        assert!(config.links.is_empty());
        assert!(config.dependencies.is_empty());
    }

    #[test]
    fn test_parse_links_only_toml() {
        let content = r#"
[links]
".zshrc" = "~/.zshrc"
"#;
        let config = Config::from_toml(content).unwrap();
        assert_eq!(config.links.len(), 1);
        assert!(config.dependencies.is_empty());
    }

    #[test]
    fn test_validate_empty_toml() {
        let content = "";
        assert!(validate_toml(content).is_ok());
    }

    #[test]
    fn test_validate_empty_json() {
        let content = "{}";
        assert!(validate_json(content).is_ok());
    }
}
