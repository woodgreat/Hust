//! Plugin System Module
//! Provides extensible transformation plugin interface

use thiserror::Error;

/// Plugin Error
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("Plugin load failed: {0}")]
    LoadError(String),

    #[error("Plugin execution failed: {0}")]
    ExecutionError(String),

    #[error("Plugin not found: {0}")]
    NotFound(String),
}

/// Plugin trait - all transformation plugins must implement
pub trait Plugin {
    /// Plugin name
    fn name(&self) -> &str;

    /// Plugin description
    fn description(&self) -> &str;

    /// Version
    fn version(&self) -> &str;

    /// Transform code
    /// Input source code, return transformed code
    fn transform(&self, source: &str) -> Result<String, PluginError>;

    /// Priority (smaller value executes first)
    fn priority(&self) -> u32 {
        100
    }
}

/// Plugin Manager
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    /// Create new manager
    pub fn new() -> Self {
        Self { plugins: Vec::new() }

    }

    /// Register plugin
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    /// Execute all plugins (sorted by priority)
    pub fn execute_all(&self, source: &str) -> Result<String, PluginError> {
        // Sort by priority
        let mut sorted = self.plugins.iter().collect::<Vec<_>>();
        sorted.sort_by_key(|p| p.priority());

        // Execute sequentially
        let mut result = source.to_string();
        for plugin in sorted {
            result = plugin.transform(&result)?;
        }

        Ok(result)
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

// Built-in plugin examples
mod builtins {
    use super::*;

    /// Syntax simplification plugin
    pub struct SyntaxSimplifyPlugin;

    impl Plugin for SyntaxSimplifyPlugin {
        fn name(&self) -> &str { "syntax-simplify" }
        fn description(&self) -> &str { "Simplify syntax sugar for cleaner code" }
        fn version(&self) -> &str { "0.1.0" }

        fn transform(&self, source: &str) -> Result<String, PluginError> {
            // TODO: Implement simplification logic
            Ok(source.to_string())
        }
    }
}

pub use builtins::*;
