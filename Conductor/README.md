# Hust Conductor (hustc)

The Conductor is the CLI entry point for the Hust language. It reads
configuration, discovers language cores, selects exactly one core, and
orchestrates transpilation/build/run.

## Role

- **User entry point**: `hustc` is what users invoke
- **Core discovery**: Finds `hust_rust` and `hust_c` cores
- **Core selection**: Auto (Rust preferred) or explicit (`--core-rust` / `--core-c`)
- **Orchestration**: Delegates transpile/build to the selected core

## Build

Prerequisites:
- Rust toolchain (edition 2021), developed with rustc 1.98.0
- Cargo (comes with rustup)

```bash
cd Conductor
cargo build --release
```

Binary: `target/release/hustc`

Debug build (faster compile, slower runtime):

```bash
cargo build
# binary at target/debug/hustc
```

## Install

From repo root (recommended):

```bash
cd ..          # repo root
./install.sh
```

This installs `hustc` to `~/.local/bin/` and cores to `~/.local/bin/hust_work/hust_core/`.

Manual install (advanced):

```bash
cp target/release/hustc ~/.local/bin/
```

## Usage

```bash
hustc run main.hust                 # Single file, default Rust core
hustc run myproj --core-c           # Project, explicit C core
hustc build myproj -o bin/app       # Build release
hustc check myproj --core-rust      # Transpile only
hustc --cores                       # List discovered cores
hustc --version                     # Show version
```

## Core Discovery Order

1. `hustc_dir/hust_work/hust_core/<core>/<core>` (new default)
2. `hustc_dir/<core>` (legacy fallback)
3. `PATH` environment variable
4. `HUST_RUST_BIN` / `HUST_C_BIN` override

## Testing

```bash
cargo test                          # Unit + integration tests
cd .. && ./tests/run_all.sh         # Full regression gate
```

## Documentation

- `../README.md` - Repo root quick start
- `../Hust-Docs/doc/develop_guide.md` - Full development guide
- `../Hust-Docs/doc/conductor_multicore_design.md` - Multi-core design

## License

MIT - see `../MIT.md`
