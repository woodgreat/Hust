//! RUST System Namespace Mappings
//! Hardcoded mappings from Hust system namespaces to Rust std/core/crate paths
//!
//! Design (owner, 2026.10.04):
//! - Module-based organization for easy replacement and extension
//! - One-to-one mappings for supported items
//! - Wildcard (*) expands to hardcoded supported items only
//! - Uses Rust's native path conventions (std::io, not std.io)

/// Mapping entry: Hust system namespace -> Rust path
#[derive(Debug, Clone)]
pub struct RustMapping {
    /// Hust system namespace path (e.g., "RUST.STD.IO")
    pub hust_path: &'static str,
    /// Corresponding Rust path (e.g., "std::io")
    pub rust_path: &'static str,
    /// Supported items for wildcard expansion
    pub items: &'static [&'static str],
}

/// Hardcoded RUST system namespace mappings
pub const RUST_MAPPINGS: &[RustMapping] = &[
    // Hust standard library (HUST namespace)
    // Maps to Rust std library equivalents
    RustMapping {
        hust_path: "HUST",
        rust_path: "std",
        items: &[
            "math", "string", "vec", "io", "fs", "env", "process", "fmt",
        ],
    },
    RustMapping {
        hust_path: "HUST.MATH",
        rust_path: "std::math",
        items: &[
            "abs", "pow", "sqrt", "sin", "cos", "tan",
            "floor", "ceil", "round", "trunc", "fract",
            "min", "max", "clamp",
        ],
    },
    RustMapping {
        hust_path: "HUST.STRING",
        rust_path: "std::string",
        items: &[
            "String", "str", "from_utf8", "from_utf16",
        ],
    },
    RustMapping {
        hust_path: "HUST.VEC",
        rust_path: "std::vec",
        items: &[
            "Vec", "vec",
        ],
    },
    // Standard I/O
    RustMapping {
        hust_path: "RUST.STD.IO",
        rust_path: "std::io",
        items: &[
            "stdin", "stdout", "stderr",
            "Read", "Write", "BufRead", "BufReader", "BufWriter",
            "Error", "ErrorKind", "Result",
        ],
    },
    // Collections
    RustMapping {
        hust_path: "RUST.STD.COLLECTIONS",
        rust_path: "std::collections",
        items: &[
            "HashMap", "HashSet", "BTreeMap", "BTreeSet",
            "Vec", "VecDeque", "LinkedList", "BinaryHeap",
        ],
    },
    // Core memory operations
    RustMapping {
        hust_path: "RUST.CORE.MEM",
        rust_path: "core::mem",
        items: &[
            "size_of", "align_of", "size_of_val", "align_of_val",
            "drop", "replace", "swap", "take",
        ],
    },
    // Time
    RustMapping {
        hust_path: "RUST.STD.TIME",
        rust_path: "std::time",
        items: &[
            "Duration", "Instant", "SystemTime", "SystemTimeError",
        ],
    },
    // File system
    RustMapping {
        hust_path: "RUST.STD.FS",
        rust_path: "std::fs",
        items: &[
            "File", "OpenOptions", "DirEntry", "ReadDir",
            "create_dir", "create_dir_all", "remove_dir", "remove_dir_all",
            "remove_file", "rename", "copy", "metadata",
        ],
    },
    // Environment
    RustMapping {
        hust_path: "RUST.STD.ENV",
        rust_path: "std::env",
        items: &[
            "args", "args_os", "var", "var_os", "set_var", "remove_var",
            "current_dir", "current_exe", "home_dir", "temp_dir",
        ],
    },
    // Process
    RustMapping {
        hust_path: "RUST.STD.PROCESS",
        rust_path: "std::process",
        items: &[
            "Command", "Child", "ChildStdin", "ChildStdout", "ChildStderr",
            "exit", "abort", "id",
        ],
    },
    // String formatting
    RustMapping {
        hust_path: "RUST.STD.FMT",
        rust_path: "std::fmt",
        items: &[
            "Display", "Debug", "Formatter", "Result", "Error",
            "format", "write", "format_args",
        ],
    },
    // Macros (special handling)
    RustMapping {
        hust_path: "RUST.STD.MACROS",
        rust_path: "std",
        items: &[
            "println", "print", "eprintln", "eprint",
            "format", "vec", "stringify", "concat",
            "assert", "assert_eq", "assert_ne", "debug_assert",
            "panic", "todo", "unimplemented", "unreachable",
        ],
    },
];

/// Find Rust mapping by Hust system namespace path
///
/// # Arguments
/// * `hust_path` - Hust system namespace (e.g., "RUST.STD.IO")
///
/// # Returns
/// * `Some(&RustMapping)` if found
/// * `None` if not a recognized system namespace
///
/// # Example
/// ```
/// use hust_rust::rust_mappings::find_mapping;
///
/// let mapping = find_mapping("RUST.STD.IO").unwrap();
/// assert_eq!(mapping.rust_path, "std::io");
/// ```
pub fn find_mapping(hust_path: &str) -> Option<&'static RustMapping> {
    RUST_MAPPINGS.iter().find(|m| m.hust_path == hust_path)
}

/// Check if a namespace is a recognized RUST system namespace
///
/// # Arguments
/// * `hust_path` - Namespace path to check
///
/// # Returns
/// * `true` if it's a RUST system namespace
/// * `false` otherwise
pub fn is_rust_namespace(hust_path: &str) -> bool {
    find_mapping(hust_path).is_some()
}

/// Expand wildcard (*) to supported items for a system namespace
///
/// # Arguments
/// * `hust_path` - Hust system namespace (e.g., "RUST.STD.IO")
///
/// # Returns
/// * `Some(Vec<&str>)` - List of supported item names
/// * `None` - If namespace not recognized
///
/// # Example
/// ```
/// use hust_rust::rust_mappings::expand_wildcard;
///
/// let items = expand_wildcard("RUST.STD.IO").unwrap();
/// assert!(items.contains(&"stdin"));
/// assert!(items.contains(&"stdout"));
/// ```
pub fn expand_wildcard(hust_path: &str) -> Option<Vec<&'static str>> {
    find_mapping(hust_path).map(|m| m.items.to_vec())
}

/// Get the Rust use statement for a mapped item
///
/// # Arguments
/// * `hust_path` - Hust system namespace (e.g., "RUST.STD.IO")
/// * `item` - Item name (e.g., "stdin") or "*" for wildcard
///
/// # Returns
/// * `Some(String)` - Rust use statement (e.g., "use std::io::stdin;")
/// * `None` - If mapping not found or item not supported
///
/// # Example
/// ```
/// use hust_rust::rust_mappings::get_rust_use;
///
/// let use_stmt = get_rust_use("RUST.STD.IO", "stdin").unwrap();
/// assert_eq!(use_stmt, "use std::io::stdin;");
/// ```
pub fn get_rust_use(hust_path: &str, item: &str) -> Option<String> {
    let mapping = find_mapping(hust_path)?;

    if item == "*" {
        // Wildcard: expand to all supported items
        let items: Vec<_> = mapping
            .items
            .iter()
            .map(|i| format!("{}", i))
            .collect();
        Some(format!("use {}::{{{}}};", mapping.rust_path, items.join(", ")))
    } else {
        // Single item: check if supported
        if mapping.items.contains(&item) {
            Some(format!("use {}::{};", mapping.rust_path, item))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_mapping() {
        let mapping = find_mapping("RUST.STD.IO").unwrap();
        assert_eq!(mapping.rust_path, "std::io");
        assert!(mapping.items.contains(&"stdin"));
        assert!(mapping.items.contains(&"stdout"));
    }

    #[test]
    fn test_find_mapping_not_found() {
        assert!(find_mapping("RUST.UNKNOWN").is_none());
        assert!(find_mapping("Zoo").is_none());
    }

    #[test]
    fn test_is_rust_namespace() {
        assert!(is_rust_namespace("RUST.STD.IO"));
        assert!(is_rust_namespace("RUST.STD.COLLECTIONS"));
        assert!(!is_rust_namespace("Zoo"));
        assert!(!is_rust_namespace("RUST"));  // Too short, not specific enough
    }

    #[test]
    fn test_expand_wildcard() {
        let items = expand_wildcard("RUST.STD.IO").unwrap();
        assert!(items.contains(&"stdin"));
        assert!(items.contains(&"stdout"));
        assert!(items.contains(&"Read"));
        assert!(items.contains(&"Write"));
        assert!(!items.contains(&"unknown_item"));
    }

    #[test]
    fn test_expand_wildcard_not_found() {
        assert!(expand_wildcard("RUST.UNKNOWN").is_none());
    }

    #[test]
    fn test_get_rust_use_single() {
        let use_stmt = get_rust_use("RUST.STD.IO", "stdin").unwrap();
        assert_eq!(use_stmt, "use std::io::stdin;");
    }

    #[test]
    fn test_get_rust_use_wildcard() {
        let use_stmt = get_rust_use("RUST.STD.IO", "*").unwrap();
        assert!(use_stmt.starts_with("use std::io::{"));
        assert!(use_stmt.contains("stdin"));
        assert!(use_stmt.contains("stdout"));
        assert!(use_stmt.ends_with("};"));
    }

    #[test]
    fn test_get_rust_use_unsupported_item() {
        assert!(get_rust_use("RUST.STD.IO", "unsupported").is_none());
    }

    #[test]
    fn test_get_rust_use_unknown_namespace() {
        assert!(get_rust_use("RUST.UNKNOWN", "stdin").is_none());
    }

    #[test]
    fn test_all_mappings_have_items() {
        for mapping in RUST_MAPPINGS {
            assert!(!mapping.hust_path.is_empty());
            assert!(!mapping.rust_path.is_empty());
            assert!(!mapping.items.is_empty());
        }
    }

    #[test]
    fn test_no_duplicate_hust_paths() {
        let mut paths: Vec<_> = RUST_MAPPINGS.iter().map(|m| m.hust_path).collect();
        paths.sort();
        paths.dedup();
        assert_eq!(paths.len(), RUST_MAPPINGS.len());
    }
}
