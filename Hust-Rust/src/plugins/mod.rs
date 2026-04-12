//! 插件系统模块
//! 提供可扩展的转换插件接口

use thiserror::Error;

/// 插件错误
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("插件加载失败: {0}")]
    LoadError(String),
    
    #[error("插件执行失败: {0}")]
    ExecutionError(String),
    
    #[error("插件不存在: {0}")]
    NotFound(String),
}

/// 插件 trait - 所有转换插件必须实现
pub trait Plugin {
    /// 插件名称
    fn name(&self) -> &str;
    
    /// 插件描述
    fn description(&self) -> &str;
    
    /// 版本
    fn version(&self) -> &str;
    
    /// 转换代码
    /// 输入源代码，返回转换后的代码
    fn transform(&self, source: &str) -> Result<String, PluginError>;
    
    /// 优先级（数值越小越先执行）
    fn priority(&self) -> u32 {
        100
    }
}

/// 插件管理器
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginManager {
    /// 创建新管理器
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }
    
    /// 注册插件
    pub fn register(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }
    
    /// 执行所有插件（按优先级排序）
    pub fn execute_all(&self, source: &str) -> Result<String, PluginError> {
        // 按优先级排序
        let mut sorted = self.plugins.iter().collect::<Vec<_>>();
        sorted.sort_by_key(|p| p.priority());
        
        // 依次执行
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

// 内置插件示例
mod builtins {
    use super::*;
    
    /// 语法简化插件
    pub struct SyntaxSimplifyPlugin;
    
    impl Plugin for SyntaxSimplifyPlugin {
        fn name(&self) -> &str { "syntax-simplify" }
        fn description(&self) -> &str { "简化语法糖，让代码更简洁" }
        fn version(&self) -> &str { "0.1.0" }
        
        fn transform(&self, source: &str) -> Result<String, PluginError> {
            // TODO: 实现简化逻辑
            Ok(source.to_string())
        }
    }
}

pub use builtins::*;