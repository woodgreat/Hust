# Hust-Go Language Core

The Go language core for Hust. Transpiles Hust source code to Go and compiles
with the Go toolchain.

**Version:** 0.1.1.20261128125000

## Role

- **Language**: Go (1.21+)
- **Binary**: `hust_go`
- **Target**: Go source code + go build
- **Runtime**: Go standard library

## Build

Prerequisites:
- Go 1.21 or later

```bash
cd Hust-Go
go build -o hust_go ./src
```

Binary: `hust_go`

## Install

From repo root (recommended):

```bash
cd ..          # repo root
./install.sh
```

This installs:
- `hust_go` to `~/.local/bin/hust_work/hust_core/hust_go/hust_go`

Manual install (advanced):

```bash
go build -o hust_go ./src
mkdir -p ~/.local/bin/hust_work/hust_core/hust_go
cp hust_go ~/.local/bin/hust_work/hust_core/hust_go/
```

## Usage

Normal use goes through `hustc` (the Conductor):

```bash
hustc run main.hust --core-go          # Run with Go core
hustc build myproj --core-go           # Build with Go core
```

Direct core usage (advanced):

```bash
./hust_go transpile main.hust --out-dir build/
./hust_go build main.hust --out-dir build/
./hust_go info
```

## Core Contract

Implements the standard core contract:

```bash
hust_go transpile <entry> [--project] --out-dir <dir>
hust_go build     <entry> [--project] --out-dir <dir> [--release|--debug]
hust_go info
hust_go --version
```

## Dialect

Go dialect uses `GO.*` system imports:

```hust
use GO.fmt.*;

void main() {
    fmt.Println("Hello from Hust-Go!");
}
```

## Testing

```bash
cd tests
./run_tests.sh            # 134/134 + negative 15/15
./run_tests.sh --quick    # transpile only
./core_contract.sh        # info / transpile / build / dialect / project mode
```

Coverage matrix, known boundaries, and how to add tests: `tests/tests.md`.

## Implementation

- `src/lexer.go`, `src/parser.go`, `src/ast.go`: structured front end
- `src/translator.go`: type inference + Go emission
- `src/preprocessor.go`: comment/string protection + dialect detection
- `src/main.go`: core contract CLI

Supported: primitives, variables/constants, operators, control flow, functions,
arrays (1D/2D/3D, push/pop/insert/removeAt/clear/slice/concat), strings,
classes/constructors/inheritance/interfaces, enums, project mode.

## Documentation

- `../README.md` - Repo root quick start
- `../../Hust-Docs/doc/core_go_write_guide_dg_cpt4.md` - Go core development guide
- `../../Hust-Docs/doc/core_testing_guide.md` - Testing guide
- `../../Hust-Docs/doc/core_development_playbook.md` - Adding a new core

## License

MIT - see `../MIT.md`
