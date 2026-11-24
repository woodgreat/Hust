//! `settings.config` reader.
//!
//! Format compatibility is intentionally close to the existing cores: INI
//! sections, `key = value` lines, `#` comments, and unknown keys ignored.
//! The conductor only reads the fields it needs.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::error::{Error, Result};

pub const CONFIG_FILE: &str = "settings.config";
pub const DEFAULT_ENTRY: &str = "main.hust";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CorePreference {
    #[default]
    Auto,
    Rust,
    C,
}

impl CorePreference {
    pub fn parse(value: &str) -> std::result::Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "auto" => Ok(CorePreference::Auto),
            "rust" => Ok(CorePreference::Rust),
            "c" => Ok(CorePreference::C),
            other => Err(format!(
                "invalid core `{}` (expected auto, rust, or c)",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectConfig {
    pub name: Option<String>,
    pub version: Option<String>,
    pub entry: String,
    pub core: CorePreference,
    pub module_paths: Option<String>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: None,
            version: None,
            entry: DEFAULT_ENTRY.to_string(),
            core: CorePreference::Auto,
            module_paths: None,
        }
    }
}

/// Load `settings.config` from a project directory. A missing file is not an
/// error; callers get the defaults instead.
pub fn load_from_dir(dir: &Path) -> Result<ProjectConfig> {
    let path = dir.join(CONFIG_FILE);
    match fs::read_to_string(&path) {
        Ok(content) => parse(&content).map_err(|message| {
            Error::usage(format!("invalid config `{}`: {}", path.display(), message))
        }),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(ProjectConfig::default()),
        Err(err) => Err(Error::usage(format!(
            "cannot read config `{}`: {}",
            path.display(),
            err
        ))),
    }
}

pub fn parse(content: &str) -> std::result::Result<ProjectConfig, String> {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let mut config = ProjectConfig::default();
    let mut section = String::new();

    for (index, raw_line) in content.lines().enumerate() {
        let line_no = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        if line.starts_with('[') {
            if !line.ends_with(']') || line.len() < 3 {
                return Err(format!("line {}: malformed section header", line_no));
            }
            section = line[1..line.len() - 1].trim().to_ascii_lowercase();
            continue;
        }

        let (key, value) = match line.split_once('=') {
            Some((key, value)) => (key.trim().to_ascii_lowercase(), clean_value(value)),
            None => {
                return Err(format!(
                    "line {}: expected `key = value` or a section header",
                    line_no
                ))
            }
        };
        if key.is_empty() {
            return Err(format!("line {}: empty key", line_no));
        }

        if section == "modules" {
            if key == "paths" {
                config.module_paths = non_empty(value);
            }
            continue;
        }

        // `[package]` is accepted as an alias for historical configs.
        if section == "project" || section == "package" {
            match key.as_str() {
                "name" => config.name = non_empty(value),
                "version" => config.version = non_empty(value),
                "entry" => {
                    if let Some(entry) = non_empty(value) {
                        config.entry = entry;
                    }
                }
                "core" => {
                    config.core = CorePreference::parse(&value)
                        .map_err(|message| format!("line {}: {}", line_no, message))?;
                }
                _ => {}
            }
        }
    }

    if config.entry.trim().is_empty() {
        config.entry = DEFAULT_ENTRY.to_string();
    }
    Ok(config)
}

fn clean_value(raw: &str) -> String {
    let value = raw.trim();
    let value = if value.len() >= 2 {
        let quoted = (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''));
        if quoted {
            &value[1..value.len() - 1]
        } else {
            value
        }
    } else {
        value
    };
    value.trim().to_string()
}

fn non_empty(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_when_fields_are_missing() {
        let config = parse("").unwrap();
        assert_eq!(config.entry, "main.hust");
        assert_eq!(config.core, CorePreference::Auto);
        assert_eq!(config.name, None);
        assert_eq!(config.module_paths, None);
    }

    #[test]
    fn parses_project_and_modules() {
        let config = parse(
            "# comment\n[project]\nname = demo\nversion = 0.1.0\nentry = app.hust\ncore = c\n\n[modules]\npaths = src, lib\n",
        )
        .unwrap();
        assert_eq!(config.name.as_deref(), Some("demo"));
        assert_eq!(config.version.as_deref(), Some("0.1.0"));
        assert_eq!(config.entry, "app.hust");
        assert_eq!(config.core, CorePreference::C);
        assert_eq!(config.module_paths.as_deref(), Some("src, lib"));
    }

    #[test]
    fn accepts_package_section_and_quoted_values() {
        let config = parse("[package]\nname = \"demo app\"\ncore = 'rust'\n").unwrap();
        assert_eq!(config.name.as_deref(), Some("demo app"));
        assert_eq!(config.core, CorePreference::Rust);
    }

    #[test]
    fn ignores_unknown_sections_and_keys() {
        let config =
            parse("[project]\nname = demo\nunknown = whatever\n[cores]\nrust = /tmp/hust_rust\n")
                .unwrap();
        assert_eq!(config.name.as_deref(), Some("demo"));
    }

    #[test]
    fn accepts_semicolon_comments_and_bom() {
        let config = parse("\u{feff}; header\n[project]\nname = demo\n").unwrap();
        assert_eq!(config.name.as_deref(), Some("demo"));
    }

    #[test]
    fn rejects_invalid_core() {
        let error = parse("[project]\ncore = go\n").unwrap_err();
        assert!(error.contains("invalid core"), "{}", error);
    }

    #[test]
    fn rejects_malformed_lines() {
        assert!(parse("[project\nname = demo\n").is_err());
        assert!(parse("[project]\nname demo\n").is_err());
        assert!(parse("[project]\n = demo\n").is_err());
    }

    #[test]
    fn empty_entry_falls_back_to_default() {
        let config = parse("[project]\nentry =\n").unwrap();
        assert_eq!(config.entry, "main.hust");
    }
}
