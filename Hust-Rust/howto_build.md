# How to Build Hust-Rust

This guide covers building the Rust language core from source on Linux and
Windows. Normal users should invoke the `hustc` conductor; this guide is for
building the `hust_rust` core itself. Current version at time of writing:
`0.1.11+20260908` (Wood format display: `0.1.11.20260908`).

## Prerequisites

- **Rust toolchain** (edition 2021). Developed and verified with rustc 1.98.0.
  Install via [rustup](https://rustup.rs):
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- All crate dependencies (regex, clap, serde, syn, ...) are managed by Cargo
  and fetched automatically on first build.
- No other system libraries are required.

## Build on Linux

### Standard build

```bash
cd Hust-Rust
cargo build --release
```

The binary is produced at `target/release/hust_rust`.

For a quick debug build (faster compile, slower runtime):

```bash
cargo build
# binary at target/debug/hust_rust
```

### Restricted / sandboxed environments (read-only $HOME)

If `~/.cargo` lives on a read-only filesystem (common in AI sandboxes or
containers with a read-only home mount), the default `cargo build` fails with:

```
error: failed to open ~/.cargo/registry/cache/... Read-only file system (os error 30)
```

Workaround: point `CARGO_HOME` at a writable location:

```bash
CARGO_HOME=/tmp/cargo-home cargo build --release
```

This downloads all crates fresh into the new location (first build takes
longer; subsequent builds reuse the cache).

### The portable build directory (important behavior)

Hust creates its working directory **relative to the location of the
`hust_rust` binary**, not relative to the source file being compiled:

```
<binary dir>/build/
├── temp/   # intermediate Cargo project
└── dist/   # compiled output
```

Two practical consequences:

1. Running the binary from `target/release/` puts `build/` inside Cargo's own
   output tree — it works but pollutes `target/`. Prefer an installed copy.
2. If a build is killed mid-run, a stale `.cargo-build-lock` may remain in
   `build/dist/debug/`. If subsequent runs hang, delete the lock file (with no
   `hust_rust` process running) and retry.

## Build on Windows

> Note: the project originated on Windows, but the steps below follow the
> standard Rust cross-platform flow and have not been re-verified for
> 0.1.11 specifically.

### Prerequisites

1. Rust via rustup (same as Linux). On Windows choose the **MSVC** host
   toolchain, which additionally requires:
   - **Visual Studio Build Tools** (C++ workload) — provides `link.exe`
2. Or use the GNU toolchain (`rustup default stable-gnu`) with MinGW-w64,
   avoiding the Visual Studio dependency.

### Build

```powershell
cd Hust-Rust
cargo build --release
```

The binary is produced at `target\release\hust_rust.exe`.

If your user profile is on a restricted drive, the same `CARGO_HOME`
workaround applies:

```powershell
$env:CARGO_HOME = "D:\cargo-home"
cargo build --release
```

## Installing

The recommended way is the repo-root `install.sh`, which builds the Conductor,
Rust core, and C core and installs all three:

```bash
cd ..          # repo root
./install.sh
hustc --version
```

This installs `hustc` to `$PREFIX/bin` (default `~/.local/bin`) and each core
to `$PREFIX/bin/hust_work/hust_core/<core>/` with its own `include/` and
`lib/` directories.

The old `hust` binary is retired. Update existing scripts by replacing
`hust` with `hustc`; source files and `settings.config` do not change.

### Manual core install (advanced)

If you only need the Rust core binary:

```bash
cp target/release/hust_rust ~/.local/bin/
```

| Location | Command | Notes |
|----------|---------|-------|
| `~/.local/bin` (recommended) | `cp target/release/hust_rust ~/.local/bin/` | User-level, in PATH on most distros |
| `~/.cargo/bin` | `cp target/release/hust_rust ~/.cargo/bin/` | Already in PATH if you use rustup |
| `/usr/local/bin` | `sudo cp target/release/hust_rust /usr/local/bin/` | System-wide, all users |
| Custom dir | `cp target/release/hust_rust ~/bin/` | Only if the dir is in your `$PATH` |

The portable build directory follows the core binary: a legacy
`hust_rust run` creates `build/` next to `hust_rust`.

### One-command script

```bash
./install.sh
hustc --version
hustc --cores
```

### Windows

Either:

- Copy `target\release\hust_rust.exe` into a directory already on your `PATH`
  (e.g. create `C:\tools\hust\` and add it via *System Properties →
  Environment Variables*), or
- Keep `hust_rust.exe` in a project folder and invoke it by full path — the
  portable `build\` directory will be created next to the exe.

Normal use goes through `hustc.exe`.

## Verify installation

```bash
hustc --version
# Wood format output: 0.1.11.20260908

hustc --cores
hustc --help
```

## Running the test suite

Single test:

```bash
hustc run tests/all_arrays.hust
```

Full suite:

```bash
cd Hust-Rust
./tests/run_tests.sh            # baseline-aware: documented known failures do not fail
./tests/run_tests.sh --strict   # require a fully green suite
./tests/run_tests.sh --quick    # top-level and project builds only
```

The runner covers top-level files, project directories, unit tests, the core
contract, and representative runtime samples. Project directories are copied
to a temporary directory, so the runner does not overwrite tracked binaries.
Known failures are listed in `tests/tests.md` and reported as `KNOWN`.

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| `Read-only file system (os error 30)` during build | `~/.cargo` on a read-only mount | `CARGO_HOME=/tmp/cargo-home cargo build --release` |
| `hustc run` hangs with no output | Stale lock from a killed legacy core run, or a test that times out (q3/q3_test) | Remove `build/dist/debug/.cargo-build-lock`; always `timeout` individual runs |
| `Permission denied` installing to `/usr/local/bin` | Need root | Use `sudo`, or install to `~/.local/bin` instead |
| `hustc: command not found` after install | Target dir not in `$PATH` | Use full path, or add the dir to `$PATH` / restart shell |
| `expected one of . ; ? } ...` on array code | Hust source file has unbalanced braces | Count `{` vs `}` — generate literals programmatically for high dimensions |

## Version history note

- `0.1.10+20260908` — fixed nested array index transpile + `len()` condition
  conversion; environment migration to Linux.
- `0.1.11+20260908` — N-dimensional array declarations (N ≥ 2), stress-tested
  to 8 dimensions; `install.sh`; `howto_build.md` (this file).
