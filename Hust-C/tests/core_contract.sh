#!/bin/bash
# Core protocol smoke test for Hust-C.
#
# Usage:
#   ./tests/core_contract.sh
#   HUST_C_BIN=/path/to/hust_c ./tests/core_contract.sh
#
# Covers info/transpile/build (release and debug), project mode, and the
# legacy `hust_c <entry> -o <file>` command.

set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HUST_C="${HUST_C_BIN:-$ROOT/hust_c}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

[ -x "$HUST_C" ] || fail "hust_c binary not found: $HUST_C"

info="$("$HUST_C" info)"
case "$info" in
    *'"protocol":1'*'"name":"c"'*'"transpile":true'*'"build":true'*) ;;
    *) fail "unexpected info: $info" ;;
esac

cat > "$TMP/main.hust" <<'EOF'
use C.stdio.*;
use C.math.*;
void main() {
    f64 x = pow(2.0, 3.0);
    printf("%.1f\n", x);
}
EOF

"$HUST_C" transpile "$TMP/main.hust" --out-dir "$TMP/out" > /dev/null
[ -f "$TMP/out/main.c" ] || fail "transpile did not write main.c"
[ -f "$TMP/out/include/hust_vec.h" ] || fail "transpile did not copy runtime headers"

build_out="$("$HUST_C" build "$TMP/main.hust" --out-dir "$TMP/build" 2> /dev/null)"
exe="${build_out#executable=}"
[ "$exe" != "$build_out" ] || fail "build did not report executable: $build_out"
[ -x "$exe" ] || fail "reported executable is missing: $exe"
result="$("$exe")"
[ "$result" = "8.0" ] || fail "unexpected program output: $result"

debug_out="$("$HUST_C" build "$TMP/main.hust" --out-dir "$TMP/debug" --debug 2> /dev/null)"
case "$debug_out" in
    *"executable=$TMP/debug/main") ;;
    *) fail "debug build did not report the expected path: $debug_out" ;;
esac

mkdir -p "$TMP/project"
cat > "$TMP/project/settings.config" <<'EOF'
[project]
name = contract_project
entry = main.hust
core = c
EOF
cat > "$TMP/project/main.hust" <<'EOF'
use C.stdio.*;
void main() {
    printf("project ok\n");
}
EOF
"$HUST_C" build "$TMP/project/main.hust" --project --out-dir "$TMP/project-out" > /dev/null 2>&1 \
    || fail "project build failed"
project_result="$("$TMP/project-out/main")"
[ "$project_result" = "project ok" ] || fail "unexpected project output: $project_result"

"$HUST_C" "$TMP/main.hust" -o "$TMP/legacy.c"
gcc -std=c99 -I "$ROOT/include" "$TMP/legacy.c" -o "$TMP/legacy" -lm
legacy_result="$("$TMP/legacy")"
[ "$legacy_result" = "8.0" ] || fail "legacy transpile output mismatch: $legacy_result"

echo "PASS: Hust-C core contract"
