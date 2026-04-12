//! Hust-Rust 库核心
//! 提供转译器功能和插件系统

pub mod translator;
pub mod plugins;

// 重新导出
pub use translator::{Translator, TranspileOptions, TranspileError};
pub use plugins::{Plugin, PluginError, PluginManager};

// 主错误类型
pub use thiserror::Error as HustError;

/// 结果类型别名
pub type HustResult<T> = std::result::Result<T, translator::TranspileError>;

/// 版本信息
pub const VERSION: &str = env!("CARGO_PKG_VERSION");