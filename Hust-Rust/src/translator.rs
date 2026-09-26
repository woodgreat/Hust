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
        // Early scan: check if any inheritance chain exceeds Rust's default
        // recursion limit (128). If so, emit #![recursion_limit = "32767"]
        // (i16::MAX — a fixed generous ceiling; the limit is only an upper
        // bound and does not affect compile-time performance).
        let max_depth = self.compute_max_inheritance_depth(source);
        let needs_limit = max_depth > 100; // headroom below 128

        let mut output = source.to_string();

        // Rule 0: Initialization reminder (owner design, 2026.09.20) —
        // 非阻断浅扫描：声明后无任何写入 → 提醒"使用前要先初始化，否则
        // 语言核可能报错"。先于一切变换执行，行号对应原始文件。
        self.remind_init_before_use(&output)?;

        // V0.6 transform rules (order matters!):
        // 1. Interface definitions (before class to handle implements)
        // 2. Class definitions (before functions to handle methods)
        // 3. Remove use statements
        // 4. Function definitions with visibility
        // 5-12. Other transformations...

        // Rule 1: Transform interface definitions to traits
        // interface Shape { public f64 area(); } -> trait Shape { fn area(&self) -> f64; }
        output = self.transform_interface_definitions(&output)?;

        // Rule 1.5: interface implementation methods must be public
        // (owner decision 2026.09.12: private method implementing a public
        // interface leaks via the trait path — fail-fast with guidance,
        // enforced here because generate_trait_impl returns String)
        self.check_interface_impl_visibility(&output)?;

        // Rule 2: Transform class definitions
        // class Point { i32 x; public i32 getX() { return self.x; } }
        // -> struct Point { x: i32 } impl Point { fn get_x(&self) -> i32 { self.x } }
        // First, transform inherited field access in global code (before class definitions are transformed)
        output = self.transform_inherited_field_access(&output)?;
        output = self.transform_class_definitions(&output)?;

        // Rule 3: Remove use statements (they're handled at module level)
        output = self.remove_use_statements(&output)?;

        // Rule 4: Function definition transform with visibility
        // public void func() -> pub fn func()
        output = self.transform_function_definitions(&output)?;

        // Rule 4.5: reject `const` on array declarations
        // Arrays are mutable by default (r6); r2's `const` applies to scalar
        // variables only. Without this guard the raw `const` survives into
        // `const let mut ...`, an illegal Rust construct with a cryptic error.
        Self::reject_const_array(&output)?;

        // Rule 4.6: reject `.to_vec()` on static arrays (all dimensions)
        // Principle (owner, 2026.09.09): a static array stays static — Hust
        // provides no automatic static->dynamic conversion; build dynamic
        // arrays with push([])/push(v) instead. Dynamic-on-dynamic calls
        // (e.g. `c[0].to_vec()` where c is `i32[][]`) are unaffected.
        Self::reject_static_to_vec(&output)?;

        // Rule 4.7: array dimensions must be literals or const names
        let const_names = Self::scan_const_dim_names(&output);
        Self::check_array_dimensions(&output, &const_names)?;

        // Rule 4.8: reject `const` without initializer (a const must have a
        // value to be a constant — `const i32 a, b;` / `const i32 a;` are
        // meaningless; use plain variables for delayed initialization)
        Self::reject_const_no_init(&output)?;

        // Rule 5: Transform multi-dimensional array declaration
        output = self.transform_multi_array_decl(&output)?;

        // Rule 6: Array declaration transform (fixed arrays)
        output = self.transform_array_declarations(&output)?;

        // Rule 7: Transform dynamic array declaration
        output = self.transform_dynamic_array_decl(&output)?;

        // Rule 7.5: `x.push([])` pushes an empty (nested) dynamic array —
        // how a row is created for Vec<Vec<T>> (2026.09.09)
        output = self.translate_empty_push(&output)?;

        // Rule 7.6: array literal assignment — x.f = {a, b, c}; or x[i] = {a};
        // -> x.f = [a, b, c];  (Rust: `{a,b,c}` is a block expression, not an
        // array literal; 2026.09.18, found by owner's q5). One level only —
        // nested literals ({{..},{..}}) not yet handled here.
        output = self.transform_array_literal_assign(&output)?;

        // Rule 8: Transform C-style for loops (MUST run BEFORE variable declarations)
        output = self.transform_for_loop(&output)?;

        // Rule 9: Variable declaration transform
        output = self.transform_variable_declarations(&output)?;

        // Rule 9.5: string literal assignment — x.f = "Alice"; -> .to_string()
        // (AFTER Rule 9 so declarations (`String name = "x";`) are handled
        // there first; 2026.09.18, found by owner's q5)
        output = self.translate_string_assign(&output)?;

        // Rule 10: Remove parentheses from if/while conditions
        output = self.remove_condition_parens(&output)?;

        // Rule 11: Transform String initialization
        output = self.transform_string_init(&output)?;
        
        // Rule 11.5: Transform string literals in function calls and returns
        // greet("World") -> greet("World".to_string())
        // return "Hi"; -> return "Hi".to_string();
        output = self.transform_string_literals_in_functions(&output)?;

        // Rule 12: Transform pass to ()
        output = self.transform_pass(&output)?;

        // Rule 13: Transform class instantiation
        // ClassName var; -> let mut var: ClassName = ClassName { ... };
        output = self.transform_class_instantiation(&output)?;

        // Rule 14: Transform method calls from camelCase to snake_case
        // obj.methodName() -> obj.method_name()
        output = self.transform_method_calls(&output)?;

        // Rule 15: Transform array slices
        // i32[] first5Scores = scores[0..5]; -> let first5Scores: &[i32] = &scores[0..5];
        output = self.transform_array_slices(&output)?;

        // Rule 16: Transform C-style type cast (type)expr to expr as type
        output = self.transform_type_cast(&output)?;

        // Rule 17: Normalize float literals
        // f32 x = 0; -> f32 x = 0.0;
        output = self.transform_float_literals(&output)?;

        // Rule 18: Transform array indices to use as usize
        // Rust requires usize for array indexing: scores[i] -> scores[(i) as usize]
        output = self.transform_array_indices(&output)?;

        // Prepend #![recursion_limit] if deep inheritance detected
        if needs_limit {
            output = format!("#![recursion_limit = \"32767\"]\n\n{}", output);
        }

        Ok(output)
    }

    /// Compute maximum inheritance depth in source.
    /// Returns the deepest class chain length (e.g., A extends B extends C => 3).
    fn compute_max_inheritance_depth(&self, source: &str) -> usize {
        let class_table = self.extract_class_table(source);
        let mut max_depth = 0;
        
        for class_name in class_table.keys() {
            let mut depth = 1;
            let mut current = class_table.get(class_name);
            let mut visited: HashSet<String> = HashSet::new();
            visited.insert(class_name.clone());
            
            while let Some(info) = current {
                if let Some(parent) = &info.parent {
                    if !visited.insert(parent.clone()) {
                        break; // Cycle
                    }
                    depth += 1;
                    current = class_table.get(parent);
                } else {
                    break;
                }
            }
            
            if depth > max_depth {
                max_depth = depth;
            }
        }
        
        max_depth
    }

    /// Rule 0: Initialization reminder (owner design, 2026.09.20).
    /// 语言核原则：不合成默认值，要不要默认值由程序员负责。转译器只在
    /// 转译时提醒（非阻断，不改变 exit code）：声明后未见任何写入的变量，
    /// 使用前要先初始化，否则将来语言核可能报错。
    ///
    /// 刻意只做浅扫描，不做定值初始化流分析：
    /// - 允许漏报（有赋值但顺序不对等情形，过渡期由 rustc E0381/E0382
    ///   拦截，将来由语言核拦截）
    /// - 绝不误拦编译
    /// - 跨函数同名变量的写入也会使提醒静默（漏报方向，可接受）
    /// class/interface 体内被掩蔽，字段声明（如 class 里的 `i32 id;`）不
    /// 触发提醒；动态数组声明自带 Vec::new() 初始化，不参与提醒。
    fn remind_init_before_use(&self, source: &str) -> Result<(), TranspileError> {
        use regex::Regex;

        // ---- 掩蔽 class/interface 体（保留换行，行号对应原始文件）----
        let head_re = Regex::new(r"\b(?:class|interface)\s+\w+[^{;]*\{")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        let bytes = source.as_bytes();
        let mut spans: Vec<(usize, usize)> = Vec::new();
        for m in head_re.find_iter(source) {
            let mut depth = 0i32;
            let mut i = m.end() - 1; // position of '{'
            let mut end = bytes.len();
            while i < bytes.len() {
                match bytes[i] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            spans.push((m.start(), end));
        }
        let mut masked = source.to_string();
        // 从后往前替换：前面的偏移不受影响
        for (start, end) in spans.iter().rev() {
            let blank: String = source[*start..*end]
                .chars()
                .map(|c| if c == '\n' { '\n' } else { ' ' })
                .collect();
            masked.replace_range(*start..*end, &blank);
        }

        let line_of = |pos: usize| masked[..pos].matches('\n').count() + 1;

        // ---- 收集无初始化器声明（与 Rule 6/9/13 的声明形式同形）----
        let mut candidates: Vec<(usize, String)> = Vec::new();
        let scalar_re = Regex::new(
            r"\b(?:i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|char|String)\s+([a-zA-Z_][a-zA-Z0-9_]*(?:\s*,\s*[a-zA-Z_][a-zA-Z0-9_]*)*)\s*;",
        )
        .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        let array_re = Regex::new(
            r"\b(?:i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool)(?:\[[a-zA-Z0-9_]+\])+\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*;",
        )
        .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        let class_re = Regex::new(
            r"\b([A-Z][a-zA-Z0-9_]*)\s+([a-z_][a-zA-Z0-9_]*(?:\s*,\s*[a-z_][a-zA-Z0-9_]*)*)\s*;",
        )
        .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        for cap in scalar_re.captures_iter(&masked) {
            let line = line_of(cap.get(0).unwrap().start());
            for raw in cap[1].split(',') {
                candidates.push((line, raw.trim().to_string()));
            }
        }
        for cap in array_re.captures_iter(&masked) {
            candidates.push((line_of(cap.get(0).unwrap().start()), cap[1].to_string()));
        }
        for cap in class_re.captures_iter(&masked) {
            if self.is_primitive_type(&cap[1]) {
                continue; // String 等原始类型已由 scalar_re 覆盖
            }
            let line = line_of(cap.get(0).unwrap().start());
            for raw in cap[2].split(',') {
                candidates.push((line, raw.trim().to_string()));
            }
        }

        // ---- 对每个名字做"有无写入"浅查 ----
        let mut reminded: HashSet<String> = HashSet::new();
        for (line, name) in candidates {
            if !reminded.insert(name.clone()) {
                continue;
            }
            let write_re = Regex::new(&format!(
                r"\b{}\s*(?:=[^=]|[+\-*/]=|\+\+|--|\[[^\]]*\]\s*=[^=]|\.\s*\w+\s*=[^=])",
                regex::escape(&name)
            ))
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;
            if !write_re.is_match(&masked) {
                eprintln!(
                    "[Hust 提醒] 第 {} 行: 变量 `{}` 声明后未赋值——使用前要先初始化，否则语言核可能报错。",
                    line, name
                );
            }
        }
        Ok(())
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

    /// V0.9: Transform array index expressions to use as usize
    /// Rust requires usize for array indexing, but Hust uses signed types.
    /// scores[i] -> scores[(i) as usize], arr[j-1] -> arr[(j-1) as usize]
    /// Handles nested arrays: matrix[i][j] -> matrix[(i) as usize][(j) as usize]
    ///
    /// Design note (拆离法, two-phase):
    /// Phase 1 hosts are variable names:  x[i]        -> x[(i) as usize]
    /// Phase 2 hosts are ']' (prev level): ][j]       -> ][(j) as usize
    /// Phase 1 consumes the ']' that Phase 2 needs as a host, so one pass
    /// cannot reach inner levels of nested arrays. Alternating phases in a
    /// loop peels one nesting level per iteration; converges in depth passes.
    fn transform_array_indices(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        let mut result = source.to_string();

        // 2-phase peeling. Index pattern starts with [a-zA-Z_]: untransformed
        // indices begin with a letter, transformed ones with '(' — so already
        // converted levels DO NOT MATCH and their ']' stays available as the
        // host for the next inner level (otherwise each pass consumes it and
        // 3+ dimensional chains stall one level short of convergence).
        let re_var = Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\[([a-zA-Z_][^\[\]]*)\]")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        let re_bracket = Regex::new(r"\]\[([a-zA-Z_][^\[\]]*)\]")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let is_transformable = |idx: &str| -> bool {
            !idx.contains(" as usize") && !idx.contains("..") && idx.parse::<i64>().is_err()
        };

        let mut changed = true;
        let mut rounds = 0;
        // One pass peels at least one nesting level (phase 1 the chain head,
        // phase 2 the deepest unconverted level), so N dims need N-1 rounds.
        // Cap sized for 13-D stress tests + headroom.
        while changed && rounds < 32 {
            changed = false;
            rounds += 1;

            let pass1 = re_var.replace_all(&result, |caps: &regex::Captures| {
                let idx = caps[2].trim();
                if is_transformable(idx) {
                    changed = true;
                    format!("{}[({}) as usize]", &caps[1], idx)
                } else {
                    caps[0].to_string()
                }
            });
            result = pass1.to_string();

            let pass2 = re_bracket.replace_all(&result, |caps: &regex::Captures| {
                let idx = caps[1].trim();
                if is_transformable(idx) {
                    changed = true;
                    format!("][({}) as usize]", idx)
                } else {
                    caps[0].to_string()
                }
            });
            result = pass2.to_string();
        }

        Ok(result)
    }

    /// V0.7: Transform C-style type cast (type)expr to expr as type
    /// (f32)sum -> sum as f32
    /// (i32)(a + b) -> (a + b) as i32
    /// (f32)(sum / 10) -> sum as f32 / 10 as f32 [distribute to operands for float]
    /// 
    /// 2026.09.26: 函数签名和函数体分离处理
    /// - 函数签名（fn name(...) -> type）不转换，程序员明确声明的类型
    /// - 函数体（{ ... }）转换类型，程序员可能需要类型转换
    fn transform_type_cast(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // 1. 识别函数签名边界（fn name(...) -> type {）
        // 匹配函数签名：fn name(params) -> return_type {
        let sig_re = Regex::new(r"fn\s+[a-zA-Z_][a-zA-Z0-9_]*\s*\([^)]*\)\s*(?:->\s*[^{]+)?\s*\{")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let mut result = String::new();
        let mut last_end = 0;

        // 2. 遍历所有函数签名，分离签名和函数体
        for m in sig_re.find_iter(source) {
            // 添加函数签名之前的代码（不转换）
            result.push_str(&source[last_end..m.start()]);
            
            // 添加函数签名（不转换，但不包含 {）
            let sig = m.as_str();
            let sig_without_brace = sig.trim_end_matches('{').trim_end();
            result.push_str(sig_without_brace);
            result.push_str(" {"); // 添加 { 作为函数体开始
            
            // 提取函数体（从 { 之后到匹配的 }）
            let body_start = m.end(); // { 之后的位置
            let mut depth = 1;
            let mut pos = m.end();
            let bytes = source.as_bytes();
            
            while pos < bytes.len() && depth > 0 {
                match bytes[pos] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                pos += 1;
            }
            
            if depth == 0 {
                // 找到匹配的 }，提取函数体（不包含 { 和 }）
                let body = &source[body_start..pos - 1]; // 排除最后的 }
                // 对函数体进行类型转换
                let transformed_body = self.transform_type_cast_in_body(body)?;
                result.push_str(&transformed_body);
                result.push_str("}"); // 添加 } 作为函数体结束
                last_end = pos;
            } else {
                // 未找到匹配的 }，添加剩余部分
                result.push_str(&source[body_start..]);
                last_end = source.len();
                break;
            }
        }
        
        // 添加剩余部分（不转换）
        result.push_str(&source[last_end..]);

        Ok(result)
    }
    
    /// 对函数体进行类型转换（辅助函数）
    fn transform_type_cast_in_body(&self, body: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Pattern 1: (f32) or (f64) wrapping an expression with operators
        // Distribute the cast to operands for floating point types
        // e.g., (f32)(sum / 10) -> sum as f32 / 10 as f32
        let re_float = Regex::new(r"\((f32|f64)\)\s*\(([^)]+)\)")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let mut result = re_float
            .replace_all(body, |caps: &regex::Captures| {
                let type_name = &caps[1];
                let expr = &caps[2];
                // Distribute cast to each operand in the expression
                // char_indices() yields BYTE offsets — using chars().enumerate()
                // here mixed char counts into byte slices and panicked on CJK
                let mut res = String::new();
                let mut depth = 0;
                let mut last_op_pos = 0;

                for (i, c) in expr.char_indices() {
                    match c {
                        '(' => {
                            depth += 1;
                        }
                        ')' => {
                            depth -= 1;
                        }
                        '+' | '-' | '*' | '/' if depth == 0 => {
                            let operand = expr[last_op_pos..i].trim();
                            if !operand.is_empty() {
                                res.push_str(operand);
                                res.push_str(&format!(" as {}", type_name));
                            }
                            res.push(c);
                            last_op_pos = i + 1; // operators are 1-byte ASCII
                        }
                        _ => {}
                    }
                }

                let last_operand = expr[last_op_pos..].trim();
                if !last_operand.is_empty() {
                    res.push_str(last_operand);
                    res.push_str(&format!(" as {}", type_name));
                }

                res
            })
            .to_string();

        // Pattern 2 (enhanced 2026.09.11): (type)operand — operand extracted
        // with paren-depth-aware scanning, so complex operands like
        // (i32)m[i].len() and (usize)(j - 1) work. Emitted as
        // `(operand) as type` (parens guard operator precedence). Replaces
        // the old tight-unit capture that choked on `m[i].len()`.
        let re_cast_head = Regex::new(
            r"\((i8|i16|i32|i64|u8|u16|u32|u64|usize|isize|f32|f64|bool|char)\)\s*",
        )
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let mut replacements: Vec<(usize, usize, String)> = Vec::new();
        for m in re_cast_head.find_iter(&result) {
            let type_name = match re_cast_head
                .captures(&result[m.start()..])
                .and_then(|c| c.get(1))
                .map(|g| g.as_str().to_string())
            {
                Some(t) => t,
                None => continue,
            };
            if let Some((operand, op_end)) = self.extract_cast_operand(&result, m.end()) {
                if !operand.is_empty() {
                    replacements.push((
                        m.start(),
                        op_end,
                        format!("({}) as {}", operand, type_name),
                    ));
                }
            }
        }
        // Replace from the end to keep earlier offsets valid
        for (start, end, rep) in replacements.into_iter().rev() {
            result = format!("{}{}{}", &result[..start], rep, &result[end..]);
        }

        Ok(result)
    }

    /// Distribute type cast to operands in an expression
    /// e.g., "sum / 10" with f32 -> "sum as f32 / 10 as f32"
    fn distribute_cast_to_operands(&self, expr: &str, type_name: &str) -> String {
        // Find operators and split the expression
        let mut result = String::new();
        let mut depth = 0;
        let mut i = 0;

        while i < expr.len() {
            let c = expr.chars().nth(i).unwrap();
            match c {
                '(' => {
                    depth += 1;
                    result.push(c);
                }
                ')' => {
                    depth -= 1;
                    result.push(c);
                }
                '+' | '-' | '*' | '/' if depth == 0 => {
                    // This is a top-level operator
                    // Insert "as type" before the operator
                    result.push_str(&format!(" as {}", type_name));
                    result.push(c);
                }
                _ => {
                    result.push(c);
                }
            }
            i += 1;
        }

        // Add cast to the last operand
        result = result.trim().to_string();
        if !result.ends_with(&format!("as {}", type_name)) {
            result.push_str(&format!(" as {}", type_name));
        }

        result
    }

    /// Extract the operand of a C-style cast `(type)operand` starting right
    /// after the `(type)`. The operand is a unary expression: prefix unary
    /// operators (- ! ~), identifiers/literals, and postfix chains
    /// (`.method()`, `[index]`, parenthesized groups). Terminates at
    /// depth-0 binary operators, `,`, `;`, or an unbalanced closing bracket.
    /// String literals are skipped intact. (2026.09.11, r27 enhancement)
    fn extract_cast_operand(&self, source: &str, start: usize) -> Option<(String, usize)> {
        let bytes = source.as_bytes();
        let mut pos = start;
        let mut depth_paren = 0i32;
        let mut depth_brack = 0i32;
        let mut in_string = false;
        // Marks where a complete operand unit has ended — used to tell a
        // depth-0 binary `-` (terminate) from a prefix unary `-` (continue).
        let mut operand_end = start;

        while pos < bytes.len() {
            let c = bytes[pos] as char;
            if in_string {
                if c == '\\' {
                    pos += 2;
                    continue;
                }
                if c == '"' {
                    in_string = false;
                }
                pos += 1;
                continue;
            }
            match c {
                '"' => {
                    in_string = true;
                    pos += 1;
                }
                '(' => {
                    depth_paren += 1;
                    pos += 1;
                }
                ')' => {
                    if depth_paren == 0 && depth_brack == 0 {
                        break; // unbalanced: belongs to an outer construct
                    }
                    depth_paren -= 1;
                    pos += 1;
                    if depth_paren == 0 && depth_brack == 0 {
                        operand_end = pos; // a full parenthesized group done
                    }
                }
                '[' => {
                    depth_brack += 1;
                    pos += 1;
                }
                ']' => {
                    if depth_brack == 0 && depth_paren == 0 {
                        break;
                    }
                    depth_brack -= 1;
                    pos += 1;
                    if depth_paren == 0 && depth_brack == 0 {
                        operand_end = pos;
                    }
                }
                ',' | ';' => {
                    if depth_paren == 0 && depth_brack == 0 {
                        break;
                    }
                    pos += 1;
                }
                '+' | '*' | '/' | '%' | '<' | '>' | '=' | '&' | '|' | '^' => {
                    if depth_paren == 0 && depth_brack == 0 {
                        break; // depth-0 binary operator: operand ends here
                    }
                    pos += 1;
                }
                '-' | '!' | '~' => {
                    if depth_paren == 0 && depth_brack == 0 && operand_end > start {
                        break; // binary minus/not after an operand: terminate
                    }
                    // prefix unary at operand start: part of the operand
                    pos += 1;
                }
                _ => {
                    pos += 1;
                    operand_end = pos;
                }
            }
        }

        if pos == start {
            return None; // nothing captured
        }
        Some((source[start..pos].trim_end().to_string(), pos))
    }

    /// V0.7: Normalize float literals
    /// f32 x = 0; -> f32 x = 0.0;
    /// Handles: f32/f64 types assigned integer values
    fn transform_float_literals(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Match: f32 or f64 variable assignments where the value is an integer
        // Pattern: : f32 = integer or : f64 = integer
        let re = Regex::new(r":\s*(f32|f64)\s*=\s*(\d+)\s*;")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let type_name = &caps[1];
            let value = &caps[2];
            format!(": {} = {}.0;", type_name, value)
        });

        Ok(result.to_string())
    }

    /// V0.4: Transform array slices
    /// i32[] front = arr[0..3]; -> let front: &[i32] = &arr[0..3];
    fn transform_array_slices(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Pattern: type[] var_name = array[start..end];
        let re = Regex::new(r"\b(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool)\[\]\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*([a-zA-Z_][a-zA-Z0-9_]*)\[(\d+)\.\.(\d+)\];")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let type_name = &caps[1];
            let var_name = &caps[2];
            let array_name = &caps[3];
            let start = &caps[4];
            let end = &caps[5];

            format!(
                "let {}: &[{}] = &{}[{}..{}];",
                var_name, type_name, array_name, start, end
            )
        });

        Ok(result.to_string())
    }

    /// V0.6: Transform class instantiation
    /// ClassName var; -> let mut var: ClassName = ClassName::default();
    /// ClassName var = new ClassName(args); -> let mut var: ClassName = ClassName::new(args);
    /// Outer.Inner var; -> let mut var: Outer_Inner = Outer_Inner::default();  (nested class)
    ///
    /// (owner design, 2026.09.20: 语言核不合成默认值，要不要默认值由程序员
    /// 负责。此处 ::default() 仅为转译器过渡期的后端兼容措施 —— Rust 不允许
    /// 对未初始化绑定逐字段赋值(E0381)，无默认值则逐字段写法无法编译。将来
    /// 语言核不承诺默认值：声明后无任何写入时转译器会提醒（见
    /// remind_init_before_use），提醒先行，报错留给语言核。)
    fn transform_class_instantiation(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Pattern 1: ClassName var = new ClassName(args);
        // Transform to: let mut var: ClassName = ClassName::new(args);
        // Supports nested classes: Outer.Inner var = new Outer.Inner(args);
        let re_new = Regex::new(r"\b([A-Z][a-zA-Z0-9_]*(?:\.[A-Z][a-zA-Z0-9_]*)*)\s+([a-z_][a-zA-Z0-9_]*)\s*=\s*new\s+([A-Z][a-zA-Z0-9_]*(?:\.[A-Z][a-zA-Z0-9_]*)*)\s*\(([^)]*)\)\s*;")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let mut result = re_new
            .replace_all(source, |caps: &regex::Captures| {
                let var_type = &caps[1];
                let var_name = &caps[2];
                let class_name = &caps[3];
                let args = &caps[4];
                
                // Check if types match
                if var_type != class_name {
                    // Type mismatch - keep as-is for now
                    return caps[0].to_string();
                }
                
                // Convert nested class name to Rust format: Outer.Inner -> Outer_Inner
                let rust_type = var_type.replace('.', "_");
                
                format!("let mut {}: {} = {}::new({});", var_name, rust_type, rust_type, args)
            })
            .to_string();

        // Pattern 2: ClassName var;  /  ClassName a, b, c;  (comma list,
        // 2026.09.09 — same shape as scalar no-init declarations)
        // Supports nested classes: Outer.Inner var;
        let re = Regex::new(r"\b([A-Z][a-zA-Z0-9_]*(?:\.[A-Z][a-zA-Z0-9_]*)*)\s+([a-z_][a-zA-Z0-9_]*(?:\s*,\s*[a-z_][a-zA-Z0-9_]*)*)\s*;")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        result = re
            .replace_all(&result, |caps: &regex::Captures| {
                let class_name = &caps[1];
                let var_names: Vec<&str> = caps[2]
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect();

                // Check if it looks like a class name (starts with uppercase)
                // and not a primitive type
                if self.is_primitive_type(class_name) {
                    // Keep as-is for primitive types (comma list per var)
                    let vars = var_names
                        .iter()
                        .map(|v| format!("{} {};", class_name, v))
                        .collect::<Vec<_>>()
                        .join("\n");
                    return vars;
                }

                // Transform to Rust struct instantiation with default values
                // For now, use Default::default() - requires #[derive(Default)]
                // Convert nested class name to Rust format: Outer.Inner -> Outer_Inner
                let rust_type = class_name.replace('.', "_");
                var_names
                    .iter()
                    .map(|v| format!("let mut {}: {} = {}::default();", v, rust_type, rust_type))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .to_string();

        Ok(result)
    }

    /// Transform inherited field access in global code
    /// obj.field -> obj.ParentChain.field (for fields inherited from parent classes)
    fn transform_inherited_field_access(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;
        
        // Build class table from source
        let class_table = self.extract_class_table(source);
        eprintln!("DEBUG: found {} classes in source", class_table.len());
        for (name, info) in &class_table {
            eprintln!("DEBUG: class {} extends {:?}", name, info.parent);
        }
        
        // For each class, build field path map
        let mut all_field_maps: std::collections::HashMap<String, std::collections::HashMap<String, String>> = std::collections::HashMap::new();
        for class_name in class_table.keys() {
            let field_map = self.build_field_path_map(class_name, &class_table, source);
            if !field_map.is_empty() {
                all_field_maps.insert(class_name.clone(), field_map);
            }
        }
        
        // If no inheritance, return early
        if all_field_maps.is_empty() {
            eprintln!("DEBUG: no inheritance field maps found");
            return Ok(source.to_string());
        }
        
        eprintln!("DEBUG: found {} classes with inherited fields", all_field_maps.len());
        for (class_name, field_map) in &all_field_maps {
            eprintln!("DEBUG: class {} has inherited fields: {:?}", class_name, field_map.keys().collect::<Vec<_>>());
        }
        
        let mut result = source.to_string();
        eprintln!("DEBUG: source length: {}, first 200 chars: {:?}", result.len(), &result[..result.len().min(200)]);
        
        // For each class with inherited fields, transform obj.field -> obj.ParentChain.field
        // We need to find variable declarations of these types and transform their field accesses
        for (class_name, field_map) in &all_field_maps {
            // Find variable declarations in Hust source: ClassName var = new ClassName(...)
            // or ClassName var;
            let var_decl_re = Regex::new(&format!(
                r"{}\s+([a-z_][a-zA-Z0-9_]*)\s*=\s*new\s+{}",
                regex::escape(class_name),
                regex::escape(class_name)
            ));
            
            if let Ok(re) = var_decl_re {
                let result_clone = result.clone();
                let captures: Vec<_> = re.captures_iter(&result_clone).collect();
                eprintln!("DEBUG: pattern: {:?}", re.as_str());
                eprintln!("DEBUG: found {} variable declarations for class {}", captures.len(), class_name);
                for caps in captures {
                    let var_name = caps[1].to_string();
                    eprintln!("DEBUG: transforming field access for variable: {} (type: {})", var_name, class_name);
                    
                    // Transform var.field -> var.ParentChain.field for each inherited field
                    for (field, path) in field_map {
                        let pattern = format!(r"\b{}\.{}", regex::escape(&var_name), regex::escape(field));
                        if let Ok(field_re) = Regex::new(&pattern) {
                            let replacement = format!("{}.{}", var_name, path);
                            // Add .field at the end
                            let replacement = format!("{}.{}", replacement, field);
                            result = field_re.replace_all(&result, replacement.as_str()).to_string();
                        }
                    }
                }
            }
        }
        
        Ok(result)
    }
    
    /// Check if a type name is a primitive type
    fn is_primitive_type(&self, type_name: &str) -> bool {
        matches!(
            type_name,
            "i8" | "i16"
                | "i32"
                | "i64"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "f32"
                | "f64"
                | "bool"
                | "char"
                | "String"
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

    /// Reject `const` applied to array declarations (r6/r18, 2026.09.09).
    /// Arrays are mutable by default; r2's `const` covers scalar variables
    /// only. Catches both `const i32[2] a = ...` and the C-style
    /// `const i32 a[2] = ...`, failing fast with guidance instead of
    /// emitting the illegal `const let mut ...`.
    fn reject_const_array(source: &str) -> Result<(), TranspileError> {
        use regex::Regex;
        let re = Regex::new(
            r"\bconst\s+(?:i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool)(?:\s*\[|\s+[a-zA-Z_][a-zA-Z0-9_]*\s*\[)",
        )
        .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        if re.is_match(source) {
            return Err(TranspileError::TransformError(
                "syntax error: arrays are mutable by default and do not support `const` (r6/r18). \
                 Remove `const` from the array declaration, e.g. `i32[2] a = {1, 2};`. \
                 For read-only access to the data, use a slice: `i32[] s = arr[0..2];`"
                    .to_string(),
            ));
        }
        Ok(())
    }

    /// Reject `.to_vec()` on ANY static array (one- or multi-dimensional).
    /// Principle (owner, 2026.09.09): static stays static — no automatic
    /// static->dynamic conversion. Build dynamic arrays with push([]) /
    /// push(v) instead. Dynamic-on-dynamic calls (`c[0].to_vec()` where c is
    /// `i32[][]`) are not affected — the host is not a static array.
    fn reject_static_to_vec(source: &str) -> Result<(), TranspileError> {
        use regex::Regex;

        // 1. names declared as static arrays (1+ [N] groups)
        let decl_re = Regex::new(
            r"\b(?:i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool)((?:\[\d+\])+)\s+([a-zA-Z_][a-zA-Z0-9_]*)\b",
        )
        .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        let static_names: HashSet<String> = decl_re
            .captures_iter(source)
            .map(|c| c[2].to_string())
            .collect();

        if static_names.is_empty() {
            return Ok(());
        }

        // 2. every `.to_vec()` call: resolve its host identifier (any chained
        // form included — row-level `m[0].to_vec()` is also a static->dynamic
        // conversion and is rejected the same way)
        let call_re = Regex::new(r"\.to_vec\(\)")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        for m in call_re.find_iter(source) {
            let bytes = source.as_bytes();
            let mut end = m.start(); // position of the '.'
            let mut start = end;
            while start > 0 {
                let c = bytes[start - 1] as char;
                if c.is_ascii_alphanumeric() || c == '_' {
                    start -= 1;
                } else {
                    break;
                }
            }
            if start == end {
                // Host may be an indexed chain like `m[0].to_vec()` — the
                // char before '.' is ']', not an identifier. Fall through to
                // the chain walk below instead of skipping the call.
            }
            // Walk left over index chains: m[0][1].to_vec() -> root m.
            // Row-level m[0].to_vec() is ALSO a static->dynamic conversion
            // and must be rejected the same way.
            let mut pos = start;
            let mut root_start = start;
            loop {
                while pos > 0 && (bytes[pos - 1] as char).is_whitespace() {
                    pos -= 1;
                }
                if pos > 0 && bytes[pos - 1] == b']' {
                    let mut d = 0i32;
                    while pos > 0 {
                        let c = bytes[pos - 1] as char;
                        if c == ']' {
                            d += 1;
                        } else if c == '[' {
                            d -= 1;
                            if d == 0 {
                                pos -= 1;
                                break;
                            }
                        }
                        pos -= 1;
                    }
                    if pos == 0 {
                        break;
                    }
                    let mut s2 = pos;
                    while s2 > 0 {
                        let c = bytes[s2 - 1] as char;
                        if c.is_ascii_alphanumeric() || c == '_' {
                            s2 -= 1;
                        } else {
                            break;
                        }
                    }
                    if s2 == pos {
                        break; // no identifier before the '['
                    }
                    root_start = s2;
                    pos = s2;
                } else {
                    break;
                }
            }
            // root identifier (up to the first '[' / non-word char)
            let mut root_end = root_start;
            while root_end < source.len() {
                let c = source.as_bytes()[root_end] as char;
                if c.is_ascii_alphanumeric() || c == '_' {
                    root_end += 1;
                } else {
                    break;
                }
            }
            let root = &source[root_start..root_end];
            if static_names.contains(root) {
                return Err(TranspileError::TransformError(
                    "syntax error: .to_vec() on a static array is not supported — \
                     a static array stays static (r18), Hust provides no automatic \
                     static-to-dynamic conversion. Build the dynamic array directly \
                     with push([]) / push(v)."
                        .to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Collect names of `const <int-type> NAME = <digits>;` declarations —
    /// compile-time constants usable as array dimensions (2026.09.09).
    /// Float/bool/String consts and expression initializers are NOT included
    /// (they are runtime immutable bindings per r2).
    fn scan_const_dim_names(source: &str) -> HashSet<String> {
        use regex::Regex;
        let re = Regex::new(
            r"\bconst\s+(?:i8|i16|i32|i64|u8|u16|u32|u64)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*(\d+)\s*;",
        )
        .unwrap();
        re.captures_iter(source)
            .map(|c| c[1].to_string())
            .collect()
    }

    /// Array dimension expression: an integer literal stays as-is; a const
    /// name gets `as usize` — the const keeps its declared type (e.g. i32,
    /// so plain constant usage still matches i32 signatures, q1 regression
    /// fix 2026.09.12), and the dimension site does the usize conversion.
    fn dim_expr(&self, dim: &str) -> String {
        if dim.chars().all(|c| c.is_ascii_digit()) {
            dim.to_string()
        } else {
            format!("{} as usize", dim)
        }
    }

    /// Array dimensions must be an integer literal or a const name from
    /// scan_const_dim_names. A plain variable (`i32 dims = 5; i32[dims][dims] c;`)
    /// is a syntax error with guidance — fail fast (2026.09.09).
    fn check_array_dimensions(
        source: &str,
        const_names: &HashSet<String>,
    ) -> Result<(), TranspileError> {
        use regex::Regex;
        let re = Regex::new(
            r"\b(?:i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool)((?:\[(?:\d+|[a-zA-Z_][a-zA-Z0-9_]*)\])+)\s+[a-zA-Z_][a-zA-Z0-9_]*\s*[;=]",
        )
        .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        let dim_re = Regex::new(r"\[([a-zA-Z_][a-zA-Z0-9_]*)\]")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        for caps in re.captures_iter(source) {
            let dims_str = &caps[1];
            for d in dim_re.captures_iter(dims_str) {
                let dim = &d[1];
                if dim.parse::<usize>().is_err() && !const_names.contains(dim) {
                    return Err(TranspileError::TransformError(format!(
                        "syntax error: array dimension `{}` must be an integer literal or a \
                         const (declare with `const i32 {} = <n>;` — runtime sizes belong to \
                         dynamic arrays `i32[][]`)",
                        dim, dim
                    )));
                }
            }
        }
        Ok(())
    }

    /// C-style array type -> Rust form: i32[5] -> [i32; 5],
    /// i32[2][3] -> [[i32; 3]; 2]. Non-array types pass through.
    fn c_array_type_to_rust(&self, t: &str) -> String {
        use regex::Regex;
        let re = Regex::new(r"^([a-zA-Z_]\w*)((?:\[\d+\])+)$").unwrap();
        if let Some(c) = re.captures(t) {
            let base = &c[1];
            let dims: Vec<String> = Regex::new(r"\[(\d+)\]")
                .unwrap()
                .captures_iter(&c[2])
                .map(|d| d[1].to_string())
                .collect();
            let mut ty = base.to_string();
            for d in dims.iter().rev() {
                ty = format!("[{}; {}]", ty, d);
            }
            ty
        } else {
            t.to_string()
        }
    }

    /// Reject `const` declarations without initializer (2026.09.09).
    /// `const i32 a;` / `const i32 a, b;` have no value — meaningless as a
    /// constant. Delayed initialization belongs to plain variables.
    fn reject_const_no_init(source: &str) -> Result<(), TranspileError> {
        use regex::Regex;
        let re = Regex::new(
            r"\bconst\s+(?:i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|char|String)\s+[a-zA-Z_][a-zA-Z0-9_]*(?:\s*,\s*[a-zA-Z_][a-zA-Z0-9_]*)*\s*;",
        )
        .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        if re.is_match(source) {
            return Err(TranspileError::TransformError(
                "syntax error: `const` declarations require an initializer (a constant \
                 must have a value). For delayed initialization use plain variables: \
                 `i32 a;` or `i32 a, b, c;`"
                    .to_string(),
            ));
        }
        Ok(())
    }

    /// V0.4: Transform array declarations
    /// i32[5] arr = {1,2,3,4,5}; -> let mut arr: [i32; 5] = [1,2,3,4,5];
    /// i32[5] arr;               -> let mut arr: [i32; 5] = [0; 5];
    /// (no-initializer form: rectangular static array, zero-filled, 2026.09.09)
    fn transform_array_declarations(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        let zero_lit = |t: &str| -> &'static str {
            match t {
                "f32" | "f64" => "0.0",
                "bool" => "false",
                _ => "0",
            }
        };

        // Match: type[size] name = {elements};  (size: literal or const name)
        // Include the trailing semicolon in the match so we replace it completely
        // Support multidimensional: type[size1][size2]... name = {elements};
        let re = Regex::new(r"\b(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool)((?:\[[a-zA-Z0-9_]+\])+)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*\{([^}]+)\};")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let type_name = &caps[1];
            let dims_str = &caps[2]; // e.g., "[5][5]" or "[dims][dims]"
            let var_name = &caps[3];
            let elements = &caps[4];
            
            // Parse dimensions
            let dim_re = Regex::new(r"\[([a-zA-Z0-9_]+)\]").unwrap();
            let dims: Vec<String> = dim_re.captures_iter(dims_str)
                .map(|c| self.dim_expr(&c[1]))
                .collect();
            
            eprintln!("[DEBUG] dims_str='{}', dims={:?}, elements='{}'", dims_str, dims, elements);
            
            // Check if elements is just "0" (zero initialization)
            let is_zero_init = elements.trim() == "0";
            
            if is_zero_init {
                // Build nested zero initialization: [[0; N]; M] or [0; N]
                let zero_val = zero_lit(type_name);
                let mut init = format!("[{}; {}]", zero_val, dims[0]);
                for dim in &dims[1..] {
                    init = format!("[{}; {}]", init, dim);
                }
                // Build type annotation: [i32; 5] or [[i32; 5]; 5]
                let mut type_ann = type_name.to_string();
                for dim in &dims {
                    type_ann = format!("[{}; {}]", type_ann, dim);
                }
                format!("let mut {}: {} = {};", var_name, type_ann, init)
            } else {
                // Regular array initialization {1, 2, 3} -> [1, 2, 3]
                // For multidimensional, this needs nested braces, but we don't support that yet
                if dims.len() > 1 {
                    // Multidimensional with non-zero init - not supported yet
                    eprintln!("[Hust 提醒] 多维数组暂只支持 {{0}} 全零初始化");
                    format!("let mut {}: {};", var_name, dims_str)
                } else {
                    format!("let mut {}: [{}; {}] = [{}];", var_name, type_name, dims[0], elements)
                }
            }
        });

        // No-initializer form: type[size] name;  (mutually exclusive with `= {`)
        let re_no_init = Regex::new(r"\b(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool)\[([a-zA-Z0-9_]+)\]\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*;")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re_no_init.replace_all(&result, |caps: &regex::Captures| {
            let type_name = &caps[1];
            let size = &caps[2];
            let var_name = &caps[3];
            let dim = self.dim_expr(size);
            // No zero-fill (owner principle: initialization is the user's
            // responsibility — uninit binding, rustc guards E0381)
            format!(
                "let mut {}: [{}; {}];",
                var_name,
                type_name,
                dim
            )
        });

        Ok(result.to_string())
    }

    /// V0.1: Transform variable declarations
    /// i32 x = 42; -> let mut x: i32 = 42;
    /// const i32 N = 42; -> const N: usize = 42;   (integer literal:
    ///     compile-time constant, usable as array dimension, 2026.09.09)
    /// const i32 x = expr; -> let x: i32 = expr;   (non-literal initializer:
    ///     immutable binding per r2, NOT usable as array dimension)
    /// const f32 x = 1.5; / const bool x = true; -> let x: <type> = ...;
    fn transform_variable_declarations(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Match: [const] type var = value;  (value captured so the const
        // branch can tell a literal from a runtime expression)
        // NOTE: a `;` inside a string literal in the initializer would
        // truncate the capture — not seen in practice, recorded as a limit.
        let re = Regex::new(r"(?:(const)\s+)?\b(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|char|String)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*([^;]+);")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let is_const = caps.get(1).is_some();
            let type_name = &caps[2];
            let var_name = &caps[3];
            let value = caps[4].trim();

            // Get the full match and check if this is inside a for loop header
            let full_match = caps.get(0).unwrap();
            let start = full_match.start();

            // Look at the 15 characters before this match
            // Byte-offset lookback must land on a char boundary (CJK comments
            // are 3 bytes/char; a blind `start - 15` can split a codepoint)
            let mut before_start = if start >= 15 { start - 15 } else { 0 };
            while before_start > 0 && !source.is_char_boundary(before_start) {
                before_start -= 1;
            }
            let before = &source[before_start..start];

            // If preceded by "for (" (possibly with whitespace), this is a for loop variable
            // Don't transform it - the for loop transformer will handle it
            let is_for_loop_var = before.contains("for (") || before.contains("for(");

            if is_const
                && !value.is_empty()
                && value.chars().all(|c| c.is_ascii_digit())
            {
                // r5 evolution (2026.09.09, revised 2026.09.12): const +
                // integer literal -> compile-time constant, KEEPING the
                // declared type (const a: i32 = 10) — plain constant usage
                // must still match i32 signatures (q1 regression). Array
                // dimensions get their `as usize` at the dimension site
                // instead (see multi/single array decl).
                format!("const {}: {} = {};", var_name, type_name, value)
            } else if is_const {
                // r2: immutable binding (expression or non-integer literal) —
                // runtime value, not usable as array dimension.
                // r3: String literal init needs .to_string() (owned String)
                if type_name == "String" && value.starts_with('"') {
                    format!("let {}: String = {}.to_string();", var_name, value)
                } else {
                    format!("let {}: {} = {};", var_name, type_name, value)
                }
            } else if is_for_loop_var {
                // Keep original format - for loop transformer will handle this
                format!("{} {} = {};", type_name, var_name, value)
            } else if type_name == "String" && value.starts_with('"') {
                // r3: String declaration with literal -> owned String
                format!("let mut {}: String = {}.to_string();", var_name, value)
            } else {
                // regular variable: mutable in Rust ("let mut")
                format!("let mut {}: {} = {};", var_name, type_name, value)
            }
        });

        // No-initializer declarations: type name;  /  type a, b, c;
        // (C-style comma list, 2026.09.09) -> one `let mut name: type;` per
        // variable. Delayed initialization — rustc flow analysis guards
        // use-before-assign (E0381). const form rejected at Rule 4.8.
        // Mutually exclusive with the `= value` form above (name followed by
        // `;`, not `=`).
        let re_no_init = Regex::new(r"\b(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|char|String)\s+([a-zA-Z_][a-zA-Z0-9_]*(?:\s*,\s*[a-zA-Z_][a-zA-Z0-9_]*)*)\s*;")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re_no_init.replace_all(&result, |caps: &regex::Captures| {
            let type_name = &caps[1];
            let names: Vec<&str> = caps[2]
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            names
                .iter()
                .map(|n| format!("let mut {}: {};", n, type_name))
                .collect::<Vec<_>>()
                .join("\n")
        });

        Ok(result.to_string())
    }

    /// V0.5: Transform function definitions with return types and visibility
    /// void main() -> fn main()
    /// pub i32 add(i32 a, i32 b) -> pub fn add(a: i32, b: i32) -> i32
    fn transform_function_definitions(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Pattern: [public] type name(params) { body }
        // Capture optional public, return type, name, and parameters
        // Note: Must NOT match variable declarations like "i32 i = 0;" or "i32 i();"
        // So we require that the name is followed by (params) directly without = in between
        // 2026.09.25: Support both "pub" and "public" keywords (owner decision: use public)
        let re = Regex::new(r"(?m)^\s*((?:pub|public)\s+)?\b(void|i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|char|String)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(([^)]*)\)\s*\{")
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
                format!(
                    "{}fn {}({}) -> {} {{",
                    vis, func_name, transformed_params, ret_type
                )
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

    /// V0.2: Transform i++ and i-- to i += 1 and i -= 1
    /// i++ -> i += 1
    /// i-- -> i -= 1
    fn transform_increment_decrement(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Match var++ or var--
        let re = Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\s*(\+\+|--)\s*;")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let var_name = &caps[1];
            let op = &caps[2];
            if op == "++" {
                format!("{} += 1;", var_name)
            } else {
                format!("{} -= 1;", var_name)
            }
        });

        Ok(result.to_string())
    }

    /// V0.2: Transform C-style for loop to Rust while loop
    /// for (i32 i = 0; i < n; i = i + 1) { body }
    /// -> let mut i: i32 = 0; while i < n { body; i = i + 1; }
    /// for (i32 i = 0; i < n; i++) { body } (with ++ shorthand)
    /// -> let mut i: i32 = 0; while i < n { body; i += 1; }

    /// L1 condition conversion: only handle the exact form "var op xxx.len()"
    /// (both directions), matched with full-string anchors (^ $).
    ///
    /// Design rationale:
    /// - Anchoring the WHOLE condition naturally excludes compound conditions
    ///   like "i + 1 < arr.len()" — those are left untouched so rustc reports
    ///   the type error and the user opts in with an explicit (usize) cast.
    ///   Fail-safe: we never generate wrong code, we just don't help.
    /// - Simple form covers the vast majority of real loops.
    /// Warning for users: prefer "i + 1 < len" over "i < len - 1"; with usize,
    /// "len - 1" underflows when the array is empty.
    /// L1/L1.5 condition conversion (2026.09.11): right side may be
    /// `.len()` (plain or with an index chain) — both usize-typed and force
    /// the i32 loop var to `as usize`. Full-string anchored; compound
    /// conditions pass through. const-name comparison (`i < N`) needs NO
    /// conversion — consts keep their declared i32 type (q1 regression fix).
    fn transform_len_comparison(condition: &str) -> String {
        use regex::Regex;

        let trimmed = condition.trim();

        // Pattern 1 (L1.5, 2026.09.11): var op <base>[index-chain]*.len()
        // e.g. "i < m.len()" AND "j < m[i].len()" — the N-dim row-length
        // traversal form. The bracket chain is preserved verbatim; the
        // index inside gets its as-usize peel later (Rule 18, peeling runs
        // after this pass). Zero brackets degrades to the plain form.
        let re_fwd = Regex::new(
            r"^([a-zA-Z_]\w*)\s*(<=|>=|<|>)\s*([a-zA-Z_]\w*(?:\[[^\[\]]+\])*)\.len\(\)$",
        )
        .unwrap();
        if let Some(c) = re_fwd.captures(trimmed) {
            return format!("({} as usize) {} {}.len()", &c[1], &c[2], &c[3]);
        }

        // Pattern 3: xxx.len() op var   e.g. "dynamic.len() > i"
        let re_rev = Regex::new(
            r"^([a-zA-Z_]\w*)\.len\(\)\s*(<=|>=|<|>)\s*([a-zA-Z_]\w*)$",
        )
        .unwrap();
        if let Some(c) = re_rev.captures(trimmed) {
            return format!("{}.len() {} ({} as usize)", &c[1], &c[2], &c[3]);
        }

        trimmed.to_string()
    }

    /// Extract the for body starting after the header's `)`. Returns
    /// (body, end_offset) — body includes its trailing `;` (single-statement
    /// form) or the statements inside the braces (block form); end_offset is
    /// exclusive of the terminator. Shared by typed/untyped for passes.
    fn extract_for_body(
        &self,
        source: &str,
        after_header_start: usize,
    ) -> Option<(String, usize)> {
        let after_header = &source[after_header_start..];
        let trimmed_after = after_header.trim_start();
        // Body type: braced iff first non-whitespace char is `{`. Never use
        // find('{') — println format strings like "i = {}" would misroute.
        if trimmed_after.starts_with('{') {
            let open_brace_idx = after_header.len() - trimmed_after.len();
            let body_start_abs = after_header_start + open_brace_idx + 1;
            let mut brace_depth = 1;
            let mut in_string = false;
            let mut body_end_abs = body_start_abs;
            let body_src = &source[body_start_abs..];
            for (i, c) in body_src.char_indices() {
                if c == '"' {
                    in_string = !in_string;
                }
                if !in_string {
                    match c {
                        '{' => brace_depth += 1,
                        '}' => {
                            brace_depth -= 1;
                            if brace_depth == 0 {
                                body_end_abs = body_start_abs + i;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
            if brace_depth != 0 {
                return None; // malformed, no matching closing brace
            }
            Some((
                source[body_start_abs..body_end_abs].trim().to_string(),
                body_end_abs + 1,
            ))
        } else {
            // Single statement: scan to first `;` outside strings; the `;` is
            // consumed as the boundary and re-appended to the body.
            let mut stmt_end = None;
            let mut in_string = false;
            for (i, c) in after_header.char_indices() {
                if c == '"' {
                    in_string = !in_string;
                } else if !in_string && c == ';' {
                    stmt_end = Some(i);
                    break;
                }
            }
            let stmt_end = stmt_end?;
            Some((
                format!("{};", after_header[..stmt_end].trim()),
                after_header_start + stmt_end + 1,
            ))
        }
    }

    /// Resolve the variable updated by a marked ++/-- clause.
    fn update_var_name_of(&self, update: &str, default_var: &str) -> String {
        use regex::Regex;
        if update.contains("###HUST_PREFIX###") {
            return Regex::new(r"###HUST_PREFIX###\+\+([a-zA-Z_][a-zA-Z0-9_]*)###HUST_END###")
                .unwrap()
                .captures(update)
                .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
                .unwrap_or_else(|| default_var.to_string());
        }
        if update.contains("###HUST_POSTFIX###") {
            return Regex::new(r"###HUST_POSTFIX###([a-zA-Z_][a-zA-Z0-9_]*)\+\+###HUST_END###")
                .unwrap()
                .captures(update)
                .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
                .unwrap_or_else(|| default_var.to_string());
        }
        default_var.to_string()
    }

    /// Build the trailing update statement(s) for a while loop from the for
    /// update clause. Handles (in priority order): code-block updates (with
    /// inner ++/-- conversion), single marked ++/--, plain expressions.
    /// BRANCH ORDER MATTERS — a block almost always contains ++/-- whose
    /// markers would divert it into the single-operator branch (V0.1.8 bug).
    fn build_update_stmt(&self, update: &str, default_var: &str) -> String {
        use regex::Regex;
        let update_trimmed = update.trim();

        if update_trimmed.starts_with('{') && update_trimmed.ends_with('}') {
            let block_content = &update_trimmed[1..update_trimmed.len() - 1].trim();
            let mut processed_block = block_content
                .replace("###HUST_PREFIX###", "")
                .replace("###HUST_POSTFIX###", "")
                .replace("###HUST_END###", "");
            let inc_re = Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\s*\+\+").unwrap();
            processed_block = inc_re
                .replace_all(&processed_block, "$1 += 1")
                .to_string();
            let dec_re = Regex::new(r"\b([a-zA-Z_][a-zA-Z0-9_]*)\s*--").unwrap();
            processed_block = dec_re
                .replace_all(&processed_block, "$1 -= 1")
                .to_string();
            return format!("{}\n", processed_block);
        }

        let is_prefix = update.contains("###HUST_PREFIX###");
        let is_postfix = update.contains("###HUST_POSTFIX###");
        if is_prefix || is_postfix {
            let var_name = self.update_var_name_of(update, default_var);
            let is_increment = update.contains("++");
            return if is_increment {
                format!("{} = {} + 1;", var_name, var_name)
            } else {
                format!("{} = {} - 1;", var_name, var_name)
            };
        }

        // Plain expression — always lands at the end of the while block, so
        // the missing `;` is legal as a trailing expression; keep as-is.
        update_trimmed.to_string()
    }

    fn transform_for_loop(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        let mut result = source.to_string();

        // Pattern to match for loop header (body extracted via brace-matching)
        let header_re = Regex::new(
            r"for\s*\(\s*(i8|i16|i32|i64|u8|u16|u32|u64)\s+([a-zA-Z_]\w*)\s*=\s*([^;]+)\s*;\s*([^;]+)\s*;\s*([^\)]+)\)"
        ).map_err(|e| TranspileError::TransformError(e.to_string()))?;

        // Step 1: Add markers to preserve prefix/postfix information
        // Postfix: i++ -> ###HUST_POSTFIX###i++###HUST_END###
        let postfix_increment_re = Regex::new(r"(\b[a-zA-Z_][a-zA-Z0-9_]*)\+\+")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        result = postfix_increment_re
            .replace_all(&result, "###HUST_POSTFIX###$1++###HUST_END###")
            .to_string();

        // Postfix: i-- -> ###HUST_POSTFIX###i--###HUST_END###
        let postfix_decrement_re = Regex::new(r"(\b[a-zA-Z_][a-zA-Z0-9_]*)\-\-")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        result = postfix_decrement_re
            .replace_all(&result, "###HUST_POSTFIX###$1--###HUST_END###")
            .to_string();

        // Prefix: ++i -> ###HUST_PREFIX###++i###HUST_END###
        let prefix_increment_re = Regex::new(r"\+\+\s*([a-zA-Z_][a-zA-Z0-9_]*)")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        result = prefix_increment_re
            .replace_all(&result, "###HUST_PREFIX###++$1###HUST_END###")
            .to_string();

        // Prefix: --i -> ###HUST_PREFIX###--i###HUST_END###
        let prefix_decrement_re = Regex::new(r"\-\-\s*([a-zA-Z_][a-zA-Z0-9_]*)")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        result = prefix_decrement_re
            .replace_all(&result, "###HUST_PREFIX###--$1###HUST_END###")
            .to_string();

        // Compile-time constant names (const + integer literal) usable as
        // array dimensions and comparison operands (r33 L1/Patterns)
        let const_names = Self::scan_const_dim_names(source);

        // Step 2: Transform for loops using markers
        // Use two-pass approach: regex for header, brace-matching for body
        loop {
            // Only match the for loop header (no body capture)
            let header_match = match header_re.find(&result) {
                Some(m) => m,
                None => break,
            };

            let header_text = header_match.as_str();
            let caps = header_re.captures(&result).ok_or_else(|| {
                TranspileError::TransformError("Failed to capture for loop header".to_string())
            })?;

            let var_type = &caps[1];
            let var_name = &caps[2];
            let init_value = caps[3].trim();
            let condition = caps[4].trim();
            let update = caps[5].trim();

            let (body, full_for_end) = match self.extract_for_body(&result, header_match.end()) {
                Some(x) => x,
                None => break, // malformed body
            };
            let full_for_start = header_match.start();
            ;

            // Update statement: block / marked ++/-- / plain (see helper —
            // branch order matters, block must be checked first)
            let const_names = Self::scan_const_dim_names(&result);
            let update_stmt = self.build_update_stmt(update, var_name);

            // L1 condition conversion: "i < arr.len()" -> "(i as usize) < arr.len()"
            // Anchored match — compound conditions are left for rustc to report
            let transformed_condition = Self::transform_len_comparison(condition);

            // Typed header: the loop variable is declared here (`let mut`)
            let rust_while = format!(
                "let mut {}: {} = {}; while {} {{{}\n{}}}",
                var_name, var_type, init_value, transformed_condition, body, update_stmt
            );

            result = format!(
                "{}{}{}",
                &result[..full_for_start],
                rust_while,
                &result[full_for_end..]
            );
        }

        // --- Step 2b: untyped for headers (2026.09.09) ---
        // C-style loop variable declared OUTSIDE the loop (paired with the
        // no-initializer declarations): for(i = 1; i < n; i++) { ... }
        // Generates an assignment (`i = 1;`) instead of a `let` — the loop
        // variable must be declared earlier (rustc E0425 otherwise).
        let untyped_re = Regex::new(
            r"for\s*\(\s*([a-zA-Z_]\w*)\s*=\s*([^;]+)\s*;\s*([^;]+)\s*;\s*([^\)]+)\)",
        )
        .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        loop {
            let m = match untyped_re.find(&result) {
                Some(m) => m,
                None => break,
            };
            let caps = untyped_re.captures(&result).ok_or_else(|| {
                TranspileError::TransformError(
                    "Failed to capture untyped for loop header".to_string(),
                )
            })?;

            let var_name = &caps[1];
            let init_value = caps[2].trim();
            let condition = caps[3].trim();
            let update = caps[4].trim();

            let (body, full_for_end) = match self.extract_for_body(&result, m.end()) {
                Some(x) => x,
                None => break, // malformed body
            };
            let full_for_start = m.start();

            let update_stmt = self.build_update_stmt(update, var_name);
            let transformed_condition = Self::transform_len_comparison(condition);

            // Untyped header: NO `let` — assignment into the pre-declared
            // loop variable (first assignment completes its delayed init)
            let rust_while = format!(
                "{} = {}; while {} {{{}\n{}}}",
                var_name, init_value, transformed_condition, body, update_stmt
            );

            result = format!(
                "{}{}{}",
                &result[..full_for_start],
                rust_while,
                &result[full_for_end..]
            );
        }

        // Step 3: Remove all markers (safety cleanup)
        // This ensures no markers remain in final output
        result = result
            .replace("###HUST_PREFIX###", "")
            .replace("###HUST_POSTFIX###", "")
            .replace("###HUST_END###", "");

        // Step 4: Safety check - verify no markers remain
        if result.contains("###HUST_") {
            return Err(TranspileError::TransformError(
                "Internal error: Hust markers not properly removed".to_string(),
            ));
        }

        Ok(result)
    }

    /// Add markers around for loop bodies to protect content
    fn protect_for_bodies(source: &str) -> String {
        use regex::Regex;

        let mut result = source.to_string();
        loop {
            let for_pattern = Regex::new(r"for\s*\([^)]+\)\s*\{").unwrap();

            if let Some(m) = for_pattern.find(&result) {
                let marker = "###FOR_MARKER###";
                result = format!("{}{}{}", &result[..m.end()], marker, &result[m.end()..]);
            } else {
                break;
            }
        }
        result
    }

    /// Extract body between markers
    fn extract_protected_body(source: &str) -> (String, String) {
        let marker = "###FOR_MARKER###";
        if let Some(pos) = source.find(marker) {
            let after_marker = &source[pos + marker.len()..];
            // Find matching closing brace
            let mut depth = 1;
            let mut in_string = false;
            let mut escape = false;

            for (i, c) in after_marker.chars().enumerate() {
                if escape {
                    escape = false;
                    continue;
                }
                if c == '\\' {
                    escape = true;
                    continue;
                }
                if c == '"' {
                    in_string = !in_string;
                    continue;
                }

                if !in_string {
                    match c {
                        '{' => depth += 1,
                        '}' => depth -= 1,
                        _ => {}
                    }
                }

                if depth == 0 {
                    let body = &after_marker[..i];
                    let after = &after_marker[i + 1..];
                    return (body.to_string(), after.to_string());
                }
            }
        }
        (source.to_string(), String::new())
    }

    /// Parse condition like "i < 4" and return (op, limit)
    fn parse_condition<'a>(&self, condition: &'a str) -> (&'a str, &'a str) {
        if condition.contains("<=") {
            ("<=", condition.split("<=").nth(1).unwrap_or("0").trim())
        } else if condition.contains('<') {
            ("<", condition.split('<').nth(1).unwrap_or("0").trim())
        } else if condition.contains(">=") {
            (">=", condition.split(">=").nth(1).unwrap_or("0").trim())
        } else if condition.contains('>') {
            (">", condition.split('>').nth(1).unwrap_or("0").trim())
        } else {
            ("<", "0")
        }
    }

    /// Find the end position of a for loop (including the closing brace)
    fn find_for_loop_end(&self, source: &str, start: usize) -> Option<usize> {
        let mut paren_depth = 0;
        let mut brace_depth = 0;
        let mut in_for = false;

        // char_indices() yields BYTE offsets, required by the slice math below
        // (chars().enumerate() counts chars and breaks on multi-byte input)
        for (i, c) in source[start..].char_indices() {
            match c {
                '(' => {
                    if in_for {
                        paren_depth += 1;
                    }
                }
                ')' => {
                    if in_for {
                        if paren_depth == 0 {
                            // Next character should be {
                            let rest = &source[start + i + 1..];
                            if let Some(j) = rest.find('{') {
                                // Find the matching }
                                let after_brace = &rest[j + 1..];
                                let mut bd = 1;
                                for (k, rc) in after_brace.char_indices() {
                                    match rc {
                                        '{' => bd += 1,
                                        '}' => {
                                            bd -= 1;
                                            if bd == 0 {
                                                return Some(start + i + 1 + j + 1 + k + 1);
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            return None;
                        }
                        paren_depth -= 1;
                    }
                }
                'f' if source[start + i..].starts_with("for (") => {
                    in_for = true;
                }
                _ => {}
            }
        }
        None
    }

    /// Transform a single for loop to while loop
    /// After transform_for_loop_vars, the input is:
    /// for (COND; UPDATE) {
    ///     body
    /// }
    fn transform_single_for_loop(&self, for_loop: &str) -> Option<String> {
        use regex::Regex;

        // Pattern: for (COND; VAR = VAR OP NUM)\s*{ or for (COND; VAR = VAR OP NUM) {
        // Allow optional whitespace and newline between ) and {
        let re = Regex::new(
            r"for\s*\(\s*([^;]+)\s*;\s*([a-zA-Z_]\w*)\s*=\s*([a-zA-Z_]\w*)\s*([\+\-])\s*(\d+)\s*\)\s*\{?"
        ).ok()?;

        let caps = re.captures(for_loop)?;

        let condition = &caps[1].trim();
        let update_var = &caps[2];
        let update_op = &caps[4];

        // Find the body - look for the opening { after the for loop header
        // and get everything until the matching }
        let after_header = &for_loop[caps.get(0).unwrap().end()..];
        let body_start = after_header.find('{')?;
        let body_rest = &after_header[body_start..];

        // Find the matching closing brace
        let mut depth = 0;
        let mut body_end = 0;
        for (i, c) in body_rest.chars().enumerate() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        body_end = i;
                        break;
                    }
                }
                _ => {}
            }
        }

        let body = &body_rest[1..body_end]; // Skip the opening {

        // Build update statement based on operator
        let update_stmt = format!("{}= 1;", update_var);

        // Build the while loop
        Some(format!("while {} {{{}{}\n}}", condition, body, update_stmt))
    }

    /// V0.2: Remove parentheses from if/while conditions (Rust style)
    /// if (x > 5) -> if x > 5
    fn remove_condition_parens(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Match: if (condition) { or while (condition) {
        // But NOT for loop headers
        // Replace with: if condition { or while condition {
        let re = Regex::new(r"\b(if|while)\s*\(\s*([^)]+)\s*\)\s*\{")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let keyword = &caps[1];
            let condition = &caps[2];
            format!("{} {} {{", keyword, condition)
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

    /// Transform string literals in function calls and returns to String
    /// greet("World") -> greet("World".to_string())
    /// return "Hi"; -> return "Hi".to_string();
    fn transform_string_literals_in_functions(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;
        let mut result = source.to_string();
        
        // Pattern 1: Function calls with string literal arguments
        // Match: fn_name("literal") or fn_name(arg1, "literal", ...)
        // We need to find function calls that expect String parameters
        
        // First, find all function definitions with String parameters
        let fn_def_re = Regex::new(r"fn\s+([a-zA-Z_]\w*)\s*\(([^)]*)\)\s*(?:->\s*String\s*)?\{")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        
        let mut string_fn_names: Vec<String> = Vec::new();
        let mut string_return_fns: Vec<String> = Vec::new();
        
        for caps in fn_def_re.captures_iter(&result) {
            let fn_name = caps[1].to_string();
            let params = caps[2].to_string();
            
            // Check if any parameter is String type
            if params.contains("String") {
                string_fn_names.push(fn_name.clone());
            }
            
            // Check if return type is String (captured in the regex)
            let full_match = caps.get(0).unwrap().as_str();
            if full_match.contains("-> String") {
                string_return_fns.push(fn_name);
            }
        }
        
        // Pattern 2: Transform string literal arguments in function calls
        // For each function with String parameters, find calls and transform literals
        for fn_name in &string_fn_names {
            // Match: fn_name(arg1, "literal", ...) or fn_name("literal")
            // This is a simplified approach - we transform all string literals in the call
            let call_re = Regex::new(&format!(
                r"{}\s*\(([^)]+)\)",
                regex::escape(fn_name)
            )).map_err(|e| TranspileError::TransformError(e.to_string()))?;
            
            result = call_re.replace_all(&result, |caps: &regex::Captures| {
                let args = &caps[1];
                // Transform string literals in arguments
                let transformed_args = self.transform_string_literals_in_args(args);
                format!("{}({})", fn_name, transformed_args)
            }).to_string();
        }
        
        // Pattern 3: Transform return statements with string literals
        // return "literal"; -> return "literal".to_string();
        let return_re = Regex::new(r#"return\s+"([^"]+)"\s*;"#)
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        
        result = return_re.replace_all(&result, |caps: &regex::Captures| {
            let literal = &caps[1];
            format!("return \"{}\".to_string();", literal)
        }).to_string();
        
        Ok(result)
    }
    
    /// Helper: Transform string literals in function arguments
    fn transform_string_literals_in_args(&self, args: &str) -> String {
        use regex::Regex;
        
        // Match string literals: "..." (not followed by .to_string())
        // Simple approach: match all string literals, then check if already has .to_string()
        let lit_re = Regex::new(r#""([^"]+)""#).unwrap();
        
        let mut result = String::new();
        let mut last_end = 0;
        
        for caps in lit_re.captures_iter(args) {
            let match_start = caps.get(0).unwrap().start();
            let match_end = caps.get(0).unwrap().end();
            let literal = &caps[1];
            
            // Add text before this match
            result.push_str(&args[last_end..match_start]);
            
            // Check if followed by .to_string()
            let after = &args[match_end..];
            if after.starts_with(".to_string()") {
                // Already has .to_string(), keep as-is
                result.push_str(&format!("\"{}\"", literal));
            } else {
                // Add .to_string()
                result.push_str(&format!("\"{}\".to_string()", literal));
            }
            
            last_end = match_end;
        }
        
        // Add remaining text
        result.push_str(&args[last_end..]);
        
        result
    }

    /// V0.3: Transform pass to ()
    /// pass; -> ();
    fn transform_pass(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        let re =
            Regex::new(r"\bpass\s*;").map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, "();");

        Ok(result.to_string())
    }

    /// V0.4: Transform dynamic array declaration
    /// i32[] arr; -> let mut arr: Vec<i32> = Vec::new();
    fn transform_dynamic_array_decl(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // Match: type[] name; — generalized to N stacked [] groups (2026.09.09),
        // e.g. i32[] v -> Vec<i32>, i32[][] c -> Vec<Vec<i32>>
        let re = Regex::new(
            r"\b(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool)((?:\[\])+)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*;",
        )
        .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let bracket_re = Regex::new(r"\[\]")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let type_name = &caps[1];
            let dims_str = &caps[2];
            let var_name = &caps[3];

            // Build nested Vec type: innermost is T, one Vec<...> per []
            let mut ty = type_name.to_string();
            let depth = bracket_re.find_iter(dims_str).count();
            for _ in 0..depth {
                ty = format!("Vec<{}>", ty);
            }
            format!("let mut {}: {} = Vec::new();", var_name, ty)
        });

        Ok(result.to_string())
    }

    /// `x.push([])` -> `x.push(Vec::new())`: pushing an empty array literal
    /// creates a new row for nested dynamic arrays (Vec<Vec<T>>).
    fn translate_empty_push(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;
        let re = Regex::new(r"\.push\(\[\]\)")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        Ok(re.replace_all(source, ".push(Vec::new())").to_string())
    }

    /// Array literal assignment: `x.f = {a, b, c};` -> `x.f = [a, b, c];`
    /// (Rust parses `{a, b, c}` as a block expression — array literals on
    /// assignment need `[...]`. 2026.09.18, found by owner's q5.)
    /// One nesting level; nested `{{..},{..}}` literals not yet handled.
    fn transform_array_literal_assign(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;
        // target = identifier, optionally with an index/method suffix
        let re = Regex::new(
            r"((?:\b[a-zA-Z_]\w*)+(?:\[[^\[\]]+\]|\.\w+\([^()]*\))*)\s*=\s*\{([^{}]+)\}\s*;",
        )
        .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let target = &caps[1];
            let elements = caps[2].trim();
            format!("{} = [{}];", target, elements)
        });

        Ok(result.to_string())
    }

    /// String literal assignment: `x.f = "Alice";` -> `x.f = "Alice".to_string();`
    /// (a bare "..." is &str; String targets need .to_string() — r3 covered
    /// declarations, this covers assignments. 2026.09.18, owner's q5.)
    /// The string literal is captured as a WHOLE group (group 2, with
    /// escape-sequence support) — no boundary assumptions from substring
    /// slicing, so trailing `;` or any following text never leaks in.
    /// `!=` comparisons are skipped (char before target is `!`).
    fn translate_string_assign(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;
        let re = Regex::new(r#"([a-zA-Z_][\w.\[\]]*)\s*=\s*("(?:[^"\\]|\\.)*")\s*;"#)
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let mut replacements: Vec<(usize, usize, String)> = Vec::new();
        for caps in re.captures_iter(source) {
            let m = caps.get(0).unwrap();
            // skip `!=` comparisons (char before the match, skipping spaces,
            // is `!` — matches `x != "y"`; `==` cannot match this pattern's
            // single `=` so no false positives there)
            let bytes = source.as_bytes();
            let mut p = m.start();
            while p > 0 && (bytes[p - 1] as char).is_whitespace() {
                p -= 1;
            }
            if p > 0 && bytes[p - 1] == b'!' {
                continue;
            }
            let target = caps[1].trim();
            let strlit = &caps[2]; // exact literal, quotes included
            replacements.push((
                m.start(),
                m.end(),
                format!("{} = {}.to_string();", target, strlit),
            ));
        }

        let mut result = source.to_string();
        for (start, end, rep) in replacements.into_iter().rev() {
            result = format!("{}{}{}", &result[..start], rep, &result[end..]);
        }

        Ok(result)
    }

    /// V0.4: Transform multi-dimensional array declaration
    /// 2026.09.08 dimension generalization (拆离法 sibling):
    /// i32[3][4] matrix = {{1,2,3,4},{5,6,7,8},{9,10,11,12}};
    /// -> let mut matrix: [[i32; 4]; 3] = [[1,2,3,4],[5,6,7,8],[9,10,11,12]];
    /// i32[2][2][2] cube = {{{1,2},{3,4}},{{5,6},{7,8}}};
    /// -> let mut cube: [[[i32; 2]; 2]; 2] = [[[1,2],[3,4]],[[5,6],[7,8]]];
    /// The regex captures the whole dimension string "[s1][s2]...[sn]" WITHOUT
    /// counting levels; the Rust type is built by looping dims right-to-left
    /// (rightmost = innermost), matching the 2-D behavior this replaces.
    fn transform_multi_array_decl(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;

        // 2+ adjacent dimension groups (size: literal or const name); 1-D is
        // handled by transform_array_declarations
        let re = Regex::new(
            r"(?m)\b(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool)((?:\[(?:\d+|[a-zA-Z_][a-zA-Z0-9_]*)\]){2,})\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*=\s*\{([\s\S]*?)\}\s*;",
        )
        .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let dim_re = Regex::new(r"\[([a-zA-Z0-9_]+)\]")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re.replace_all(source, |caps: &regex::Captures| {
            let type_name = &caps[1];
            let dims_str = &caps[2];
            let var_name = &caps[3];
            let elements = &caps[4];

            // Build Rust type: loop dims right-to-left, rightmost dim innermost
            let mut ty = type_name.to_string();
            let dims: Vec<_> = dim_re.captures_iter(dims_str).collect();
            for d in dims.iter().rev() {
                ty = format!("[{}; {}]", ty, self.dim_expr(&d[1]));
            }

            // Check if elements is just "0" (zero initialization)
            let is_zero_init = elements.trim() == "0";
            
            if is_zero_init {
                // Build nested zero initialization: [[0; N]; M] or [0; N]
                let zero_val = match type_name {
                    "f32" | "f64" => "0.0",
                    "bool" => "false",
                    _ => "0",
                };
                let mut init = format!("[{}; {}]", zero_val, self.dim_expr(&dim_re.captures_iter(dims_str).next().unwrap()[1]));
                for d in dim_re.captures_iter(dims_str).skip(1) {
                    init = format!("[{}; {}]", init, self.dim_expr(&d[1]));
                }
                format!("let mut {}: {} = {};", var_name, ty, init)
            } else {
                // Element braces -> brackets (per-char replace, depth-agnostic)
                let rust_elements = elements
                    .replace("{", "[")
                    .replace("}", "]")
                    .replace(";", "");

                format!(
                    "let mut {}: {} = [{}];",
                    var_name,
                    ty,
                    rust_elements.trim()
                )
            }
        });

        // No-initializer form: type[d1][d2]...[dn] name;  (size: literal or
        // const name)
        // Rectangular static array, zero-filled to the same nesting depth
        // (mutually exclusive with the `= { ... }` initializer form above)
        let re_no_init = Regex::new(
            r"(?m)\b(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool)((?:\[(?:\d+|[a-zA-Z_][a-zA-Z0-9_]*)\]){2,})\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*;",
        )
        .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let result = re_no_init.replace_all(&result, |caps: &regex::Captures| {
            let type_name = &caps[1];
            let dims_str = &caps[2];
            let var_name = &caps[3];

            let zero_lit = match type_name {
                "f32" | "f64" => "0.0",
                "bool" => "false",
                _ => "0",
            };

            // Build type and zero value in the same right-to-left nesting
            let mut ty = type_name.to_string();
            let mut val = zero_lit.to_string();
            let dims: Vec<_> = dim_re.captures_iter(dims_str).collect();
            for d in dims.iter().rev() {
                let n = &d[1];
                ty = format!("[{}; {}]", ty, self.dim_expr(n));
            }

            // No zero-fill (owner principle 2026.09.12: initialization is the
            // user's responsibility — uninit binding, rustc flow analysis
            // guards use-before-assign E0381)
            format!("let mut {}: {};", var_name, ty)
        });

        Ok(result.to_string())
    }

    /// Transpile and write to file
    pub fn transpile_to_file(
        &self,
        input_path: &PathBuf,
        output_path: &PathBuf,
    ) -> Result<(), TranspileError> {
        let result = self.transpile_file(input_path)?;
        std::fs::write(output_path, result)?;
        Ok(())
    }

    /// V0.5: Transpile multiple modules into a single Rust file
    /// Merges all modules, handling imports, visibility, and namespaces
    /// 2026.09.25: Namespace support - auto-resolve function calls with prefixes
    pub fn transpile_modules(
        &self,
        modules: &[Module],
        entry_module: &Module,
    ) -> Result<String, TranspileError> {
        use crate::namespace::{NamespaceRegistry, FunctionInfo, ClassInfo};
        use regex::Regex;

        let mut registry = NamespaceRegistry::new();
        let mut module_sources: Vec<(String, String, String)> = Vec::new(); // (name, source, file_path)

        // Phase 1: Parse all namespace declarations and build registry
        for module in modules {
            let file_path = module.path.to_string_lossy().to_string();
            
            // Parse namespace declaration
            let (ns, after_ns) = registry
                .parse_declaration(&module.source, &file_path)
                .map_err(|e| TranspileError::TransformError(e.to_string()))?;

            let ns_name = ns.unwrap_or_else(|| registry.default_space.clone());

            // Parse use statements
            let (_uses, remaining) = registry.parse_use_statements(&after_ns, &file_path);

            // Extract function declarations for registry
            let func_re = Regex::new(r"(?m)^\s*(public\s+)?\b(void|i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|char|String)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(([^)]*)\)\s*\{")
                .map_err(|e| TranspileError::TransformError(e.to_string()))?;

            eprintln!("DEBUG: extracting functions from module '{}' with remaining length {}", module.name, remaining.len());
            
            for caps in func_re.captures_iter(&remaining) {
                let is_pub = caps.get(1).is_some();
                let ret_type = caps[2].to_string();
                let func_name = caps[3].to_string();
                let params = caps[4].to_string();

                eprintln!("DEBUG: registering function '{}' in namespace '{}'", func_name, ns_name);
                
                registry.register_function(FunctionInfo {
                    name: func_name,
                    namespace: ns_name.clone(),
                    is_public: is_pub,
                    params,
                    ret_type,
                });
            }

            // Extract class declarations for registry (with inheritance detection)
            let class_re = Regex::new(r"(?m)^\s*class\s+([a-zA-Z_][a-zA-Z0-9_]*)(?:\s+extends\s+([a-zA-Z_][a-zA-Z0-9_]*))?")
                .map_err(|e| TranspileError::TransformError(e.to_string()))?;

            for caps in class_re.captures_iter(&remaining) {
                let class_name = caps[1].to_string();
                let parent_class = caps.get(2).map(|m| m.as_str().to_string());
                
                // Check for nested classes (Class.Inner patterns in source)
                let nested_re = Regex::new(&format!(r"{}\.([a-zA-Z_][a-zA-Z0-9_]*)", class_name))
                    .map_err(|e| TranspileError::TransformError(e.to_string()))?;
                
                let mut nested = std::collections::HashMap::new();
                for ncaps in nested_re.captures_iter(&remaining) {
                    let inner_name = ncaps[1].to_string();
                    let full_path = format!("{}.{}", class_name, inner_name);
                    nested.insert(inner_name, full_path);
                }
                
                registry.register_class(ClassInfo {
                    name: class_name.clone(),
                    namespace: ns_name.clone(),
                    is_public: true, // TODO: parse public keyword
                    parent: parent_class.clone(),
                    nested,
                    methods: std::collections::HashMap::new(),
                });
                
                // Apply inheritance (过继) - move parent to child's namespace
                if let Some(parent) = parent_class {
                    registry.apply_inheritance(&class_name, &parent, &ns_name);
                }
            }

            module_sources.push((module.name.clone(), remaining, file_path));
        }

        // Phase 2: Transpile with namespace resolution
        let mut all_code = String::new();

        // Add a comment header
        all_code.push_str("// Generated by Hust transpiler\n");
        all_code.push_str("// Multi-module compilation with namespace support\n\n");

        // Transpile each module (except entry) as a separate section
        for (name, source, _file_path) in &module_sources {
            if name != &entry_module.name {
                all_code.push_str(&format!("// Module: {}\n", name));
                let transpiled = self.transpile(source)?;
                all_code.push_str(&transpiled);
                all_code.push_str("\n\n");
            }
        }

        // Transpile entry module last (main function)
        all_code.push_str(&format!("// Entry module: {}\n", entry_module.name));
        let entry_source = module_sources
            .iter()
            .find(|(n, _, _)| n == &entry_module.name)
            .map(|(_, s, _)| s.clone())
            .unwrap_or_else(|| entry_module.source.clone());

        let entry_transpiled = self.transpile(&entry_source)?;
        all_code.push_str(&entry_transpiled);

        // Phase 2.5: Detect function name conflicts across namespaces
        // Build a map of function name -> list of namespaces
        let mut func_namespaces: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for funcs in registry.functions.values() {
            for func in funcs {
                func_namespaces
                    .entry(func.name.clone())
                    .or_insert_with(Vec::new)
                    .push(func.namespace.clone());
            }
        }
        
        // Check for conflicts (same function name in multiple namespaces)
        for (func_name, namespaces) in &func_namespaces {
            if namespaces.len() > 1 {
                // Check if this function is called in the original source (before transpilation)
                for (_, source, _) in &module_sources {
                    // Simple pattern: funcName( (not preceded by . or word char)
                    let pattern = format!(r"(^|[^.\w]){}\s*\(", regex::escape(func_name));
                    if let Ok(re) = Regex::new(&pattern) {
                        if re.is_match(source) {
                            // Function is called but exists in multiple namespaces - conflict!
                            return Err(TranspileError::TransformError(format!(
                                "Error: ambiguous function call '{}'\n\
                                 Found in multiple namespaces: {}\n\
                                 Help: use explicit namespace prefix, e.g., {}.{}(...)",
                                func_name,
                                namespaces.join(", "),
                                namespaces[0], func_name
                            )));
                        }
                    }
                }
            }
        }

        // Phase 3: Post-process - resolve namespace calls with `.` separator
        // Build set of known function names that need prefix
        let known_funcs: std::collections::HashSet<String> = registry
            .functions
            .keys()
            .cloned()
            .collect();

        // For each known function, replace bare calls with namespaced calls
        let mut result = all_code;
        eprintln!("DEBUG: known_funcs = {:?}", known_funcs);
        for func_name in &known_funcs {
            // Skip main - it's the entry point
            if func_name == "main" {
                continue;
            }

            // Find the namespace for this function (using entry file context)
            let entry_file = module_sources
                .iter()
                .find(|(n, _, _)| n == &entry_module.name)
                .map(|(_, _, f)| f.clone())
                .unwrap_or_default();
            
            eprintln!("DEBUG: resolving function '{}' in file '{}'", func_name, entry_file);
            
            if let Ok(Some(ns)) = registry.resolve_with_imports(func_name, &entry_file, &registry.default_space) {
                eprintln!("DEBUG: resolved '{}' to namespace '{}'", func_name, ns);
                if ns != registry.default_space {
                    // Replace bare calls with namespaced calls using `.` separator
                    // Match: funcName( but not: ns.funcName( or .funcName(
                    let pattern = format!(r"(?<![.\w]){}\s*\(", regex::escape(func_name));
                    if let Ok(re) = Regex::new(&pattern) {
                        result = re
                            .replace_all(&result, format!("{}::{}(", ns, func_name))  // Rust uses ::
                            .to_string();
                    }
                }
            } else {
                eprintln!("DEBUG: failed to resolve '{}'", func_name);
            }
        }

        // Phase 3.5: Handle alias calls (mo.funcName) and namespace calls (ns.funcName)
        // Get entry file for use statement lookup
        let entry_file_for_alias = module_sources
            .iter()
            .find(|(n, _, _)| n == &entry_module.name)
            .map(|(_, _, f)| f.clone())
            .unwrap_or_default();
        
        // Get all use statements from entry file
        if let Some(uses) = registry.use_statements.get(&entry_file_for_alias) {
            for use_stmt in uses {
                // Handle alias: use ns as alias; → alias.funcName( → funcName(
                // Namespace is logical grouping, all functions are in same crate
                if let Some(alias) = &use_stmt.alias {
                    // Replace alias.funcName( with funcName(
                    let pattern = format!(r"{}\.(\w+)\s*\(", regex::escape(alias));
                    if let Ok(re) = Regex::new(&pattern) {
                        result = re
                            .replace_all(&result, "${1}(")
                            .to_string();
                    }
                }
            }
        }

        // Phase 3.6: Handle namespace direct calls (ns.funcName) and detect non-public calls
        // Get all namespaces from registry
        let all_namespaces: Vec<String> = registry.spaces.keys().cloned().collect();
        
        for ns in &all_namespaces {
            // Skip system namespaces
            if ns == "RUST" || ns == "HUST" || NamespaceRegistry::is_reserved_namespace(ns) {
                continue;
            }
            
            // Find all functions in this namespace
            let ns_functions: Vec<_> = registry.functions.values()
                .flatten()
                .filter(|f| f.namespace == *ns)
                .collect();
            
            for func in ns_functions {
                // Check if function is called as ns.funcName(
                let pattern = format!(r"{}\.{}", regex::escape(ns), regex::escape(&func.name));
                if let Ok(re) = Regex::new(&pattern) {
                    if re.is_match(&result) {
                        // Function is called with namespace prefix
                        if !func.is_public {
                            // Non-public function called from outside - error!
                            return Err(TranspileError::TransformError(format!(
                                "Error: function '{}' is private in namespace '{}'\n\
                                 Help: remove 'public' keyword from function definition to make it private,\n\
                                 or call it from within the same namespace.",
                                func.name, ns
                            )));
                        } else {
                            // Public function - replace ns.funcName( with funcName(
                            let call_pattern = format!(r"{}\.{}", regex::escape(ns), regex::escape(&func.name));
                            if let Ok(call_re) = Regex::new(&call_pattern) {
                                result = call_re
                                    .replace_all(&result, &func.name)
                                    .to_string();
                            }
                        }
                    }
                }
            }
        }

        // Phase 3.7: Detect function name conflicts across namespaces
        // Build a map of function name -> list of namespaces
        let mut func_namespaces: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for funcs in registry.functions.values() {
            for func in funcs {
                func_namespaces
                    .entry(func.name.clone())
                    .or_insert_with(Vec::new)
                    .push(func.namespace.clone());
            }
        }
        
        // Check for conflicts (same function name in multiple namespaces)
        for (func_name, namespaces) in &func_namespaces {
            if namespaces.len() > 1 {
                // Check if this function is called in the code
                let pattern = format!(r"(?<![.\w]){}\s*\(", regex::escape(func_name));
                if let Ok(re) = Regex::new(&pattern) {
                    if re.is_match(&result) {
                        // Function is called but exists in multiple namespaces - conflict!
                        return Err(TranspileError::TransformError(format!(
                            "Error: ambiguous function call '{}'\n\
                             Found in multiple namespaces: {}\n\
                             Help: use explicit namespace prefix, e.g., {}.{}(...)",
                            func_name,
                            namespaces.join(", "),
                            namespaces[0], func_name
                        )));
                    }
                }
            }
        }

        // Phase 4: Print distribution report
        registry.print_report();

        Ok(result)
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

            // Build signature WITH a fail-fast default body — an empty trait
            // impl would be an E0046 error, and a silent zero-value default
            // would hide "forgot to implement". panic body: compiles, and
            // any un-overridden call fails loudly; the class's own method
            // Pure signature (no default body) — the trait impl in the
            // class's own `impl Student for` block provides real methods
            // (all methods, visibility per two-layer model), satisfying
            // the trait without placeholders.
            // Use &mut self for all interface methods (conservative approach:
            // allows both read-only and mutable implementations)
            let sig = if ret_type == "void" {
                format!("\n    fn {}(&mut self{});", rust_method, rust_params)
            } else {
                format!("\n    fn {}(&mut self{}) -> {};", rust_method, rust_params, ret_type)
            };

            result.push(sig);
        }

        if result.is_empty() {
            String::new()
        } else {
            format!("\n    {}\n", result.join("\n    "))
        }
    }

    /// Extract nested class table from source
    /// Returns NestedClassTable with outer->inner mappings and full definitions
    fn extract_nested_class_table(&self, source: &str) -> NestedClassTable {
        use regex::Regex;
        let mut table = NestedClassTable::default();
        
        // Find all class definitions with their bodies
        // Use brace-depth matching to handle nested braces correctly
        let class_re = Regex::new(r"(?m)^\s*class\s+([A-Z][a-zA-Z0-9_]*)\s*\{").unwrap();
        
        for caps in class_re.captures_iter(source) {
            let class_name = caps[1].to_string();
            let body_start = caps.get(0).unwrap().end();
            
            // Extract body with brace matching
            let mut depth = 1;
            let mut pos = body_start;
            let bytes = source.as_bytes();
            while pos < bytes.len() && depth > 0 {
                match bytes[pos] as char {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => {}
                }
                pos += 1;
            }
            
            if depth == 0 {
                let body = &source[body_start..pos - 1];
                
                // Check for nested class definitions in body
                let nested_re = Regex::new(r"(?m)^\s*class\s+([A-Z][a-zA-Z0-9_]*)\s*\{").unwrap();
                for nested_caps in nested_re.captures_iter(body) {
                    let inner_name = nested_caps[1].to_string();
                    
                    // Record nesting relationship
                    table.outer_to_inner
                        .entry(class_name.clone())
                        .or_insert_with(Vec::new)
                        .push(inner_name.clone());
                    table.inner_to_outer.insert(inner_name.clone(), class_name.clone());
                    
                    // Extract full nested class definition
                    let nested_start = nested_caps.get(0).unwrap().start();
                    let mut nested_depth = 1;
                    let mut nested_pos = nested_caps.get(0).unwrap().end();
                    let body_bytes = body.as_bytes();
                    while nested_pos < body_bytes.len() && nested_depth > 0 {
                        match body_bytes[nested_pos] as char {
                            '{' => nested_depth += 1,
                            '}' => nested_depth -= 1,
                            _ => {}
                        }
                        nested_pos += 1;
                    }
                    
                    if nested_depth == 0 {
                        let full_def = &body[nested_start..nested_pos];
                        let key = format!("{}.{}", class_name, inner_name);
                        table.nested_defs.insert(key, full_def.to_string());
                    }
                }
            }
        }
        
        table
    }

    /// V0.6: Transform class definitions to Rust struct + impl
    /// Supports nested classes using table-driven approach (类似继承表)
    fn transform_class_definitions(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;
        use std::collections::HashMap;

        // Pre-pass: extract all class definitions with brace-depth matching.
        let class_table = self.extract_class_table(source);
        
        // Pre-pass: extract nested class table
        let nested_table = self.extract_nested_class_table(source);
        
        // Debug: print nested table info
        eprintln!("DEBUG: nested_table.outer_to_inner = {:?}", nested_table.outer_to_inner);
        eprintln!("DEBUG: nested_table.nested_defs keys = {:?}", nested_table.nested_defs.keys().collect::<Vec<_>>());
        
        // Remove nested class definitions from source (they'll be generated separately)
        let mut cleaned_source = source.to_string();
        for (key, def) in &nested_table.nested_defs {
            // Remove the nested class definition from its outer class body
            cleaned_source = cleaned_source.replace(def.as_str(), "");
        }

        // Match class definition with optional public, extends and implements
        let re = Regex::new(r"(?m)^\s*(public\s+)?class\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*(?:extends\s+(\w+))?\s*(?:implements\s+([\w,\s]+))?\s*\{([\s\S]*?)^\}")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        let mut result = re.replace_all(&cleaned_source, |caps: &regex::Captures| {
            let is_public = caps.get(1).is_some();
            let class_name = &caps[2];
            let parent_class = caps.get(3).map(|m| m.as_str());
            let interfaces = caps.get(4).map(|m| m.as_str());
            let body = &caps[5];

            // Parse class body into fields and methods
            let (fields, methods) = self.parse_class_body(body);

            // Collect all interfaces from parent chain (for auto-delegation)
            let mut all_interfaces: Vec<String> = Vec::new();
            let mut parent_chain: Vec<String> = Vec::new();
            
            if let Some(ifs) = interfaces {
                for if_name in ifs.split(',').map(|s| s.trim()) {
                    if !if_name.is_empty() {
                        all_interfaces.push(if_name.to_string());
                    }
                }
            }
            
            let mut visited: HashSet<String> = HashSet::new();
            let mut current_parent = parent_class.map(|s| s.to_string());
            while let Some(ref parent) = current_parent {
                if !visited.insert(parent.clone()) {
                    break;
                }
                parent_chain.push(parent.clone());
                if let Some(class_info) = class_table.get(parent) {
                    for if_name in &class_info.interfaces {
                        if !all_interfaces.contains(if_name) {
                            all_interfaces.push(if_name.clone());
                        }
                    }
                    current_parent = class_info.parent.clone();
                } else {
                    current_parent = None;
                }
            }

            let mut trait_methods: HashSet<String> = HashSet::new();
            let mut interface_methods_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
            
            for if_name in &all_interfaces {
                let t_re = Regex::new(&format!(r"(?s)trait\s+{}\s*\{{([^}}]*)\}}", if_name)).expect("static regex");
                if let Some(tc) = t_re.captures(source) {
                    let f_re = Regex::new(r"fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(").expect("static regex");
                    let mut methods_in_iface: Vec<String> = Vec::new();
                    for f in f_re.captures_iter(&tc[1]) {
                        trait_methods.insert(f[1].to_string());
                        methods_in_iface.push(f[1].to_string());
                    }
                    interface_methods_map.insert(if_name.clone(), methods_in_iface);
                }
            }

            // Generate struct with visibility
            let struct_def = self.generate_struct(class_name, &fields, parent_class, is_public);

            // Generate impl block
            let impl_def = self.generate_impl_with_delegation(
                class_name,
                &methods,
                parent_class,
                interfaces,
                &trait_methods,
                &parent_chain,
                &interface_methods_map,
                &class_table,
                source,
            );

            format!("{}\n{}", struct_def, impl_def)
        }).to_string();

        // Generate nested class definitions (flat Outer_Inner structs)
        if !nested_table.nested_defs.is_empty() {
            result.push_str("\n\n// Nested classes (flattened)\n");
            for (key, def) in &nested_table.nested_defs {
                // key is "Outer.Inner", convert to "Outer_Inner"
                let flat_name = key.replace('.', "_");
                let nested_code = self.generate_flat_nested_class(&flat_name, def);
                result.push_str(&nested_code);
                result.push('\n');
            }
        }

        Ok(result)
    }

    /// Generate flat nested class definition
    fn generate_flat_nested_class(&self, flat_name: &str, class_def: &str) -> String {
        // Extract body from class definition
        let body_start = class_def.find('{').unwrap_or(0) + 1;
        let body_end = class_def.rfind('}').unwrap_or(class_def.len());
        let body = &class_def[body_start..body_end];
        
        // Parse fields and methods
        let (fields, methods) = self.parse_class_body(body);
        
        // Generate struct
        let mut result = format!("#[derive(Default)]\nstruct {} {{", flat_name);
        for field in &fields {
            let rust_field = self.to_snake_case(&field.name);
            result.push_str(format!("\n    {}: {},", rust_field, field.type_name).as_str());
        }
        result.push_str("\n}\n");
        
        // Generate impl block
        result.push_str(format!("impl {} {{", flat_name).as_str());
        for method in &methods {
            let visibility = match method.visibility {
                Visibility::Public => "pub ",
                Visibility::Private => "",
            };
            
            // Add &self parameter for methods (Hust methods are instance methods by default)
            let params_with_self = if method.params.is_empty() {
                "&self".to_string()
            } else {
                format!("&self, {}", method.params)
            };
            
            if method.ret_type == "void" || method.ret_type.is_empty() {
                result.push_str(format!("\n    {}fn {}({}) {{", visibility, method.name, params_with_self).as_str());
            } else {
                result.push_str(format!("\n    {}fn {}({}) -> {} {{", visibility, method.name, params_with_self, method.ret_type).as_str());
            }
            
            if let Some(body) = &method.body {
                for line in body.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        result.push_str(format!("\n        {}", trimmed).as_str());
                    }
                }
            }
            
            result.push_str("\n    }");
        }
        result.push_str("\n}\n");
        
        result
    }

    /// Pre-parse all class definitions in source.
    /// Returns a HashMap: class_name -> ClassInfo { parent, interfaces }
    /// Uses brace-depth matching to correctly handle nested braces in class bodies.
    /// Skips nested classes (those inside other classes) - they are handled separately.
    fn extract_class_table(&self, source: &str) -> std::collections::HashMap<String, ClassInfo> {
        use regex::Regex;
        let mut table = std::collections::HashMap::new();
        
        // First, get nested class table to know which classes are nested
        let nested_table = self.extract_nested_class_table(source);
        let nested_classes: std::collections::HashSet<String> = nested_table.inner_to_outer.keys().cloned().collect();
        
        // Match class header: class Name [extends Parent] [implements I1, I2...] {
        let header_re = Regex::new(
            r"(?m)^\s*class\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*(?:extends\s+(\w+))?\s*(?:implements\s+([\w,\s]+))?\s*\{"
        ).expect("static regex");
        
        for caps in header_re.captures_iter(source) {
            let class_name = caps[1].to_string();
            
            // Skip nested classes - they are handled by nested class table
            if nested_classes.contains(&class_name) {
                continue;
            }
            
            let parent = caps.get(2).map(|m| m.as_str().to_string());
            let interfaces: Vec<String> = caps.get(3)
                .map(|m| m.as_str().split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect())
                .unwrap_or_default();
            
            table.insert(class_name, ClassInfo { parent, interfaces });
        }
        
        table
    }

    /// Parse class body into fields and methods (rewritten 2026.09.18)
    /// Method bodies are extracted with brace-depth matching — the old
    /// `[^{}]*` pattern broke on nested blocks (if/for inside methods),
    /// leaking method fragments into field parsing (`avg: return`).
    fn parse_class_body(&self, body: &str) -> (Vec<ClassField>, Vec<ClassMethod>) {
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        use regex::Regex;

        // Method signature start: [public] [Type] name ( ... ) {
        // (brace-depth-aware body extraction — methods may contain nested
        //  if/for blocks, which the old [^{}]* pattern could not handle)
        // Supports constructors: public ClassName(...) with no return type
        // Constructor: public ClassName(...) - no return type
        // Regular method: [public] ReturnType name(...)
        let sig_re = Regex::new(
            r"(?s)(public\s+)?(?:(\w+)\s+)?([a-zA-Z_]\w*)\s*\(([^)]*)\)\s*\{",
        )
        .unwrap();

        // 1. locate method spans (signature start + brace-matched body end)
        let mut spans: Vec<(usize, usize)> = Vec::new();
        let mut pos = 0usize;
        while let Some(m) = sig_re.find_at(body, pos) {
            let mut depth = 1usize;
            let mut p = m.end();
            let mut in_string = false;
            while p < body.len() {
                let c = body.as_bytes()[p] as char;
                if in_string {
                    if c == '\\' {
                        p += 1;
                        continue;
                    }
                    if c == '"' {
                        in_string = false;
                    }
                } else {
                    match c {
                        '"' => in_string = true,
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                p += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                p += 1;
            }
            spans.push((m.start(), p));
            pos = p;
        }

        // 2. methods: parse each span
        for (s, e) in &spans {
            if let Some(method) = self.parse_method(&body[*s..*e]) {
                methods.push(method);
            }
        }

        // 3. fields: non-method text, line by line (skip blanks/comments)
        let mut last = 0usize;
        let mut gaps = String::new();
        for (s, e) in &spans {
            gaps.push_str(&body[last..*s]);
            last = *e;
        }
        gaps.push_str(&body[last..]);
        for line in gaps.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with("//") || t.starts_with("/*") {
                continue;
            }
            if let Some(field) = self.parse_field(line) {
                fields.push(field);
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

        // Guard: a Rust/Hust keyword misparsed as a type (method-extraction
        // fallback safety — e.g. `return avg;` -> type `return` name `avg`)
        const NOT_TYPES: &[&str] = &[
            "return", "if", "else", "for", "while", "let", "fn", "match", "true", "false",
            "println", "print", "self", "break", "continue", "pub", "public",
        ];
        if NOT_TYPES.contains(&type_name) {
            return None;
        }

        Some(ClassField {
            name: field_name.to_string(),
            // C-style array field type (i32[5]) -> Rust form ([i32; 5])
            type_name: self.c_array_type_to_rust(type_name),
            visibility,
        })
    }

    /// Parse a method declaration (supports multi-line with (?s))
    /// Body capture is greedy to the span's final `}` — the span from
    /// parse_class_body is brace-matched, so its last `}` closes the method.
    /// Supports constructors: public ClassName(...) with no return type
    fn parse_method(&self, text: &str) -> Option<ClassMethod> {
        use regex::Regex;

        // First, try to match constructor: public ClassName(params) { body }
        // Constructor has no return type, just ClassName(params)
        let ctor_re = Regex::new(r"(?s)^\s*public\s+([A-Z]\w*)\s*\(([^)]*)\)\s*(\{.*\})?").unwrap();
        
        if let Some(caps) = ctor_re.captures(text) {
            // This is a constructor
            let name = caps[1].to_string();
            let params = caps[2].to_string();
            let body = caps.get(3).map(|m| m.as_str().to_string());
            
            return Some(ClassMethod {
                name,
                ret_type: "Self".to_string(),
                params,
                body,
                visibility: Visibility::Public,
            });
        }

        // Regular method: [public] ReturnType name(params) { body }
        let re =
            Regex::new(r"(?s)^\s*(public\s+)?(\w+)\s+([a-zA-Z_]\w*)\s*\(([^)]*)\)\s*(\{.*\})?").unwrap();

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
            visibility: if is_public {
                Visibility::Public
            } else {
                Visibility::Private
            },
        })
    }

    /// Generate Rust struct from class fields
    fn generate_struct(
        &self,
        class_name: &str,
        fields: &[ClassField],
        parent_class: Option<&str>,
        is_public: bool,
    ) -> String {
        // Add derive macros for Default
        let visibility = if is_public { "pub " } else { "" };
        let mut result = format!("#[derive(Default)]\n{}struct {} {{", visibility, class_name);

        // If has parent, include parent as field
        // Field name preserves the class name as written (case-sensitive,
        // owner decision 2026.09.22: respect author's naming, no forced
        // snake_case conversion)
        if let Some(parent) = parent_class {
            result.push_str(format!("\n    {}: {},", parent, parent).as_str());
        }

        // Add own fields with visibility
        for field in fields {
            let rust_field = self.to_snake_case(&field.name);
            let field_vis = match field.visibility {
                Visibility::Public => "pub ",
                Visibility::Private => "",
            };
            result.push_str(format!("\n    {}{}: {},", field_vis, rust_field, field.type_name).as_str());
        }

        result.push_str("\n}\n");
        result
    }

    /// Generate impl block with auto-delegation for inherited interfaces.
    /// When class C extends B extends A, and A implements IA, B implements IB,
    /// C must automatically implement IA and IB via delegation to parent.
    fn generate_impl_with_delegation(
        &self,
        class_name: &str,
        methods: &[ClassMethod],
        parent_class: Option<&str>,
        interfaces: Option<&str>,
        trait_methods: &HashSet<String>,
        parent_chain: &[String],
        interface_methods_map: &std::collections::HashMap<String, Vec<String>>,
        class_table: &std::collections::HashMap<String, ClassInfo>,
        source: &str,  // Add source for method signature extraction
    ) -> String {
        let mut result = String::new();

        // Separate interfaces: own vs inherited from parent chain
        let own_interfaces: Vec<String> = interfaces
            .map(|s| s.split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect())
            .unwrap_or_default();
        
        let inherited_interfaces: Vec<String> = interface_methods_map
            .keys()
            .filter(|k| !own_interfaces.contains(k))
            .cloned()
            .collect();

        // Generate trait implementations for own interfaces (normal)
        for if_name in &own_interfaces {
            let trait_impl =
                self.generate_trait_impl(class_name, if_name, methods, trait_methods);
            result.push_str(&trait_impl);
        }

        // Generate trait implementations for inherited interfaces (auto-delegation)
        for if_name in &inherited_interfaces {
            let delegate_impl = self.generate_delegate_trait_impl(
                class_name,
                if_name,
                parent_class,
                parent_chain,
                &interface_methods_map[if_name],
                class_table,
                source,  // Pass source for method signature extraction
            );
            result.push_str(&delegate_impl);
        }

        // Generate inherent impl block for class methods
        let inherent_impl = self.generate_inherent_impl(class_name, methods, class_table, source);
        result.push_str(&inherent_impl);

        result
    }

    /// Generate a trait impl that delegates all methods to the parent class.
    /// For class C extends B extends A:
    /// - If B implements the interface: self.b.method()
    /// - If only A implements it: self.b.a.method()
    fn generate_delegate_trait_impl(
        &self,
        class_name: &str,
        interface_name: &str,
        parent_class: Option<&str>,
        parent_chain: &[String],
        method_names: &[String],
        class_table: &std::collections::HashMap<String, ClassInfo>,
        source: &str,  // Add source to extract method signatures
    ) -> String {
        let mut result = format!("impl {} for {} {{\n", interface_name, class_name);

        // Find which parent in the chain implements this interface
        let mut delegation_path = String::new();
        let mut found = false;
        
        if let Some(parent) = parent_class {
            // Check immediate parent first
            let parent_implements = self.class_implements_interface(parent, interface_name, class_table);
            if parent_implements {
                delegation_path = format!("self.{}", parent);
                found = true;
            } else {
                // Check grandparents (cycle-safe)
                let mut visited: HashSet<String> = HashSet::new();
                visited.insert(parent.to_string());
                let mut path = format!("self.{}", parent);
                for ancestor in parent_chain.iter().skip(1) {
                    if !visited.insert(ancestor.clone()) {
                        break; // Cycle detected
                    }
                    path.push_str(format!(".{}", ancestor).as_str());
                    if self.class_implements_interface(ancestor, interface_name, class_table) {
                        delegation_path = path;
                        found = true;
                        break;
                    }
                }
            }
        }
        
        if !found {
            // Fallback: use immediate parent
            if let Some(parent) = parent_class {
                delegation_path = format!("self.{}", parent);
            } else {
                delegation_path = "self".to_string();
            }
        }

        for method_name in method_names {
            // Extract method signature from interface definition
            let sig_re = regex::Regex::new(&format!(
                r"fn\s+{}\s*\(([^)]*)\)",
                regex::escape(method_name)
            )).unwrap();
            
            let params = if let Some(caps) = sig_re.captures(source) {
                caps[1].to_string()
            } else {
                String::new()
            };
            
            // Remove &mut self / &self from params (it's already in the signature)
            let params_clean = params
                .replace("&mut self", "")
                .replace("&self", "")
                .trim()
                .trim_start_matches(',')
                .trim()
                .to_string();
            
            // Build parameter list for delegation (pass through all params)
            let param_names: Vec<String> = params_clean
                .split(',')
                .filter_map(|p| {
                    let p = p.trim();
                    if p.is_empty() { return None; }
                    // Extract parameter name (format: "type name" or "name")
                    // Handle both "factor: i32" (Rust style) and "i32 factor" (C style)
                    let parts: Vec<&str> = p.split_whitespace().collect();
                    if parts.len() >= 2 {
                        // Check if first part ends with ':' (Rust style: "factor: i32")
                        if parts[0].ends_with(':') {
                            // Rust style: "factor: i32" -> extract "factor"
                            Some(parts[0].trim_end_matches(':').to_string())
                        } else {
                            // C style: "i32 factor" -> extract "factor" (last part)
                            Some(parts[parts.len() - 1].to_string())
                        }
                    } else {
                        // Single word, might be just name
                        Some(parts[0].to_string())
                    }
                })
                .collect();
            let param_list = param_names.join(", ");
            
            // Delegate with &mut self and pass through all parameters
            let sig_params = if params_clean.is_empty() {
                "&mut self".to_string()
            } else {
                format!("&mut self, {}", params_clean)
            };
            
            // Build delegation call - handle empty param list
            let delegation_call = if param_list.is_empty() {
                format!("{}.{}()", delegation_path, method_name)
            } else {
                format!("{}.{}({})", delegation_path, method_name, param_list)
            };
            
            result.push_str(&format!(
                "    fn {}({}) {{\n        {}\n    }}\n",
                method_name,
                sig_params,
                delegation_call
            ));
        }

        result.push_str("}\n");
        result
    }
    
    /// Check if a class implements a specific interface (using pre-parsed table)
    fn class_implements_interface(
        &self,
        class_name: &str,
        interface_name: &str,
        class_table: &std::collections::HashMap<String, ClassInfo>,
    ) -> bool {
        if let Some(info) = class_table.get(class_name) {
            return info.interfaces.iter().any(|i| i == interface_name);
        }
        false
    }

    /// Generate Rust impl block from class methods
    /// If implements interfaces, generates separate impl blocks
    fn generate_impl(
        &self,
        class_name: &str,
        methods: &[ClassMethod],
        _parent_class: Option<&str>,
        interfaces: Option<&str>,
        trait_methods: &HashSet<String>,
        class_table: &std::collections::HashMap<String, ClassInfo>,
        source: &str,
    ) -> String {
        let mut result = String::new();

        // Generate trait implementations for interfaces
        if let Some(ifs) = interfaces {
            let if_names: Vec<&str> = ifs.split(',').map(|s| s.trim()).collect();
            for if_name in if_names {
                let trait_impl =
                    self.generate_trait_impl(class_name, if_name, methods, trait_methods);
                result.push_str(&trait_impl);
            }
        }

        // Generate inherent impl block for class methods
        let inherent_impl = self.generate_inherent_impl(class_name, methods, class_table, source);
        result.push_str(&inherent_impl);

        result
    }

    /// Interface implementation methods must be public (owner decision
    /// 2026.09.12): a private method implementing a public interface leaks
    /// via the trait path (reachable once the trait is imported) — Java/Go
    /// enforce the same (weaker access = compile error). Fail-fast with
    /// guidance, replacing the cryptic E0308 at the call site.
    /// Enforced at pipeline level because generate_trait_impl returns String.
    /// Method names are snake_case-normalized on both sides before matching
    /// (trait methods are already snake_case; class methods may be camelCase).
    fn check_interface_impl_visibility(&self, source: &str) -> Result<(), TranspileError> {
        use regex::Regex;

        // per class: implements list + class body
        let class_re = Regex::new(
            r"(?m)^\s*class\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*(?:extends\s+\w+)?\s*implements\s+([a-zA-Z_][a-zA-Z0-9_,\s]+)\s*\{([\s\S]*?)^\}",
        )
        .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        // class method: [public] RetType name(params) { body }
        let method_re = Regex::new(
            r"(?s)(public\s+)?(\w+)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\([^)]*\)\s*\{[^{}]*\}",
        )
        .map_err(|e| TranspileError::TransformError(e.to_string()))?;

        for ccaps in class_re.captures_iter(source) {
            let class_name = &ccaps[1];
            let if_names: Vec<&str> = ccaps[2].split(',').map(|s| s.trim()).collect();
            let body = &ccaps[3];

            // interface member names -> owning interface (from generated
            // trait definitions) — HashMap so the error can name the interface
            let mut if_methods: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            for if_name in if_names {
                let t_re = Regex::new(&format!(
                    r"(?s)trait\s+{}\s*\{{([^}}]*)\}}",
                    if_name
                ))
                .map_err(|e| TranspileError::TransformError(e.to_string()))?;
                if let Some(tc) = t_re.captures(source) {
                    let f_re = Regex::new(r"fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(")
                        .map_err(|e| TranspileError::TransformError(e.to_string()))?;
                    for f in f_re.captures_iter(&tc[1]) {
                        if_methods.insert(f[1].to_string(), if_name.to_string());
                    }
                }
            }

            // class methods: non-public ones implementing an interface -> error
            // (names snake_case-normalized: trait members are snake_case,
            // class methods may be camelCase — compare in one canonical form)
            for mcaps in method_re.captures_iter(body) {
                let is_public = mcaps.get(1).is_some();
                let ret_type = &mcaps[2];
                let m_rust = self.to_snake_case(&mcaps[3]);
                if !is_public {
                    if let Some(if_name) = if_methods.get(&m_rust) {
                        return Err(TranspileError::TransformError(format!(
                            "syntax error: interface method `{}` must be declared `public` — \
                             private methods cannot implement the public interface `{}` (r26/r23). \
                             Fix: `public {} {}(...);`",
                            &mcaps[3], if_name, ret_type, &mcaps[3]
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// Generate trait implementation for an interface.
    /// Only methods that are members of this interface go into the trait
    /// impl (routing via trait_methods, 2026.09.11) — other class methods
    /// stay inherent-only (E0407 otherwise). Two-layer visibility: trait
    /// impl methods take visibility from the interface; inherent methods
    /// follow r26 (private default) and are called first by name resolution.
    fn generate_trait_impl(
        &self,
        class_name: &str,
        interface_name: &str,
        methods: &[ClassMethod],
        trait_methods: &HashSet<String>,
    ) -> String {
        let mut result = format!("impl {} for {} {{", interface_name, class_name);

        for method in methods {
            let rust_name = self.to_snake_case(&method.name);
            if !trait_methods.contains(&rust_name) {
                continue; // not an interface member: inherent-only
            }
            // NOTE: interface implementation methods must be public — this is
            // enforced at pipeline level (Rule 1.5 check_interface_impl_
            // visibility), because this function returns String (no Err path).
            let rust_params = self.transform_method_params(&method.params);

            // self param: always &mut self to match trait declaration
            // (trait methods are declared with &mut self for flexibility)
            let self_param = "&mut self";

            let sig = if method.ret_type == "void" {
                format!("\n    fn {}({}{}) {{", rust_name, self_param, rust_params)
            } else {
                format!(
                    "\n    fn {}({}{}) -> {} {{",
                    rust_name, self_param, rust_params, method.ret_type
                )
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
        class_table: &std::collections::HashMap<String, ClassInfo>,
        source: &str,
    ) -> String {
        let mut result = format!("impl {} {{", class_name);
        
        // Build field path map for inheritance
        let field_path_map = self.build_field_path_map(class_name, class_table, source);

        for method in methods {
            let vis = match method.visibility {
                Visibility::Public => "pub ",
                Visibility::Private => "",
            };

            let rust_name = self.to_snake_case(&method.name);
            let rust_params = self.transform_method_params(&method.params);

            // needs_mut: field-WRITE detection only — `self.x =` / `self.x +=` etc.
            // (the old "contains self. && contains =" misfired on
            //  `sum += self.scores[i]`, which only READS a field)
            // Constructors always need &mut self (they initialize fields)
            let needs_mut = if method.ret_type == "Self" {
                true
            } else {
                method
                    .body
                    .as_ref()
                    .map(|b| {
                        // Detect field writes: self.field = value or self.field += value
                        // Pattern: self.field or this.field followed by = or +=, -=, *=, /=
                        // Also support array element assignment: self.field[index] = value
                        regex::Regex::new(r"(?:self|this)\.\w+(?:\[\d+\])?\s*(?:[-+*/])?=").unwrap().is_match(b)
                    })
                    .unwrap_or(false)
            };
            let self_param = if needs_mut { "&mut self" } else { "&self" };

            // Constructor: generate new() method instead of method with &mut self
            if method.ret_type == "Self" {
                // Generate: pub fn new(params) -> Self { ... }
                let new_params = self.transform_params(&method.params);
                let sig = if new_params.is_empty() {
                    format!("\n    {}fn new() -> Self {{", vis)
                } else {
                    format!("\n    {}fn new({}) -> Self {{", vis, new_params)
                };
                result.push_str(&sig);
                
                // For constructor, we need to collect field assignments and generate Self { ... }
                let mut field_assignments = Vec::new();
                if let Some(ref body) = method.body {
                    let body_content = body.trim_start_matches('{').trim_end_matches('}');
                    // Extract field assignments: self.field = value; or this.field = value;
                    let assign_re = regex::Regex::new(r"(?:self|this)\.(\w+)\s*=\s*([^;]+);").unwrap();
                    for cap in assign_re.captures_iter(body_content) {
                        let field_name = &cap[1];
                        let mut value = cap[2].trim().to_string();
                        // Convert Hust array literal {a, b, c} to Rust [a, b, c]
                        if value.starts_with('{') && value.ends_with('}') {
                            value = format!("[{}]", &value[1..value.len()-1]);
                        }
                        field_assignments.push(format!("{}: {}", field_name, value));
                    }
                }
                
                if field_assignments.is_empty() {
                    result.push_str("\n        Self::default()");
                } else {
                    // Use inheritance table to build proper nested initialization
                    // For class C extends B extends A:
                    // Self { B: B::new(args), own_field: val, ..Default::default() }
                    let parent_from_table = class_table.get(class_name).and_then(|info| info.parent.clone());
                    if let Some(parent) = parent_from_table {
                        // Extract super() args from body to pass to parent constructor
                        let super_args = if let Some(ref body) = method.body {
                            let super_re = regex::Regex::new(r"super\s*\(([^)]*)\)").unwrap();
                            super_re.captures(body)
                                .map(|cap| cap[1].trim().to_string())
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };
                        
                        if super_args.is_empty() {
                            // No super() call, use parent Default
                            result.push_str(&format!("\n        Self {{ {}, ..Default::default() }}", field_assignments.join(", ")));
                        } else {
                            // Call parent constructor with super() args
                            result.push_str(&format!("\n        Self {{\n            {}: {}::new({}),\n            {},\n            ..Default::default()\n        }}", 
                                parent, parent, super_args,
                                field_assignments.join(",\n            ")
                            ));
                        }
                    } else {
                        result.push_str(&format!("\n        Self {{ {}, ..Default::default() }}", field_assignments.join(", ")));
                    }
                }
                
                result.push_str("\n    }");
            } else {
                // Regular method
                let sig = if method.ret_type == "void" {
                    format!(
                        "\n    {}fn {}({}{}) {{",
                        vis, rust_name, self_param, rust_params
                    )
                } else {
                    format!(
                        "\n    {}fn {}({}{}) -> {} {{",
                        vis, rust_name, self_param, rust_params, method.ret_type
                    )
                };

                result.push_str(&sig);

                if let Some(ref body) = method.body {
                    let body_content = body.trim_start_matches('{').trim_end_matches('}');
                    let transformed_body = self.transform_self_references_with_fields(
                        body_content, 
                        Some(&field_path_map)
                    );
                    result.push_str(&transformed_body);
                } else {
                    result.push_str("()");
                }

                result.push_str("\n    }");
            }
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
    /// - this.x -> self.x
    /// - super(...) -> self.Parent::new(...)
    /// - method calls: obj.methodName() -> obj.method_name()
    /// - inherited field access: obj.field -> obj.ParentChain.field
    fn transform_self_references(&self, body: &str) -> String {
        self.transform_self_references_with_fields(body, None)
    }
    
    /// Transform method body with optional field path mapping for inheritance
    fn transform_self_references_with_fields(
        &self, 
        body: &str,
        field_path_map: Option<&std::collections::HashMap<String, String>>,
    ) -> String {
        let mut result = body.to_string();

        // Transform this.x -> self.x (Hust uses 'this', Rust uses 'self')
        // Pattern: this.field or this.method() -> self.field or self.method()
        use regex::Regex;
        let this_re = Regex::new(r"\bthis\b").unwrap();
        result = this_re.replace_all(&result, "self").to_string();
        
        // Transform inherited field access: obj.field -> obj.ParentChain.field
        // Only for fields that are inherited (not defined in current class)
        if let Some(ref map) = field_path_map {
            for (field, path) in map.iter() {
                // Pattern: obj.field (not preceded by . or followed by ()
                // Use word boundary to avoid matching method calls
                let pattern = format!(r"(?<!\.)\b(\w+)\.{}{}", regex::escape(field), r"(?![\w(])");
                if let Ok(re) = Regex::new(&pattern) {
                    result = re.replace_all(&result, |caps: &regex::Captures| {
                        let obj = &caps[1];
                        format!("{}.{}", obj, path)
                    }).to_string();
                }
            }
        }

        // Transform super(...) -> self.Parent::new(...)
        // Note: This is a simplified version. In a real implementation,
        // we would need to know the parent class name.
        // For now, we'll just remove the super() call and let the user handle it
        let super_re = Regex::new(r"\bsuper\s*\([^)]*\)\s*;").unwrap();
        result = super_re.replace_all(&result, "// super() call removed").to_string();

        // Transform method calls from camelCase to snake_case
        // Pattern: .methodName( -> .method_name(
        let re = Regex::new(r"\.([a-z][a-zA-Z0-9]*)\s*\(").unwrap();
        result = re
            .replace_all(&result, |caps: &regex::Captures| {
                let method_name = &caps[1];
                let rust_method = self.to_snake_case(method_name);
                format!(".{}(", rust_method)
            })
            .to_string();

        result
    }

    /// Build field path map for inheritance: field_name -> "Parent.GrandParent"
    /// For class C extends B extends A:
    /// - If A has field x, B has field y, C has field z:
    ///   - x -> "B.A" (accessed via self.B.A.x)
    ///   - y -> "B" (accessed via self.B.y)
    ///   - z -> None (defined in C, no path needed)
    fn build_field_path_map(
        &self,
        class_name: &str,
        class_table: &std::collections::HashMap<String, ClassInfo>,
        source: &str,
    ) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        
        // Get own fields (defined in this class)
        let own_fields: HashSet<String> = self.extract_class_fields(source, class_name);
        
        // Walk up parent chain
        let mut visited: HashSet<String> = HashSet::new();
        let mut current_parent = class_table.get(class_name).and_then(|info| info.parent.clone());
        let mut path_prefix = String::new();
        
        while let Some(ref parent) = current_parent {
            if !visited.insert(parent.clone()) {
                break; // Cycle detected
            }
            
            // Build path prefix: e.g., "Rect.Shape" for ColorRect -> Rect -> Shape
            if path_prefix.is_empty() {
                path_prefix = parent.clone();
            } else {
                path_prefix = format!("{}.{}", path_prefix, parent);
            }
            
            // Get parent's fields
            let parent_fields = self.extract_class_fields(source, parent);
            for field in parent_fields {
                // Only add if not defined in current class and not already in map
                if !own_fields.contains(&field) && !map.contains_key(&field) {
                    map.insert(field, path_prefix.clone());
                }
            }
            
            // Continue up the chain
            current_parent = class_table.get(parent).and_then(|info| info.parent.clone());
        }
        
        map
    }
    
    /// Extract field names from a class definition in source
    fn extract_class_fields(&self, source: &str, class_name: &str) -> HashSet<String> {
        use regex::Regex;
        let mut fields = HashSet::new();
        
        // Find class definition
        let class_re = Regex::new(&format!(
            r"(?m)^\s*class\s+{}\s*(?:extends\s+\w+)?\s*(?:implements\s+[\w,\s]+)?\s*\{{([\s\S]*?)^\}}",
            regex::escape(class_name)
        ));
        
        if let Ok(re) = class_re {
            if let Some(caps) = re.captures(source) {
                let body = &caps[1];
                // Match field declarations: Type name; or Type name = value;
                // Also match array types: Type[dim] name;
                // Exclude method definitions (which have parentheses)
                let field_re = Regex::new(r"(?m)^\s*(?:public\s+|private\s+)?(\w+(?:\[\d+\])?)\s+([a-zA-Z_]\w*)\s*(?:=[^;]*)?;\s*$").unwrap();
                for fcaps in field_re.captures_iter(body) {
                    let field_name = fcaps[2].to_string();
                    // Skip if it looks like a method call (has parentheses after)
                    let after = &body[fcaps.get(0).unwrap().end()..];
                    if !after.trim_start().starts_with('(') {
                        fields.insert(field_name);
                    }
                }
            }
        }
        
        fields
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

/// Class info for pre-parsed inheritance table
#[derive(Debug)]
struct ClassInfo {
    parent: Option<String>,
    interfaces: Vec<String>,
}

/// Nested class table entry
#[derive(Debug, Default)]
struct NestedClassTable {
    /// outer_class_name -> list of inner_class_names
    outer_to_inner: std::collections::HashMap<String, Vec<String>>,
    /// inner_class_name -> outer_class_name (reverse lookup)
    inner_to_outer: std::collections::HashMap<String, String>,
    /// full nested class definitions: "Outer.Inner" -> class_body
    nested_defs: std::collections::HashMap<String, String>,
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
