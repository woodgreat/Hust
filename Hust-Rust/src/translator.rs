//! 转译器核心模块
//! 将 Hust 代码转译为 Rust 代码

use std::path::PathBuf;
use thiserror::Error;

/// 转译错误
#[derive(Error, Debug)]
pub enum TranspileError {
    #[error("文件读取失败: {0}")]
    FileRead(#[from] std::io::Error),
    
    #[error("语法分析失败: {0}")]
    ParseError(String),
    
    #[error("转换失败: {0}")]
    TransformError(String),
    
    #[error("输出失败: {0}")]
    WriteError(String),
}

/// 转译选项
#[derive(Debug, Clone, Default)]
pub struct TranspileOptions {
    /// 是否保留注释
    pub preserve_comments: bool,
    /// 是否输出调试信息
    pub debug: bool,
    /// 是否启用所有插件
    pub enable_all_plugins: bool,
}

/// 转译器
#[derive(Debug)]
pub struct Translator {
    options: TranspileOptions,
}

impl Translator {
    /// 创建新转译器
    pub fn new(options: TranspileOptions) -> Self {
        Self { options }
    }
    
    /// 默认配置
    pub fn default() -> Self {
        Self::new(TranspileOptions::default())
    }
    
    /// 转译单个文件
    pub fn transpile_file(&self, input_path: &PathBuf) -> Result<String, TranspileError> {
        // 1. 读取源文件
        let source = std::fs::read_to_string(input_path)?;
        
        // 2. 转译
        self.transpile(&source)
    }
    
    /// 转译源代码 (V0.1 最简实现)
    pub fn transpile(&self, source: &str) -> Result<String, TranspileError> {
        let mut output = source.to_string();
        
        // V0.1 转换规则：
        // 1. 变量声明: "类型 变量名 = 值;" -> "let 变量名: 类型 = 值;"
        // 2. 函数定义: "void 函数名(...)" -> "fn 函数名(...)"
        // 3. 其他内容原样保留
        
        // 规则1: 变量声明转换
        // 匹配模式: (类型) (变量名) = (值);
        // 支持类型: i8, i16, i32, i64, u8, u16, u32, u64, f32, f64, bool, char, String
        output = self.transform_variable_declarations(&output)?;
        
        // 规则2: 函数定义转换
        // void main() -> fn main()
        // i32 add(...) -> fn add(...) -> i32
        output = self.transform_function_definitions(&output)?;
        
        Ok(output)
    }
    
    /// V0.1: 转换变量声明
    /// i32 x = 42; -> let x: i32 = 42;
    fn transform_variable_declarations(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;
        
        // 匹配: 类型 变量名 = 值;
        // 类型: i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|char|String
        // 变量名: [a-zA-Z_][a-zA-Z0-9_]*
        let re = Regex::new(r"\b(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|char|String)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*=")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        
        let result = re.replace_all(source, |caps: &regex::Captures| {
            let type_name = &caps[1];
            let var_name = &caps[2];
            format!("let {}: {} =", var_name, type_name)
        });
        
        Ok(result.to_string())
    }
    
    /// V0.1: 转换函数定义
    /// void main() -> fn main()
    /// i32 add(i32 a, i32 b) -> fn add(a: i32, b: i32) -> i32
    fn transform_function_definitions(&self, source: &str) -> Result<String, TranspileError> {
        use regex::Regex;
        
        // 先处理 void 函数: void name(...) -> fn name(...)
        let re_void = Regex::new(r"\bvoid\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        let result = re_void.replace_all(source, |caps: &regex::Captures| {
            let func_name = &caps[1];
            format!("fn {}(", func_name)
        });
        
        // 再处理带返回值的函数: 类型 name(参数) -> fn name(参数) -> 类型
        // 这个比较复杂，V0.1 先简化处理，只匹配简单形式
        let re_ret = Regex::new(r"\b(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|char|String)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\(")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        let result = re_ret.replace_all(&result, |caps: &regex::Captures| {
            let type_name = &caps[1];
            let func_name = &caps[2];
            format!("fn {}(", func_name)
        });
        
        // 处理参数类型: 类型 参数名 -> 参数名: 类型
        // V0.1 简化：只处理单个参数的情况
        let re_param = Regex::new(r"\(\s*(i8|i16|i32|i64|u8|u16|u32|u64|f32|f64|bool|char|String)\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\)")
            .map_err(|e| TranspileError::TransformError(e.to_string()))?;
        let result = re_param.replace_all(&result, |caps: &regex::Captures| {
            let type_name = &caps[1];
            let param_name = &caps[2];
            format!("({}: {})", param_name, type_name)
        });
        
        Ok(result.to_string())
    }
    
    /// 转译并输出到文件
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