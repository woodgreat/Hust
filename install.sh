#!/usr/bin/env bash
# Hust installer/updater - build release artifacts and install them.
# Usage: ./install.sh
#   PREFIX=/custom/prefix ./install.sh
set -e

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
INCLUDE_DIR="$PREFIX/include"
CARGO_HOME="${CARGO_HOME:-/tmp/cargo-home}"
export CARGO_HOME

echo "==> Building Conductor"
(cd "$REPO_ROOT/Conductor" && cargo build --release)

echo "==> Building Rust core"
(cd "$REPO_ROOT/Hust-Rust" && cargo build --release)

echo "==> Building C core"
(cd "$REPO_ROOT/Hust-C" && make)

for artifact in \
    "$REPO_ROOT/Conductor/target/release/hustc" \
    "$REPO_ROOT/Hust-Rust/target/release/hust_rust" \
    "$REPO_ROOT/Hust-C/hust_c"; do
    [ -x "$artifact" ] || { echo "build failed: $artifact missing" >&2; exit 1; }
done

mkdir -p "$BIN_DIR" "$INCLUDE_DIR"
install -m 755 "$REPO_ROOT/Conductor/target/release/hustc" "$BIN_DIR/hustc"
install -m 755 "$REPO_ROOT/Hust-Rust/target/release/hust_rust" "$BIN_DIR/hust_rust"
install -m 755 "$REPO_ROOT/Hust-C/hust_c" "$BIN_DIR/hust_c"

# Installed hust_c finds runtime headers at <exe_dir>/../include.
install -m 644 "$REPO_ROOT/Hust-C/include/hust_error.h" "$INCLUDE_DIR/hust_error.h"
install -m 644 "$REPO_ROOT/Hust-C/include/hust_vec.h" "$INCLUDE_DIR/hust_vec.h"
install -m 644 "$REPO_ROOT/Hust-C/include/hust_string.h" "$INCLUDE_DIR/hust_string.h"

echo
echo "Installed:"
echo "  $BIN_DIR/hustc"
echo "  $BIN_DIR/hust_rust"
echo "  $BIN_DIR/hust_c"
echo "  $INCLUDE_DIR/hust_error.h"
echo "  $INCLUDE_DIR/hust_vec.h"
echo "  $INCLUDE_DIR/hust_string.h"

if [ -e "$BIN_DIR/hust" ]; then
    echo
    echo "Note: the retired '$BIN_DIR/hust' binary is still present."
    echo "      Remove it manually; use '$BIN_DIR/hustc' from now on."
fi

echo
echo "Self-check:"
"$BIN_DIR/hustc" --cores
