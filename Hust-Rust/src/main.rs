//! Hust-Rust 命令行入口
//! 用途：hust run main.hust

use std::path::PathBuf;
use std::process::Command;
use clap::{Parser, Subcommand};
use anyhow::{Result, Context};

use hust_rust::Translator;

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
            println!("[Hust] 运行文件: {:?}", file);
            
            // 1. 转译 Hust -> Rust
            let translator = Translator::default();
            let rust_code = translator.transpile_file(&file)
                .context("转译失败")?;
            
            // 2. 创建临时目录
            let temp_dir = std::env::temp_dir().join("hust_run");
            std::fs::create_dir_all(&temp_dir)?;
            
            // 3. 创建 src 目录并写入转译后的 Rust 代码
            let src_dir = temp_dir.join("src");
            std::fs::create_dir_all(&src_dir)?;
            let rs_file = src_dir.join("main.rs");
            std::fs::write(&rs_file, &rust_code)
                .context("写入临时文件失败")?;
            
            // 4. 创建 Cargo.toml
            let cargo_toml = r#"[package]
name = "hust_temp"
version = "0.1.0"
edition = "2021"
"#;
            std::fs::write(temp_dir.join("Cargo.toml"), cargo_toml)?;
            
            // 5. 调用 cargo run
            println!("[Hust] 正在编译并运行...");
            let status = Command::new("cargo")
                .arg("run")
                .current_dir(&temp_dir)
                .status()
                .context("调用 cargo 失败")?;
            
            if !status.success() {
                anyhow::bail!("运行失败");
            }
            
            println!("[Hust] 完成");
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