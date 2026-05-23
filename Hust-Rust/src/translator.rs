//! Transpiler Core Module
//! Transpiles Hust code to Rust code

use std::path::PathBuf;
use thiserror::Error;

/// Transpile Error
#[derive(Error, Debug)]
pub enum TranspileError {
    #[error("File read failed: {0}")]
    FileRead(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Transform error: {0}")]
    TransformError(String),

    #[error("Write error: {0}")]
    WriteError(String),
}

/// Transpile Options
#[derive(Debug, Clone, Default)]
pub struct TranspileOptions {
    /// Whether to preserve comments
    pub preserve_comments: bool,
    /// Whether to output debug info
    pub debug: bool,
    /// Whether to enable all plugins
    pub enable_all_plugins: bool,
}

/// Transpiler
#[derive(Debug)]
pub struct Translator {
    options: TranspileOptions,
}

impl Translator {
    /// Create new transpiler
    pub fn new(options: TranspileOptions) -> Self {
        Self { options }
    }

    /// Default configuration
    pub fn default() -> Self {
        Self::new(TranspileOptions::default())
    }

    /// Transpile single file
    pub fn transpile_file(&self, input_path: &PathBuf) -> Result<String, TranspileError> {
        // 1. Read source file
        let source = std::fs::read_to_string(input_path)?;

        // 2. Transpile
        self.transpile(&source)
    }

    /// Transpile source code (V0.2 with control flow)
    pub fn transpile(&self, source: &str) -> Result<String, TranspileError> {
        let mut output = source.to_string();

        // V0.2/V0.3 transform rules (order matters!):
        // 1. Function definitions FIRST (before variable declarations)
        // 2. C-style for loops
        // 3. Variable declarations
        // 4. if/while condition parentheses removal

        // Rule 1: Function definition transform FIRST
        // This handles return types and parameter types
        output = self.transform_function_definitions(&output)?;

        // Rule 2: Transform C-style for loops to Rust iterator style
        // for (i32 i = 0; i < n; i = i + 1) -> for i in 0..n
        output = self.transform_for_loop(&output)?;

        // Rule 3: Variable declaration transform (not inside for loops)
        output = self.transform_variable_declarations(&output)?;

        // Rule 4: Remove parentheses from if/while conditions (Rust style)
        // if (x > 5) -> if x > 5
        // while (x > 5) -> while x > 5
        output = self.remove_condition_parens(&output)?;

        // Rule 5: Transform String initialization
        // String s = "hello"; -> let mut s: String = "hello".to_string();
        output = self.transform_string_init(&output)?;

        Ok(output)
    }

    /// V0.1: Transform variable declarations
    /// i32 x = 42; -> let x: i32 = 42;
    fn transform_variable_declarations(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Match: type var = value;
        // Type: i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|char|String
        // Var name: [a-zA-Z_][a-zA-Z0-9_]*
        let re = Regex::new(r"\b(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|char|String)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*=")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let type_name = &caps[1];
            let var_name = &caps[2];
            // Hust: default is mutable, const is immutable
            // Rust: default is immutable, mut is mutable
            format!("let mut {}: {} =", var_name, type_name)
        });

        Ok(result.to_string())
    }

    /// V0.3: Transform function definitions with return types
    /// void main() -> fn main()
    /// i32 add(i32 a, i32 b) -> fn add(a: i32, b: i32) -> i32
    fn transform_function_definitions(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Pattern: type name(params) { body }
        // Capture return type, name, and parameters
        let re = Regex::new(r"\b(void|i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|char|String)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(([^)]*)\)\s*\{")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let ret_type = &caps[1];
            let func_name = &caps[2];
            let params = &caps[3];
            
            // Transform parameters: "i32 a, i32 b" -> "a: i32, b: i32"
            let transformed_params = self.transform_params(params);
            
            // Build function signature
            if ret_type == "void" {
                format!("fn {}({}) {{", func_name, transformed_params)
            } else {
                format!("fn {}({}) -> {} {{", func_name, transformed_params, ret_type)
            }
        });

        Ok(result.to_string())
    }

    /// Transform function parameters
    /// "i32 a, i32 b" -> "a: i32, b: i32"
    fn transform_params(&self, params: &str) -> String {
        if params.trim().is_empty() {
            return String::new();
        }

        let mut result = Vec::new();
        // Split by comma and process each parameter
        for param in params.split(',') {
            let param = param.trim();
            if param.is_empty() {
                continue;
            }
            
            // Parse "type name"
            let parts: Vec<&str> = param.split_whitespace().collect();
            if parts.len() == 2 {
                let type_name = parts[0];
                let var_name = parts[1];
                result.push(format!("{}: {}", var_name, type_name));
            } else {
                // Keep as-is if can't parse
                result.push(param.to_string());
            }
        }
        
        result.join(", ")
    }

    /// V0.2: Transform C-style for loop to Rust iterator style
    /// for (i32 i = 0; i < n; i = i + 1) -> for i in 0..n
    fn transform_for_loop(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Pattern: for (type var = start; var < end; var = var + 1)
        // Simplified: only handles i = i + 1 increment, no backreferences
        let re = Regex::new(
            r"for\s*\(\s*(i8|i16|i32|i64|u8|u16|u32|u64)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*(\d+)\s*;\s*[a-zA-Z_][a-zA-Z0-9_]*\s*<\s*(\d+)\s*;\s*[a-zA-Z_][a-zA-Z0-9_]*\s*=\s*[a-zA-Z_][a-zA-Z0-9_]*\s*\+\s*1\s*\)"
        ).map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let var_name = &caps[2];
            let start = &caps[3];
            let end = &caps[4];
            format!("for {} in {}..{}", var_name, start, end)
        });

        Ok(result.to_string())
    }

    /// V0.2: Remove parentheses from if/while conditions (Rust style)
    /// if (x > 5) -> if x > 5
    fn remove_condition_parens(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Match: if (condition) or while (condition)
        // Replace with: if condition or while condition
        let re = Regex::new(r"\b(if|while)\s*\(\s*([^)]+)\s*\)")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let keyword = &caps[1];
            let condition = &caps[2];
            format!("{} {}", keyword, condition)
        });

        Ok(result.to_string())
    }

    /// V0.3: Transform String initialization with .to_string()
    /// String s = "hello"; -> let mut s: String = "hello".to_string();
    fn transform_string_init(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Match: let mut var: String = "value";
        let re = Regex::new(r"let mut ([a-zA-Z_][a-zA-Z0-9_]*): String = ([^;]+);")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let var_name = &caps[1];
            let value = &caps[2];
            // Check if value is a string literal
            if value.trim().starts_with('"') {
                format!("let mut {}: String = {}.to_string();", var_name, value)
            } else {
                // Keep as-is for non-string-literal values
                format!("let mut {}: String = {};", var_name, value)
            }
        });

        Ok(result.to_string())
    }

    /// Transpile and write to file
    pub fn transpile_to_file(&self, input_path: &PathBuf, output_path: &PathBuf) -> Result<(), TranspileError> {
        let result = self.transpile_file(input_path)?;
        std::fs::write(output_path, result)?;
        Ok(())
    }
}

/// Main entry function
pub fn transpile(source: &str) -> Result<String, TranspileError> {
    let translator = Translator::default();
    translator.transpile(source)
}