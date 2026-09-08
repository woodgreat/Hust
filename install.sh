#!/usr/bin/env bash
# Hust installer/updater — build release and install to ~/.local/bin
# Usage: ./install.sh   (from Hust repo root, or any path)
set -e

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO_ROOT/Hust-Rust"

# Build (sandbox-safe CARGO_HOME; harmless if ~/.cargo is writable)
CARGO_HOME="${CARGO_HOME:-/tmp/cargo-home}" cargo build --release

BIN="$REPO_ROOT/Hust-Rust/target/release/hust"
[ -x "$BIN" ] || { echo "build failed: $BIN missing"; exit 1; }

mkdir -p ~/.local/bin
cp "$BIN" ~/.local/bin/hust

echo "Installed:"
hust --version 2>/dev/null || "$HOME/.local/bin/hust" --version
