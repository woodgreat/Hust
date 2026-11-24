//! User-facing command orchestration.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};

use crate::cli::{Action, BuildMode, Command as UserCommand, CoreChoice, Options, HELP};
use crate::core::{self, CoreKind, CoreOp, DiscoveredCore, Discovery, SearchSpec};
use crate::error::{Error, Result, EXIT_BUILD_FAILED, EXIT_RUN_FAILED, EXIT_TRANSPILE_FAILED};
use crate::target::{self, ResolvedTarget};
use crate::tempdir::TempDir;
use crate::{PROTOCOL, VERSION};

pub fn run<I>(args: I) -> i32
where
    I: IntoIterator<Item = OsString>,
{
    match dispatch(args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("Error: {}", error.message);
            error.code
        }
    }
}

fn dispatch<I>(args: I) -> Result<i32>
where
    I: IntoIterator<Item = OsString>,
{
    match crate::cli::parse(args)? {
        Action::Help => {
            print!("{}", HELP);
            Ok(0)
        }
        Action::Version => {
            print_version(&core::discover(&SearchSpec::from_env()));
            Ok(0)
        }
        Action::ListCores => {
            print_cores(&core::discover(&SearchSpec::from_env()));
            Ok(0)
        }
        Action::Execute(options) => execute(options),
    }
}

fn execute(options: Options) -> Result<i32> {
    let resolved = target::resolve(&options.target, options.project)?;
    let discovery = core::discover(&SearchSpec::from_env());

    let op = match options.command {
        UserCommand::Check => CoreOp::Transpile,
        UserCommand::Run | UserCommand::Build => CoreOp::Build,
    };
    let explicit = options.core.map(|choice| match choice {
        CoreChoice::Rust => CoreKind::Rust,
        CoreChoice::C => CoreKind::C,
    });
    let selected = core::select(explicit, resolved.config.core, &discovery, op)?;

    match options.command {
        UserCommand::Check => check(&options, &resolved, selected),
        UserCommand::Build => build(&options, &resolved, selected),
        UserCommand::Run => run_target(&options, &resolved, selected),
    }
}

fn check(options: &Options, resolved: &ResolvedTarget, core: &DiscoveredCore) -> Result<i32> {
    let temp = TempDir::create(options.keep_temp)?;
    let stdout = invoke(core, CoreOp::Transpile, resolved, options.mode, temp.path())?;
    forward_core_stdout(&stdout);
    Ok(0)
}

fn build(options: &Options, resolved: &ResolvedTarget, core: &DiscoveredCore) -> Result<i32> {
    let temp = TempDir::create(options.keep_temp)?;
    let stdout = invoke(core, CoreOp::Build, resolved, options.mode, temp.path())?;
    let executable = parse_executable(&stdout)?;
    let destination = install_executable(&executable, options.output.as_deref(), &resolved.root)?;
    println!("executable={}", destination.display());
    Ok(0)
}

fn run_target(options: &Options, resolved: &ResolvedTarget, core: &DiscoveredCore) -> Result<i32> {
    let temp = TempDir::create(options.keep_temp)?;
    let stdout = invoke(core, CoreOp::Build, resolved, options.mode, temp.path())?;
    let executable = parse_executable(&stdout)?;

    let status = Command::new(&executable)
        .args(&options.program_args)
        .current_dir(&resolved.root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|error| Error::run(format!("cannot run `{}`: {}", executable.display(), error)))?;
    Ok(exit_code(&status))
}

fn invoke(
    core: &DiscoveredCore,
    op: CoreOp,
    resolved: &ResolvedTarget,
    mode: BuildMode,
    out_dir: &Path,
) -> Result<String> {
    let mut command = Command::new(&core.path);
    command.arg(op.label());
    command.arg(&resolved.entry);
    if resolved.project {
        command.arg("--project");
    }
    command.arg("--out-dir").arg(out_dir);
    if op == CoreOp::Build {
        command.arg(mode_flag(mode));
    }
    command.current_dir(&resolved.root);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());

    let output = command.output().map_err(|error| {
        Error::unavailable(format!(
            "cannot execute `{}`: {}",
            core.path.display(),
            error
        ))
    })?;

    if !output.status.success() {
        let code = match op {
            CoreOp::Transpile => EXIT_TRANSPILE_FAILED,
            CoreOp::Build => EXIT_BUILD_FAILED,
        };
        return Err(Error::new(
            code,
            format!(
                "{} core `{}` failed ({}).",
                core.kind.label(),
                core.path.display(),
                core::status_label(&output.status)
            ),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn mode_flag(mode: BuildMode) -> &'static str {
    match mode {
        BuildMode::Debug => "--debug",
        BuildMode::Release => "--release",
    }
}

fn parse_executable(stdout: &str) -> Result<PathBuf> {
    let mut reported: Option<&str> = None;
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(path) = trimmed.strip_prefix("executable=") {
            reported = Some(path.trim());
        } else {
            // The contract reserves stdout for machine-readable lines. Forward
            // anything else to stderr so it does not corrupt our own output.
            eprintln!("{}", line);
        }
    }

    let raw = reported.ok_or_else(|| Error::build("core did not report `executable=<path>`."))?;
    if raw.is_empty() {
        return Err(Error::build("core reported an empty executable path."));
    }

    let path = PathBuf::from(raw);
    if !path.is_absolute() {
        return Err(Error::build(format!(
            "core reported a relative executable path `{}`.",
            path.display()
        )));
    }
    if !path.is_file() {
        return Err(Error::build(format!(
            "reported executable does not exist: {}",
            path.display()
        )));
    }
    Ok(path)
}

fn install_executable(
    executable: &Path,
    output: Option<&Path>,
    project_root: &Path,
) -> Result<PathBuf> {
    let file_name = executable
        .file_name()
        .ok_or_else(|| Error::build("executable path has no file name."))?;

    let destination = match output {
        Some(output) => {
            let absolute = if output.is_absolute() {
                output.to_path_buf()
            } else {
                std::env::current_dir()
                    .map_err(|error| {
                        Error::build(format!("cannot read the current directory: {}", error))
                    })?
                    .join(output)
            };
            if absolute.is_dir()
                || output
                    .to_string_lossy()
                    .ends_with(std::path::MAIN_SEPARATOR)
            {
                fs::create_dir_all(&absolute).map_err(|error| {
                    Error::build(format!(
                        "cannot create output directory `{}`: {}",
                        absolute.display(),
                        error
                    ))
                })?;
                absolute.join(file_name)
            } else {
                if let Some(parent) = absolute.parent() {
                    if !parent.as_os_str().is_empty() {
                        fs::create_dir_all(parent).map_err(|error| {
                            Error::build(format!(
                                "cannot create output directory `{}`: {}",
                                parent.display(),
                                error
                            ))
                        })?;
                    }
                }
                absolute
            }
        }
        None => project_root.join(file_name),
    };

    if !same_file(executable, &destination) {
        fs::copy(executable, &destination).map_err(|error| {
            Error::build(format!(
                "cannot install executable to `{}`: {}",
                destination.display(),
                error
            ))
        })?;
    }
    make_executable(&destination)?;

    fs::canonicalize(&destination).map_err(|error| {
        Error::build(format!(
            "cannot resolve installed executable `{}`: {}",
            destination.display(),
            error
        ))
    })
}

fn same_file(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)
        .map_err(|error| {
            Error::build(format!(
                "cannot inspect executable `{}`: {}",
                path.display(),
                error
            ))
        })?
        .permissions();
    permissions.set_mode(permissions.mode() | 0o111);
    fs::set_permissions(path, permissions).map_err(|error| {
        Error::build(format!(
            "cannot mark `{}` as executable: {}",
            path.display(),
            error
        ))
    })
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

fn exit_code(status: &ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }
    EXIT_RUN_FAILED
}

fn forward_core_stdout(stdout: &str) {
    for line in stdout.lines() {
        if !line.trim().is_empty() {
            eprintln!("{}", line);
        }
    }
}

fn print_cores(discovery: &Discovery) {
    for kind in CoreKind::ALL {
        match discovery.get(kind) {
            Some(core) => {
                println!("{}: available", kind.label());
                println!("  path: {}", core.path.display());
                println!("  version: {}", core.display_version());
                println!("  language: {}", core.info.language);
                println!("  protocol: {}", core.info.protocol);
                println!("  capabilities:{}", capabilities(core));
                println!("  toolchain: available");
            }
            None => {
                println!("{}: unavailable", kind.label());
                if let Some(failure) = discovery.failure(kind) {
                    println!("  reason: {}", failure.reason);
                }
            }
        }
    }
}

fn capabilities(core: &DiscoveredCore) -> String {
    let mut capabilities = Vec::new();
    if core.info.transpile {
        capabilities.push("transpile");
    }
    if core.info.build {
        capabilities.push("build");
    }
    if capabilities.is_empty() {
        " none".to_string()
    } else {
        format!(" {}", capabilities.join(" "))
    }
}

fn print_version(discovery: &Discovery) {
    println!("hustc {} (protocol {})", VERSION, PROTOCOL);
    for kind in CoreKind::ALL {
        match discovery.get(kind) {
            Some(core) => println!(
                "{}: {} ({})",
                kind.label(),
                core.display_version(),
                core.path.display()
            ),
            None => println!("{}: unavailable", kind.label()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "hustc-app-{}-{}-{}",
                name,
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn parses_executable_line_and_ignores_logs() {
        let dir = TestDir::new("executable");
        let executable = dir.path().join("app");
        fs::write(&executable, "binary").unwrap();

        let stdout = format!("building\nexecutable={}\n", executable.display());
        assert_eq!(parse_executable(&stdout).unwrap(), executable);
    }

    #[test]
    fn takes_the_last_executable_line() {
        let dir = TestDir::new("last-executable");
        let first = dir.path().join("first");
        let second = dir.path().join("second");
        fs::write(&first, "binary").unwrap();
        fs::write(&second, "binary").unwrap();

        let stdout = format!(
            "executable={}\nexecutable={}\n",
            first.display(),
            second.display()
        );
        assert_eq!(parse_executable(&stdout).unwrap(), second);
    }

    #[test]
    fn rejects_missing_or_relative_executable() {
        assert_eq!(
            parse_executable("building\n").unwrap_err().code,
            EXIT_BUILD_FAILED
        );
        assert_eq!(
            parse_executable("executable=app\n").unwrap_err().code,
            EXIT_BUILD_FAILED
        );
    }

    #[test]
    fn installs_next_to_the_project_by_default() {
        let dir = TestDir::new("install-default");
        let source = dir.path().join("source-app");
        fs::write(&source, "binary").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&source, fs::Permissions::from_mode(0o755)).unwrap();
        }

        let installed = install_executable(&source, None, dir.path()).unwrap();
        assert!(installed.is_file());
        assert_eq!(installed.file_name().unwrap(), "source-app");
        assert_eq!(
            installed.parent().unwrap(),
            fs::canonicalize(dir.path()).unwrap()
        );
    }

    #[test]
    fn output_path_can_be_a_file_or_directory() {
        let dir = TestDir::new("install-output");
        let source = dir.path().join("app");
        fs::write(&source, "binary").unwrap();

        let file =
            install_executable(&source, Some(&dir.path().join("renamed")), dir.path()).unwrap();
        assert_eq!(file.file_name().unwrap(), "renamed");

        let out_dir = dir.path().join("bin");
        fs::create_dir(&out_dir).unwrap();
        let in_dir = install_executable(&source, Some(&out_dir), dir.path()).unwrap();
        assert_eq!(in_dir, fs::canonicalize(out_dir.join("app")).unwrap());
    }
}
