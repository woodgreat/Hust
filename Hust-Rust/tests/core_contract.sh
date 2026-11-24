#!/bin/bash
# Core protocol smoke test for Hust-Rust.
#
# Usage:
#   ./tests/core_contract.sh
#   HUST_BIN=/path/to/hust ./tests/core_contract.sh
#
# Covers info/transpile/build (release and debug), project mode, and the
# legacy `run` command.

set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HUST="${HUST_BIN:-$ROOT/target/debug/hust}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

[ -x "$HUST" ] || fail "hust binary not found: $HUST"

info="$("$HUST" info)"
case "$info" in
    *'"protocol":1'*'"name":"rust"'*'"transpile":true'*'"build":true'*) ;;
    *) fail "unexpected info: $info" ;;
esac

mkdir -p "$TMP/src"
cat > "$TMP/src/main.hust" <<'EOF'
fn main() {
    println!("contract ok");
}
EOF

"$HUST" transpile "$TMP/src/main.hust" --out-dir "$TMP/out" > /dev/null
[ -f "$TMP/out/Cargo.toml" ] || fail "transpile did not write Cargo.toml"
[ -f "$TMP/out/src/main.rs" ] || fail "transpile did not write src/main.rs"

build_out="$("$HUST" build "$TMP/src/main.hust" --out-dir "$TMP/build" 2> /dev/null)"
exe="${build_out#executable=}"
[ "$exe" != "$build_out" ] || fail "build did not report executable: $build_out"
[ -x "$exe" ] || fail "reported executable is missing: $exe"
result="$("$exe")"
[ "$result" = "contract ok" ] || fail "unexpected program output: $result"

debug_out="$("$HUST" build "$TMP/src/main.hust" --out-dir "$TMP/debug" --debug 2> /dev/null)"
case "$debug_out" in
    *"/target/debug/main") ;;
    *) fail "debug build did not use the debug profile: $debug_out" ;;
esac

# C dialect code must not be silently treated as a missing module.
cat > "$TMP/c_dialect.hust" <<'EOF'
use C.stdio.*;
void main() {
    printf("wrong core\n");
}
EOF
if dialect_out="$("$HUST" build "$TMP/c_dialect.hust" --out-dir "$TMP/c-dialect-out" 2>&1)"; then
    fail "C dialect code unexpectedly built on the Rust core"
fi
case "$dialect_out" in
    *"cannot run on the Rust core"*"--core-c"*) ;;
    *) fail "missing C dialect hint: $dialect_out" ;;
esac

mkdir -p "$TMP/project"
cat > "$TMP/project/settings.config" <<'EOF'
[project]
name = contract_project
version = 0.1.0
entry = main.hust
EOF
cat > "$TMP/project/main.hust" <<'EOF'
fn main() {
    println!("project ok");
}
EOF
"$HUST" build "$TMP/project/main.hust" --project --out-dir "$TMP/project-out" > /dev/null 2>&1 \
    || fail "project build failed"
[ -x "$TMP/project-out/target/release/contract_project" ] \
    || fail "project executable missing"

legacy="$("$HUST" run "$TMP/src/main.hust" 2>&1)"
case "$legacy" in
    *"contract ok"*) ;;
    *) fail "legacy run failed: $legacy" ;;
esac

echo "PASS: Hust-Rust core contract"
