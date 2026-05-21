//! Hust-Rust CLI Entry
//! Usage: hust run main.hust

use std::path::PathBuf;
use std::process::Command;
use clap::{Parser, Subcommand};
use anyhow::{Result, Context};

use hust_rust::Translator;

/// Hust Language Transpiler - Rust Adapter
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run Hust source file
    Run {
        /// Source file path
        file: PathBuf,
    },
    /// Build project
    Build {
        /// Project directory (default: current directory)
        #[arg(short, long, default_value = ".")]
        project_dir: PathBuf,
    },
    /// Check syntax
    Check {
        /// Source file path
        file: PathBuf,
    },
    /// Format code
    Fmt {
        /// Source file path
        file: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { file } => {
            println!("[Hust] Running: {:?}", file);
            
            // 1. Transpile Hust -> Rust
            let translator = Translator::default();
            let rust_code = translator.transpile_file(&file)
                .context("Transpilation failed")?;
            
            // 2. Create temp directory
            let temp_dir = std::env::temp_dir().join("hust_run");
            std::fs::create_dir_all(&temp_dir)?;
            
            // 3. Create src directory and write transpiled Rust code
            let src_dir = temp_dir.join("src");
            std::fs::create_dir_all(&src_dir)?;
            let rs_file = src_dir.join("main.rs");
            std::fs::write(&rs_file, &rust_code)
                .context("Failed to write temp file")?;
            
            // 4. Create Cargo.toml
            let cargo_toml = r#"[package]
name = "hust_temp"
version = "0.1.0"
edition = "2021"
"#;
            std::fs::write(temp_dir.join("Cargo.toml"), cargo_toml)?;
            
            // 5. Call cargo run
            println!("[Hust] Compiling and running...");
            let status = Command::new("cargo")
                .arg("run")
                .current_dir(&temp_dir)
                .status()
                .context("Failed to invoke cargo")?;
            
            if !status.success() {
                anyhow::bail!("Execution failed");
            }
            
            println!("[Hust] Done");
            Ok(())
        }
        Commands::Build { project_dir } => {
            println!("Building project: {:?}", project_dir);
            println!("TODO: Not implemented yet");
            Ok(())
        }
        Commands::Check { file } => {
            println!("Checking file: {:?}", file);
            println!("TODO: Not implemented yet");
            Ok(())
        }
        Commands::Fmt { file } => {
            println!("Formatting file: {:?}", file);
            println!("TODO: Not implemented yet");
            Ok(())
        }
    }
}