//! Hust-Rust CLI Entry
//! Usage: hust run main.hust

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Get Hust version in Wood format (4-digit with dot)
/// Converts "0.1.0+20260521" -> "0.1.0.20260521"
fn get_hust_version() -> &'static str {
    env!("CARGO_PKG_VERSION").replace('+', ".").leak()
}
use std::process::Command;
use clap::{Parser, Subcommand};
use anyhow::Context;

use hust_rust::{Translator, ProjectConfig, PackageConfig, ModuleResolver};

/// Hust Language Transpiler - Rust Adapter
#[derive(Parser)]
#[command(
    name = "hust",
    about = "Hust language transpiler for Rust",
    version = get_hust_version(),
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run Hust source file or project (compile + execute)
    Run {
        /// Source file or project directory
        target: PathBuf,
        /// Force project mode (use settings.config for modules)
        #[arg(long)]
        project: bool,
    },
    /// Build Hust source file or project (compile only, no run)
    Build {
        /// Project directory (default: current directory)
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Force project mode (use settings.config for modules)
        #[arg(long)]
        project: bool,
    },
    /// Check syntax of Hust source file or project (no output)
    Check {
        /// Source file or project directory
        target: PathBuf,
        /// Force project mode (use settings.config for modules)
        #[arg(long)]
        project: bool,
    },
    /// Format code
    Fmt {
        /// Source file path
        file: PathBuf,
    },
}

/// Target mode resolved from CLI arguments
enum TargetMode {
    /// Single source file (no modules)
    Single(PathBuf),
    /// Project directory (with settings.config)
    Project(PathBuf),
}

/// Resolve target path + --project flag into a mode.
/// - Directory target          -> project mode
/// - File target + --project   -> project mode (parent directory)
/// - File target, no --project -> single file mode
fn resolve_target(target: &Path, project: bool) -> anyhow::Result<TargetMode> {
    if project || target.is_dir() {
        let dir = if target.is_dir() {
            target.to_path_buf()
        } else {
            target.parent()
                .map(|p| p.to_path_buf())
                .ok_or_else(|| anyhow::anyhow!("Cannot determine project directory for {:?}", target))?
        };
        if !dir.join("settings.config").exists() {
            anyhow::bail!("'{}' is not a Hust project (settings.config not found). \
                For single file mode, drop --project.", dir.display());
        }
        Ok(TargetMode::Project(dir))
    } else {
        Ok(TargetMode::Single(target.to_path_buf()))
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { target, project } => {
            match resolve_target(&target, project)? {
                TargetMode::Project(project_dir) => {
                    println!("[Hust] Running project: {:?}", project_dir);
                    build_project(&project_dir)?;
                    // Run the compiled binary
                    let config = ProjectConfig::load(&project_dir)
                        .context("Failed to load project configuration")?;
                    let exe_name = if cfg!(windows) {
                        format!("{}.exe", config.package.name)
                    } else {
                        config.package.name.clone()
                    };
                    let exe_path = project_dir.join(&exe_name);
                    println!("[Hust] Running...");
                    let abs_exe = std::fs::canonicalize(&exe_path)
                        .context("Failed to canonicalize exe path")?;
                    let status = Command::new(&abs_exe)
                        .status()
                        .context("Failed to run executable")?;
                    if !status.success() {
                        anyhow::bail!("Execution failed");
                    }
                    println!("[Hust] Done");
                    Ok(())
                }
                TargetMode::Single(file) => {
                    println!("[Hust] Running: {:?}", file);
                    let exe_path = compile_single(&file, false, false)?
                        .context("Internal error: build mode returned no executable")?;
                    println!("[Hust] Running...");
                    let status = Command::new(&exe_path)
                        .status()
                        .context("Failed to run executable")?;
                    if !status.success() {
                        anyhow::bail!("Execution failed");
                    }
                    println!("[Hust] Done");
                    Ok(())
                }
            }
        }
        Commands::Build { dir, project } => {
            match resolve_target(&dir, project)? {
                TargetMode::Project(project_dir) => {
                    println!("[Hust] Building project: {:?}", project_dir);
                    build_project(&project_dir)
                }
                TargetMode::Single(file) => {
                    println!("[Hust] Building: {:?}", file);
                    compile_single(&file, false, false)?;
                    println!("[Hust] Build complete");
                    Ok(())
                }
            }
        }
        Commands::Check { target, project } => {
            match resolve_target(&target, project)? {
                TargetMode::Project(project_dir) => {
                    println!("[Hust] Checking project: {:?}", project_dir);
                    let config = ProjectConfig::load(&project_dir)
                        .context("Failed to load project configuration")?;
                    let entry_file = project_dir.join(&config.package.entry);
                    if !entry_file.exists() {
                        anyhow::bail!("Entry file '{}' not found in project directory",
                            config.package.entry);
                    }
                    let mut resolver = ModuleResolver::new();
                    let modules = resolver.resolve(&entry_file, &config, &project_dir)
                        .context("Failed to resolve module dependencies")?;
                    let translator = Translator::default();
                    let entry_module = modules.last()
                        .context("No entry module found")?;
                    let rust_code = translator.transpile_modules(&modules, entry_module)
                        .context("Transpilation failed")?;
                    compile_project_rust(&config, &rust_code, true)?;
                    println!("[Hust] Check passed ({} module(s))", modules.len());
                    Ok(())
                }
                TargetMode::Single(file) => {
                    println!("[Hust] Checking: {:?}", file);
                    compile_single(&file, false, true)?;
                    println!("[Hust] Check passed");
                    Ok(())
                }
            }
        }
        Commands::Fmt { file } => {
            println!("Formatting file: {:?}", file);
            println!("TODO: Not implemented yet");
            Ok(())
        }
    }
}

/// Transpile a single Hust file and compile it.
/// Returns the path to the compiled executable (build mode),
/// or Ok(None) for check mode (cargo check only, no artifacts).
fn compile_single(file: &Path, release: bool, check_only: bool) -> anyhow::Result<Option<PathBuf>> {
    // 1. Check if file contains use statements (module imports)
    let source = std::fs::read_to_string(file)
        .context("Failed to read source file")?;
    
    // If source contains use statements, resolve modules from current directory
    let rust_code = if source.contains("use ") {
        println!("[Hust] Detected use statements, resolving modules...");
        
        // Create a minimal project config for module resolution
        let current_dir = file.parent()
            .context("Failed to get parent directory")?;
        
        // Try to find modules in current directory
        let mut resolver = ModuleResolver::new();
        let config = ProjectConfig {
            package: PackageConfig {
                name: file.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("hust_temp")
                    .to_string(),
                version: "0.1.0".to_string(),
                entry: file.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("main.hust")
                    .to_string(),
                author: None,
                description: None,
            },
            module_paths: vec![current_dir.to_path_buf()],
            dependencies: HashMap::new(),
        };
        
        let modules = resolver.resolve(file, &config, current_dir)
            .context("Failed to resolve module dependencies")?;
        
        println!("[Hust] Found {} module(s)", modules.len());
        
        let translator = Translator::default();
        let entry_module = modules.last()
            .context("No entry module found")?;
        translator.transpile_modules(&modules, entry_module)
            .context("Transpilation failed")?
    } else {
        // No use statements, simple single file transpilation
        let translator = Translator::default();
        translator.transpile(&source)
            .context("Transpilation failed")?
    };

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

    // 5. Call cargo build/check with output to dist
    println!("[Hust] Compiling...");
    let mut cmd = Command::new("cargo");
    cmd.arg(if check_only { "check" } else { "build" })
        .arg("--target-dir").arg(&dist_dir)
        .current_dir(&temp_dir);
    if release {
        cmd.arg("--release");
    }
    let status = cmd.status()
        .context("Failed to invoke cargo")?;

    if !status.success() {
        anyhow::bail!("{} failed", if check_only { "Check" } else { "Compilation" });
    }

    if check_only {
        return Ok(None);
    }

    // 6. Locate the compiled executable
    let profile = if release { "release" } else { "debug" };
    let exe_name = if cfg!(windows) {
        format!("{}.exe", src_stem)
    } else {
        src_stem.to_string()
    };
    Ok(Some(dist_dir.join(profile).join(exe_name)))
}

/// Write transpiled project Rust code into a temp crate next to hust.exe
/// and run cargo build (or cargo check).
/// Returns the compiled executable path, or Ok(None) in check mode.
fn compile_project_rust(config: &ProjectConfig, rust_code: &str, check_only: bool)
    -> anyhow::Result<Option<PathBuf>>
{
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
    std::fs::write(&rs_file, rust_code)
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
"#, config.package.name, config.package.cargo_version());
    std::fs::write(temp_dir.join("Cargo.toml"), cargo_toml)?;

    // 8. Call cargo build/check with output to dist
    println!("[Hust] Compiling...");
    let status = Command::new("cargo")
        .arg(if check_only { "check" } else { "build" })
        .arg("--release")
        .arg("--target-dir")
        .arg(&dist_dir)
        .current_dir(&temp_dir)
        .status()
        .context("Failed to invoke cargo")?;

    if !status.success() {
        anyhow::bail!("{} failed", if check_only { "Check" } else { "Compilation" });
    }

    if check_only {
        return Ok(None);
    }

    // 9. Locate the compiled executable
    let exe_name = if cfg!(windows) {
        format!("{}.exe", config.package.name)
    } else {
        config.package.name.clone()
    };
    Ok(Some(dist_dir.join("release").join(exe_name)))
}

/// Build a Hust project with module support
fn build_project(project_dir: &Path) -> anyhow::Result<()> {
    // 1. Load project configuration
    let config = ProjectConfig::load(project_dir)
        .context("Failed to load project configuration")?;

    // 2. Find entry file
    let entry_file = project_dir.join(&config.package.entry);
    if !entry_file.exists() {
        anyhow::bail!("Entry file '{}' not found in project directory", config.package.entry);
    }

    // 3. Resolve all module dependencies
    println!("[Hust] Resolving modules...");
    let mut resolver = ModuleResolver::new();
    let modules = resolver.resolve(&entry_file, &config, project_dir)
        .context("Failed to resolve module dependencies")?;

    println!("[Hust] Found {} module(s)", modules.len());

    // 4. Transpile all modules into single Rust file
    let translator = Translator::default();
    let entry_module = modules.last()
        .context("No entry module found")?;
    let rust_code = translator.transpile_modules(&modules, entry_module)
        .context("Transpilation failed")?;

    // 5. Compile the transpiled project and copy executable to project directory
    let exe_path = compile_project_rust(&config, &rust_code, false)?
        .context("Internal error: build mode returned no executable")?;
    let output_exe = project_dir.join(&config.package.name);
    std::fs::copy(&exe_path, &output_exe)
        .context("Failed to copy executable")?;

    println!("[Hust] Build complete: {:?}", output_exe);
    Ok(())
}
