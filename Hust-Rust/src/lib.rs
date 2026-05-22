//! Hust-Rust Library Core
//! Provides transpiler functionality and plugin system

pub mod translator;
pub mod plugins;

// Re-exports
pub use translator::{Translator, TranspileOptions, TranspileError};
pub use plugins::{Plugin, PluginError, PluginManager};

// Main error type
pub use thiserror::Error as HustError;

/// Result type alias
pub type HustResult<T> = std::result::Result<T, translator::TranspileError>;

/// Version info
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
