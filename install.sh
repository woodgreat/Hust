#!/usr/bin/env bash
# Hust installer — install hustc and cores to ~/.local/bin/hust_work/
# 
# Layout:
#   ~/.local/bin/hustc                          # Conductor CLI
#   ~/.local/bin/hust_work/
#   ├── hustc/                                  # Conductor extra files (reserved)
#   └── hust_core/
#       ├── hust_c/
#       │   ├── hust_c                          # C core executable
#       │   ├── include/                        # C headers
#       │   └── lib/                            # C libraries
#       └── hust_rust/
#           ├── hust_rust                       # Rust core executable
#           ├── include/                        # Rust headers (if any)
#           └── lib/                            # Rust libraries (if any)
#
# Usage: ./install.sh   (from Hust repo root, or any path)
#        PREFIX=/custom/prefix ./install.sh

set -e

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
INSTALL_DIR="$PREFIX/bin"
HUST_WORK="$INSTALL_DIR/hust_work"

echo "=== Hust Installer ==="
echo "Repo: $REPO_ROOT"
echo "Install to: $INSTALL_DIR"
echo ""

# 1. Build and install Conductor (hustc)
echo "[1/3] Building Conductor (hustc)..."
cd "$REPO_ROOT/Conductor"
cargo build --release

CONDUCTOR_BIN="$REPO_ROOT/Conductor/target/release/hustc"
if [ ! -x "$CONDUCTOR_BIN" ]; then
    # Fallback: try 'hust' binary name if 'hustc' not found
    CONDUCTOR_BIN="$REPO_ROOT/Conductor/target/release/hust"
fi
[ -x "$CONDUCTOR_BIN" ] || { echo "error: build failed - conductor binary not found"; exit 1; }

mkdir -p "$HUST_WORK/hustc"
cp "$CONDUCTOR_BIN" "$INSTALL_DIR/hustc"
echo "  Installed: $INSTALL_DIR/hustc"

# 2. Build and install C core
echo "[2/3] Building C core (hust_c)..."
cd "$REPO_ROOT/Hust-C"
make clean >/dev/null 2>&1 || true
make

C_BIN="$REPO_ROOT/Hust-C/hust_c"
[ -x "$C_BIN" ] || { echo "error: build failed - hust_c binary not found"; exit 1; }

C_CORE_DIR="$HUST_WORK/hust_core/hust_c"
mkdir -p "$C_CORE_DIR/include" "$C_CORE_DIR/lib"
cp "$C_BIN" "$C_CORE_DIR/hust_c"
# Copy headers if they exist
if [ -d "$REPO_ROOT/Hust-C/include" ]; then
    cp "$REPO_ROOT/Hust-C/include/"*.h "$C_CORE_DIR/include/" 2>/dev/null || true
fi
echo "  Installed: $C_CORE_DIR/hust_c"

# 3. Build and install Rust core
echo "[3/4] Building Rust core (hust_rust)..."
cd "$REPO_ROOT/Hust-Rust"
CARGO_HOME="${CARGO_HOME:-/tmp/cargo-home}" cargo build --release

RUST_BIN="$REPO_ROOT/Hust-Rust/target/release/hust_rust"
if [ ! -x "$RUST_BIN" ]; then
    # Fallback: try 'hust' binary name if 'hust_rust' not found
    RUST_BIN="$REPO_ROOT/Hust-Rust/target/release/hust"
fi
[ -x "$RUST_BIN" ] || { echo "error: build failed - hust_rust binary not found"; exit 1; }

RUST_CORE_DIR="$HUST_WORK/hust_core/hust_rust"
mkdir -p "$RUST_CORE_DIR/include" "$RUST_CORE_DIR/lib"
cp "$RUST_BIN" "$RUST_CORE_DIR/hust_rust"
echo "  Installed: $RUST_CORE_DIR/hust_rust"

# 4. Build and install C# core
echo "[4/4] Building C# core (hust_csharp)..."
cd "$REPO_ROOT/Hust-CSharp"

# Check for dotnet
DOTNET_CMD="${DOTNET_CMD:-$HOME/.dotnet/dotnet}"
if [ ! -f "$DOTNET_CMD" ]; then
    DOTNET_CMD="dotnet"
fi
if ! command -v "$DOTNET_CMD" &> /dev/null; then
    echo "warning: dotnet not found, skipping C# core"
else
    "$DOTNET_CMD" build -c Release
    
    CSHARP_DLL="$REPO_ROOT/Hust-CSharp/bin/Release/net8.0/hust_csharp.dll"
    [ -f "$CSHARP_DLL" ] || { echo "error: build failed - hust_csharp.dll not found"; exit 1; }
    
    CSHARP_CORE_DIR="$HUST_WORK/hust_core/hust_csharp"
    mkdir -p "$CSHARP_CORE_DIR"
    cp "$CSHARP_DLL" "$CSHARP_CORE_DIR/hust_csharp.dll"
    # Copy runtime config if exists
    if [ -f "$REPO_ROOT/Hust-CSharp/bin/Release/net8.0/hust_csharp.runtimeconfig.json" ]; then
        cp "$REPO_ROOT/Hust-CSharp/bin/Release/net8.0/hust_csharp.runtimeconfig.json" "$CSHARP_CORE_DIR/"
    fi
    echo "  Installed: $CSHARP_CORE_DIR/hust_csharp.dll"
fi

# Summary
echo ""
echo "=== Installation Complete ==="
echo ""
echo "Installed files:"
echo "  Conductor:  $INSTALL_DIR/hustc"
echo "  C core:     $C_CORE_DIR/hust_c"
echo "  Rust core:  $RUST_CORE_DIR/hust_rust"
if [ -n "$CSHARP_CORE_DIR" ]; then
    echo "  C# core:    $CSHARP_CORE_DIR/hust_csharp.dll"
fi
echo ""
echo "Directory structure:"
echo "  $INSTALL_DIR/hustc"
echo "  $HUST_WORK/"
echo "  ├── hustc/          (reserved for conductor extras)"
echo "  └── hust_core/"
echo "      ├── hust_c/"
echo "      │   ├── hust_c"
echo "      │   ├── include/"
echo "      │   └── lib/"
echo "      ├── hust_rust/"
echo "      │   ├── hust_rust"
echo "      │   ├── include/"
echo "      │   └── lib/"
if [ -n "$CSHARP_CORE_DIR" ]; then
    echo "      └── hust_csharp/"
    echo "          └── hust_csharp.dll"
fi
echo ""
echo "Verify installation:"
echo "  $INSTALL_DIR/hustc --version"
echo "  $INSTALL_DIR/hustc --cores"
echo ""
