#!/bin/bash
# Core protocol smoke test for Hust-Go.
#
# Usage:
#   ./tests/core_contract.sh
#   HUST_GO_BIN=/path/to/hust_go GO_BIN=/path/to/go ./tests/core_contract.sh

set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

GO_BIN="${GO_BIN:-}"
if [ -z "$GO_BIN" ] || [ ! -x "$GO_BIN" ]; then
    if command -v go >/dev/null 2>&1; then
        GO_BIN="$(command -v go)"
    elif [ -x "$HOME/go-sdk/go/bin/go" ]; then
        GO_BIN="$HOME/go-sdk/go/bin/go"
    else
        fail "go toolchain not found"
    fi
fi

CORE="${HUST_GO_BIN:-$ROOT/hust_go}"
[ -x "$CORE" ] || fail "hust_go binary not found: $CORE"

info="$("$CORE" info)"
case "$info" in
    *'"protocol":1'*'"name":"go"'*'"transpile":true'*'"build":true'*'"toolchain":true'*) ;;
    *) fail "unexpected info: $info" ;;
esac

cat > "$TMP/main.hust" <<'EOF'
use GO.fmt.*;

void main() {
    i32[] nums = {1, 2, 3};
    i32 sum = 0;
    for (i32 i = 0; i < nums.len(); i++) {
        sum = sum + nums[i];
    }
    fmt.Println("sum=" + sum);
}
EOF

"$CORE" transpile "$TMP/main.hust" --out-dir "$TMP/out" > /dev/null
[ -f "$TMP/out/main.go" ] || fail "transpile did not write main.go"
grep -q "package main" "$TMP/out/main.go" || fail "generated file is not a Go program"

stdout="$("$CORE" build "$TMP/main.hust" --out-dir "$TMP/build" 2> "$TMP/build.err")"
[ "$(printf '%s\n' "$stdout" | wc -l)" -eq 1 ] || fail "build stdout is not a single line: $stdout"
exe="${stdout#executable=}"
[ "$exe" != "$stdout" ] || fail "build did not report executable: $stdout"
[ -x "$exe" ] || fail "reported executable is missing: $exe"
result="$("$exe")"
[ "$result" = "sum=6" ] || fail "unexpected program output: $result"

debug_out="$("$CORE" build "$TMP/main.hust" --out-dir "$TMP/debug" --debug 2> /dev/null)"
debug_exe="${debug_out#executable=}"
[ -x "$debug_exe" ] || fail "debug build did not produce an executable: $debug_out"

# Dialect error must be teaching-style and exit 4.
cat > "$TMP/cdialect.hust" <<'EOF'
use C.stdio.*;

void main() {
    printf("no");
}
EOF
set +e
"$CORE" transpile "$TMP/cdialect.hust" --out-dir "$TMP/cout" > /dev/null 2> "$TMP/cdialect.err"
code=$?
set -e
[ "$code" -eq 4 ] || fail "C dialect on Go core should exit 4, got $code"
grep -q "Unsupported dialect for Go core" "$TMP/cdialect.err" || fail "dialect error lacks an actionable message"

mkdir -p "$TMP/project"
cat > "$TMP/project/settings.config" <<'EOF'
[project]
name = contract_project
entry = main.hust
core = go
EOF
cat > "$TMP/project/main.hust" <<'EOF'
use GO.fmt.*;

void main() {
    fmt.Println("project ok");
}
EOF
"$CORE" build "$TMP/project" --project --out-dir "$TMP/project-out" > /dev/null 2>&1 \
    || fail "project build failed"
[ -x "$TMP/project-out/main" ] || fail "project executable missing"
project_result="$("$TMP/project-out/main")"
[ "$project_result" = "project ok" ] || fail "unexpected project output: $project_result"

echo "PASS: Hust-Go core contract"
