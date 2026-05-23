//! Transpiler Core Module
//! Transpiles Hust code to Rust code

use std::collections::HashSet;
use std::path::PathBuf;
use thiserror::Error;

use crate::project::Module;

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
    /// Module context for multi-file compilation
    pub module_context: Option<ModuleContext>,
}

/// Module context for transpilation
#[derive(Debug, Clone, Default)]
pub struct ModuleContext {
    /// Current module name
    pub current_module: String,
    /// Imported module names
    pub imports: Vec<String>,
    /// Public functions in this module (for re-export)
    pub public_functions: HashSet<String>,
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

    /// Transpile source code (V0.5 with module support)
    pub fn transpile(&self, source: &str) -> Result<String, TranspileError> {
        let mut output = source.to_string();

        // V0.5 transform rules (order matters!):
        // 1. Remove use statements (processed separately)
        // 2. Function definitions with pub visibility
        // 3. Multi-dimensional arrays
        // 4. Fixed arrays
        // 5. Dynamic arrays
        // 6. C-style for loops
        // 7. Variable declarations
        // 8. if/while condition parentheses removal

        // Rule 1: Remove use statements (they're handled at module level)
        output = self.remove_use_statements(&output)?;

        // Rule 2: Function definition transform with pub visibility
        // pub void func() -> pub fn func()
        // void func() -> fn func() (private by default)
        output = self.transform_function_definitions(&output)?;

        // Rule 3: Transform multi-dimensional array declaration
        output = self.transform_multi_array_decl(&output)?;

        // Rule 4: Array declaration transform (fixed arrays)
        output = self.transform_array_declarations(&output)?;

        // Rule 5: Transform dynamic array declaration
        output = self.transform_dynamic_array_decl(&output)?;

        // Rule 6: Transform C-style for loops
        output = self.transform_for_loop(&output)?;

        // Rule 7: Variable declaration transform
        output = self.transform_variable_declarations(&output)?;

        // Rule 8: Remove parentheses from if/while conditions
        output = self.remove_condition_parens(&output)?;

        // Rule 9: Transform String initialization
        output = self.transform_string_init(&output)?;

        // Rule 10: Transform pass to ()
        output = self.transform_pass(&output)?;

        Ok(output)
    }

    /// V0.5: Remove use statements (handled at module level)
    fn remove_use_statements(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Match: use module_name;
        let re = Regex::new(r"(?m)^\s*use\s+[a-zA-Z_][a-zA-Z0-9_]*\s*;")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, "");

        Ok(result.to_string())
    }

    /// V0.4: Transform array declarations
    /// i32[5] arr = {1,2,3,4,5}; -> let mut arr: [i32; 5] = [1,2,3,4,5];
    fn transform_array_declarations(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Match: type[size] name = {elements};
        let re = Regex::new(r"\b(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool)\[(\d+)\]\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*\{([^}]+)\}")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let type_name = &caps[1];
            let size = &caps[2];
            let var_name = &caps[3];
            let elements = &caps[4];
            // Convert {1, 2, 3} to [1, 2, 3]
            format!("let mut {}: [{}; {}] = [{}];", var_name, type_name, size, elements)
        });

        Ok(result.to_string())
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

    /// V0.5: Transform function definitions with return types and visibility
    /// void main() -> fn main()
    /// pub i32 add(i32 a, i32 b) -> pub fn add(a: i32, b: i32) -> i32
    fn transform_function_definitions(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Pattern: [pub] type name(params) { body }
        // Capture optional pub, return type, name, and parameters
        let re = Regex::new(r"(?m)^\s*(pub\s+)?\b(void|i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|char|String)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(([^)]*)\)\s*\{")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let is_pub = caps.get(1).is_some();
            let ret_type = &caps[2];
            let func_name = &caps[3];
            let params = &caps[4];

            // Transform parameters: "i32 a, i32 b" -> "a: i32, b: i32"
            let transformed_params = self.transform_params(params);

            // Build visibility prefix
            let vis = if is_pub { "pub " } else { "" };

            // Build function signature
            if ret_type == "void" {
                format!("{}fn {}({}) {{", vis, func_name, transformed_params)
            } else {
                format!("{}fn {}({}) -> {} {{", vis, func_name, transformed_params, ret_type)
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
        // Handles both numeric literals and expressions like dynamic.len()
        let re = Regex::new(
            r"for\s*\(\s*(i8|i16|i32|i64|u8|u16|u32|u64)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*(\d+)\s*;\s*[a-zA-Z_][a-zA-Z0-9_]*\s*<\s*([^;]+)\s*;\s*[a-zA-Z_][a-zA-Z0-9_]*\s*=\s*[a-zA-Z_][a-zA-Z0-9_]*\s*\+\s*1\s*\)"
        ).map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let var_name = &caps[2];
            let start = &caps[3];
            let end = &caps[4].trim();
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

    /// V0.3: Transform pass to ()
    /// pass; -> ();
    fn transform_pass(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        let re = Regex::new(r"\bpass\s*;")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, "();");

        Ok(result.to_string())
    }

    /// V0.4: Transform dynamic array declaration
    /// i32[] arr; -> let mut arr: Vec<i32> = Vec::new();
    fn transform_dynamic_array_decl(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Match: type[] name;
        let re = Regex::new(r"\b(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool)\[\]\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*;")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let type_name = &caps[1];
            let var_name = &caps[2];
            format!("let mut {}: Vec<{}> = Vec::new();", var_name, type_name)
        });

        Ok(result.to_string())
    }

    /// V0.4: Transform multi-dimensional array declaration
    /// i32[3][4] matrix = {{1,2,3,4},{5,6,7,8},{9,10,11,12}};
    /// -> let mut matrix: [[i32; 4]; 3] = [[1,2,3,4],[5,6,7,8],[9,10,11,12]];
    fn transform_multi_array_decl(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Match: type[size1][size2] name = { {...}, {...} };
        // Use multiline mode and match nested braces by finding the closing brace
        let re = Regex::new(r"(?m)\b(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool)\[(\d+)\]\[(\d+)\]\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*\{([\s\S]*?)\}\s*;")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let type_name = &caps[1];
            let size1 = &caps[2];
            let size2 = &caps[3];
            let var_name = &caps[4];
            let elements = &caps[5];
            // Convert {{...},{...}} to [[...],[...]]
            let rust_elements = elements.replace("{", "[").replace("}", "]").replace(";", "");
            format!("let mut {}: [[{}; {}]; {}] = [{}];", var_name, type_name, size2, size1, rust_elements.trim())
        });

        Ok(result.to_string())
    }

    /// Transpile and write to file
    pub fn transpile_to_file(&self, input_path: &PathBuf, output_path: &PathBuf) -> Result<(), TranspileError> {
        let result = self.transpile_file(input_path)?;
        std::fs::write(output_path, result)?;
        Ok(())
    }

    /// V0.5: Transpile multiple modules into a single Rust file
    /// Merges all modules, handling imports and visibility
    pub fn transpile_modules(&self,
        modules: &[Module],
        entry_module: &Module,
    ) -> Result<String, TranspileError> {
        let mut all_code = String::new();

        // Add a comment header
        all_code.push_str("// Generated by Hust transpiler\n");
        all_code.push_str("// Multi-module compilation\n\n");

        // Transpile each module (except entry) as a separate section
        for module in modules {
            if module.name != entry_module.name {
                all_code.push_str(&format!("// Module: {}\n", module.name));
                let transpiled = self.transpile(&module.source)?;
                all_code.push_str(&transpiled);
                all_code.push_str("\n\n");
            }
        }

        // Transpile entry module last (main function)
        all_code.push_str(&format!("// Entry module: {}\n", entry_module.name));
        let entry_transpiled = self.transpile(&entry_module.source)?;
        all_code.push_str(&entry_transpiled);

        Ok(all_code)
    }
}

/// Main entry function
pub fn transpile(source: &str) -> Result<String, TranspileError> {
    let translator = Translator::default();
    translator.transpile(source)
}