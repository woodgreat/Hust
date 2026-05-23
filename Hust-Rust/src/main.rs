//! Hust-Rust CLI Entry
//! Usage: hust run main.hust

use std::path::PathBuf;

/// Get Hust version in Wood format (4-digit with dot)
/// Converts "0.1.0+20260521" -> "0.1.0.20260521"
fn get_hust_version() -> &'static str {
    env!("CARGO_PKG_VERSION").replace('+', ".").leak()
}
use std::process::Command;
use clap::{Parser, Subcommand};
use anyhow::{Result, Context};

use hust_rust::{Translator, ProjectConfig, ModuleResolver};

/// Hust Language Transpiler - Rust Adapter
#[derive(Parser)]
#[command(
    author,
    about,
    version = get_hust_version(),
)]
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

            // 2. Create build directory relative to hust.exe location
            // This ensures build/ is always next to hust.exe, not in current working dir
            let exe_dir = std::env::current_exe()?
                .parent()
                .context("Failed to get exe directory")?
                .to_path_buf();
            let build_dir = exe_dir.join("build");
            let temp_dir = build_dir.join("temp");
            let dist_dir = build_dir.join("dist");
            std::fs::create_dir_all(&temp_dir)?;
            std::fs::create_dir_all(&dist_dir)?;

            // 3. Create src directory and write transpiled Rust code
            let src_dir = temp_dir.join("src");
            std::fs::create_dir_all(&src_dir)?;
            let rs_file = src_dir.join("main.rs");
            std::fs::write(&rs_file, &rust_code)
                .context("Failed to write temp file")?;

            // 4. Create Cargo.toml with source file name as package name
            let src_stem = file.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("hust_temp");
            let cargo_toml = format!(r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[profile.dev]
opt-level = 0

[profile.release]
opt-level = 3
"#, src_stem);
            std::fs::write(temp_dir.join("Cargo.toml"), cargo_toml)?;

            // 5. Call cargo build with output to dist
            println!("[Hust] Compiling...");
            let status = Command::new("cargo")
                .arg("build")
                .arg("--target-dir")
                .arg(&dist_dir)
                .current_dir(&temp_dir)
                .status()
                .context("Failed to invoke cargo")?;

            if !status.success() {
                anyhow::bail!("Compilation failed");
            }

            // 6. Run the compiled binary
            println!("[Hust] Running...");
            // Get executable name from source file name
            let src_stem = file.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("hust_temp");
            let exe_name = if cfg!(windows) {
                format!("{}.exe", src_stem)
            } else {
                src_stem.to_string()
            };
            let exe_path = dist_dir.join("debug").join(&exe_name);

            let status = Command::new(&exe_path)
                .status()
                .context("Failed to run executable")?;

            if !status.success() {
                anyhow::bail!("Execution failed");
            }

            println!("[Hust] Done");
            Ok(())
        }
        Commands::Build { project_dir } => {
            println!("[Hust] Building project: {:?}", project_dir);

            // 1. Load project configuration
            let config = ProjectConfig::load(&project_dir)
                .context("Failed to load project configuration")?;

            // 2. Find entry file
            let entry_file = project_dir.join(&config.package.entry);
            if !entry_file.exists() {
                anyhow::bail!("Entry file '{}' not found in project directory", config.package.entry);
            }

            // 3. Resolve all module dependencies
            println!("[Hust] Resolving modules...");
            let mut resolver = ModuleResolver::new();
            let modules = resolver.resolve(&entry_file,&config,&project_dir
            ).context("Failed to resolve module dependencies")?;

            println!("[Hust] Found {} module(s)", modules.len());

            // 4. Transpile all modules into single Rust file
            let translator = Translator::default();
            let entry_module = modules.last()
                .context("No entry module found")?;
            let rust_code = translator.transpile_modules(&modules,
                entry_module
            ).context("Transpilation failed")?;

            // 5. Create build directory relative to hust.exe
            let exe_dir = std::env::current_exe()?
                .parent()
                .context("Failed to get exe directory")?
                .to_path_buf();
            let build_dir = exe_dir.join("build");
            let temp_dir = build_dir.join("temp");
            let dist_dir = build_dir.join("dist");
            std::fs::create_dir_all(&temp_dir)?;
            std::fs::create_dir_all(&dist_dir)?;

            // 6. Create src directory and write transpiled Rust code
            let src_dir = temp_dir.join("src");
            std::fs::create_dir_all(&src_dir)?;
            let rs_file = src_dir.join("main.rs");
            std::fs::write(&rs_file, &rust_code)
                .context("Failed to write temp file")?;

            // 7. Create Cargo.toml with project name
            let cargo_toml = format!(r#"[package]
name = "{}"
version = "{}"
edition = "2021"

[profile.dev]
opt-level = 0

[profile.release]
opt-level = 3
"#, config.package.name, config.package.version);
            std::fs::write(temp_dir.join("Cargo.toml"), cargo_toml)?;

            // 8. Call cargo build with output to dist
            println!("[Hust] Compiling...");
            let status = Command::new("cargo")
                .arg("build")
                .arg("--release")
                .arg("--target-dir")
                .arg(&dist_dir)
                .current_dir(&temp_dir)
                .status()
                .context("Failed to invoke cargo")?;

            if !status.success() {
                anyhow::bail!("Compilation failed");
            }

            // 9. Copy executable to project directory
            let exe_name = if cfg!(windows) {
                format!("{}.exe", config.package.name)
            } else {
                config.package.name.clone()
            };
            let exe_path = dist_dir.join("release").join(&exe_name);
            let output_exe = project_dir.join(&exe_name);
            std::fs::copy(&exe_path, &output_exe)
                .context("Failed to copy executable")?;

            println!("[Hust] Build complete: {:?}", output_exe);
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