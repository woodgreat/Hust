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

    /// Transpile source code (V0.6 with class support)
    pub fn transpile(&self, source: &str) -> Result<String, TranspileError> {
        let mut output = source.to_string();

        // V0.6 transform rules (order matters!):
        // 1. Interface definitions (before class to handle implements)
        // 2. Class definitions (before functions to handle methods)
        // 3. Remove use statements
        // 4. Function definitions with visibility
        // 5-12. Other transformations...

        // Rule 1: Transform interface definitions to traits
        // interface Shape { public f64 area(); } -> trait Shape { fn area(&self) -> f64; }
        output = self.transform_interface_definitions(&output)?;

        // Rule 2: Transform class definitions
        // class Point { i32 x; public i32 getX() { return self.x; } }
        // -> struct Point { x: i32 } impl Point { fn get_x(&self) -> i32 { self.x } }
        output = self.transform_class_definitions(&output)?;

        // Rule 3: Remove use statements (they're handled at module level)
        output = self.remove_use_statements(&output)?;

        // Rule 4: Function definition transform with visibility
        // public void func() -> pub fn func()
        output = self.transform_function_definitions(&output)?;

        // Rule 5: Transform multi-dimensional array declaration
        output = self.transform_multi_array_decl(&output)?;

        // Rule 6: Array declaration transform (fixed arrays)
        output = self.transform_array_declarations(&output)?;

        // Rule 7: Transform dynamic array declaration
        output = self.transform_dynamic_array_decl(&output)?;

        // Rule 8: Transform C-style for loops
        output = self.transform_for_loop(&output)?;

        // Rule 9: Variable declaration transform
        output = self.transform_variable_declarations(&output)?;

        // Rule 10: Remove parentheses from if/while conditions
        output = self.remove_condition_parens(&output)?;

        // Rule 11: Transform String initialization
        output = self.transform_string_init(&output)?;

        // Rule 12: Transform pass to ()
        output = self.transform_pass(&output)?;

        // Rule 13: Transform class instantiation
        // ClassName var; -> let mut var: ClassName = ClassName { ... };
        output = self.transform_class_instantiation(&output)?;

        // Rule 14: Transform method calls from camelCase to snake_case
        // obj.methodName() -> obj.method_name()
        output = self.transform_method_calls(&output)?;

        Ok(output)
    }

    /// V0.6: Transform method calls from camelCase to snake_case
    fn transform_method_calls(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Pattern: .methodName( -> .method_name(
        let re = Regex::new(r"\.([a-z][a-zA-Z0-9]*)\s*\(")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let method_name = &caps[1];
            let rust_method = self.to_snake_case(method_name);
            format!(".{}(", rust_method)
        });

        Ok(result.to_string())
    }

    /// V0.6: Transform class instantiation
    /// ClassName var; -> let mut var: ClassName = ClassName { field: default };
    fn transform_class_instantiation(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Pattern: ClassName var;
        // Match capitalized word followed by variable name
        let re = Regex::new(r"\b([A-Z][a-zA-Z0-9_]*)\s+([a-z_][a-zA-Z0-9_]*)\s*;")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let class_name = &caps[1];
            let var_name = &caps[2];

            // Check if it looks like a class name (starts with uppercase)
            // and not a primitive type
            if self.is_primitive_type(class_name) {
                // Keep as-is for primitive types
                format!("{} {};", class_name, var_name)
            } else {
                // Transform to Rust struct instantiation with default values
                // For now, use Default::default() - requires #[derive(Default)]
                format!("let mut {}: {} = {}::default();", var_name, class_name, class_name)
            }
        });

        Ok(result.to_string())
    }

    /// Check if a type name is a primitive type
    fn is_primitive_type(&self, type_name: &str) -> bool {
        matches!(type_name,
            "i8" | "i16" | "i32" | "i64" |
            "u8" | "u16" | "u32" | "u64" |
            "f32" | "f64" | "bool" | "char" | "String"
        )
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

    /// V0.6: Transform interface definitions to Rust traits
    /// interface Shape { public f64 area(); } -> trait Shape { fn area(&self) -> f64; }
    fn transform_interface_definitions(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Match interface definition
        // interface Name { method declarations }
        let re = Regex::new(r"(?m)^\s*interface\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\{([^}]+)\}")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let interface_name = &caps[1];
            let body = &caps[2];

            // Transform method declarations in interface
            let trait_body = self.transform_interface_methods(body);

            format!("trait {} {{{}}}\n", interface_name, trait_body)
        });

        Ok(result.to_string())
    }

    /// Transform interface method declarations to trait method signatures
    fn transform_interface_methods(&self, body: &str) -> String {
        use regex::Regex;

        let mut result = Vec::new();

        // Match method declarations: public ReturnType methodName(params);
        let re = Regex::new(r"(?m)^\s*(public\s+)?(\w+)\s+(\w+)\s*\(([^)]*)\)\s*;").unwrap();

        for caps in re.captures_iter(body) {
            let ret_type = &caps[2];
            let method_name = &caps[3];
            let params = &caps[4];

            // Convert method name to snake_case
            let rust_method = self.to_snake_case(method_name);

            // Transform parameters
            let rust_params = self.transform_method_params(params);

            // Build signature
            let sig = if ret_type == "void" {
                format!("fn {}(&self{});", rust_method, rust_params)
            } else {
                format!("fn {}(&self{}) -> {};", rust_method, rust_params, ret_type)
            };

            result.push(sig);
        }

        if result.is_empty() {
            String::new()
        } else {
            format!("\n    {}\n", result.join("\n    "))
        }
    }

    /// V0.6: Transform class definitions to Rust struct + impl
    fn transform_class_definitions(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Match class definition with optional extends and implements
        // The body is matched up to a line containing only }
        let re = Regex::new(r"(?m)^\s*class\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*(?:extends\s+(\w+))?\s*(?:implements\s+([\w,\s]+))?\s*\{([\s\S]*?)^\}")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let class_name = &caps[1];
            let parent_class = caps.get(2).map(|m| m.as_str());
            let interfaces = caps.get(3).map(|m| m.as_str());
            let body = &caps[4];

            // Parse class body into fields and methods
            let (fields, methods) = self.parse_class_body(body);

            // Generate struct
            let struct_def = self.generate_struct(class_name, &fields, parent_class);

            // Generate impl block
            let impl_def = self.generate_impl(class_name, &methods, parent_class, interfaces);

            format!("{}\n{}", struct_def, impl_def)
        });

        Ok(result.to_string())
    }

    /// Parse class body into fields and methods
    /// First extracts all methods, then treats remaining lines as fields
    fn parse_class_body(&self, body: &str) -> (Vec<ClassField>, Vec<ClassMethod>) {
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut remaining = body.to_string();

        // Extract all methods first (supports multi-line with (?s))
        use regex::Regex;
        let method_re = Regex::new(r"(?s)(public\s+)?(\w+)\s+(\w+)\s*\(([^)]*)\)\s*\{[^{}]*\}").unwrap();

        for caps in method_re.captures_iter(body) {
            let full_match = caps.get(0).unwrap();
            let method_text = full_match.as_str();

            if let Some(method) = self.parse_method(method_text) {
                methods.push(method);
                // Remove this method from remaining text
                remaining = remaining.replace(method_text, "");
            }
        }

        // Parse remaining lines as fields
        for line in remaining.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") {
                continue;
            }

            // Field declaration: Type name;
            if line.contains(";") && !line.contains("(") {
                if let Some(field) = self.parse_field(line) {
                    fields.push(field);
                }
            }
        }

        (fields, methods)
    }

    /// Parse a field declaration
    fn parse_field(&self, line: &str) -> Option<ClassField> {
        // Pattern: [visibility] Type name;
        let parts: Vec<&str> = line.split_whitespace().collect();

        let mut idx = 0;
        let visibility = if parts.get(idx) == Some(&"public") {
            idx += 1;
            Visibility::Public
        } else {
            Visibility::Private
        };

        if parts.len() < idx + 2 {
            return None;
        }

        let type_name = parts[idx];
        let field_name = parts[idx + 1].trim_end_matches(";");

        Some(ClassField {
            name: field_name.to_string(),
            type_name: type_name.to_string(),
            visibility,
        })
    }

    /// Parse a method declaration (supports multi-line with (?s))
    fn parse_method(&self, text: &str) -> Option<ClassMethod> {
        use regex::Regex;

        // Pattern: [public] ReturnType name(params) [{ body }]
        // (?s) makes . match newlines
        let re = Regex::new(r"(?s)^\s*(public\s+)?(\w+)\s+(\w+)\s*\(([^)]*)\)\s*(\{[^{}]*\})?").unwrap();

        let caps = re.captures(text)?;

        let is_public = caps.get(1).is_some();
        let ret_type = caps[2].to_string();
        let name = caps[3].to_string();
        let params = caps[4].to_string();
        let body = caps.get(5).map(|m| m.as_str().to_string());

        Some(ClassMethod {
            name,
            ret_type,
            params,
            body,
            visibility: if is_public { Visibility::Public } else { Visibility::Private },
        })
    }

    /// Generate Rust struct from class fields
    fn generate_struct(
        &self,
        class_name: &str,
        fields: &[ClassField],
        parent_class: Option<&str>,
    ) -> String {
        // Add derive macros for Default
        let mut result = format!("#[derive(Default)]\nstruct {} {{", class_name);

        // If has parent, include parent as field
        if let Some(parent) = parent_class {
            result.push_str(format!("\n    {}: {},", self.to_snake_case(parent), parent).as_str());
        }

        // Add own fields
        for field in fields {
            let rust_field = self.to_snake_case(&field.name);
            result.push_str(format!("\n    {}: {},", rust_field, field.type_name).as_str());
        }

        result.push_str("\n}\n");
        result
    }

    /// Generate Rust impl block from class methods
    /// If implements interfaces, generates separate impl blocks
    fn generate_impl(
        &self,
        class_name: &str,
        methods: &[ClassMethod],
        _parent_class: Option<&str>,
        interfaces: Option<&str>,
    ) -> String {
        let mut result = String::new();

        // Generate trait implementations for interfaces
        if let Some(ifs) = interfaces {
            let if_names: Vec<&str> = ifs.split(',').map(|s| s.trim()).collect();
            for if_name in if_names {
                let trait_impl = self.generate_trait_impl(class_name, if_name, methods);
                result.push_str(&trait_impl);
            }
        }

        // Generate inherent impl block for class methods
        let inherent_impl = self.generate_inherent_impl(class_name, methods);
        result.push_str(&inherent_impl);

        result
    }

    /// Generate trait implementation for an interface
    fn generate_trait_impl(
        &self,
        class_name: &str,
        interface_name: &str,
        methods: &[ClassMethod],
    ) -> String {
        let mut result = format!("impl {} for {} {{", interface_name, class_name);

        // Only include public methods in trait impl
        for method in methods {
            if !matches!(method.visibility, Visibility::Public) {
                continue;
            }

            let rust_name = self.to_snake_case(&method.name);
            let rust_params = self.transform_method_params(&method.params);

            let needs_mut = method.body.as_ref()
                .map(|b| b.contains("self.") && b.contains("="))
                .unwrap_or(false);
            let self_param = if needs_mut { "&mut self" } else { "&self" };

            let sig = if method.ret_type == "void" {
                format!("\n    fn {}({}{}) {{", rust_name, self_param, rust_params)
            } else {
                format!("\n    fn {}({}{}) -> {} {{", rust_name, self_param, rust_params, method.ret_type)
            };

            result.push_str(&sig);

            if let Some(ref body) = method.body {
                let body_content = body.trim_start_matches('{').trim_end_matches('}');
                let transformed_body = self.transform_self_references(body_content);
                result.push_str(&transformed_body);
            }

            result.push_str("\n    }");
        }

        result.push_str("\n}\n");
        result
    }

    /// Generate inherent impl block (class methods)
    fn generate_inherent_impl(
        &self,
        class_name: &str,
        methods: &[ClassMethod],
    ) -> String {
        let mut result = format!("impl {} {{", class_name);

        for method in methods {
            let vis = match method.visibility {
                Visibility::Public => "pub ",
                Visibility::Private => "",
            };

            let rust_name = self.to_snake_case(&method.name);
            let rust_params = self.transform_method_params(&method.params);

            let needs_mut = method.body.as_ref()
                .map(|b| b.contains("self.") && b.contains("="))
                .unwrap_or(false);
            let self_param = if needs_mut { "&mut self" } else { "&self" };

            let sig = if method.ret_type == "void" {
                format!("\n    {}fn {}({}{}) {{", vis, rust_name, self_param, rust_params)
            } else {
                format!("\n    {}fn {}({}{}) -> {} {{", vis, rust_name, self_param, rust_params, method.ret_type)
            };

            result.push_str(&sig);

            if let Some(ref body) = method.body {
                let body_content = body.trim_start_matches('{').trim_end_matches('}');
                let transformed_body = self.transform_self_references(body_content);
                result.push_str(&transformed_body);
            } else {
                result.push_str("()");
            }

            result.push_str("\n    }");
        }

        result.push_str("\n}\n");
        result
    }

    /// Transform method parameters (add comma before self params)
    fn transform_method_params(&self, params: &str) -> String {
        if params.trim().is_empty() {
            String::new()
        } else {
            format!(", {}", self.transform_params(params))
        }
    }

    /// Transform method body content
    /// - self.x -> self.x
    /// - method calls: obj.methodName() -> obj.method_name()
    fn transform_self_references(&self, body: &str) -> String {
        let mut result = body.to_string();

        // Transform method calls from camelCase to snake_case
        // Pattern: .methodName( -> .method_name(
        use regex::Regex;
        let re = Regex::new(r"\.([a-z][a-zA-Z0-9]*)\s*\(").unwrap();
        result = re.replace_all(&result, |caps: &regex::Captures| {
            let method_name = &caps[1];
            let rust_method = self.to_snake_case(method_name);
            format!(".{}(", rust_method)
        }).to_string();

        result
    }

    /// Convert camelCase to snake_case
    fn to_snake_case(&self, name: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = name.chars().collect();

        for (i, c) in chars.iter().enumerate() {
            if c.is_uppercase() && i > 0 {
                result.push('_');
                result.push(c.to_ascii_lowercase());
            } else {
                result.push(c.to_ascii_lowercase());
            }
        }

        result
    }
}

/// Class field representation
#[derive(Debug)]
struct ClassField {
    name: String,
    type_name: String,
    visibility: Visibility,
}

/// Class method representation
#[derive(Debug)]
struct ClassMethod {
    name: String,
    ret_type: String,
    params: String,
    body: Option<String>,
    visibility: Visibility,
}

/// Visibility enum
#[derive(Debug)]
enum Visibility {
    Public,
    Private,
}

/// Main entry function
pub fn transpile(source: &str) -> Result<String, TranspileError> {
    let translator = Translator::default();
    translator.transpile(source)
}