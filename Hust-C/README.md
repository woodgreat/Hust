# Hust-C Language Core

The C language core for Hust. Transpiles Hust source code to C and compiles
with gcc.

**Version:** 0.1.1.20261127084000

## Role

- **Language**: C (C99)
- **Binary**: `hust_c`
- **Target**: C source code + gcc compilation
- **Runtime**: `hust_error.h`, `hust_vec.h`, `hust_string.h`

## Build

Prerequisites:
- gcc with C99 support
- make

```bash
cd Hust-C
make
```

Binary: `hust_c`

Clean build:

```bash
make clean && make
```

## Install

From repo root (recommended):

```bash
cd ..          # repo root
./install.sh
```

This installs:
- `hust_c` to `~/.local/bin/hust_work/hust_core/hust_c/hust_c`
- Headers to `~/.local/bin/hust_work/hust_core/hust_c/include/`
- Libraries to `~/.local/bin/hust_work/hust_core/hust_c/lib/`

Manual install (advanced):

```bash
cp hust_c ~/.local/bin/
cp include/*.h ~/.local/include/
```

## Usage

Normal use goes through `hustc` (the Conductor):

```bash
hustc run main.hust --core-c          # Run with C core
hustc build myproj --core-c           # Build with C core
```

Direct core usage (advanced):

```bash
./hust_c transpile main.hust --out-dir build/
./hust_c build main.hust --out-dir build/
./hust_c info
```

## Runtime Headers

| Header | Purpose |
|--------|---------|
| `hust_error.h` | Error codes + global error info |
| `hust_vec.h` | Dynamic array (HustVec) implementation |
| `hust_string.h` | String type (HustString) implementation |

## Testing

```bash
./tests/run_tests.sh            # Full test suite (178 tests)
./tests/run_tests.sh negative   # Negative tests only (10 tests)
```

## Core Contract

Implements the standard core contract:

```bash
hust_c transpile <entry> [--project] --out-dir <dir>
hust_c build     <entry> [--project] --out-dir <dir> [--release|--debug]
hust_c info
hust_c --version
```

## Documentation

- `../README.md` - Repo root quick start
- `../Hust-Docs/doc/core_c_write_guide_dg_cpt2.md` - C core development guide
- `../Hust-Docs/doc/core_testing_guide.md` - Testing guide
- `../Hust-Docs/doc/core_development_playbook.md` - Adding a new core

## License

MIT - see `../MIT.md`
