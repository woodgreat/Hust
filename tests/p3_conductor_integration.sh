#!/bin/bash
# P3 Conductor integration test with the real Rust and C cores.
#
# Prerequisites (build once):
#   (cd Conductor && cargo build)
#   (cd Hust-Rust && cargo build)
#   (cd Hust-C && make)
#
# Usage:
#   ./tests/p3_conductor_integration.sh
#   HUSTC_BIN=... HUST_RUST_BIN=... HUST_C_BIN=... ./tests/p3_conductor_integration.sh
#
# Covers:
#   - core discovery (--cores)
#   - pure Hust run on both cores
#   - Rust dialect on the Rust core, C dialect on the C core
#   - build with -o and installed executable
#   - check on both cores
#   - auto selection does not fall back on a dialect error
#   - legacy core `run` and `hust_c <entry> -o` still work

set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HUSTC="${HUSTC_BIN:-$ROOT/Conductor/target/debug/hustc}"
HUST_RUST="${HUST_RUST_BIN:-$ROOT/Hust-Rust/target/debug/hust_rust}"
HUST_C="${HUST_C_BIN:-$ROOT/Hust-C/hust_c}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

[ -x "$HUSTC" ] || fail "hustc not found: $HUSTC (run cargo build in Conductor)"
[ -x "$HUST_RUST" ] || fail "Rust core not found: $HUST_RUST (run cargo build in Hust-Rust)"
[ -x "$HUST_C" ] || fail "C core not found: $HUST_C (run make in Hust-C)"
command -v cargo > /dev/null || fail "cargo not found in PATH"
command -v gcc > /dev/null || fail "gcc not found in PATH"

# Keep PATH deterministic: only toolchain directories plus an empty bin
# directory, so discovery must use HUST_RUST_BIN / HUST_C_BIN below.
tool_path=""
for tool in cargo rustc gcc; do
    resolved="$(command -v "$tool" || true)"
    [ -n "$resolved" ] || fail "$tool not found in PATH"
    tool_dir="$(dirname "$resolved")"
    case ":$tool_path:" in
        *":$tool_dir:"*) ;;
        *) tool_path="${tool_path:+$tool_path:}$tool_dir" ;;
    esac
done
mkdir -p "$TMP/bin"
export PATH="$TMP/bin:$tool_path:/usr/bin:/bin"

export HUST_RUST_BIN="$HUST_RUST"
export HUST_C_BIN="$HUST_C"

make_project() {
    local dir="$1"
    local name="$2"
    local core="$3"
    local source="$4"
    mkdir -p "$dir"
    cat > "$dir/settings.config" <<EOF
[project]
name = $name
version = 0.1.0
entry = main.hust
core = $core
EOF
    printf '%s\n' "$source" > "$dir/main.hust"
}

make_project "$TMP/pure" pure_app auto \
'void main() {
    i32 x = 40 + 2;
}'

make_project "$TMP/rust" rust_app rust \
'fn main() {
    println!("rust ok");
}'

make_project "$TMP/c" c_app c \
'use C.stdio.*;
void main() {
    printf("c ok\n");
}'

make_project "$TMP/c_auto" c_auto auto \
'use C.stdio.*;
void main() {
    printf("should not run\n");
}'

cores="$("$HUSTC" --cores)"
case "$cores" in
    *"rust: available"*) ;;
    *) fail "Rust core not listed as available: $cores" ;;
esac
case "$cores" in
    *"c: available"*) ;;
    *) fail "C core not listed as available: $cores" ;;
esac

# Pure Hust must run on both cores.
"$HUSTC" run "$TMP/pure" --core-rust > "$TMP/pure-rust.out" 2> "$TMP/pure-rust.err" \
    || fail "pure Hust failed on the Rust core"
[ ! -s "$TMP/pure-rust.out" ] || fail "pure Hust produced unexpected Rust output"

"$HUSTC" run "$TMP/pure" --core-c > "$TMP/pure-c.out" 2> "$TMP/pure-c.err" \
    || fail "pure Hust failed on the C core"
[ ! -s "$TMP/pure-c.out" ] || fail "pure Hust produced unexpected C output"

# Dialect-specific programs.
rust_out="$("$HUSTC" run "$TMP/rust" --core-rust 2> "$TMP/rust.err")"
[ "$rust_out" = "rust ok" ] || fail "unexpected Rust core output: $rust_out"

c_out="$("$HUSTC" run "$TMP/c" --core-c 2> "$TMP/c.err")"
[ "$c_out" = "c ok" ] || fail "unexpected C core output: $c_out"

# check must not invoke the target compiler and must succeed.
"$HUSTC" check "$TMP/rust" --core-rust > /dev/null 2>&1 \
    || fail "check failed on the Rust core"
"$HUSTC" check "$TMP/c" --core-c > /dev/null 2>&1 \
    || fail "check failed on the C core"

# build -o installs the executable at the requested path.
rust_line="$("$HUSTC" build "$TMP/rust" --core-rust -o "$TMP/out/rust-app" 2> "$TMP/rust-build.err")"
rust_exe="${rust_line#executable=}"
[ "$rust_exe" != "$rust_line" ] || fail "Rust build did not report executable"
[ -x "$rust_exe" ] || fail "Rust executable missing: $rust_exe"
[ "$("$rust_exe")" = "rust ok" ] || fail "installed Rust executable failed"

c_line="$("$HUSTC" build "$TMP/c" --core-c -o "$TMP/out/c-app" 2> "$TMP/c-build.err")"
c_exe="${c_line#executable=}"
[ "$c_exe" != "$c_line" ] || fail "C build did not report executable"
[ -x "$c_exe" ] || fail "C executable missing: $c_exe"
[ "$("$c_exe")" = "c ok" ] || fail "installed C executable failed"

# auto picks Rust first; a C dialect project must fail without falling back.
set +e
"$HUSTC" run "$TMP/c_auto" > "$TMP/auto.out" 2> "$TMP/auto.err"
auto_status=$?
set -e
[ "$auto_status" -eq 5 ] || fail "expected auto dialect failure (exit 5), got $auto_status"
case "$(cat "$TMP/auto.err")" in
    *"C dialect import"*"--core-c"*) ;;
    *) fail "missing C dialect hint: $(cat "$TMP/auto.err")" ;;
esac
case "$(cat "$TMP/auto.out")" in
    *"should not run"*) fail "auto unexpectedly fell back to the C core" ;;
esac

# Legacy commands must remain usable.
legacy_rust="$("$HUST_RUST" run "$TMP/rust/main.hust" 2>&1)"
case "$legacy_rust" in
    *"rust ok"*) ;;
    *) fail "legacy Rust run failed: $legacy_rust" ;;
esac

"$HUST_C" "$TMP/c/main.hust" -o "$TMP/legacy.c"
gcc -std=c99 -I "$ROOT/Hust-C/include" "$TMP/legacy.c" -o "$TMP/legacy" -lm
[ "$("$TMP/legacy")" = "c ok" ] || fail "legacy C transpile output mismatch"

echo "PASS: P3 Conductor real-core integration"
