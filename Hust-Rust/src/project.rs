//! Project Configuration Module
//! Handles settings.config parsing and project structure

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Project configuration error
#[derive(Error, Debug)]
pub enum ProjectError {
    #[error("Config read failed: {0}")]
    ConfigRead(#[from] std::io::Error),

    #[error("Config parse error: {0}")]
    ParseError(String),

    #[error("Module not found: {0}")]
    ModuleNotFound(String),

    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),
}

/// Project configuration (settings.config)
#[derive(Debug, Clone, Default)]
pub struct ProjectConfig {
    /// Project metadata
    pub package: PackageConfig,
    /// Module search paths
    pub module_paths: Vec<PathBuf>,
    /// Dependencies (future use)
    pub dependencies: HashMap<String, String>,
}

/// Package configuration
#[derive(Debug, Clone, Default)]
pub struct PackageConfig {
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub entry: String, // Entry file, default: main.hust
}

impl ProjectConfig {
    /// Load configuration from project directory
    pub fn load(project_dir: &Path) -> Result<Self, ProjectError> {
        let config_path = project_dir.join("settings.config");

        if !config_path.exists() {
            // Return default config if no settings.config
            return Ok(Self::default_with_entry("main.hust"));
        }

        let content = std::fs::read_to_string(&config_path)?;
        Self::parse(&content)
    }

    /// Parse configuration from string
    pub fn parse(content: &str) -> Result<Self, ProjectError> {
        let mut config = ProjectConfig::default();
        let mut current_section = "";

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Section header [section]
            if line.starts_with('[') && line.ends_with(']') {
                current_section = &line[1..line.len() - 1];
                continue;
            }

            // Key = Value
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                let value = line[eq_pos + 1..].trim();

                match current_section {
                    "project" | "package" => {
                        match key {
                            "name" => config.package.name = value.to_string(),
                            "version" => config.package.version = value.to_string(),
                            "author" => config.package.author = Some(value.to_string()),
                            "entry" => config.package.entry = value.to_string(),
                            _ => {}
                        }
                    }
                    "modules" => {
                        if key == "paths" {
                            config.module_paths = value
                                .split(',')
                                .map(|s| PathBuf::from(s.trim()))
                                .collect();
                        }
                    }
                    _ => {}
                }
            }
        }

        // Set default entry if not specified
        if config.package.entry.is_empty() {
            config.package.entry = "main.hust".to_string();
        }

        // Set default module paths if not specified
        if config.module_paths.is_empty() {
            config.module_paths = vec![PathBuf::from("src"), PathBuf::from("lib")];
        }

        Ok(config)
    }

    /// Create default config with specified entry
    pub fn default_with_entry(entry: &str) -> Self {
        ProjectConfig {
            package: PackageConfig {
                name: "unnamed".to_string(),
                version: "0.1.0".to_string(),
                author: None,
                entry: entry.to_string(),
            },
            module_paths: vec![PathBuf::from("src"), PathBuf::from("lib")],
            dependencies: HashMap::new(),
        }
    }

    /// Find module file by name
    pub fn find_module(&self, module_name: &str, project_dir: &Path) -> Result<PathBuf, ProjectError> {
        // Try each module path
        for path in &self.module_paths {
            let module_path = project_dir.join(path).join(format!("{}.hust", module_name));
            if module_path.exists() {
                return Ok(module_path);
            }
        }

        // Try project root
        let root_path = project_dir.join(format!("{}.hust", module_name));
        if root_path.exists() {
            return Ok(root_path);
        }

        Err(ProjectError::ModuleNotFound(module_name.to_string()))
    }
}

/// Module information
#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub path: PathBuf,
    pub source: String,
    pub imports: Vec<String>, // List of imported module names
}

impl Module {
    /// Load module from file
    pub fn load(path: &Path) -> Result<Self, ProjectError> {
        let source = std::fs::read_to_string(path)?;
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let imports = Self::extract_imports(&source);

        Ok(Module {
            name,
            path: path.to_path_buf(),
            source,
            imports,
        })
    }

    /// Extract import statements from source
    /// Format: use module_name;
    fn extract_imports(source: &str) -> Vec<String> {
        let mut imports = Vec::new();

        for line in source.lines() {
            let line = line.trim();
            if line.starts_with("use ") && line.ends_with(';') {
                // Extract module name: "use math;" -> "math"
                let module_name = line[4..line.len() - 1].trim();
                if !module_name.is_empty() {
                    imports.push(module_name.to_string());
                }
            }
        }

        imports
    }
}

/// Module resolver - handles dependency resolution
pub struct ModuleResolver {
    loaded_modules: HashMap<String, Module>,
    resolution_stack: Vec<String>,
}

impl ModuleResolver {
    pub fn new() -> Self {
        ModuleResolver {
            loaded_modules: HashMap::new(),
            resolution_stack: Vec::new(),
        }
    }

    /// Resolve all dependencies starting from entry module
    pub fn resolve(
        &mut self,
        entry_path: &Path,
        config: &ProjectConfig,
        project_dir: &Path,
    ) -> Result<Vec<Module>, ProjectError> {
        let entry_module = Module::load(entry_path)?;
        let entry_name = entry_module.name.clone();

        self.load_dependencies(&entry_module, config, project_dir)?;

        // Return modules in dependency order (entry last)
        let mut result: Vec<Module> = self
            .loaded_modules
            .values()
            .filter(|m| m.name != entry_name)
            .cloned()
            .collect();
        result.push(entry_module);

        Ok(result)
    }

    /// Recursively load module dependencies
    fn load_dependencies(
        &mut self,
        module: &Module,
        config: &ProjectConfig,
        project_dir: &Path,
    ) -> Result<(), ProjectError> {
        // Check for circular dependency
        if self.resolution_stack.contains(&module.name) {
            return Err(ProjectError::CircularDependency(format!(
                "{} -> {}",
                self.resolution_stack.join(" -> "),
                module.name
            )));
        }

        // Skip if already loaded
        if self.loaded_modules.contains_key(&module.name) {
            return Ok(());
        }

        self.resolution_stack.push(module.name.clone());

        // Load each imported module
        for import_name in &module.imports {
            if self.loaded_modules.contains_key(import_name) {
                continue;
            }

            let module_path = config.find_module(import_name, project_dir)?;
            let imported_module = Module::load(&module_path)?;

            // Recursively load its dependencies first
            self.load_dependencies(&imported_module, config, project_dir)?;

            self.loaded_modules.insert(import_name.clone(), imported_module);
        }

        self.resolution_stack.pop();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let config_str = r#"
[project]
name = my_project
version = 0.1.0
author = wood
entry = main.hust

[modules]
paths = src, lib
"#;

        let config = ProjectConfig::parse(config_str).unwrap();
        assert_eq!(config.package.name, "my_project");
        assert_eq!(config.package.version, "0.1.0");
        assert_eq!(config.package.author, Some("wood".to_string()));
        assert_eq!(config.package.entry, "main.hust");
        assert_eq!(config.module_paths.len(), 2);
    }

    #[test]
    fn test_extract_imports() {
        let source = r#"
use math;
use utils;

void main() {
    i32 x = 1;
}
"#;

        let imports = Module::extract_imports(source);
        assert_eq!(imports, vec!["math", "utils"]);
    }
}
