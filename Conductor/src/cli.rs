//! Command-line parsing. Hand-rolled so the conductor stays dependency-free.

use std::ffi::OsString;
use std::path::PathBuf;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Run,
    Build,
    Check,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreChoice {
    Rust,
    C,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildMode {
    Debug,
    Release,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    pub command: Command,
    pub target: PathBuf,
    pub project: bool,
    pub core: Option<CoreChoice>,
    pub mode: BuildMode,
    pub output: Option<PathBuf>,
    pub keep_temp: bool,
    pub program_args: Vec<OsString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Help,
    Version,
    ListCores,
    Execute(Options),
}

pub const HELP: &str = "\
Usage:
  hustc run   <target> [options] [-- <program args>...]
  hustc build <target> [options]
  hustc check <target> [options]
  hustc --cores
  hustc --version
  hustc --help

Commands:
  run                  Build and run the target
  build                Build the target
  check                Transpile without invoking the target compiler

Options:
  --project            Treat the target as a project
  --core-rust          Use the Rust core
  --core-c             Use the C core
  --release            Build in release mode (default for build)
  --debug              Build in debug mode (default for run)
  -o, --output <path>  Output path for the built executable (build only)
  --keep-temp          Keep the temporary build directory
";

pub fn parse<I>(args: I) -> Result<Action>
where
    I: IntoIterator<Item = OsString>,
{
    let raw: Vec<OsString> = args.into_iter().collect();

    let dash_position = raw.iter().position(|arg| is(arg, "--"));
    let before_dash = dash_position
        .map(|position| &raw[..position])
        .unwrap_or(&raw[..]);

    if before_dash
        .iter()
        .any(|arg| is(arg, "--help") || is(arg, "-h"))
    {
        return Ok(Action::Help);
    }
    if before_dash
        .iter()
        .any(|arg| is(arg, "--version") || is(arg, "-V"))
    {
        if raw.len() == 1 {
            return Ok(Action::Version);
        }
        return Err(Error::usage("`--version` must be used alone"));
    }
    if before_dash.iter().any(|arg| is(arg, "--cores")) {
        if raw.len() == 1 {
            return Ok(Action::ListCores);
        }
        return Err(Error::usage("`--cores` must be used alone"));
    }

    let first = raw
        .first()
        .ok_or_else(|| Error::usage(format!("missing command.\n\n{}", HELP)))?;
    let command = match first.to_str() {
        Some("run") => Command::Run,
        Some("build") => Command::Build,
        Some("check") => Command::Check,
        Some(text) if text.starts_with('-') => {
            return Err(Error::usage(format!(
                "unknown option `{}`.\n\n{}",
                text, HELP
            )))
        }
        _ => {
            return Err(Error::usage(format!(
                "unknown command `{}`.\n\n{}",
                first.to_string_lossy(),
                HELP
            )))
        }
    };

    let mut target: Option<PathBuf> = None;
    let mut project = false;
    let mut core: Option<CoreChoice> = None;
    let mut mode: Option<BuildMode> = None;
    let mut output: Option<PathBuf> = None;
    let mut keep_temp = false;
    let mut program_args: Vec<OsString> = Vec::new();

    let mut index = 1;
    while index < raw.len() {
        let arg = &raw[index];

        if is(arg, "--") {
            if command != Command::Run {
                return Err(Error::usage(
                    "program arguments after `--` are only valid for `run`",
                ));
            }
            program_args.extend(raw[index + 1..].iter().cloned());
            break;
        }

        match arg.to_str() {
            Some("--project") => project = true,
            Some("--core-rust") => set_core(&mut core, CoreChoice::Rust)?,
            Some("--core-c") => set_core(&mut core, CoreChoice::C)?,
            Some("--release") => set_mode(&mut mode, BuildMode::Release)?,
            Some("--debug") => set_mode(&mut mode, BuildMode::Debug)?,
            Some("--keep-temp") => keep_temp = true,
            Some("-o") | Some("--output") => {
                index += 1;
                let value = raw
                    .get(index)
                    .ok_or_else(|| Error::usage("`--output` requires a path"))?;
                output = Some(PathBuf::from(value));
            }
            Some(text) if text.starts_with("--output=") => {
                let value = &text["--output=".len()..];
                if value.is_empty() {
                    return Err(Error::usage("`--output` requires a path"));
                }
                output = Some(PathBuf::from(value));
            }
            Some(text) if text.starts_with('-') && text != "-" => {
                return Err(Error::usage(format!("unknown option `{}`.", text)))
            }
            _ => {
                if target.is_some() {
                    return Err(Error::usage(format!(
                        "unexpected argument `{}`.",
                        arg.to_string_lossy()
                    )));
                }
                target = Some(PathBuf::from(arg));
            }
        }
        index += 1;
    }

    let target = target.ok_or_else(|| {
        Error::usage(format!(
            "missing <target>.\n\nUsage: hustc {} <target> [options]",
            command_name(command)
        ))
    })?;

    if output.is_some() && command != Command::Build {
        return Err(Error::usage("`--output` is only valid for `build`"));
    }
    if command == Command::Check && mode.is_some() {
        return Err(Error::usage(
            "`--release` and `--debug` are not valid for `check`",
        ));
    }

    let mode = mode.unwrap_or(match command {
        Command::Build => BuildMode::Release,
        Command::Run | Command::Check => BuildMode::Debug,
    });

    Ok(Action::Execute(Options {
        command,
        target,
        project,
        core,
        mode,
        output,
        keep_temp,
        program_args,
    }))
}

fn command_name(command: Command) -> &'static str {
    match command {
        Command::Run => "run",
        Command::Build => "build",
        Command::Check => "check",
    }
}

fn set_core(slot: &mut Option<CoreChoice>, value: CoreChoice) -> Result<()> {
    if let Some(existing) = slot {
        if *existing != value {
            return Err(Error::usage(
                "`--core-rust` and `--core-c` cannot be combined",
            ));
        }
    } else {
        *slot = Some(value);
    }
    Ok(())
}

fn set_mode(slot: &mut Option<BuildMode>, value: BuildMode) -> Result<()> {
    if let Some(existing) = slot {
        if *existing != value {
            return Err(Error::usage("`--release` and `--debug` cannot be combined"));
        }
    } else {
        *slot = Some(value);
    }
    Ok(())
}

fn is(arg: &OsString, expected: &str) -> bool {
    arg.to_str() == Some(expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(args: &[&str]) -> Result<Action> {
        parse(args.iter().map(OsString::from))
    }

    fn options(args: &[&str]) -> Options {
        match parse_args(args).unwrap() {
            Action::Execute(options) => options,
            other => panic!("unexpected action: {:?}", other),
        }
    }

    #[test]
    fn parses_run_with_defaults() {
        let options = options(&["run", "main.hust"]);
        assert_eq!(options.command, Command::Run);
        assert_eq!(options.target, PathBuf::from("main.hust"));
        assert_eq!(options.mode, BuildMode::Debug);
        assert!(!options.project);
        assert_eq!(options.core, None);
    }

    #[test]
    fn build_defaults_to_release() {
        assert_eq!(options(&["build", "main.hust"]).mode, BuildMode::Release);
    }

    #[test]
    fn release_overrides_run_default() {
        assert_eq!(
            options(&["run", "main.hust", "--release"]).mode,
            BuildMode::Release
        );
        assert_eq!(
            options(&["build", "main.hust", "--debug"]).mode,
            BuildMode::Debug
        );
    }

    #[test]
    fn parses_core_and_project_flags() {
        let options = options(&["build", "--project", "proj", "--core-c"]);
        assert!(options.project);
        assert_eq!(options.core, Some(CoreChoice::C));
        assert_eq!(options.target, PathBuf::from("proj"));
    }

    #[test]
    fn parses_output_forms() {
        assert_eq!(
            options(&["build", "main.hust", "-o", "bin/app"]).output,
            Some(PathBuf::from("bin/app"))
        );
        assert_eq!(
            options(&["build", "main.hust", "--output=bin/app"]).output,
            Some(PathBuf::from("bin/app"))
        );
    }

    #[test]
    fn passes_arguments_after_double_dash() {
        let options = options(&["run", "main.hust", "--", "--core-c", "x"]);
        assert_eq!(
            options.program_args,
            vec![OsString::from("--core-c"), OsString::from("x")]
        );
    }

    #[test]
    fn rejects_program_args_for_build() {
        let error = parse_args(&["build", "main.hust", "--", "x"]).unwrap_err();
        assert!(error.message.contains("only valid for `run`"));
    }

    #[test]
    fn rejects_conflicting_core_flags() {
        let error = parse_args(&["run", "main.hust", "--core-rust", "--core-c"]).unwrap_err();
        assert!(error.message.contains("cannot be combined"));
    }

    #[test]
    fn rejects_conflicting_mode_flags() {
        let error = parse_args(&["run", "main.hust", "--release", "--debug"]).unwrap_err();
        assert!(error.message.contains("cannot be combined"));
    }

    #[test]
    fn check_rejects_build_only_options() {
        assert!(parse_args(&["check", "main.hust", "--release"]).is_err());
        assert!(parse_args(&["check", "main.hust", "-o", "x"]).is_err());
    }

    #[test]
    fn rejects_missing_target() {
        let error = parse_args(&["run"]).unwrap_err();
        assert!(error.message.contains("missing <target>"));
    }

    #[test]
    fn rejects_unknown_option_and_command() {
        assert!(parse_args(&["run", "main.hust", "--nope"]).is_err());
        assert!(parse_args(&["fly", "main.hust"]).is_err());
    }

    #[test]
    fn recognizes_global_actions() {
        assert_eq!(parse_args(&["--help"]).unwrap(), Action::Help);
        assert_eq!(parse_args(&["run", "--help"]).unwrap(), Action::Help);
        assert_eq!(parse_args(&["--version"]).unwrap(), Action::Version);
        assert_eq!(parse_args(&["--cores"]).unwrap(), Action::ListCores);
        assert!(parse_args(&["--cores", "run"]).is_err());
        assert!(parse_args(&["--version", "x"]).is_err());
    }
}
