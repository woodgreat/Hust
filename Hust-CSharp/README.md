# Hust-CSharp

Hust language core for C# - transpiles Hust source code to C#.

**Version:** 0.1.1.20261127083700

## Overview

Hust-CSharp is the C# language core for the Hust programming language. It transpiles Hust source code to C# and builds with the .NET toolchain.

## Build

```bash
cd Hust-CSharp
dotnet build -c Release
```

## Install

```bash
# From repository root
./install.sh
```

Or manually:

```bash
# Build
dotnet build -c Release

# Install to ~/.local/bin/hust_work/hust_core/csharp/
mkdir -p ~/.local/bin/hust_work/hust_core/csharp
cp bin/Release/net8.0/hust_csharp ~/.local/bin/hust_work/hust_core/csharp/
cp -r include ~/.local/bin/hust_work/hust_core/csharp/
cp -r lib ~/.local/bin/hust_work/hust_core/csharp/
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

C# dialect uses `C#.*` system imports:

```hust
use C#.Console.*;
use C#.System.*;

void main() {
    Console.WriteLine("Hello from Hust-CSharp!");
}
```

## Testing

```bash
cd tests
./run_tests.sh
```

## Documentation

- [Development Guide](../Hust-Docs/doc/develop_guide.md)
- [Core Development Playbook](../Hust-Docs/doc/core_development_playbook.md)
- [Core Testing Guide](../Hust-Docs/doc/core_testing_guide.md)

## License

MIT License - see [MIT.md](../MIT.md)
