#!/bin/bash
# Full repository regression: Conductor + Hust-Rust + Hust-C + P3 + install.
#
# Usage:
#   ./tests/run_all.sh                    # full default gate (no install rebuild)
#   ./tests/run_all.sh --strict           # Hust-Rust must be fully green
#   ./tests/run_all.sh --with-install     # also install to a temp PREFIX and smoke it
#   ./tests/run_all.sh --quick            # skip Hust-Rust unit/runtime and install
#   ./tests/run_all.sh --keep-logs        # keep logs even on success
#
# Logs are kept automatically on failure and printed at the end.

set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d)"
LOGS="$TMP/logs"
mkdir -p "$LOGS"

STRICT=0
WITH_INSTALL=0
QUICK=0
KEEP_LOGS=0

for arg in "$@"; do
    case "$arg" in
        --strict) STRICT=1 ;;
        --with-install) WITH_INSTALL=1 ;;
        --quick) QUICK=1 ;;
        --keep-logs) KEEP_LOGS=1 ;;
        -h|--help)
            sed -n '2,11p' "$0"
            exit 0
            ;;
        *)
            echo "Error: unknown option '$arg'" >&2
            exit 2
            ;;
    esac
done

if [ "$QUICK" -eq 1 ]; then
    WITH_INSTALL=0
fi

groups=0
failures=0
failed_names=""

cleanup() {
    if [ "$KEEP_LOGS" -eq 1 ] || [ "$failures" -ne 0 ]; then
        echo
        echo "Logs kept: $LOGS"
    else
        rm -rf "$TMP"
    fi
}
trap cleanup EXIT

run_step() {
    local name="$1"
    local log="$2"
    shift 2
    groups=$((groups + 1))
    echo
    echo "==> $name"
    if "$@" > "$log" 2>&1; then
        echo "PASS $name"
    else
        echo "FAIL $name"
        echo "     log: $log"
        tail -20 "$log" | sed 's/^/     /'
        failures=$((failures + 1))
        failed_names="$failed_names\n  - $name"
    fi
}

step_conductor() {
    (cd "$ROOT/Conductor" && cargo test)
}

step_rust() {
    local args=""
    [ "$STRICT" -eq 1 ] && args="$args --strict"
    [ "$QUICK" -eq 1 ] && args="$args --quick"
    (cd "$ROOT/Hust-Rust" && ./tests/run_tests.sh $args)
}

step_c_build() {
    (cd "$ROOT/Hust-C" && make)
}

step_c_tests() {
    (cd "$ROOT/Hust-C" && ./tests/run_tests.sh)
}

step_c_contract() {
    (cd "$ROOT/Hust-C" && make contract-tests)
}

step_p3() {
    "$ROOT/tests/p3_conductor_integration.sh"
}

step_install() {
    local prefix="$TMP/prefix"
    local rust_dir="$TMP/installed/rust"
    local c_dir="$TMP/installed/c"
    local rust_core="$prefix/bin/hust_work/hust_core/hust_rust"
    local c_core="$prefix/bin/hust_work/hust_core/hust_c"
    local tool_path
    local cores
    local output

    rm -rf "$prefix" "$TMP/installed"
    PREFIX="$prefix" "$ROOT/install.sh"

    [ -x "$prefix/bin/hustc" ] || return 1
    [ -x "$rust_core/hust_rust" ] || return 1
    [ -x "$c_core/hust_c" ] || return 1
    [ -f "$c_core/include/hust_vec.h" ] || return 1
    [ ! -e "$prefix/bin/hust" ] || return 1

    tool_path="$(dirname "$(command -v cargo)"):$(dirname "$(command -v rustc)"):$(dirname "$(command -v gcc)"):/usr/bin:/bin"
    export PATH="$prefix/bin:$tool_path"

    cores="$("$prefix/bin/hustc" --cores)"
    case "$cores" in *"rust: available"*) ;; *) return 1 ;; esac
    case "$cores" in *"c: available"*) ;; *) return 1 ;; esac

    mkdir -p "$rust_dir" "$c_dir"
    cat > "$rust_dir/settings.config" <<'EOF'
[project]
name = install_rust
version = 0.1.0
entry = main.hust
core = rust
EOF
    cat > "$rust_dir/main.hust" <<'EOF'
fn main() {
    println!("installed rust ok");
}
EOF

    cat > "$c_dir/settings.config" <<'EOF'
[project]
name = install_c
version = 0.1.0
entry = main.hust
core = c
EOF
    cat > "$c_dir/main.hust" <<'EOF'
use C.stdio.*;
void main() {
    printf("installed c ok\n");
}
EOF

    output="$(cd "$rust_dir" && "$prefix/bin/hustc" run . --core-rust)" || return 1
    [ "$output" = "installed rust ok" ] || return 1

    output="$(cd "$c_dir" && "$prefix/bin/hustc" run . --core-c)" || return 1
    [ "$output" = "installed c ok" ] || return 1

    (cd "$c_dir" && "$prefix/bin/hustc" build . --core-c -o bin/app > /dev/null) || return 1
    output="$("$c_dir/bin/app")" || return 1
    [ "$output" = "installed c ok" ] || return 1
}

run_step "Conductor tests" "$LOGS/conductor.log" step_conductor
run_step "Hust-Rust suite" "$LOGS/rust.log" step_rust
run_step "Hust-C build" "$LOGS/c-build.log" step_c_build
run_step "Hust-C suite" "$LOGS/c.log" step_c_tests
run_step "Hust-C core contract" "$LOGS/c-contract.log" step_c_contract
run_step "P3 real-core integration" "$LOGS/p3.log" step_p3
if [ "$WITH_INSTALL" -eq 1 ]; then
    run_step "Install smoke" "$LOGS/install.log" step_install
fi

if [ -s "$LOGS/rust.log" ]; then
    echo
    echo "Hust-Rust baseline:"
    grep -E "Known failures:|Unexpected failures:|Result:" "$LOGS/rust.log" | sed 's/^/  /'
fi

echo
echo "=== Full regression summary ==="
printf "Groups:   %d\n" "$groups"
printf "Failures: %d\n" "$failures"
if [ "$failures" -ne 0 ]; then
    printf "Failed:%b\n" "$failed_names"
    echo "Result: FAIL"
    exit 1
fi
echo "Result: PASS"
