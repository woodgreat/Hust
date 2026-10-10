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
    // Standard I/O (2026-10-06: new system library path, no namespace)
    RustMapping {
        hust_path: "RUST.IO",
        rust_path: "std::io",
        items: &[
            "stdin", "stdout", "stderr",
            "Read", "Write", "BufRead", "BufReader", "BufWriter",
            "Error", "ErrorKind", "Result",
        ],
    },
    // Result type (new path)
    RustMapping {
        hust_path: "RUST.RESULT",
        rust_path: "std::result",
        items: &[
            "Result", "Ok", "Err",
        ],
    },
    // Option type (new path)
    RustMapping {
        hust_path: "RUST.OPTION",
        rust_path: "std::option",
        items: &[
            "Option", "Some", "None",
        ],
    },
    // String type (new path)
    RustMapping {
        hust_path: "RUST.STRING",
        rust_path: "std::string",
        items: &[
            "String", "FromUtf8Error", "FromUtf16Error",
        ],
    },
    // str type (new path)
    RustMapping {
        hust_path: "RUST.STR",
        rust_path: "std::str",
        items: &[
            "from_utf8", "from_utf16", "Chars", "Bytes", "Lines",
        ],
    },
    // Char type (new path)
    RustMapping {
        hust_path: "RUST.CHAR",
        rust_path: "std::char",
        items: &[
            "from_u32", "from_digit", "is_alphabetic", "is_numeric",
            "is_alphanumeric", "is_whitespace", "is_control",
            "to_uppercase", "to_lowercase",
        ],
    },
    // Path handling (new path)
    RustMapping {
        hust_path: "RUST.PATH",
        rust_path: "std::path",
        items: &[
            "Path", "PathBuf", "Component", "Components",
            "Iter", "Ancestors",
        ],
    },
    // Networking (new path)
    RustMapping {
        hust_path: "RUST.NET",
        rust_path: "std::net",
        items: &[
            "TcpStream", "TcpListener", "UdpSocket",
            "IpAddr", "Ipv4Addr", "Ipv6Addr", "SocketAddr",
        ],
    },
    // Threading (new path)
    RustMapping {
        hust_path: "RUST.THREAD",
        rust_path: "std::thread",
        items: &[
            "spawn", "sleep", "yield_now", "current",
            "JoinHandle", "Thread",
        ],
    },
    // Synchronization (new path)
    RustMapping {
        hust_path: "RUST.SYNC",
        rust_path: "std::sync",
        items: &[
            "Mutex", "RwLock", "Arc", "Barrier",
            "Condvar", "mpsc",
        ],
    },
    // Collections (new path)
    RustMapping {
        hust_path: "RUST.COLLECTIONS",
        rust_path: "std::collections",
        items: &[
            "HashMap", "HashSet", "BTreeMap", "BTreeSet",
            "VecDeque", "LinkedList", "BinaryHeap",
        ],
    },
    // Time (new path)
    RustMapping {
        hust_path: "RUST.TIME",
        rust_path: "std::time",
        items: &[
            "Duration", "Instant", "SystemTime", "SystemTimeError",
        ],
    },
    // File system (new path)
    RustMapping {
        hust_path: "RUST.FS",
        rust_path: "std::fs",
        items: &[
            "File", "OpenOptions", "DirEntry", "ReadDir",
            "create_dir", "create_dir_all", "remove_dir", "remove_dir_all",
            "remove_file", "rename", "copy", "hard_link", "soft_link",
            "metadata", "read", "read_to_string", "read_link", "write",
        ],
    },
    // Environment (new path)
    RustMapping {
        hust_path: "RUST.ENV",
        rust_path: "std::env",
        items: &[
            "args", "args_os", "var", "var_os", "set_var", "remove_var",
            "current_dir", "current_exe", "home_dir", "temp_dir",
        ],
    },
    // Process (new path)
    RustMapping {
        hust_path: "RUST.PROCESS",
        rust_path: "std::process",
        items: &[
            "Command", "Child", "ChildStdin", "ChildStdout", "ChildStderr",
            "Output", "ExitStatus", "ExitCode", "Stdio",
            "abort", "exit", "id",
        ],
    },
    // Formatting (new path)
    RustMapping {
        hust_path: "RUST.FMT",
        rust_path: "std::fmt",
        items: &[
            "Display", "Debug", "Formatter", "Result", "Error",
            "Write", "format", "format_args",
        ],
    },
    // Macros (new path)
    RustMapping {
        hust_path: "RUST.MACROS",
        rust_path: "std::macros",
        items: &[
            "println", "print", "eprintln", "eprint", "format",
            "vec", "assert", "assert_eq", "assert_ne",
            "debug_assert", "debug_assert_eq", "debug_assert_ne",
            "panic", "unimplemented", "unreachable", "todo",
        ],
    },
    // Memory (new path, from core::mem)
    RustMapping {
        hust_path: "RUST.MEM",
        rust_path: "core::mem",
        items: &[
            "size_of", "align_of", "size_of_val", "align_of_val",
            "drop", "forget", "replace", "swap", "take",
            "zeroed", "uninitialized", "transmute",
        ],
    },
    // Error trait (new path)
    RustMapping {
        hust_path: "RUST.ERROR",
        rust_path: "std::error",
        items: &[
            "Error", "ErrorKind",
        ],
    },
    // Conversion traits (new path)
    RustMapping {
        hust_path: "RUST.CONVERT",
        rust_path: "std::convert",
        items: &[
            "From", "Into", "TryFrom", "TryInto", "AsRef", "AsMut", "FromStr",
        ],
    },
    // Iterator trait (new path)
    RustMapping {
        hust_path: "RUST.ITER",
        rust_path: "std::iter",
        items: &[
            "Iterator", "IntoIterator", "FromIterator", "DoubleEndedIterator",
            "ExactSizeIterator", "Extend", "Sum", "Product",
            "once", "repeat", "empty", "from_fn", "successors",
        ],
    },
    // Operator traits (new path)
    RustMapping {
        hust_path: "RUST.OPS",
        rust_path: "std::ops",
        items: &[
            "Add", "Sub", "Mul", "Div", "Rem", "Neg", "Not",
            "BitAnd", "BitOr", "BitXor", "Shl", "Shr",
            "AddAssign", "SubAssign", "MulAssign", "DivAssign", "RemAssign",
            "BitAndAssign", "BitOrAssign", "BitXorAssign", "ShlAssign", "ShrAssign",
            "Deref", "DerefMut", "Drop", "Fn", "FnMut", "FnOnce", "Index", "IndexMut",
            "Range", "RangeFrom", "RangeFull", "RangeInclusive", "RangeTo", "RangeToInclusive",
        ],
    },
    // Comparison traits (new path)
    RustMapping {
        hust_path: "RUST.CMP",
        rust_path: "std::cmp",
        items: &[
            "Ord", "Eq", "PartialOrd", "PartialEq", "Ordering", "Reverse",
            "max", "min", "max_by", "min_by", "max_by_key", "min_by_key",
        ],
    },
    // Clone trait (new path)
    RustMapping {
        hust_path: "RUST.CLONE",
        rust_path: "std::clone",
        items: &[
            "Clone",
        ],
    },
    // Default trait (new path)
    RustMapping {
        hust_path: "RUST.DEFAULT",
        rust_path: "std::default",
        items: &[
            "Default",
        ],
    },
    // Borrow trait (new path)
    RustMapping {
        hust_path: "RUST.BORROW",
        rust_path: "std::borrow",
        items: &[
            "Borrow", "BorrowMut", "Cow", "ToOwned",
        ],
    },
    // Any trait (new path)
    RustMapping {
        hust_path: "RUST.ANY",
        rust_path: "std::any",
        items: &[
            "Any", "TypeId", "type_name", "type_name_of_val",
        ],
    },
    // Marker traits (new path)
    RustMapping {
        hust_path: "RUST.MARKER",
        rust_path: "std::marker",
        items: &[
            "Send", "Sync", "Copy", "Sized", "Unpin", "PhantomData", "PhantomPinned",
        ],
    },
    // Pin (new path)
    RustMapping {
        hust_path: "RUST.PIN",
        rust_path: "std::pin",
        items: &[
            "Pin", "pin",
        ],
    },
    // Future trait (new path)
    RustMapping {
        hust_path: "RUST.FUTURE",
        rust_path: "std::future",
        items: &[
            "Future", "poll_fn", "ready", "pending", "join", "select",
        ],
    },
    // Task module (new path)
    RustMapping {
        hust_path: "RUST.TASK",
        rust_path: "std::task",
        items: &[
            "Context", "Poll", "RawWaker", "RawWakerVTable", "Waker", "Wake",
        ],
    },
    // Arc (new path)
    RustMapping {
        hust_path: "RUST.ARC",
        rust_path: "std::sync",
        items: &[
            "Arc", "Weak",
        ],
    },
    // Rc (new path)
    RustMapping {
        hust_path: "RUST.RC",
        rust_path: "std::rc",
        items: &[
            "Rc", "Weak",
        ],
    },
    // Cell (new path)
    RustMapping {
        hust_path: "RUST.CELL",
        rust_path: "std::cell",
        items: &[
            "Cell", "RefCell", "Ref", "RefMut", "OnceCell",
        ],
    },
    // Box (new path)
    RustMapping {
        hust_path: "RUST.BOX",
        rust_path: "std::boxed",
        items: &[
            "Box",
        ],
    },
    // Vec (new path)
    RustMapping {
        hust_path: "RUST.VEC",
        rust_path: "std::vec",
        items: &[
            "Vec",
        ],
    },
    // Standard I/O (legacy path, kept for compatibility)
    RustMapping {
        hust_path: "RUST.STD.IO",
        rust_path: "std::io",
        items: &[
            "stdin", "stdout", "stderr",
            "Read", "Write", "BufRead", "BufReader", "BufWriter",
            "Error", "ErrorKind", "Result",
        ],
    },
    // Result type
    RustMapping {
        hust_path: "RUST.STD.RESULT",
        rust_path: "std::result",
        items: &[
            "Result", "Ok", "Err",
        ],
    },
    // Option type
    RustMapping {
        hust_path: "RUST.STD.OPTION",
        rust_path: "std::option",
        items: &[
            "Option", "Some", "None",
        ],
    },
    // String type
    RustMapping {
        hust_path: "RUST.STD.STRING",
        rust_path: "std::string",
        items: &[
            "String", "FromUtf8Error", "FromUtf16Error",
        ],
    },
    // str type
    RustMapping {
        hust_path: "RUST.STD.STR",
        rust_path: "std::str",
        items: &[
            "from_utf8", "from_utf16", "Chars", "Bytes", "Lines",
        ],
    },
    // Char type
    RustMapping {
        hust_path: "RUST.STD.CHAR",
        rust_path: "std::char",
        items: &[
            "from_u32", "from_digit", "is_alphabetic", "is_numeric",
            "is_alphanumeric", "is_whitespace", "is_control",
            "to_uppercase", "to_lowercase",
        ],
    },
    // Path handling
    RustMapping {
        hust_path: "RUST.STD.PATH",
        rust_path: "std::path",
        items: &[
            "Path", "PathBuf", "Component", "Components",
            "Iter", "Ancestors",
        ],
    },
    // Networking
    RustMapping {
        hust_path: "RUST.STD.NET",
        rust_path: "std::net",
        items: &[
            "TcpStream", "TcpListener", "UdpSocket",
            "IpAddr", "Ipv4Addr", "Ipv6Addr", "SocketAddr",
        ],
    },
    // Threading
    RustMapping {
        hust_path: "RUST.STD.THREAD",
        rust_path: "std::thread",
        items: &[
            "spawn", "sleep", "yield_now", "current",
            "JoinHandle", "Thread",
        ],
    },
    // Synchronization
    RustMapping {
        hust_path: "RUST.STD.SYNC",
        rust_path: "std::sync",
        items: &[
            "Mutex", "RwLock", "Arc", "Barrier",
            "Condvar", "mpsc",
        ],
    },
    // Collections
    RustMapping {
        hust_path: "RUST.STD.COLLECTIONS",
        rust_path: "std::collections",
        items: &[
            "HashMap", "HashSet", "BTreeMap", "BTreeSet",
            "VecDeque", "LinkedList", "BinaryHeap",
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
    } else if item.is_empty() {
        // Fix B1: whole module import (e.g., use RUST.IO; -> use std::io;)
        Some(format!("use {};", mapping.rust_path))
    } else {
        // Single item: check if supported
        if mapping.items.contains(&item) {
            Some(format!("use {}::{};", mapping.rust_path, item))
        } else {
            None
        }
    }
}

/// Get a list of all supported system namespace paths (for error messages)
pub fn get_supported_namespaces() -> Vec<&'static str> {
    RUST_MAPPINGS.iter().map(|m| m.hust_path).collect()
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
