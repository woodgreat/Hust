//! Language core discovery, probing, and selection.
//!
//! Discovery order (2026-11-25 updated):
//! 1. `hustc同目录/hust_work/hust_core/<core_name>/<core_name>` (new default)
//! 2. `hustc同目录/<core_name>` (legacy fallback)
//! 3. `PATH`
//! 4. `HUST_RUST_BIN` / `HUST_C_BIN` override
//!
//! The first usable candidate wins. A core is usable when `info` succeeds,
//! speaks the conductor protocol, and reports an available target toolchain.

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use crate::config::CorePreference;
use crate::error::{Error, Result};
use crate::json::{self, Json};
use crate::PROTOCOL;

pub const EXE_SUFFIX: &str = if cfg!(windows) { ".exe" } else { "" };

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreKind {
    Rust,
    C,
}

impl CoreKind {
    pub const ALL: [CoreKind; 2] = [CoreKind::Rust, CoreKind::C];

    pub fn label(self) -> &'static str {
        match self {
            CoreKind::Rust => "rust",
            CoreKind::C => "c",
        }
    }

    pub fn binary_name(self) -> &'static str {
        match self {
            CoreKind::Rust => "hust_rust",
            CoreKind::C => "hust_c",
        }
    }

    pub fn env_var(self) -> &'static str {
        match self {
            CoreKind::Rust => "HUST_RUST_BIN",
            CoreKind::C => "HUST_C_BIN",
        }
    }

    pub fn from_preference(preference: CorePreference) -> Option<Self> {
        match preference {
            CorePreference::Rust => Some(CoreKind::Rust),
            CorePreference::C => Some(CoreKind::C),
            CorePreference::Auto => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreOp {
    Transpile,
    Build,
}

impl CoreOp {
    pub fn label(self) -> &'static str {
        match self {
            CoreOp::Transpile => "transpile",
            CoreOp::Build => "build",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreInfo {
    pub protocol: u32,
    pub name: String,
    pub language: String,
    pub transpile: bool,
    pub build: bool,
    /// Optional protocol extension. Absent means the core claims its target
    /// toolchain is available.
    pub toolchain: bool,
}

impl CoreInfo {
    pub fn from_json(value: &Json) -> std::result::Result<Self, String> {
        let protocol = value
            .get("protocol")
            .and_then(Json::as_u32)
            .ok_or("missing integer field `protocol`")?;
        let name = required_string(value, "name")?;
        let language = required_string(value, "language")?;
        let transpile = required_bool(value, "transpile")?;
        let build = required_bool(value, "build")?;
        let toolchain = match value.get("toolchain") {
            Some(field) => field
                .as_bool()
                .ok_or("field `toolchain` must be a boolean")?,
            None => true,
        };
        Ok(Self {
            protocol,
            name,
            language,
            transpile,
            build,
            toolchain,
        })
    }

    pub fn parse_line(line: &str) -> std::result::Result<Self, String> {
        let value = json::parse(line.trim())?;
        Self::from_json(&value)
    }

    pub fn supports(&self, op: CoreOp) -> bool {
        match op {
            CoreOp::Transpile => self.transpile,
            CoreOp::Build => self.build,
        }
    }
}

fn required_string(value: &Json, field: &str) -> std::result::Result<String, String> {
    value
        .get(field)
        .and_then(Json::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("missing string field `{}`", field))
}

fn required_bool(value: &Json, field: &str) -> std::result::Result<bool, String> {
    value
        .get(field)
        .and_then(Json::as_bool)
        .ok_or_else(|| format!("missing boolean field `{}`", field))
}

#[derive(Debug, Clone)]
pub struct DiscoveredCore {
    pub kind: CoreKind,
    pub path: PathBuf,
    pub info: CoreInfo,
    pub version: Option<String>,
}

impl DiscoveredCore {
    pub fn supports(&self, op: CoreOp) -> bool {
        self.info.supports(op)
    }

    pub fn display_version(&self) -> String {
        self.version
            .clone()
            .unwrap_or_else(|| "version unknown".to_string())
    }
}

#[derive(Debug, Clone)]
pub struct CoreFailure {
    pub kind: CoreKind,
    pub reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct Discovery {
    pub available: Vec<DiscoveredCore>,
    pub failures: Vec<CoreFailure>,
}

impl Discovery {
    pub fn get(&self, kind: CoreKind) -> Option<&DiscoveredCore> {
        self.available.iter().find(|core| core.kind == kind)
    }

    pub fn failure(&self, kind: CoreKind) -> Option<&CoreFailure> {
        self.failures.iter().find(|failure| failure.kind == kind)
    }
}

/// Environment used for discovery. Kept explicit so tests can exercise the
/// search order without mutating process-global variables.
#[derive(Debug, Clone, Default)]
pub struct SearchSpec {
    pub exe_dir: Option<PathBuf>,
    pub path_var: Option<OsString>,
    pub rust_bin: Option<OsString>,
    pub c_bin: Option<OsString>,
}

impl SearchSpec {
    pub fn from_env() -> Self {
        let exe_dir = env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf));
        Self {
            exe_dir,
            path_var: env::var_os("PATH"),
            rust_bin: env::var_os("HUST_RUST_BIN"),
            c_bin: env::var_os("HUST_C_BIN"),
        }
    }

    fn env_override(&self, kind: CoreKind) -> Option<&OsString> {
        match kind {
            CoreKind::Rust => self.rust_bin.as_ref(),
            CoreKind::C => self.c_bin.as_ref(),
        }
    }

    /// Candidate paths in priority order, without duplicates.
    ///
    /// 2026-11-25: New default location is `hust_work/hust_core/<core>/<core>`,
    /// with legacy fallback to same-directory lookup.
    pub fn candidates(&self, kind: CoreKind) -> Vec<PathBuf> {
        let file_name = format!("{}{}", kind.binary_name(), EXE_SUFFIX);
        let mut candidates = Vec::new();

        if let Some(dir) = &self.exe_dir {
            // New default: hust_work/hust_core/<core_name>/<core_name>
            let hust_work_path = dir
                .join("hust_work")
                .join("hust_core")
                .join(kind.binary_name())
                .join(&file_name);
            candidates.push(hust_work_path);

            // Legacy fallback: same directory as hustc
            candidates.push(dir.join(&file_name));
        }
        if let Some(path_var) = &self.path_var {
            for dir in env::split_paths(path_var) {
                if dir.as_os_str().is_empty() {
                    continue;
                }
                candidates.push(dir.join(&file_name));
            }
        }
        if let Some(override_path) = self.env_override(kind) {
            if !override_path.is_empty() {
                candidates.push(PathBuf::from(override_path));
            }
        }

        let mut unique = Vec::new();
        for candidate in candidates {
            if !unique.contains(&candidate) {
                unique.push(candidate);
            }
        }
        unique
    }
}

pub fn discover(search: &SearchSpec) -> Discovery {
    let mut discovery = Discovery::default();
    for kind in CoreKind::ALL {
        match discover_kind(kind, search) {
            Ok(core) => discovery.available.push(core),
            Err(reason) => discovery.failures.push(CoreFailure { kind, reason }),
        }
    }
    discovery
}

fn discover_kind(
    kind: CoreKind,
    search: &SearchSpec,
) -> std::result::Result<DiscoveredCore, String> {
    let candidates = search.candidates(kind);
    let existing: Vec<&PathBuf> = candidates.iter().filter(|path| path.is_file()).collect();

    if existing.is_empty() {
        return Err(format!(
            "{} not found (searched hust_work/hust_core/{0}/, executable directory, PATH, and {})",
            kind.binary_name(),
            kind.env_var()
        ));
    }

    let mut reasons = Vec::new();
    for candidate in existing {
        match probe(candidate, kind) {
            Ok((info, version)) => {
                return Ok(DiscoveredCore {
                    kind,
                    path: candidate.clone(),
                    info,
                    version,
                })
            }
            Err(reason) => reasons.push(format!("{}: {}", candidate.display(), reason)),
        }
    }

    if reasons.len() == 1 {
        Err(reasons.remove(0))
    } else {
        Err(format!(
            "no usable {} found ({})",
            kind.binary_name(),
            reasons.join("; ")
        ))
    }
}

fn probe(path: &Path, kind: CoreKind) -> std::result::Result<(CoreInfo, Option<String>), String> {
    let info_output = run_capture(path, &["info"])?;
    if !info_output.status.success() {
        return Err(format!(
            "`info` failed ({})",
            status_label(&info_output.status)
        ));
    }
    let stdout = String::from_utf8_lossy(&info_output.stdout);
    let line = stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or("`info` produced no output")?;
    let info = CoreInfo::parse_line(line).map_err(|error| format!("invalid `info`: {}", error))?;

    if info.protocol != PROTOCOL {
        return Err(format!(
            "unsupported protocol {} (expected {})",
            info.protocol, PROTOCOL
        ));
    }
    if info.name != kind.label() {
        return Err(format!(
            "`info` reports core `{}` (expected `{}`)",
            info.name,
            kind.label()
        ));
    }
    if !info.transpile && !info.build {
        return Err("core reports no capabilities".to_string());
    }
    if !info.toolchain {
        return Err("target toolchain is not available".to_string());
    }

    Ok((info, run_version(path)))
}

fn run_capture(path: &Path, args: &[&str]) -> std::result::Result<Output, String> {
    Command::new(path)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("cannot execute: {}", error))
}

fn run_version(path: &Path) -> Option<String> {
    let output = run_capture(path, &["--version"]).ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

pub fn status_label(status: &std::process::ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("exit code {}", code),
        None => "terminated by signal".to_string(),
    }
}

/// Select exactly one core. An explicit choice (CLI first, then config) never
/// falls back; `auto` tries Rust, then C, by availability only.
pub fn select(
    explicit: Option<CoreKind>,
    preference: CorePreference,
    discovery: &Discovery,
    op: CoreOp,
) -> Result<&DiscoveredCore> {
    let requested = explicit.or_else(|| CoreKind::from_preference(preference));

    if let Some(kind) = requested {
        let core = discovery.get(kind).ok_or_else(|| {
            let reason = discovery
                .failure(kind)
                .map(|failure| failure.reason.clone())
                .unwrap_or_else(|| "core is not available".to_string());
            explicit_error(kind, &reason)
        })?;
        if !core.supports(op) {
            return Err(Error::unavailable(format!(
                "{} core does not support `{}`.\n  Core: {}\n  Fix: install a newer core or choose another core.",
                kind.label(),
                op.label(),
                core.path.display()
            )));
        }
        return Ok(core);
    }

    for kind in CoreKind::ALL {
        if let Some(core) = discovery.get(kind) {
            if core.supports(op) {
                return Ok(core);
            }
        }
    }

    Err(no_core_error(discovery, op))
}

fn explicit_error(kind: CoreKind, reason: &str) -> Error {
    Error::unavailable(format!(
        "{} core is unavailable: {}.\n  Fix: install {}, set {}, or choose another core.",
        kind.label(),
        reason,
        kind.binary_name(),
        kind.env_var()
    ))
}

fn no_core_error(discovery: &Discovery, op: CoreOp) -> Error {
    let mut lines = vec![format!(
        "no usable language core found for `{}`.",
        op.label()
    )];
    for kind in CoreKind::ALL {
        match discovery.get(kind) {
            Some(core) if !core.supports(op) => lines.push(format!(
                "  {}: {} does not support `{}`",
                kind.label(),
                core.path.display(),
                op.label()
            )),
            Some(core) => lines.push(format!(
                "  {}: available at {}",
                kind.label(),
                core.path.display()
            )),
            None => {
                let reason = discovery
                    .failure(kind)
                    .map(|failure| failure.reason.clone())
                    .unwrap_or_else(|| "not available".to_string());
                lines.push(format!("  {}: {}", kind.label(), reason));
            }
        }
    }
    lines.push("  Fix: install a core or pass --core-rust/--core-c.".to_string());
    Error::unavailable(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn core(kind: CoreKind, transpile: bool, build: bool) -> DiscoveredCore {
        DiscoveredCore {
            kind,
            path: PathBuf::from(format!("/fake/{}", kind.binary_name())),
            info: CoreInfo {
                protocol: PROTOCOL,
                name: kind.label().to_string(),
                language: kind.label().to_string(),
                transpile,
                build,
                toolchain: true,
            },
            version: None,
        }
    }

    #[test]
    fn parses_protocol_json() {
        let info = CoreInfo::parse_line(
            r#"{"protocol":1,"name":"rust","language":"rust","transpile":true,"build":true}"#,
        )
        .unwrap();
        assert_eq!(info.protocol, 1);
        assert_eq!(info.name, "rust");
        assert!(info.transpile);
        assert!(info.build);
        assert!(info.toolchain);
    }

    #[test]
    fn toolchain_defaults_to_true_but_can_be_disabled() {
        let info = CoreInfo::parse_line(
            r#"{"protocol":1,"name":"c","language":"c","transpile":true,"build":false,"toolchain":false}"#,
        )
        .unwrap();
        assert!(!info.toolchain);
        assert!(!info.build);
    }

    #[test]
    fn rejects_incomplete_info() {
        assert!(CoreInfo::parse_line(r#"{"protocol":1,"name":"rust"}"#).is_err());
        assert!(CoreInfo::parse_line(
            r#"{"protocol":1,"name":"rust","language":"rust","transpile":"yes","build":true}"#
        )
        .is_err());
        assert!(CoreInfo::parse_line(r#"{"protocol":1,"name":"rust","language":"rust","transpile":true,"build":true,"toolchain":"yes"}"#).is_err());
    }

    #[test]
    fn candidates_follow_discovery_order() {
        let spec = SearchSpec {
            exe_dir: Some(PathBuf::from("/exe")),
            path_var: Some(OsString::from("/one:/two")),
            rust_bin: Some(OsString::from("/custom/rust")),
            c_bin: Some(OsString::from("/custom/c")),
        };
        let rust = spec.candidates(CoreKind::Rust);
        // New default: hust_work/hust_core/hust_rust/hust_rust
        assert_eq!(rust[0], PathBuf::from("/exe/hust_work/hust_core/hust_rust/hust_rust"));
        // Legacy fallback: same directory as hustc
        assert_eq!(rust[1], PathBuf::from("/exe/hust_rust"));
        assert_eq!(rust[2], PathBuf::from("/one/hust_rust"));
        assert_eq!(rust[3], PathBuf::from("/two/hust_rust"));
        assert_eq!(rust[4], PathBuf::from("/custom/rust"));

        let c = spec.candidates(CoreKind::C);
        // New default: hust_work/hust_core/hust_c/hust_c
        assert_eq!(c[0], PathBuf::from("/exe/hust_work/hust_core/hust_c/hust_c"));
        // Legacy fallback: same directory as hustc
        assert_eq!(c[1], PathBuf::from("/exe/hust_c"));
        assert_eq!(c.last().unwrap(), &PathBuf::from("/custom/c"));
    }

    #[test]
    fn candidates_are_deduplicated() {
        let spec = SearchSpec {
            exe_dir: Some(PathBuf::from("/one")),
            path_var: Some(OsString::from("/one:/one")),
            rust_bin: None,
            c_bin: None,
        };
        // hust_work path + legacy path = 2 unique candidates (PATH duplicates removed)
        assert_eq!(spec.candidates(CoreKind::Rust).len(), 2);
    }

    #[test]
    fn auto_prefers_rust() {
        let discovery = Discovery {
            available: vec![
                core(CoreKind::Rust, true, true),
                core(CoreKind::C, true, true),
            ],
            failures: vec![],
        };
        let selected = select(None, CorePreference::Auto, &discovery, CoreOp::Build).unwrap();
        assert_eq!(selected.kind, CoreKind::Rust);
    }

    #[test]
    fn auto_falls_back_to_c() {
        let discovery = Discovery {
            available: vec![core(CoreKind::C, true, true)],
            failures: vec![CoreFailure {
                kind: CoreKind::Rust,
                reason: "hust_rust not found".to_string(),
            }],
        };
        let selected = select(None, CorePreference::Auto, &discovery, CoreOp::Build).unwrap();
        assert_eq!(selected.kind, CoreKind::C);
    }

    #[test]
    fn explicit_missing_core_does_not_fall_back() {
        let discovery = Discovery {
            available: vec![core(CoreKind::Rust, true, true)],
            failures: vec![CoreFailure {
                kind: CoreKind::C,
                reason: "hust_c not found".to_string(),
            }],
        };
        let error = select(
            Some(CoreKind::C),
            CorePreference::Auto,
            &discovery,
            CoreOp::Build,
        )
        .unwrap_err();
        assert_eq!(error.code, crate::error::EXIT_CORE_UNAVAILABLE);
        assert!(error.message.contains("c core is unavailable"));
        assert!(error.message.contains("hust_c not found"));
    }

    #[test]
    fn config_preference_is_used_when_cli_is_silent() {
        let discovery = Discovery {
            available: vec![
                core(CoreKind::Rust, true, true),
                core(CoreKind::C, true, true),
            ],
            failures: vec![],
        };
        let selected = select(None, CorePreference::C, &discovery, CoreOp::Build).unwrap();
        assert_eq!(selected.kind, CoreKind::C);
    }

    #[test]
    fn cli_choice_overrides_config_preference() {
        let discovery = Discovery {
            available: vec![
                core(CoreKind::Rust, true, true),
                core(CoreKind::C, true, true),
            ],
            failures: vec![],
        };
        let selected = select(
            Some(CoreKind::Rust),
            CorePreference::C,
            &discovery,
            CoreOp::Build,
        )
        .unwrap();
        assert_eq!(selected.kind, CoreKind::Rust);
    }

    #[test]
    fn missing_capability_is_reported() {
        let discovery = Discovery {
            available: vec![core(CoreKind::Rust, true, false)],
            failures: vec![],
        };
        let error = select(None, CorePreference::Auto, &discovery, CoreOp::Build).unwrap_err();
        assert!(error.message.contains("no usable language core"));
    }

    #[test]
    fn auto_skips_core_without_the_needed_capability() {
        let discovery = Discovery {
            available: vec![
                core(CoreKind::Rust, false, false),
                core(CoreKind::C, true, true),
            ],
            failures: vec![],
        };
        let selected = select(None, CorePreference::Auto, &discovery, CoreOp::Build).unwrap();
        assert_eq!(selected.kind, CoreKind::C);
    }
}
