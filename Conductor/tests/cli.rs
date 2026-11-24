//! End-to-end tests for `hustc` using fake language cores.
//!
//! Each test gets its own sandbox, fake `hust_rust`/`hust_c` shell scripts, and
//! a restricted `PATH`, so discovery is deterministic and no real core or
//! target toolchain is required.

#![cfg(unix)]

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static COUNTER: AtomicU64 = AtomicU64::new(0);

const CORE_TEMPLATE: &str = r#"#!/bin/sh

log="${FAKE_LOG:-}"
record() {
  if [ -n "$log" ]; then
    printf '%s\n' "$*" >> "$log"
  fi
}

case "${1:-}" in
  --version)
    echo "@NAME@ 9.9.9"
    exit 0
    ;;
  info)
    echo '@INFO@'
    exit 0
    ;;
  transpile)
    shift
    out=""
    entry=""
    project=""
    while [ $# -gt 0 ]; do
      case "$1" in
        --out-dir) out="$2"; shift 2 ;;
        --project) project="--project"; shift ;;
        *) entry="$1"; shift ;;
      esac
    done
    record "transpile entry=$entry out=$out $project"
    if [ @TRANSPILE_EXIT@ -ne 0 ]; then
      echo "simulated transpile error from @NAME@" >&2
      exit @TRANSPILE_EXIT@
    fi
    mkdir -p "$out"
    : > "$out/transpiled.txt"
    echo "transpile log from @NAME@"
    exit 0
    ;;
  build)
    shift
    out=""
    entry=""
    mode=""
    project=""
    while [ $# -gt 0 ]; do
      case "$1" in
        --out-dir) out="$2"; shift 2 ;;
        --release|--debug) mode="$1"; shift ;;
        --project) project="--project"; shift ;;
        *) entry="$1"; shift ;;
      esac
    done
    record "build entry=$entry out=$out mode=$mode $project"
    if [ @BUILD_EXIT@ -ne 0 ]; then
      echo "simulated compile error from @NAME@" >&2
      exit @BUILD_EXIT@
    fi
    mkdir -p "$out"
    cat > "$out/app" <<'HUST_PROGRAM_EOF'
#!/bin/sh
@PROGRAM@
HUST_PROGRAM_EOF
    chmod +x "$out/app"
    echo "compiling on @NAME@"
    echo "executable=$out/app"
    exit 0
    ;;
esac

echo "unknown command from @NAME@" >&2
exit 2
"#;

const PROJECT_TEMPLATE: &str = r#"#!/bin/sh
echo "hello from @NAME@"
printf 'args:'
for arg in "$@"; do
  printf ' %s' "$arg"
done
echo
exit @RUN_EXIT@
"#;

struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "hustc-it-{}-{}-{}",
            name,
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("project")).unwrap();
        fs::create_dir_all(root.join("cores")).unwrap();
        fs::write(root.join("project").join("main.hust"), "fn main() {}\n").unwrap();
        Self { root }
    }

    fn project(&self) -> PathBuf {
        self.root.join("project")
    }

    fn cores(&self) -> PathBuf {
        self.root.join("cores")
    }

    fn log_path(&self) -> PathBuf {
        self.root.join("run.log")
    }

    fn search_path(&self) -> OsString {
        OsString::from(format!("{}:/usr/bin:/bin", self.cores().display()))
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_hustc"));
        command
            .current_dir(self.project())
            .env_remove("HUST_RUST_BIN")
            .env_remove("HUST_C_BIN")
            .env("PATH", self.search_path())
            .env("FAKE_LOG", self.log_path());
        command
    }

    fn install_core(&self, label: &str, settings: FakeCore) -> PathBuf {
        self.install_core_in(&self.cores(), label, settings)
    }

    fn install_core_in(&self, dir: &Path, label: &str, settings: FakeCore) -> PathBuf {
        let name = format!("hust_{}", label);
        let info = format!(
            r#"{{"protocol":{},"name":"{}","language":"{}","transpile":true,"build":true,"toolchain":{}}}"#,
            settings.protocol, label, label, settings.toolchain
        );
        let program = PROJECT_TEMPLATE
            .replace("@NAME@", label)
            .replace("@RUN_EXIT@", &settings.run_exit.to_string());
        let script = CORE_TEMPLATE
            .replace("@NAME@", &name)
            .replace("@INFO@", &info)
            .replace("@BUILD_EXIT@", &settings.build_exit.to_string())
            .replace("@TRANSPILE_EXIT@", &settings.transpile_exit.to_string())
            .replace("@PROGRAM@", &program);

        fs::create_dir_all(dir).unwrap();
        let path = dir.join(&name);
        let staged = dir.join(format!("{}.tmp", name));
        // Write and close before the script becomes visible; executing a file
        // that is still open for writing can fail with ETXTBSY on Linux.
        fs::write(&staged, script).unwrap();
        let mut permissions = fs::metadata(&staged).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&staged, permissions).unwrap();
        fs::rename(&staged, &path).unwrap();
        path
    }

    fn log(&self) -> String {
        fs::read_to_string(self.log_path()).unwrap_or_default()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[derive(Clone, Copy)]
struct FakeCore {
    protocol: u32,
    toolchain: bool,
    transpile_exit: i32,
    build_exit: i32,
    run_exit: i32,
}

impl FakeCore {
    fn working() -> Self {
        Self {
            protocol: 1,
            toolchain: true,
            transpile_exit: 0,
            build_exit: 0,
            run_exit: 0,
        }
    }
}

fn text(output: &Output) -> (String, String) {
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Execute a freshly copied binary. Linux can transiently reject exec with
/// ETXTBSY while the copy's write count is still settling, so retry briefly.
fn output_with_retry(mut command: Command) -> Output {
    const ETXTBSY: i32 = 26;
    for attempt in 0..20 {
        match command.output() {
            Ok(output) => return output,
            Err(error) if error.raw_os_error() == Some(ETXTBSY) && attempt < 19 => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => panic!("failed to execute command: {}", error),
        }
    }
    unreachable!("retry loop always returns or panics")
}

fn executable_line(stdout: &str) -> PathBuf {
    let line = stdout
        .lines()
        .rev()
        .find(|line| line.starts_with("executable="))
        .expect("missing executable= line");
    PathBuf::from(&line["executable=".len()..])
}

fn out_dir_from_log(log: &str) -> PathBuf {
    let marker = " out=";
    let line = log
        .lines()
        .find(|line| line.contains(marker))
        .expect("missing out-dir in log");
    let rest = &line[line.find(marker).unwrap() + marker.len()..];
    let value = rest.split_whitespace().next().unwrap();
    PathBuf::from(value)
}

#[test]
fn check_failure_exits_with_code_four() {
    let sandbox = Sandbox::new("check-failure");
    sandbox.install_core(
        "rust",
        FakeCore {
            transpile_exit: 1,
            ..FakeCore::working()
        },
    );

    let output = sandbox
        .command()
        .args(["check", "main.hust", "--core-rust"])
        .output()
        .unwrap();
    let (_, stderr) = text(&output);
    assert_eq!(output.status.code(), Some(4));
    assert!(stderr.contains("rust core"), "stderr: {}", stderr);
}

#[test]
fn run_defaults_to_debug_and_propagates_program_args() {
    let sandbox = Sandbox::new("run-default");
    sandbox.install_core("rust", FakeCore::working());

    let output = sandbox
        .command()
        .args(["run", "main.hust", "--core-rust", "--", "one", "--two"])
        .output()
        .unwrap();
    let (stdout, stderr) = text(&output);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr);
    assert!(stdout.contains("hello from rust"), "stdout: {}", stdout);
    assert!(stdout.contains("args: one --two"), "stdout: {}", stdout);
    assert!(
        sandbox.log().contains("mode=--debug"),
        "log: {}",
        sandbox.log()
    );
}

#[test]
fn build_defaults_to_release_and_installs_next_to_project() {
    let sandbox = Sandbox::new("build-default");
    sandbox.install_core("rust", FakeCore::working());

    let output = sandbox
        .command()
        .args(["build", "main.hust", "--core-rust"])
        .output()
        .unwrap();
    let (stdout, stderr) = text(&output);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr);
    let installed = executable_line(&stdout);
    assert!(installed.is_absolute());
    assert!(installed.is_file());
    assert_eq!(
        installed.parent().unwrap(),
        fs::canonicalize(sandbox.project()).unwrap()
    );
    assert!(
        sandbox.log().contains("mode=--release"),
        "log: {}",
        sandbox.log()
    );

    let run = Command::new(&installed).output().unwrap();
    assert_eq!(run.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&run.stdout).contains("hello from rust"));
}

#[test]
fn build_honors_output_path_and_creates_parents() {
    let sandbox = Sandbox::new("build-output");
    sandbox.install_core("rust", FakeCore::working());

    let output = sandbox
        .command()
        .args(["build", "main.hust", "--core-rust", "-o", "out/bin/custom"])
        .output()
        .unwrap();
    let (stdout, stderr) = text(&output);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr);

    let installed = executable_line(&stdout);
    assert_eq!(
        installed,
        fs::canonicalize(sandbox.project().join("out/bin/custom")).unwrap()
    );
    assert!(installed.is_file());
}

#[test]
fn project_directory_enables_project_mode() {
    let sandbox = Sandbox::new("project");
    sandbox.install_core("rust", FakeCore::working());
    fs::write(
        sandbox.project().join("settings.config"),
        "[project]\nname = demo\nentry = main.hust\ncore = rust\n\n[modules]\npaths = .\n",
    )
    .unwrap();

    let output = sandbox
        .command()
        .args(["build", ".", "--core-rust"])
        .output()
        .unwrap();
    let (_, stderr) = text(&output);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr);

    let log = sandbox.log();
    assert!(log.contains("entry="), "log: {}", log);
    assert!(log.contains("--project"), "log: {}", log);
}

#[test]
fn config_core_is_used_and_cli_overrides_it() {
    let sandbox = Sandbox::new("config-core");
    sandbox.install_core("rust", FakeCore::working());
    sandbox.install_core("c", FakeCore::working());
    fs::write(
        sandbox.project().join("settings.config"),
        "[project]\nentry = main.hust\ncore = c\n",
    )
    .unwrap();

    let configured = sandbox
        .command()
        .args(["run", "main.hust"])
        .output()
        .unwrap();
    let (stdout, stderr) = text(&configured);
    assert_eq!(configured.status.code(), Some(0), "stderr: {}", stderr);
    assert!(stdout.contains("hello from c"), "stdout: {}", stdout);

    let overridden = sandbox
        .command()
        .args(["run", "main.hust", "--core-rust"])
        .output()
        .unwrap();
    let (stdout, stderr) = text(&overridden);
    assert_eq!(overridden.status.code(), Some(0), "stderr: {}", stderr);
    assert!(stdout.contains("hello from rust"), "stdout: {}", stdout);
}

#[test]
fn auto_prefers_rust_and_falls_back_to_c_when_missing() {
    let sandbox = Sandbox::new("auto-order");
    sandbox.install_core("rust", FakeCore::working());
    sandbox.install_core("c", FakeCore::working());

    let rust = sandbox
        .command()
        .args(["run", "main.hust"])
        .output()
        .unwrap();
    let (stdout, _) = text(&rust);
    assert_eq!(rust.status.code(), Some(0));
    assert!(stdout.contains("hello from rust"), "stdout: {}", stdout);

    fs::remove_file(sandbox.cores().join("hust_rust")).unwrap();

    let c = sandbox
        .command()
        .args(["run", "main.hust"])
        .output()
        .unwrap();
    let (stdout, stderr) = text(&c);
    assert_eq!(c.status.code(), Some(0), "stderr: {}", stderr);
    assert!(stdout.contains("hello from c"), "stdout: {}", stdout);
}

#[test]
fn explicit_missing_core_fails_without_fallback() {
    let sandbox = Sandbox::new("explicit-missing");
    sandbox.install_core("rust", FakeCore::working());

    let output = sandbox
        .command()
        .args(["run", "main.hust", "--core-c"])
        .output()
        .unwrap();
    let (_, stderr) = text(&output);

    assert_eq!(output.status.code(), Some(3));
    assert!(
        stderr.contains("c core is unavailable"),
        "stderr: {}",
        stderr
    );
    assert!(stderr.contains("hust_c not found"), "stderr: {}", stderr);
    assert!(sandbox.log().is_empty(), "log: {}", sandbox.log());
}

#[test]
fn build_failure_is_not_retried_on_another_core() {
    let sandbox = Sandbox::new("no-fallback-on-failure");
    sandbox.install_core(
        "rust",
        FakeCore {
            build_exit: 1,
            ..FakeCore::working()
        },
    );
    sandbox.install_core("c", FakeCore::working());

    let output = sandbox
        .command()
        .args(["build", "main.hust"])
        .output()
        .unwrap();
    let (_, stderr) = text(&output);

    assert_eq!(output.status.code(), Some(5));
    assert!(stderr.contains("rust core"), "stderr: {}", stderr);
    let log = sandbox.log();
    assert_eq!(log.matches("build entry=").count(), 1, "log: {}", log);
    assert!(log.contains("mode=--release"), "log: {}", log);
}

#[test]
fn check_calls_transpile_and_cleans_the_temp_directory() {
    let sandbox = Sandbox::new("check");
    sandbox.install_core("c", FakeCore::working());

    let output = sandbox
        .command()
        .args(["check", "main.hust", "--core-c"])
        .output()
        .unwrap();
    let (stdout, stderr) = text(&output);

    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr);
    assert!(stdout.is_empty(), "stdout: {}", stdout);
    assert!(stderr.contains("transpile log"), "stderr: {}", stderr);

    let log = sandbox.log();
    assert!(log.contains("transpile entry="), "log: {}", log);
    assert!(!log.contains("build entry="), "log: {}", log);

    let out_dir = out_dir_from_log(&log);
    assert!(
        !out_dir.exists(),
        "temp directory was not cleaned: {}",
        out_dir.display()
    );
}

#[test]
fn keep_temp_preserves_the_build_directory() {
    let sandbox = Sandbox::new("keep-temp");
    sandbox.install_core("rust", FakeCore::working());

    let output = sandbox
        .command()
        .args(["check", "main.hust", "--core-rust", "--keep-temp"])
        .output()
        .unwrap();
    let (_, stderr) = text(&output);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr);

    let out_dir = out_dir_from_log(&sandbox.log());
    assert!(
        out_dir.is_dir(),
        "kept directory missing: {}",
        out_dir.display()
    );
    let _ = fs::remove_dir_all(&out_dir);
}

#[test]
fn protocol_mismatch_makes_a_core_unavailable() {
    let sandbox = Sandbox::new("protocol");
    sandbox.install_core(
        "rust",
        FakeCore {
            protocol: 2,
            ..FakeCore::working()
        },
    );
    sandbox.install_core("c", FakeCore::working());

    let auto = sandbox
        .command()
        .args(["run", "main.hust"])
        .output()
        .unwrap();
    let (stdout, _) = text(&auto);
    assert_eq!(auto.status.code(), Some(0));
    assert!(stdout.contains("hello from c"), "stdout: {}", stdout);

    let explicit = sandbox
        .command()
        .args(["run", "main.hust", "--core-rust"])
        .output()
        .unwrap();
    let (_, stderr) = text(&explicit);
    assert_eq!(explicit.status.code(), Some(3));
    assert!(
        stderr.contains("unsupported protocol 2"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn unavailable_toolchain_makes_a_core_unavailable() {
    let sandbox = Sandbox::new("toolchain");
    sandbox.install_core(
        "rust",
        FakeCore {
            toolchain: false,
            ..FakeCore::working()
        },
    );

    let output = sandbox
        .command()
        .args(["build", "main.hust", "--core-rust"])
        .output()
        .unwrap();
    let (_, stderr) = text(&output);
    assert_eq!(output.status.code(), Some(3));
    assert!(
        stderr.contains("toolchain is not available"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn cores_lists_paths_capabilities_and_failures() {
    let sandbox = Sandbox::new("cores");
    sandbox.install_core("rust", FakeCore::working());

    let output = sandbox.command().arg("--cores").output().unwrap();
    let (stdout, stderr) = text(&output);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr);
    assert!(stdout.contains("rust: available"), "stdout: {}", stdout);
    assert!(stdout.contains("path:"), "stdout: {}", stdout);
    assert!(
        stdout.contains("capabilities: transpile build"),
        "stdout: {}",
        stdout
    );
    assert!(stdout.contains("c: unavailable"), "stdout: {}", stdout);
    assert!(stdout.contains("not found"), "stdout: {}", stdout);
}

#[test]
fn version_reports_conductor_and_discovered_cores() {
    let sandbox = Sandbox::new("version");
    sandbox.install_core("rust", FakeCore::working());

    let output = sandbox.command().arg("--version").output().unwrap();
    let (stdout, stderr) = text(&output);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr);
    assert!(stdout.contains("hustc 0.1.0"), "stdout: {}", stdout);
    assert!(stdout.contains("protocol 1"), "stdout: {}", stdout);
    assert!(
        stdout.contains("rust: hust_rust 9.9.9"),
        "stdout: {}",
        stdout
    );
    assert!(stdout.contains("c: unavailable"), "stdout: {}", stdout);
}

#[test]
fn same_directory_core_wins_over_path() {
    let sandbox = Sandbox::new("same-dir");
    let bin = sandbox.root.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let hustc = bin.join("hustc");
    // Copy to a temporary name and rename into place: executing a freshly
    // copied file can race with the closing writer and fail with ETXTBSY.
    let staged = bin.join("hustc.tmp");
    fs::copy(env!("CARGO_BIN_EXE_hustc"), &staged).unwrap();
    let mut permissions = fs::metadata(&staged).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&staged, permissions).unwrap();
    fs::rename(&staged, &hustc).unwrap();

    sandbox.install_core_in(&bin, "rust", FakeCore::working());
    sandbox.install_core("c", FakeCore::working());

    let mut command = Command::new(&hustc);
    command
        .current_dir(sandbox.project())
        .env_remove("HUST_RUST_BIN")
        .env_remove("HUST_C_BIN")
        .env("PATH", sandbox.search_path())
        .env("FAKE_LOG", sandbox.log_path())
        .args(["run", "main.hust"]);
    let output = output_with_retry(command);
    let (stdout, stderr) = text(&output);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr);
    assert!(stdout.contains("hello from rust"), "stdout: {}", stdout);
}

#[test]
fn env_override_finds_a_core_outside_path() {
    let sandbox = Sandbox::new("env-override");
    let external = sandbox.root.join("external");
    let c_core = sandbox.install_core_in(&external, "c", FakeCore::working());

    let output = Command::new(env!("CARGO_BIN_EXE_hustc"))
        .current_dir(sandbox.project())
        .env_remove("HUST_RUST_BIN")
        .env("HUST_C_BIN", &c_core)
        .env("PATH", sandbox.search_path())
        .env("FAKE_LOG", sandbox.log_path())
        .args(["run", "main.hust", "--core-c"])
        .output()
        .unwrap();
    let (stdout, stderr) = text(&output);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr);
    assert!(stdout.contains("hello from c"), "stdout: {}", stdout);
}

#[test]
fn path_core_wins_over_env_override() {
    let sandbox = Sandbox::new("path-before-env");
    let path_core = sandbox.install_core("c", FakeCore::working());
    let external = sandbox.root.join("external");
    let env_core = sandbox.install_core_in(&external, "c", FakeCore::working());

    let output = Command::new(env!("CARGO_BIN_EXE_hustc"))
        .current_dir(sandbox.project())
        .env_remove("HUST_RUST_BIN")
        .env("HUST_C_BIN", &env_core)
        .env("PATH", sandbox.search_path())
        .arg("--cores")
        .output()
        .unwrap();
    let (stdout, stderr) = text(&output);
    assert_eq!(output.status.code(), Some(0), "stderr: {}", stderr);
    assert!(
        stdout.contains(&format!("path: {}", path_core.display())),
        "stdout: {}",
        stdout
    );
    assert!(
        !stdout.contains(&format!("path: {}", env_core.display())),
        "stdout: {}",
        stdout
    );
}

#[test]
fn no_available_core_is_an_explicit_error() {
    let sandbox = Sandbox::new("no-core");

    let output = sandbox
        .command()
        .args(["run", "main.hust"])
        .output()
        .unwrap();
    let (_, stderr) = text(&output);
    assert_eq!(output.status.code(), Some(3));
    assert!(
        stderr.contains("no usable language core found"),
        "stderr: {}",
        stderr
    );
    assert!(stderr.contains("--core-rust"), "stderr: {}", stderr);
}

#[test]
fn usage_errors_exit_with_code_two() {
    let sandbox = Sandbox::new("usage");

    let missing = sandbox.command().args(["run"]).output().unwrap();
    assert_eq!(missing.status.code(), Some(2));

    let unknown = sandbox
        .command()
        .args(["run", "main.hust", "--nope"])
        .output()
        .unwrap();
    assert_eq!(unknown.status.code(), Some(2));

    let conflict = sandbox
        .command()
        .args(["run", "main.hust", "--core-rust", "--core-c"])
        .output()
        .unwrap();
    assert_eq!(conflict.status.code(), Some(2));
}

#[test]
fn missing_entry_file_is_a_usage_error() {
    let sandbox = Sandbox::new("missing-entry");
    sandbox.install_core("rust", FakeCore::working());

    let output = sandbox
        .command()
        .args(["run", "nope.hust"])
        .output()
        .unwrap();
    let (_, stderr) = text(&output);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr.contains("cannot access target"),
        "stderr: {}",
        stderr
    );
}

#[test]
fn program_exit_code_is_propagated() {
    let sandbox = Sandbox::new("exit-code");
    sandbox.install_core(
        "rust",
        FakeCore {
            run_exit: 7,
            ..FakeCore::working()
        },
    );

    let output = sandbox
        .command()
        .args(["run", "main.hust", "--core-rust"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(7));
}

#[test]
fn invalid_config_core_is_a_usage_error() {
    let sandbox = Sandbox::new("invalid-config");
    sandbox.install_core("rust", FakeCore::working());
    fs::write(
        sandbox.project().join("settings.config"),
        "[project]\ncore = go\n",
    )
    .unwrap();

    let output = sandbox
        .command()
        .args(["run", "main.hust"])
        .output()
        .unwrap();
    let (_, stderr) = text(&output);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains("invalid core"), "stderr: {}", stderr);
}
