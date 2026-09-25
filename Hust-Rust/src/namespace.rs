//! Namespace Module
//! Handles namespace declaration, resolution, and call translation
//!
//! Design (owner, 2026.09.25):
//! - Entry file (with main) = default namespace, can be explicit or implicit
//! - Files without namespace declaration join default namespace when imported
//! - Same namespace across multiple files = same space (distributed implementation)
//! - One file = one namespace only, declared at file top: `namespace Name;`
//! - Call resolution: `namespace.class.method()` → simplified forms
//! - main is in default namespace + default class, so bare calls work

use std::collections::HashMap;
use thiserror::Error;

/// Namespace error
#[derive(Error, Debug)]
pub enum NamespaceError {
    #[error("Namespace declaration must be at file top: {0}")]
    InvalidPosition(String),

    #[error("Multiple namespaces in one file: {0}")]
    MultipleNamespaces(String),

    #[error("Namespace not found: {0}")]
    NotFound(String),

    #[error("Ambiguous call: {0} found in multiple namespaces")]
    AmbiguousCall(String),
}

/// Namespace declaration info
#[derive(Debug, Clone)]
pub struct NamespaceDecl {
    pub name: String,
    pub file_path: String,
    pub line: usize,
}

/// Function info in namespace
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub namespace: String,
    pub is_public: bool,
    pub params: String,
    pub ret_type: String,
}

/// Class info in namespace
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub namespace: String,
    pub is_public: bool,
    pub parent: Option<String>,
}

/// Namespace registry - maps namespaces to their contents
#[derive(Debug, Default)]
pub struct NamespaceRegistry {
    /// namespace name -> file paths
    pub spaces: HashMap<String, Vec<String>>,
    /// function name -> namespace (for resolution)
    pub functions: HashMap<String, Vec<FunctionInfo>>,
    /// class name -> namespace
    pub classes: HashMap<String, Vec<ClassInfo>>,
    /// default namespace name (usually "default" or from entry file)
    pub default_space: String,
}

impl NamespaceRegistry {
    pub fn new() -> Self {
        Self {
            default_space: "default".to_string(),
            ..Default::default()
        }
    }

    /// Parse namespace declaration from source
    /// Returns (namespace_name, remaining_source)
    pub fn parse_declaration(&mut self, source: &str, file_path: &str) -> Result<(Option<String>, String), NamespaceError> {
        let mut namespace = None;
        let mut remaining = source.to_string();
        let mut found_ns = false;

        for (line_idx, line) in source.lines().enumerate() {
            let trimmed = line.trim();

            // Skip comments and empty lines
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
                continue;
            }

            // Check for namespace declaration
            if trimmed.starts_with("namespace ") && trimmed.ends_with(';') {
                if found_ns {
                    return Err(NamespaceError::MultipleNamespaces(file_path.to_string()));
                }

                let ns_name = trimmed[10..trimmed.len() - 1].trim().to_string();
                if ns_name.is_empty() {
                    return Err(NamespaceError::InvalidPosition(file_path.to_string()));
                }

                namespace = Some(ns_name.clone());
                found_ns = true;

                // Remove namespace line from source
                remaining = remaining.replacen(line, "", 1);

                // Register namespace
                self.spaces.entry(ns_name.clone())
                    .or_insert_with(Vec::new)
                    .push(file_path.to_string());
            }

            // First non-comment, non-namespace line = code starts
            if !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("/*") 
                && !trimmed.starts_with("namespace ") {
                break;
            }
        }

        Ok((namespace, remaining))
    }

    /// Register a function in namespace
    pub fn register_function(&mut self, func: FunctionInfo) {
        self.functions
            .entry(func.name.clone())
            .or_insert_with(Vec::new)
            .push(func);
    }

    /// Register a class in namespace
    pub fn register_class(&mut self, cls: ClassInfo) {
        self.classes
            .entry(cls.name.clone())
            .or_insert_with(Vec::new)
            .push(cls);
    }

    /// Resolve function call - find which namespace it belongs to
    pub fn resolve_function(&self, name: &str, current_ns: &str) -> Result<Option<String>, NamespaceError> {
        // Check current namespace first
        if let Some(funcs) = self.functions.get(name) {
            // Filter by current namespace
            let in_current: Vec<_> = funcs.iter()
                .filter(|f| f.namespace == current_ns)
                .collect();

            if in_current.len() == 1 {
                return Ok(Some(in_current[0].namespace.clone()));
            } else if in_current.len() > 1 {
                return Err(NamespaceError::AmbiguousCall(name.to_string()));
            }

            // Check default namespace
            let in_default: Vec<_> = funcs.iter()
                .filter(|f| f.namespace == self.default_space)
                .collect();

            if in_default.len() == 1 {
                return Ok(Some(in_default[0].namespace.clone()));
            } else if in_default.len() > 1 {
                return Err(NamespaceError::AmbiguousCall(name.to_string()));
            }

            // Check imported namespaces (would need import tracking)
            // For now, if exactly one namespace has it, use that
            if funcs.len() == 1 {
                return Ok(Some(funcs[0].namespace.clone()));
            } else if funcs.len() > 1 {
                return Err(NamespaceError::AmbiguousCall(name.to_string()));
            }
        }

        Ok(None)
    }

    /// Generate report of namespace distribution
    pub fn distribution_report(&self) -> String {
        let mut report = String::from("Namespace Distribution Report:\n");

        for (ns, files) in &self.spaces {
            report.push_str(&format!("  namespace {}:\n", ns));
            for file in files {
                report.push_str(&format!("    - {}\n", file));
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_namespace() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
namespace math;
public i32 factorial(i32 n) { return 0; }
"#;

        let (ns, remaining) = registry.parse_declaration(source, "math.hust").unwrap();
        assert_eq!(ns, Some("math".to_string()));
        assert!(!remaining.contains("namespace"));
    }

    #[test]
    fn test_default_namespace() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
void main() { }
"#;

        let (ns, _) = registry.parse_declaration(source, "main.hust").unwrap();
        assert_eq!(ns, None); // No explicit namespace = default
    }

    #[test]
    fn test_multiple_namespaces_error() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
namespace a;
namespace b;
"#;

        let result = registry.parse_declaration(source, "test.hust");
        assert!(result.is_err());
    }
}
