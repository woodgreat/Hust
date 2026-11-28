# Hust-CSharp

Hust language core for C# - transpiles Hust source code to C#.

**Version:** 0.1.3.20261128125000

## Overview

Hust-CSharp is the C# language core for the Hust programming language. It transpiles Hust source code to C# and builds with the .NET toolchain.

## Build

```bash
cd Hust-CSharp
dotnet build -c Release -o bin/core
```

## Install

```bash
# From repository root
./install.sh
```

Or manually:

```bash
# Build
dotnet build -c Release -o bin/core

# Install to ~/.local/bin/hust_work/hust_core/hust_csharp/
mkdir -p ~/.local/bin/hust_work/hust_core/hust_csharp
cp bin/core/hust_csharp.dll bin/core/hust_csharp.deps.json \
   bin/core/hust_csharp.runtimeconfig.json \
   ~/.local/bin/hust_work/hust_core/hust_csharp/
# plus a launcher named `hust_csharp` that execs `dotnet .../hust_csharp.dll`
```

## Usage

### Via Conductor (recommended)

```bash
# Run with C# core
hustc run main.hust --core-csharp

# Build with C# core
hustc build myproj --core-csharp -o bin/app

# Check (transpile only)
hustc check main.hust --core-csharp
```

### Direct core usage

```bash
# Info
hust_csharp info

# Transpile
hust_csharp transpile main.hust --out-dir ./out

# Build
hust_csharp build main.hust --out-dir ./out --release
```

## Core Contract

The core implements the standard Hust core contract:

- `hust_csharp info` - Output single-line JSON with capabilities
- `hust_csharp transpile <entry> [--project] --out-dir <dir>` - Transpile to C#
- `hust_csharp build <entry> [--project] --out-dir <dir> [--release|--debug]` - Build executable
- `hust_csharp --version` - Show version

## Dialect

C# dialect uses `CSHARP.*` system imports:

```hust
use CSHARP.Console.*;

void main() {
    Console.WriteLine("Hello from Hust-CSharp!");
}
```

## Testing

```bash
cd tests
./run_tests.sh        # 112/112：transpile -> dotnet build -> run -> diff（负向 12/12）
./core_contract.sh    # info / transpile / build / dialect / project mode
```

Coverage matrix and known boundaries: `tests/tests.md`.

## Documentation

- [Development Guide](../../Hust-Docs/doc/develop_guide.md)
- [C# Core Write Guide](../../Hust-Docs/doc/core_csharp_write_guide_dg_cpt3.md)
- [Core Development Playbook](../../Hust-Docs/doc/core_development_playbook.md)
- [Core Testing Guide](../../Hust-Docs/doc/core_testing_guide.md)

## License

MIT License - see [MIT.md](../MIT.md)
