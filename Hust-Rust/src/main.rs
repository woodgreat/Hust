//! Hust-Rust 命令行入口
//! 用途：hust run main.hust

use std::path::PathBuf;
use clap::{Parser, Subcommand};
use anyhow::Result;

/// Hust 语言转译器 - Rust 适配版本
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 运行 Hust 源文件
    Run {
        /// 源文件路径
        file: PathBuf,
    },
    /// 构建项目
    Build {
        /// 项目目录（默认当前目录）
        #[arg(short, long, default_value = ".")]
        project_dir: PathBuf,
    },
    /// 检查语法
    Check {
        /// 源文件路径
        file: PathBuf,
    },
    /// 格式化代码
    Fmt {
        /// 源文件路径
        file: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file } => {
            println!("运行文件: {:?}", file);
            // TODO: 实现运行逻辑
            println!("TODO: 实现中...");
            Ok(())
        }
        Commands::Build { project_dir } => {
            println!("构建项目: {:?}", project_dir);
            // TODO: 实现构建逻辑
            println!("TODO: 实现中...");
            Ok(())
        }
        Commands::Check { file } => {
            println!("检查文件: {:?}", file);
            // TODO: 实现检查逻辑
            println!("TODO: 实现中...");
            Ok(())
        }
        Commands::Fmt { file } => {
            println!("格式化文件: {:?}", file);
            // TODO: 实现格式化逻辑
            println!("TODO: 实现中...");
            Ok(())
        }
    }
}