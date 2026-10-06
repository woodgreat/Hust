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

/// RUST system namespace import info
/// 2026.10.04: For B方案 - collect Rust imports during parsing, generate use statements later
#[derive(Debug, Clone)]
pub struct RustImport {
    /// Hust system namespace path (e.g., "RUST.STD.IO")
    pub hust_path: String,
    /// Imported item name, or "*" for wildcard
    pub item: String,
    /// Alias if specified
    pub alias: Option<String>,
}

/// Use statement info
/// 2026.10.04: Extended to support namespace-module.item syntax
#[derive(Debug, Clone)]
pub struct UseStmt {
    /// Namespace name (None = same/default namespace)
    pub namespace: Option<String>,
    /// Module name (None = not specified)
    pub module: Option<String>,
    /// Item name: class, function, or wildcard "*" (None = import all)
    pub item: Option<String>,
    /// Alias for the imported item
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
    /// RUST system namespace imports per file (for B方案: generate Rust use statements later)
    pub rust_imports: HashMap<String, Vec<RustImport>>,
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
    /// 2026.10.04: Support multi-level nesting (Outer.Inner1.Inner2.Inner3)
    ///
    /// Algorithm:
    /// 1. Split path by '.'
    /// 2. Look up first part in classes[outer].nested
    /// 3. If found, get the full_path of the inner class
    /// 4. Look up next part in classes[full_path].nested
    /// 5. Repeat until last part
    pub fn resolve_nested_class(&self, path: &str, namespace: &str) -> Option<String> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.len() < 2 {
            return None;
        }

        // Start with the outermost class
        let outer = parts[0];
        let mut current_path = outer.to_string();

        // Iterate through inner parts
        for (i, inner) in parts.iter().enumerate().skip(1) {
            // Look up current_path in classes
            if let Some(classes) = self.classes.get(&current_path) {
                let mut found = false;
                for cls in classes.iter() {
                    if cls.namespace == namespace {
                        // Check if this class has the nested class we're looking for
                        if let Some(full_path) = cls.nested.get(*inner) {
                            current_path = full_path.clone();
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    return None;
                }
            } else {
                return None;
            }

            // If this is the last part, return the full path
            if i == parts.len() - 1 {
                return Some(current_path);
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
        // System namespaces: RUST, HUST, or any path starting with RUST. or HUST.
        // Examples: RUST, HUST, RUST.STD.IO, HUST.MATH
        name == "RUST" || name == "HUST" ||
        name.starts_with("RUST.") || name.starts_with("HUST.") ||
        // Legacy: all uppercase with underscores (e.g., RUST_STD)
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
    /// 2026.10.04: Full rewrite to support namespace-module.item syntax
    ///
    /// Grammar:
    ///   use Namespace-Class;       -> ns=Namespace, module=None, item=Class
    ///   use Namespace-module;      -> ns=Namespace, module=module, item=None
    ///   use Namespace-module.item; -> ns=Namespace, module=module, item=item
    ///   use Namespace-module.*;    -> ns=Namespace, module=module, item=*
    ///   use Namespace-*;           -> ns=Namespace, module=None, item=*
    ///   use SYS.NAME-*;            -> ns=SYS.NAME, module=None, item=*
    ///   use module;                -> ns=None, module=module, item=None
    ///   use module.item;           -> ns=None, module=module, item=item
    ///   use module.*;              -> ns=None, module=module, item=*
    ///   use Class;                 -> ns=None, module=None, item=Class
    ///   use Outer.Inner;           -> ns=None, module=None, item=Outer.Inner (nested class)
    pub fn parse_use_statements(&mut self, source: &str, file_path: &str) -> (Vec<UseStmt>, String) {
        let mut uses = Vec::new();
        let mut rust_imports = Vec::new();
        let mut remaining = source.to_string();

        for line in source.lines() {
            let trimmed = line.trim();
            // Debug: print each line being checked
            eprintln!("[DEBUG parse_use] checking line: '{}'", trimmed);
            if trimmed.starts_with("use ") && trimmed.ends_with(';') {
                eprintln!("[DEBUG parse_use] found use statement: '{}'", trimmed);
                // Parse: use ... ;
                let stmt = &trimmed[4..trimmed.len() - 1];

                // Step 1: Check for alias: use X as alias;
                let (path_part, alias) = if let Some(pos) = stmt.find(" as ") {
                    (&stmt[..pos], Some(stmt[pos + 4..].trim().to_string()))
                } else {
                    (stmt, None)
                };

                let path_part = path_part.trim();
                eprintln!("[DEBUG parse_use] path_part: '{}'", path_part);

                // Step 2: Detect namespace (check for '-')
                let (namespace, rest) = if let Some(dash_pos) = path_part.find('-') {
                    // Has namespace: Zoo-... or RUST.STD.IO-...
                    let ns = path_part[..dash_pos].trim().to_string();
                    let rest = path_part[dash_pos + 1..].trim().to_string();
                    eprintln!("[DEBUG parse_use] found dash: ns='{}', rest='{}'", ns, rest);
                    (Some(ns), rest)
                } else {
                    // No namespace
                    eprintln!("[DEBUG parse_use] no dash found");
                    (None, path_part.to_string())
                };

                // Step 3: Check if this is a RUST system namespace import
                if let Some(ref ns) = namespace {
                    let is_rust = crate::rust_mappings::is_rust_namespace(ns);
                    eprintln!("[DEBUG parse_use] is_rust_namespace('{}') = {}", ns, is_rust);
                    if is_rust {
                        // This is a RUST system namespace import - collect it separately
                        let item = rest.trim().to_string();
                        if !item.is_empty() {
                            rust_imports.push(RustImport {
                                hust_path: ns.clone(),
                                item,
                                alias: alias.clone(),
                            });
                            // Remove use line from source
                            let line_to_remove = format!("{}\n", trimmed);
                            remaining = remaining.replacen(&line_to_remove, "", 1);
                            if remaining.contains(trimmed) {
                                remaining = remaining.replacen(trimmed, "", 1);
                            }
                            continue;
                        }
                    } else if ns.starts_with("RUST.") || ns.starts_with("HUST.") {
                        // Invalid system namespace (e.g., RUST.UNKNOWN.SOMETHING)
                        // This is a teaching error - provide helpful guidance
                        let available = crate::rust_mappings::get_supported_namespaces();
                        self.warnings.push(format!(
                            "unrecognized system namespace: `{}`\n\n\
                             Supported system namespaces:\n{}\n\n\
                             Examples:\n\
                             - use RUST.STD.IO-*;\n\
                             - use RUST.STD.COLLECTIONS-*;\n\
                             - use HUST.MATH;\n\n\
                             Note: System namespaces use `.` as internal separator (e.g., RUST.STD.IO).",
                            ns, available.join(", ")
                        ));
                        // Remove the invalid use line to prevent further errors
                        let line_to_remove = format!("{}\n", trimmed);
                        remaining = remaining.replacen(&line_to_remove, "", 1);
                        if remaining.contains(trimmed) {
                            remaining = remaining.replacen(trimmed, "", 1);
                        }
                        continue;
                    }
                }

                // Step 4: Parse rest (module.item / module / Class / Outer.Inner)
                // Pass namespace context for system namespace detection
                let is_system_ns = namespace.as_ref().map_or(false, |ns| Self::is_reserved_namespace(ns));
                eprintln!("[DEBUG parse_use] is_system_ns: {}", is_system_ns);
                let (module, item) = Self::parse_use_path(&rest, is_system_ns);
                eprintln!("[DEBUG parse_use] parsed: module={:?}, item={:?}", module, item);

                // Step 5: Validate and create UseStmt
                if let Some(use_stmt) = Self::build_use_stmt(namespace, module, item, alias) {
                    uses.push(use_stmt);
                    // Remove use line from source
                    let line_to_remove = format!("{}\n", trimmed);
                    remaining = remaining.replacen(&line_to_remove, "", 1);
                    if remaining.contains(trimmed) {
                        remaining = remaining.replacen(trimmed, "", 1);
                    }
                }
            }
        }

        self.use_statements.insert(file_path.to_string(), uses.clone());
        self.rust_imports.insert(file_path.to_string(), rust_imports);
        (uses, remaining)
    }

    /// Parse use path: module.item / module / Class / Outer.Inner / module.*
    /// is_system_ns: if true, treat uppercase identifiers as modules (system namespace context)
    /// Returns (module, item)
    fn parse_use_path(path: &str, is_system_ns: bool) -> (Option<String>, Option<String>) {
        if path.is_empty() {
            return (None, None);
        }

        // Check for bare wildcard: * (import all from namespace)
        if path == "*" {
            return (None, Some("*".to_string()));
        }

        // Check for wildcard: module.*
        if path.ends_with(".*") {
            let module = path[..path.len() - 2].trim().to_string();
            return (Some(module), Some("*".to_string()));
        }

        // Check for dot: module.item or Outer.Inner
        if let Some(dot_pos) = path.rfind('.') {
            let first = path[..dot_pos].trim().to_string();
            let second = path[dot_pos + 1..].trim().to_string();

            // Check if first part is module (lowercase) or outer class (uppercase)
            if first.chars().next().map_or(false, |c| c.is_ascii_lowercase()) {
                // module.item
                return (Some(first), Some(second));
            } else {
                // Outer.Inner (nested class)
                return (None, Some(path.to_string()));
            }
        }

        // Single identifier: module or Class
        if path.chars().next().map_or(false, |c| c.is_ascii_lowercase()) {
            // module (lowercase)
            (Some(path.to_string()), None)
        } else if is_system_ns {
            // System namespace context: uppercase = module
            (Some(path.to_string()), None)
        } else {
            // Class (uppercase, user namespace context)
            (None, Some(path.to_string()))
        }
    }

    /// Build UseStmt from parsed components
    fn build_use_stmt(
        namespace: Option<String>,
        module: Option<String>,
        item: Option<String>,
        alias: Option<String>,
    ) -> Option<UseStmt> {
        // Debug: print inputs
        eprintln!("[DEBUG build_use_stmt] namespace={:?}, module={:?}, item={:?}, alias={:?}", namespace, module, item, alias);
        
        // Validate: must have at least something
        if namespace.is_none() && module.is_none() && item.is_none() {
            eprintln!("[DEBUG build_use_stmt] returning None: all fields are None");
            return None;
        }

        eprintln!("[DEBUG build_use_stmt] returning Some");
        Some(UseStmt {
            namespace,
            module,
            item,
            alias,
        })
    }

    /// Apply inheritance (过继) - copy parent class to child's namespace
    /// 2026.10.04: Copy semantics (not move) - original namespace retains the class
    /// Design: parallel universes, no conflict, original class unchanged
    pub fn apply_inheritance(&mut self, child_class: &str, parent_class: &str, child_ns: &str) {
        // Find parent class
        if let Some(parent_classes) = self.classes.get(parent_class) {
            // Clone parent info
            let parent_info = parent_classes[0].clone();

            // Copy to new namespace (过继) - original namespace retains the class
            let mut new_parent = parent_info.clone();
            new_parent.namespace = child_ns.to_string();
            self.register_class(new_parent);

            self.warnings.push(format!(
                "Class '{}' inherited by '{}' - copied to namespace '{}' (过继)",
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
                            .filter(|f| Some(f.namespace.clone()) == use_stmt.namespace)
                            .collect();
                        if !in_ns.is_empty() {
                            return Ok(use_stmt.namespace.clone());
                        }
                    }
                }
            }
        }

        Ok(None)
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
use RUST.STD.IO-*;
use HUST-MATH;
"#;

        let (uses, _) = registry.parse_use_statements(source, "test.hust");
        
        // RUST imports should be collected separately
        let rust_imports = registry.rust_imports.get("test.hust").unwrap();
        assert_eq!(rust_imports.len(), 2);
        assert_eq!(rust_imports[0].hust_path, "RUST.STD.IO");
        assert_eq!(rust_imports[0].item, "*");
        assert_eq!(rust_imports[1].hust_path, "HUST");
        assert_eq!(rust_imports[1].item, "MATH");
        
        // uses should be empty (all imports went to rust_imports)
        assert_eq!(uses.len(), 0);
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

    #[test]
    fn test_resolve_nested_class_two_levels() {
        let mut registry = NamespaceRegistry::new();
        
        // Register Outer class with nested Inner
        registry.register_nested_class("Outer", "Inner", "Outer_Inner", "default");
        
        // Resolve Outer.Inner
        let result = registry.resolve_nested_class("Outer.Inner", "default");
        assert_eq!(result, Some("Outer_Inner".to_string()));
    }

    #[test]
    fn test_resolve_nested_class_three_levels() {
        let mut registry = NamespaceRegistry::new();
        
        // Register Outer -> Inner1 -> Inner2
        registry.register_nested_class("Outer", "Inner1", "Outer_Inner1", "default");
        registry.register_nested_class("Outer_Inner1", "Inner2", "Outer_Inner1_Inner2", "default");
        
        // Resolve Outer.Inner1.Inner2
        let result = registry.resolve_nested_class("Outer.Inner1.Inner2", "default");
        assert_eq!(result, Some("Outer_Inner1_Inner2".to_string()));
    }

    #[test]
    fn test_resolve_nested_class_four_levels() {
        let mut registry = NamespaceRegistry::new();
        
        // Register Outer -> Inner1 -> Inner2 -> Inner3
        registry.register_nested_class("Outer", "Inner1", "Outer_Inner1", "default");
        registry.register_nested_class("Outer_Inner1", "Inner2", "Outer_Inner1_Inner2", "default");
        registry.register_nested_class("Outer_Inner1_Inner2", "Inner3", "Outer_Inner1_Inner2_Inner3", "default");
        
        // Resolve Outer.Inner1.Inner2.Inner3
        let result = registry.resolve_nested_class("Outer.Inner1.Inner2.Inner3", "default");
        assert_eq!(result, Some("Outer_Inner1_Inner2_Inner3".to_string()));
    }

    #[test]
    fn test_resolve_nested_class_not_found() {
        let mut registry = NamespaceRegistry::new();
        
        // Register Outer with only Inner1
        registry.register_nested_class("Outer", "Inner1", "Outer_Inner1", "default");
        
        // Try to resolve Outer.NonExistent
        let result = registry.resolve_nested_class("Outer.NonExistent", "default");
        assert_eq!(result, None);
    }

    #[test]
    fn test_resolve_nested_class_wrong_namespace() {
        let mut registry = NamespaceRegistry::new();
        
        // Register Outer in namespace "Zoo"
        registry.register_nested_class("Outer", "Inner", "Outer_Inner", "Zoo");
        
        // Try to resolve in wrong namespace
        let result = registry.resolve_nested_class("Outer.Inner", "default");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_use_class() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
use Animal;
"#;

        let (uses, _) = registry.parse_use_statements(source, "test.hust");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].namespace, None);
        assert_eq!(uses[0].module, None);
        assert_eq!(uses[0].item, Some("Animal".to_string()));
    }

    #[test]
    fn test_parse_use_module() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
use animal;
"#;

        let (uses, _) = registry.parse_use_statements(source, "test.hust");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].namespace, None);
        assert_eq!(uses[0].module, Some("animal".to_string()));
        assert_eq!(uses[0].item, None);
    }

    #[test]
    fn test_parse_use_module_class() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
use animal.Dog;
"#;

        let (uses, _) = registry.parse_use_statements(source, "test.hust");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].namespace, None);
        assert_eq!(uses[0].module, Some("animal".to_string()));
        assert_eq!(uses[0].item, Some("Dog".to_string()));
    }

    #[test]
    fn test_parse_use_module_function() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
use animal.factorial;
"#;

        let (uses, _) = registry.parse_use_statements(source, "test.hust");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].namespace, None);
        assert_eq!(uses[0].module, Some("animal".to_string()));
        assert_eq!(uses[0].item, Some("factorial".to_string()));
    }

    #[test]
    fn test_parse_use_module_wildcard() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
use animal.*;
"#;

        let (uses, _) = registry.parse_use_statements(source, "test.hust");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].namespace, None);
        assert_eq!(uses[0].module, Some("animal".to_string()));
        assert_eq!(uses[0].item, Some("*".to_string()));
    }

    #[test]
    fn test_parse_use_nested_class() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
use Animal.Dog;
"#;

        let (uses, _) = registry.parse_use_statements(source, "test.hust");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].namespace, None);
        assert_eq!(uses[0].module, None);
        assert_eq!(uses[0].item, Some("Animal.Dog".to_string()));
    }

    #[test]
    fn test_parse_use_namespace_class() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
use Zoo-Animal;
"#;

        let (uses, _) = registry.parse_use_statements(source, "test.hust");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].namespace, Some("Zoo".to_string()));
        assert_eq!(uses[0].module, None);
        assert_eq!(uses[0].item, Some("Animal".to_string()));
    }

    #[test]
    fn test_parse_use_namespace_module() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
use Zoo-animal;
"#;

        let (uses, _) = registry.parse_use_statements(source, "test.hust");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].namespace, Some("Zoo".to_string()));
        assert_eq!(uses[0].module, Some("animal".to_string()));
        assert_eq!(uses[0].item, None);
    }

    #[test]
    fn test_parse_use_namespace_module_class() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
use Zoo-animal.Dog;
"#;

        let (uses, _) = registry.parse_use_statements(source, "test.hust");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].namespace, Some("Zoo".to_string()));
        assert_eq!(uses[0].module, Some("animal".to_string()));
        assert_eq!(uses[0].item, Some("Dog".to_string()));
    }

    #[test]
    fn test_parse_use_namespace_module_function() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
use Zoo-animal.factorial;
"#;

        let (uses, _) = registry.parse_use_statements(source, "test.hust");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].namespace, Some("Zoo".to_string()));
        assert_eq!(uses[0].module, Some("animal".to_string()));
        assert_eq!(uses[0].item, Some("factorial".to_string()));
    }

    #[test]
    fn test_parse_use_namespace_module_wildcard() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
use Zoo-animal.*;
"#;

        let (uses, _) = registry.parse_use_statements(source, "test.hust");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].namespace, Some("Zoo".to_string()));
        assert_eq!(uses[0].module, Some("animal".to_string()));
        assert_eq!(uses[0].item, Some("*".to_string()));
    }

    #[test]
    fn test_parse_use_namespace_wildcard() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
use Zoo-*;
"#;

        let (uses, _) = registry.parse_use_statements(source, "test.hust");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].namespace, Some("Zoo".to_string()));
        assert_eq!(uses[0].module, None);
        assert_eq!(uses[0].item, Some("*".to_string()));
    }

    #[test]
    fn test_parse_use_with_alias() {
        let mut registry = NamespaceRegistry::new();
        let source = r#"
use Zoo-animal as za;
"#;

        let (uses, _) = registry.parse_use_statements(source, "test.hust");
        assert_eq!(uses.len(), 1);
        assert_eq!(uses[0].namespace, Some("Zoo".to_string()));
        assert_eq!(uses[0].module, Some("animal".to_string()));
        assert_eq!(uses[0].item, None);
        assert_eq!(uses[0].alias, Some("za".to_string()));
    }
}
