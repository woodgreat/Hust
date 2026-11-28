#!/usr/bin/env bash
# Hust installer — install hustc and the language cores under $PREFIX/bin.
#
# Layout:
#   $PREFIX/bin/hustc
#   $PREFIX/bin/hust_work/hust_core/hust_c/{hust_c,include,lib}
#   $PREFIX/bin/hust_work/hust_core/hust_rust/{hust_rust,include,lib}
#   $PREFIX/bin/hust_work/hust_core/hust_csharp/{hust_csharp,hust_csharp.dll,...}
#   $PREFIX/bin/hust_work/hust_core/hust_go/{hust_go,include,lib}
#
# Usage:
#   ./install.sh
#   PREFIX=/custom/prefix ./install.sh

set -e

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
INSTALL_DIR="$PREFIX/bin"
HUST_WORK="$INSTALL_DIR/hust_work"
CORE_ROOT="$HUST_WORK/hust_core"

find_cargo() {
    if command -v cargo >/dev/null 2>&1; then
        command -v cargo
    elif [ -x "$HOME/.cargo/bin/cargo" ]; then
        echo "$HOME/.cargo/bin/cargo"
    else
        echo ""
    fi
}

find_dotnet() {
    if [ -n "${DOTNET_CMD:-}" ] && [ -x "$DOTNET_CMD" ]; then
        echo "$DOTNET_CMD"
    elif [ -x "$HOME/.dotnet/dotnet" ]; then
        echo "$HOME/.dotnet/dotnet"
    elif command -v dotnet >/dev/null 2>&1; then
        command -v dotnet
    else
        echo ""
    fi
}

find_go() {
    if [ -n "${GO_BIN:-}" ] && [ -x "$GO_BIN" ]; then
        echo "$GO_BIN"
    elif command -v go >/dev/null 2>&1; then
        command -v go
    elif [ -x "$HOME/go-sdk/go/bin/go" ]; then
        echo "$HOME/go-sdk/go/bin/go"
    else
        echo ""
    fi
}

echo "=== Hust Installer ==="
echo "Repo:       $REPO_ROOT"
echo "Install to: $INSTALL_DIR"
echo ""

mkdir -p "$CORE_ROOT" "$HUST_WORK/hustc"

# 1. Conductor
echo "[1/5] Building Conductor (hustc)..."
CARGO="$(find_cargo)"
[ -n "$CARGO" ] || { echo "error: cargo not found"; exit 1; }
(cd "$REPO_ROOT/Conductor" && "$CARGO" build --release --offline)

CONDUCTOR_BIN="$REPO_ROOT/Conductor/target/release/hustc"
[ -x "$CONDUCTOR_BIN" ] || { echo "error: build failed - hustc not found"; exit 1; }
cp "$CONDUCTOR_BIN" "$INSTALL_DIR/hustc"
echo "  Installed: $INSTALL_DIR/hustc"

# 2. C core
echo "[2/5] Building C core (hust_c)..."
(cd "$REPO_ROOT/Hust-C" && make clean >/dev/null 2>&1 || true; make)

C_CORE_DIR="$CORE_ROOT/hust_c"
mkdir -p "$C_CORE_DIR/include" "$C_CORE_DIR/lib"
cp "$REPO_ROOT/Hust-C/hust_c" "$C_CORE_DIR/hust_c"
if [ -d "$REPO_ROOT/Hust-C/include" ]; then
    cp "$REPO_ROOT/Hust-C/include/"*.h "$C_CORE_DIR/include/" 2>/dev/null || true
fi
echo "  Installed: $C_CORE_DIR/hust_c"

# 3. Rust core
echo "[3/5] Building Rust core (hust_rust)..."
(cd "$REPO_ROOT/Hust-Rust" && CARGO_HOME="${CARGO_HOME:-/tmp/cargo-home}" "$CARGO" build --release)

RUST_CORE_DIR="$CORE_ROOT/hust_rust"
mkdir -p "$RUST_CORE_DIR/include" "$RUST_CORE_DIR/lib"
cp "$REPO_ROOT/Hust-Rust/target/release/hust_rust" "$RUST_CORE_DIR/hust_rust"
echo "  Installed: $RUST_CORE_DIR/hust_rust"

# 4. C# core
echo "[4/5] Building C# core (hust_csharp)..."
DOTNET="$(find_dotnet)"
if [ -z "$DOTNET" ]; then
    echo "  warning: dotnet not found, skipping C# core"
else
    (cd "$REPO_ROOT/Hust-CSharp" && "$DOTNET" build -c Release -o bin/core >/dev/null)

    CSHARP_CORE_DIR="$CORE_ROOT/hust_csharp"
    mkdir -p "$CSHARP_CORE_DIR"
    for file in hust_csharp.dll hust_csharp.deps.json hust_csharp.runtimeconfig.json; do
        if [ -f "$REPO_ROOT/Hust-CSharp/bin/core/$file" ]; then
            cp "$REPO_ROOT/Hust-CSharp/bin/core/$file" "$CSHARP_CORE_DIR/"
        fi
    done

    # `dotnet build` produces an apphost that needs DOTNET_ROOT; a launcher is
    # relocatable and matches the core contract.
    cat > "$CSHARP_CORE_DIR/hust_csharp" <<'LAUNCHER'
#!/usr/bin/env bash
set -e
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOTNET="${DOTNET_CMD:-}"
if [ -z "$DOTNET" ] || [ ! -x "$DOTNET" ]; then
  if [ -x "$HOME/.dotnet/dotnet" ]; then
    DOTNET="$HOME/.dotnet/dotnet"
  else
    DOTNET="$(command -v dotnet || true)"
  fi
fi
[ -n "$DOTNET" ] || { echo "error: dotnet not found" >&2; exit 3; }
export DOTNET_ROOT="${DOTNET_ROOT:-$(dirname "$DOTNET")}"
exec "$DOTNET" "$DIR/hust_csharp.dll" "$@"
LAUNCHER
    chmod +x "$CSHARP_CORE_DIR/hust_csharp"
    echo "  Installed: $CSHARP_CORE_DIR/hust_csharp"
fi

# 5. Go core
echo "[5/5] Building Go core (hust_go)..."
GO="$(find_go)"
if [ -z "$GO" ]; then
    echo "  warning: go not found, skipping Go core"
else
    (cd "$REPO_ROOT/Hust-Go" && "$GO" build -o hust_go ./src)

    GO_CORE_DIR="$CORE_ROOT/hust_go"
    mkdir -p "$GO_CORE_DIR/include" "$GO_CORE_DIR/lib"
    cp "$REPO_ROOT/Hust-Go/hust_go" "$GO_CORE_DIR/hust_go"
    echo "  Installed: $GO_CORE_DIR/hust_go"
fi

echo ""
echo "=== Installation Complete ==="
echo ""
echo "Directory structure:"
echo "  $INSTALL_DIR/hustc"
echo "  $HUST_WORK/"
echo "  ├── hustc/                  (reserved for conductor extras)"
echo "  └── hust_core/"
echo "      ├── hust_c/"
echo "      ├── hust_rust/"
if [ -n "${CSHARP_CORE_DIR:-}" ]; then
    echo "      ├── hust_csharp/"
fi
if [ -n "${GO_CORE_DIR:-}" ]; then
    echo "      └── hust_go/"
fi
echo ""
echo "Verify installation:"
echo "  $INSTALL_DIR/hustc --version"
echo "  $INSTALL_DIR/hustc --cores"
echo ""
