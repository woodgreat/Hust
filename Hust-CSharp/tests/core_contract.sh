#!/bin/bash
# Core protocol smoke test for Hust-CSharp.
#
# Usage:
#   ./tests/core_contract.sh
#   HUST_CSHARP_BIN=/path/to/hust_csharp ./tests/core_contract.sh
#   HUST_CSHARP_DLL=/path/to/hust_csharp.dll ./tests/core_contract.sh
#
# Covers info, transpile, build (release/debug), stdout discipline, dialect
# errors, project mode, and the generated executable.

set -eu

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

DOTNET="${DOTNET_CMD:-$HOME/.dotnet/dotnet}"
if [ ! -x "$DOTNET" ]; then
    DOTNET="$(command -v dotnet || true)"
fi

if [ -n "${HUST_CSHARP_BIN:-}" ]; then
    CORE=("$HUST_CSHARP_BIN")
elif [ -n "${HUST_CSHARP_DLL:-}" ]; then
    [ -n "$DOTNET" ] || fail "dotnet not found"
    CORE=("$DOTNET" "$HUST_CSHARP_DLL")
elif [ -f "$ROOT/bin/core/hust_csharp.dll" ]; then
    # Prefer the managed dll: the apphost needs DOTNET_ROOT, the dll does not.
    [ -n "$DOTNET" ] || fail "dotnet not found"
    CORE=("$DOTNET" "$ROOT/bin/core/hust_csharp.dll")
elif [ -x "$ROOT/bin/core/hust_csharp" ]; then
    CORE=("$ROOT/bin/core/hust_csharp")
else
    fail "hust_csharp not found; build it with dotnet build -c Release -o bin/core"
fi

run_core() {
    "${CORE[@]}" "$@"
}

info="$(run_core info)"
case "$info" in
    *'"protocol":1'*'"name":"csharp"'*'"transpile":true'*'"build":true'*) ;;
    *) fail "unexpected info: $info" ;;
esac

cat > "$TMP/main.hust" <<'EOF'
use CSHARP.Console.*;

void main() {
    i32 x = 6;
    i32 y = 7;
    Console.WriteLine("product: " + (x * y));
}
EOF

run_core transpile "$TMP/main.hust" --out-dir "$TMP/out" > /dev/null
[ -f "$TMP/out/main.cs" ] || fail "transpile did not write main.cs"

stdout="$(run_core build "$TMP/main.hust" --out-dir "$TMP/build" 2> "$TMP/build.err")"
[ "$(printf '%s\n' "$stdout" | wc -l)" -eq 1 ] || fail "build stdout is not a single line: $stdout"
exe="${stdout#executable=}"
[ "$exe" != "$stdout" ] || fail "build did not report executable: $stdout"
[ -x "$exe" ] || fail "reported executable is missing: $exe"
result="$("$exe")"
[ "$result" = "product: 42" ] || fail "unexpected program output: $result"

debug_out="$(run_core build "$TMP/main.hust" --out-dir "$TMP/debug" --debug 2> /dev/null)"
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
run_core transpile "$TMP/cdialect.hust" --out-dir "$TMP/cout" > /dev/null 2> "$TMP/cdialect.err"
code=$?
set -e
[ "$code" -eq 4 ] || fail "C dialect on C# core should exit 4, got $code"
grep -q "C\.\*" "$TMP/cdialect.err" || fail "C dialect error lacks an actionable message"

mkdir -p "$TMP/project"
cat > "$TMP/project/settings.config" <<'EOF'
[project]
name = contract_project
entry = main.hust
core = csharp
EOF
cat > "$TMP/project/main.hust" <<'EOF'
use CSHARP.Console.*;

void main() {
    Console.WriteLine("project ok");
}
EOF
run_core build "$TMP/project/main.hust" --project --out-dir "$TMP/project-out" > /dev/null 2>&1 \
    || fail "project build failed"
project_exe="$TMP/project-out/publish/main"
[ -x "$project_exe" ] || fail "project executable missing: $project_exe"
project_result="$("$project_exe")"
[ "$project_result" = "project ok" ] || fail "unexpected project output: $project_result"

echo "PASS: Hust-CSharp core contract"
