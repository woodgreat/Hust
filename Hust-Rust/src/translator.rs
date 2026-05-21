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

    /// Transpile source code (V0.1 minimal implementation)
    pub fn transpile(&self, source: &str) -> Result<String, TranspileError> {
        let mut output = source.to_string();

        // V0.1 transform rules:
        // 1. Variable declaration: "type var = value;" -> "let var: type = value;"
        // 2. Function definition: "void name(...)" -> "fn name(...)"
        // 3. Other content preserved as-is

        // Rule 1: Variable declaration transform
        // Pattern: (type) (var) = (value);
        // Supported types: i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, bool, char, String
        output = self.transform_variable_declarations(&output)?;

        // Rule 2: Function definition transform
        // void main() -> fn main()
        // i32 add(...) -> fn add(...) -> i32
        output = self.transform_function_definitions(&output)?;

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
            format!("let {}: {} =", var_name, type_name)
        });

        Ok(result.to_string())
    }

    /// V0.1: Transform function definitions
    /// void main() -> fn main()
    /// i32 add(i32 a, i32 b) -> fn add(a: i32, b: i32) -> i32
    fn transform_function_definitions(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // First handle void functions: void name(...) -> fn name(...)
        let re_void = Regex::new(r"\bvoid\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        let result = re_void.replace_all(source, |caps: &regex::Captures| {
            let func_name = &caps[1];
            format!("fn {}(", func_name)
        });

        // Then handle functions with return type: type name(params) -> fn name(params) -> type
        // Complex, simplified for V0.1
        let re_ret = Regex::new(r"\b(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|char|String)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        let result = re_ret.replace_all(&result, |caps: &regex::Captures| {
            let _type_name = &caps[1];
            let func_name = &caps[2];
            format!("fn {}(", func_name)
        });

        // Handle parameter types: type param -> param: type
        // V0.1 simplified: only single parameter
        let re_param = Regex::new(r"\(\s*(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|char|String)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\)")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        let result = re_param.replace_all(&result, |caps: &regex::Captures| {
            let type_name = &caps[1];
            let param_name = &caps[2];
            format!("({}: {})", param_name, type_name)
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

/// 主入口函数
pub fn transpile(source: &str) -> Result<String, TranspileError> {
    let translator = Translator::default();
    translator.transpile(source)
}