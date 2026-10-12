# How to Build Hust

This guide covers building the Hust transpiler from source on Linux and Windows,
and how to install the resulting binary. Current version at time of writing:
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

The binary is produced at `target/release/hust`.

For a quick debug build (faster compile, slower runtime):

```bash
cargo build
# binary at target/debug/hust
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

Hust creates its working directory **relative to the location of the hust
binary**, not relative to the source file being compiled:

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
   hust process running) and retry.

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

The binary is produced at `target\release\hust.exe`.

If your user profile is on a restricted drive, the same `CARGO_HOME`
workaround applies:

```powershell
$env:CARGO_HOME = "D:\cargo-home"
cargo build --release
```

## Installing

### Linux

Recommended (user-level, no root, on the standard PATH):

```bash
cp target/release/hust ~/.local/bin/
```

Alternatives:

| Location | Command | Notes |
|----------|---------|-------|
| `~/.local/bin` (recommended) | `cp target/release/hust ~/.local/bin/` | User-level, in PATH on most distros |
| `~/.cargo/bin` | `cp target/release/hust ~/.cargo/bin/` | Already in PATH if you use rustup |
| `/usr/local/bin` | `sudo cp target/release/hust /usr/local/bin/` | System-wide, all users |
| Custom dir | `cp target/release/hust ~/bin/` | Only if the dir is in your `$PATH` |

Remember the portable build directory follows the binary: installing to
`~/.local/bin` creates `~/.local/bin/build/` on first `hust run`.

### One-command script

The repo root (one level above `Hust-Rust/`) contains `install.sh`, which
builds release and installs to `~/.local/bin` in a single step:

```bash
./install.sh
hust --version
```

### Windows

Either:

- Copy `target\release\hust.exe` into a directory already on your `PATH`
  (e.g. create `C:\tools\hust\` and add it via *System Properties →
  Environment Variables*), or
- Keep `hust.exe` in a project folder and invoke it by full path — the
  portable `build\` directory will be created next to the exe.

## Verify installation

```bash
hust --version
# Wood format output: 0.1.11.20260908

hust --help
```

## Running the test suite

Single test:

```bash
hust run tests/all_arrays.hust
```

Full suite (43 files). **Use a per-file timeout** — two pre-existing cases
(`q3.hust`, `q3_test.hust`) currently hang and would stall an unguarded loop:

```bash
cd Hust-Rust
pass=0; fail=0
for f in tests/*.hust; do
  if timeout 30 hust run "$f" > /dev/null 2>&1; then
    pass=$((pass+1))
  else
    fail=$((fail+1)); echo "FAIL: $f"
  fi
done
echo "PASS: $pass  FAIL: $fail"
# Expected baseline at 0.1.11: 38 pass / 8 fail
```

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| `Read-only file system (os error 30)` during build | `~/.cargo` on a read-only mount | `CARGO_HOME=/tmp/cargo-home cargo build --release` |
| `hust run` hangs with no output | Stale lock from a killed run, or a test that times out (q3/q3_test) | Remove `build/dist/debug/.cargo-build-lock`; always `timeout` individual runs |
| `Permission denied` installing to `/usr/local/bin` | Need root | Use `sudo`, or install to `~/.local/bin` instead |
| `hust: command not found` after install | Target dir not in `$PATH` | Use full path, or add the dir to `$PATH` / restart shell |
| `expected one of . ; ? } ...` on array code | Hust source file has unbalanced braces | Count `{` vs `}` — generate literals programmatically for high dimensions |

## Version history note

- `0.1.10+20260908` — fixed nested array index transpile + `len()` condition
  conversion; environment migration to Linux.
- `0.1.11+20260908` — N-dimensional array declarations (N ≥ 2), stress-tested
  to 8 dimensions; `install.sh`; `howto_build.md` (this file).
