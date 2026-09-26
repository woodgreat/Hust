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
//!
//! 2026.09.25 Update:
//! - UPPERCASE namespaces reserved for Hust system (RUST, HUST, etc.)
//! - User namespaces must not be all uppercase
//! - RUST.* = Rust bridge, HUST.* = Hust standard library
//! - Namespace is flat, dots are part of the name (not hierarchy)

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

    #[error("Reserved namespace: {0} is reserved for Hust system (UPPERCASE not allowed for users)")]
    ReservedNamespace(String),

    #[error("Nested namespace not supported: '{0}'\nHust namespaces are flat - use a single name without dots\nExample: namespace math; not namespace math.utils;")]
    NestedNamespace(String),
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
    /// Nested classes: Inner class name -> full path
    pub nested: HashMap<String, String>,
    /// Methods with access control
    pub methods: HashMap<String, MethodInfo>,
}

/// Method info with access control
#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub is_public: bool,
    pub is_static: bool,
    pub ret_type: String,
    pub params: String,
}

/// Use statement info
#[derive(Debug, Clone)]
pub struct UseStmt {
    pub namespace: String,
    pub item: Option<String>,  // None = *, Some(name) = specific item
    pub alias: Option<String>,
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
    /// Warnings collected during parsing
    pub warnings: Vec<String>,
    /// Use statements per file
    pub use_statements: HashMap<String, Vec<UseStmt>>,
}

impl NamespaceRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            default_space: "default".to_string(),
            ..Default::default()
        };
        // Register system namespaces
        registry.register_system_namespaces();
        registry
    }

    /// Parse namespace declaration from source
    /// Returns (namespace_name, remaining_source)
    /// 2026.09.25: Reject UPPERCASE namespaces (reserved for Hust system)
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

                // Check for nested namespace (contains dot)
                if ns_name.contains('.') {
                    return Err(NamespaceError::NestedNamespace(ns_name));
                }

                // Check for reserved UPPERCASE namespace
                if ns_name.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
                    return Err(NamespaceError::ReservedNamespace(ns_name));
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
        // Check for duplicate class in same namespace
        if let Some(existing) = self.classes.get(&cls.name) {
            let same_ns: Vec<_> = existing.iter()
                .filter(|c| c.namespace == cls.namespace)
                .collect();
            if !same_ns.is_empty() {
                self.warnings.push(format!(
                    "Class '{}' already exists in namespace '{}' - distributed implementation",
                    cls.name, cls.namespace
                ));
            }
        }
        
        self.classes
            .entry(cls.name.clone())
            .or_insert_with(Vec::new)
            .push(cls);
    }

    /// Register a nested class
    pub fn register_nested_class(&mut self, outer: &str, inner: &str, full_path: &str, namespace: &str) {
        if let Some(classes) = self.classes.get_mut(outer) {
            for cls in classes.iter_mut() {
                if cls.namespace == namespace {
                    cls.nested.insert(inner.to_string(), full_path.to_string());
                    return;
                }
            }
        }
        // Outer class not found, create placeholder
        let mut nested = HashMap::new();
        nested.insert(inner.to_string(), full_path.to_string());
        self.register_class(ClassInfo {
            name: outer.to_string(),
            namespace: namespace.to_string(),
            is_public: true,
            parent: None,
            nested,
            methods: HashMap::new(),
        });
    }

    /// Register a method in a class
    pub fn register_method(&mut self, class_name: &str, method: MethodInfo, namespace: &str) {
        if let Some(classes) = self.classes.get_mut(class_name) {
            for cls in classes.iter_mut() {
                if cls.namespace == namespace {
                    cls.methods.insert(method.name.clone(), method);
                    return;
                }
            }
        }
        // Class not found, create placeholder
        let mut methods = HashMap::new();
        methods.insert(method.name.clone(), method);
        self.register_class(ClassInfo {
            name: class_name.to_string(),
            namespace: namespace.to_string(),
            is_public: true,
            parent: None,
            nested: HashMap::new(),
            methods,
        });
    }

    /// Check if a method is accessible (public or same class)
    pub fn is_method_accessible(&self, class_name: &str, method_name: &str, namespace: &str, current_class: Option<&str>) -> bool {
        if let Some(classes) = self.classes.get(class_name) {
            for cls in classes.iter() {
                if cls.namespace == namespace {
                    // Same class = always accessible
                    if current_class == Some(class_name) {
                        return true;
                    }
                    // Check method visibility
                    if let Some(method) = cls.methods.get(method_name) {
                        return method.is_public;
                    }
                    // Method not found, assume accessible (fallback)
                    return true;
                }
            }
        }
        // Class not found, assume accessible (fallback)
        true
    }

    /// Resolve nested class path: Outer.Inner -> full path
    pub fn resolve_nested_class(&self, path: &str, namespace: &str) -> Option<String> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.len() < 2 {
            return None;
        }
        
        let outer = parts[0];
        let inner = parts[1..].join(".");
        
        if let Some(classes) = self.classes.get(outer) {
            for cls in classes.iter() {
                if cls.namespace == namespace {
                    return cls.nested.get(&inner).cloned();
                }
            }
        }
        None
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

    /// Check if namespace is reserved (UPPERCASE)
    pub fn is_reserved_namespace(name: &str) -> bool {
        name.chars().all(|c| c.is_ascii_uppercase() || c == '_')
    }

    /// Register system namespaces (RUST, HUST)
    pub fn register_system_namespaces(&mut self) {
        // RUST - Rust bridge
        self.spaces.insert("RUST".to_string(), vec!["[system]".to_string()]);
        
        // HUST - Hust standard library
        self.spaces.insert("HUST".to_string(), vec!["[system]".to_string()]);
    }

    /// Parse use statements from source
    /// Returns list of use statements and remaining source
    /// 2026.09.25: Support system namespaces (RUST.*, HUST.*)
    pub fn parse_use_statements(&mut self, source: &str, file_path: &str) -> (Vec<UseStmt>, String) {
        let mut uses = Vec::new();
        let mut remaining = source.to_string();
        
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("use ") && trimmed.ends_with(';') {
                // Parse: use ns; / use ns.item; / use ns.*; / use ns as alias;
                let stmt = &trimmed[4..trimmed.len() - 1];
                
                // Check for alias: use ns as alias;
                let (ns_part, alias) = if let Some(pos) = stmt.find(" as ") {
                    (&stmt[..pos], Some(stmt[pos + 4..].trim().to_string()))
                } else {
                    (stmt, None)
                };
                
                // Check for wildcard: use ns.*;
                let (namespace, item) = if ns_part.ends_with(".*") {
                    let ns = ns_part[..ns_part.len() - 2].trim();
                    (ns.to_string(), None)  // None = wildcard
                } else if let Some(pos) = ns_part.rfind('.') {
                    // Check if it's a system namespace (RUST.xxx or HUST.xxx)
                    let ns = ns_part[..pos].trim();
                    let item = ns_part[pos + 1..].trim();
                    
                    // System namespaces are flat, treat entire thing as namespace
                    if ns == "RUST" || ns == "HUST" || Self::is_reserved_namespace(ns) {
                        (ns_part.trim().to_string(), Some(item.to_string()))
                    } else {
                        (ns.to_string(), Some(item.to_string()))
                    }
                } else {
                    (ns_part.trim().to_string(), None)  // Whole namespace
                };
                
                if !namespace.is_empty() {
                    uses.push(UseStmt {
                        namespace,
                        item,
                        alias,
                    });
                    // Remove use line from source (match the exact trimmed line)
                    let line_to_remove = format!("{}\n", trimmed);
                    remaining = remaining.replacen(&line_to_remove, "", 1);
                    // Also try without newline (in case it's the last line)
                    if remaining.contains(trimmed) {
                        remaining = remaining.replacen(trimmed, "", 1);
                    }
                }
            }
        }
        
        self.use_statements.insert(file_path.to_string(), uses.clone());
        (uses, remaining)
    }
    
    /// Apply inheritance (过继) - move parent class to child's namespace
    pub fn apply_inheritance(&mut self, child_class: &str, parent_class: &str, child_ns: &str) {
        // Find parent class
        if let Some(parent_classes) = self.classes.get(parent_class) {
            // Clone parent info
            let parent_info = parent_classes[0].clone();
            
            // Remove from old namespace
            if let Some(old_classes) = self.classes.get_mut(parent_class) {
                old_classes.retain(|c| c.namespace != parent_info.namespace);
            }
            
            // Add to new namespace (过继)
            let mut new_parent = parent_info.clone();
            new_parent.namespace = child_ns.to_string();
            self.register_class(new_parent);
            
            self.warnings.push(format!(
                "Class '{}' inherited by '{}' - moved to namespace '{}' (过继)",
                parent_class, child_class, child_ns
            ));
        }
    }
    
    /// Resolve with use statements - check if name is imported
    pub fn resolve_with_imports(&self, name: &str, current_file: &str, current_ns: &str) -> Result<Option<String>, NamespaceError> {
        // Check current namespace first
        if let Some(ns) = self.resolve_function(name, current_ns)? {
            return Ok(Some(ns));
        }
        
        // Check use statements
        if let Some(uses) = self.use_statements.get(current_file) {
            for use_stmt in uses {
                // Check if this use imports the function
                let matches = match &use_stmt.item {
                    None => true,  // use ns; - imports all
                    Some(item) => item == name,  // use ns.item;
                };
                
                if matches {
                    // Verify function exists in that namespace
                    if let Some(funcs) = self.functions.get(name) {
                        let in_ns: Vec<_> = funcs.iter()
                            .filter(|f| f.namespace == use_stmt.namespace)
                            .collect();
                        if !in_ns.is_empty() {
                            return Ok(Some(use_stmt.namespace.clone()));
                        }
                    }
                }
            }
        }
        
        Ok(None)
    }

    /// Generate report of namespace distribution
    pub fn distribution_report(&self) -> String {
        let mut report = String::from("=== Namespace Distribution Report ===\n\n");

        // Group by namespace
        let mut ns_files: Vec<(&String, &Vec<String>)> = self.spaces.iter().collect();
        ns_files.sort_by_key(|(k, _)| k.clone());

        for (ns, files) in ns_files {
            report.push_str(&format!("namespace \"{}\":\n", ns));
            
            // Count functions and classes
            let func_count = self.functions.values()
                .flatten()
                .filter(|f| f.namespace == *ns)
                .count();
            let class_count = self.classes.values()
                .flatten()
                .filter(|c| c.namespace == *ns)
                .count();
            
            report.push_str(&format!("  functions: {}\n", func_count));
            report.push_str(&format!("  classes: {}\n", class_count));
            report.push_str(&format!("  files ({}):\n", files.len()));
            
            for file in files {
                // Extract just filename for readability
                let fname = std::path::Path::new(file)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(file);
                report.push_str(&format!("    - {}\n", fname));
            }
            report.push('\n');
        }

        // Warnings
        if !self.warnings.is_empty() {
            report.push_str("Warnings:\n");
            for warning in &self.warnings {
                report.push_str(&format!("  ⚠ {}\n", warning));
            }
        }

        report
    }

    /// Print distribution report to stderr
    pub fn print_report(&self) {
        eprintln!("{}", self.distribution_report());
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
    fn test_reserved_namespace() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
namespace MATH;
"#;

        let result = registry.parse_declaration(source, "test.hust");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), NamespaceError::ReservedNamespace(_)));
    }

    #[test]
    fn test_system_namespaces_registered() {
        let registry = NamespaceRegistry::new();
        assert!(registry.spaces.contains_key("RUST"));
        assert!(registry.spaces.contains_key("HUST"));
    }

    #[test]
    fn test_use_system_namespace() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
use RUST.std.io.*;
use HUST.math;
"#;

        let (uses, _) = registry.parse_use_statements(source, "test.hust");
        assert_eq!(uses.len(), 2);
        assert_eq!(uses[0].namespace, "RUST.std.io");
        assert_eq!(uses[1].namespace, "HUST.math");
    }

    #[test]
    fn test_nested_namespace_rejected() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
namespace math.utils;
"#;

        let result = registry.parse_declaration(source, "test.hust");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), NamespaceError::NestedNamespace(_)));
    }

    #[test]
    fn test_nested_namespace_error_message() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
namespace a.b.c;
"#;

        let result = registry.parse_declaration(source, "test.hust");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Nested namespace not supported"));
        assert!(err.to_string().contains("a.b.c"));
    }
}
