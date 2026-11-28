#!/bin/bash
# P3 Conductor integration test with the real Rust, C, C# and Go cores.
#
# Prerequisites (build once):
#   (cd Conductor && cargo build)
#   (cd Hust-Rust && cargo build)
#   (cd Hust-C && make)
#   (cd Hust-CSharp && dotnet build -c Release -o bin/core)
#   (cd Hust-Go && go build -o hust_go ./src)
#
# Usage:
#   ./tests/p3_conductor_integration.sh
#   HUSTC_BIN=... HUST_RUST_BIN=... HUST_C_BIN=... HUST_GO_BIN=... ./tests/p3_conductor_integration.sh
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
HUST_GO="${HUST_GO_BIN:-$ROOT/Hust-Go/hust_go}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

[ -x "$HUSTC" ] || fail "hustc not found: $HUSTC (run cargo build in Conductor)"
[ -x "$HUST_RUST" ] || fail "Rust core not found: $HUST_RUST (run cargo build in Hust-Rust)"
[ -x "$HUST_C" ] || fail "C core not found: $HUST_C (run make in Hust-C)"
[ -x "$HUST_GO" ] || fail "Go core not found: $HUST_GO (run go build -o hust_go ./src in Hust-Go)"
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
export HUST_GO_BIN="$HUST_GO"

# The C# core is a managed dll; use a launcher that pins the dotnet host.
DOTNET="${DOTNET_CMD:-$HOME/.dotnet/dotnet}"
if [ ! -x "$DOTNET" ]; then
    DOTNET="$(command -v dotnet || true)"
fi
[ -n "$DOTNET" ] || fail "dotnet not found (set DOTNET_CMD)"
CSHARP_DLL="$ROOT/Hust-CSharp/bin/core/hust_csharp.dll"
[ -f "$CSHARP_DLL" ] || fail "C# core not built: $CSHARP_DLL (run dotnet build -c Release -o bin/core)"
mkdir -p "$TMP/cores"
cat > "$TMP/cores/hust_csharp" <<EOF
#!/bin/bash
exec "$DOTNET" "$CSHARP_DLL" "\$@"
EOF
chmod +x "$TMP/cores/hust_csharp"
export HUST_CSHARP_BIN="$TMP/cores/hust_csharp"

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

make_project "$TMP/csharp" csharp_app csharp \
'use CSHARP.Console.*;
void main() {
    Console.WriteLine("csharp ok");
}'

make_project "$TMP/go" go_app go \
'use GO.fmt.*;
void main() {
    fmt.Println("go ok");
}'

make_project "$TMP/go_auto" go_auto auto \
'use GO.fmt.*;
void main() {
    fmt.Println("should not run");
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
case "$cores" in
    *"csharp: available"*) ;;
    *) fail "C# core not listed as available: $cores" ;;
esac
case "$cores" in
    *"go: available"*) ;;
    *) fail "Go core not listed as available: $cores" ;;
esac

# Pure Hust must run on both cores.
"$HUSTC" run "$TMP/pure" --core-rust > "$TMP/pure-rust.out" 2> "$TMP/pure-rust.err" \
    || fail "pure Hust failed on the Rust core"
[ ! -s "$TMP/pure-rust.out" ] || fail "pure Hust produced unexpected Rust output"

"$HUSTC" run "$TMP/pure" --core-c > "$TMP/pure-c.out" 2> "$TMP/pure-c.err" \
    || fail "pure Hust failed on the C core"
[ ! -s "$TMP/pure-c.out" ] || fail "pure Hust produced unexpected C output"

"$HUSTC" run "$TMP/pure" --core-csharp > "$TMP/pure-csharp.out" 2> "$TMP/pure-csharp.err" \
    || fail "pure Hust failed on the C# core"
[ ! -s "$TMP/pure-csharp.out" ] || fail "pure Hust produced unexpected C# output"

"$HUSTC" run "$TMP/pure" --core-go > "$TMP/pure-go.out" 2> "$TMP/pure-go.err" \
    || fail "pure Hust failed on the Go core"
[ ! -s "$TMP/pure-go.out" ] || fail "pure Hust produced unexpected Go output"

# Dialect-specific programs.
rust_out="$("$HUSTC" run "$TMP/rust" --core-rust 2> "$TMP/rust.err")"
[ "$rust_out" = "rust ok" ] || fail "unexpected Rust core output: $rust_out"

c_out="$("$HUSTC" run "$TMP/c" --core-c 2> "$TMP/c.err")"
[ "$c_out" = "c ok" ] || fail "unexpected C core output: $c_out"

csharp_out="$("$HUSTC" run "$TMP/csharp" --core-csharp 2> "$TMP/csharp.err")"
[ "$csharp_out" = "csharp ok" ] || fail "unexpected C# core output: $csharp_out"

go_out="$("$HUSTC" run "$TMP/go" --core-go 2> "$TMP/go.err")"
[ "$go_out" = "go ok" ] || fail "unexpected Go core output: $go_out"

# check must not invoke the target compiler and must succeed.
"$HUSTC" check "$TMP/rust" --core-rust > /dev/null 2>&1 \
    || fail "check failed on the Rust core"
"$HUSTC" check "$TMP/c" --core-c > /dev/null 2>&1 \
    || fail "check failed on the C core"
"$HUSTC" check "$TMP/csharp" --core-csharp > /dev/null 2>&1 \
    || fail "check failed on the C# core"
"$HUSTC" check "$TMP/go" --core-go > /dev/null 2>&1 \
    || fail "check failed on the Go core"

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

csharp_line="$("$HUSTC" build "$TMP/csharp" --core-csharp -o "$TMP/out/csharp-app" 2> "$TMP/csharp-build.err")"
csharp_exe="${csharp_line#executable=}"
[ "$csharp_exe" != "$csharp_line" ] || fail "C# build did not report executable"
[ -x "$csharp_exe" ] || fail "C# executable missing: $csharp_exe"
[ "$("$csharp_exe")" = "csharp ok" ] || fail "installed C# executable failed"

go_line="$("$HUSTC" build "$TMP/go" --core-go -o "$TMP/out/go-app" 2> "$TMP/go-build.err")"
go_exe="${go_line#executable=}"
[ "$go_exe" != "$go_line" ] || fail "Go build did not report executable"
[ -x "$go_exe" ] || fail "Go executable missing: $go_exe"
[ "$("$go_exe")" = "go ok" ] || fail "installed Go executable failed"

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

# A Go-dialect project must also fail on auto without falling back.
set +e
"$HUSTC" run "$TMP/go_auto" > "$TMP/go-auto.out" 2> "$TMP/go-auto.err"
go_auto_status=$?
set -e
[ "$go_auto_status" -eq 5 ] || fail "expected auto Go dialect failure (exit 5), got $go_auto_status"
case "$(cat "$TMP/go-auto.out")" in
    *"should not run"*) fail "auto unexpectedly fell back to the Go core" ;;
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
