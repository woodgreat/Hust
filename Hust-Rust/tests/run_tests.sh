#!/bin/bash
# Full Hust-Rust test runner.
#
# Usage:
#   ./tests/run_tests.sh            # baseline-aware: documented known failures do not fail
#   ./tests/run_tests.sh --strict   # require a fully green suite
#   ./tests/run_tests.sh --quick    # skip unit, contract, and runtime checks
#
# Environment:
#   HUST_RUST_BIN   core binary (default: target/debug/hust_rust)
#
# Covered:
#   - top-level .hust files (build, except expect_fail_* which must fail)
#   - directories with settings.config (project build)
#   - cargo unit tests
#   - core_contract.sh
#   - representative runtime samples
#
# Project directories are copied to a temporary directory before building, so
# the runner never overwrites tracked binaries or leaves artifacts in tests/.

set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HUST_RUST="${HUST_RUST_BIN:-$ROOT/target/debug/hust_rust}"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

STRICT=0
QUICK=0
for arg in "$@"; do
    case "$arg" in
        --strict) STRICT=1 ;;
        --quick) QUICK=1 ;;
        -h|--help)
            sed -n '2,22p' "$0"
            exit 0
            ;;
        *)
            echo "Error: unknown option '$arg'" >&2
            exit 2
            ;;
    esac
done

# Documented pre-existing failures. Remove entries when they are fixed.
KNOWN_PROJECT_FAILURES=(
    "interface_var_test/multiblank_param"
    "shape_interface_test/container_pairmap_shape"
)
KNOWN_UNEXPECTED_PASS=(
    "private_field_external_test.hust"
    "unrecognized_use_test.hust"
)
RUNTIME_TESTS=(
    "sample.hust"
    "class_simple.hust"
    "inheritance_multi.hust"
    "array_2d_test.hust"
    "string_test.hust"
    "ctor_init_test"
    "module_test"
    "use_test/use_alias"
)

pass=0
fail=0
known=0
fixed=0
neg_ok=0
runtime_pass=0
unit_result="skipped"

contains() {
    local needle="$1"
    shift
    local item
    for item in "$@"; do
        [ "$item" = "$needle" ] && return 0
    done
    return 1
}

copy_project() {
    local rel="$1"
    local dest="$TMP/projects/$rel"
    mkdir -p "$dest"
    cp -a "$ROOT/tests/$rel/." "$dest/"
    printf '%s' "$dest"
}

run_build() {
    timeout 120 "$HUST_RUST" build "$1" > "$TMP/out" 2>&1
}

echo "==> Building hust_rust"
if ! (cd "$ROOT" && cargo build --offline > "$TMP/build.log" 2>&1); then
    tail -30 "$TMP/build.log" >&2
    echo "FAIL: cargo build --offline" >&2
    exit 1
fi
[ -x "$HUST_RUST" ] || {
    echo "FAIL: core binary not found: $HUST_RUST" >&2
    exit 1
}

echo "==> Top-level files"
for file in "$ROOT"/tests/*.hust; do
    base="$(basename "$file")"
    if [[ "$base" == expect_fail_* ]]; then
        if run_build "$file"; then
            echo "FAIL     $base (expected failure, got success)"
            fail=$((fail + 1))
        else
            neg_ok=$((neg_ok + 1))
        fi
        continue
    fi

    if contains "$base" "${KNOWN_UNEXPECTED_PASS[@]}"; then
        if run_build "$file" && grep -q "Build complete" "$TMP/out"; then
            echo "KNOWN    $base (should fail, currently succeeds)"
            known=$((known + 1))
        else
            echo "FIXED    $base (now fails as documented)"
            fixed=$((fixed + 1))
        fi
        continue
    fi

    if run_build "$file" && grep -q "Build complete" "$TMP/out"; then
        pass=$((pass + 1))
    else
        echo "FAIL     $base"
        fail=$((fail + 1))
    fi
done

echo "==> Project directories"
while IFS= read -r config; do
    dir="$(dirname "$config")"
    rel="${dir#"$ROOT"/tests/}"
    target="$(copy_project "$rel")"

    if run_build "$target" && grep -q "Build complete" "$TMP/out"; then
        if contains "$rel" "${KNOWN_PROJECT_FAILURES[@]}"; then
            echo "FIXED    $rel (project now passes)"
            fixed=$((fixed + 1))
        else
            pass=$((pass + 1))
        fi
    else
        if contains "$rel" "${KNOWN_PROJECT_FAILURES[@]}"; then
            echo "KNOWN    $rel (documented failure)"
            known=$((known + 1))
        else
            echo "FAIL     $rel (project)"
            fail=$((fail + 1))
        fi
    fi
done < <(find "$ROOT/tests" -name settings.config \
    | grep -v '/archive/' \
    | grep -v '/obsolete/' \
    | sort)

if [ "$QUICK" -eq 0 ]; then
    echo "==> Unit tests"
    if (cd "$ROOT" && cargo test --offline --lib > "$TMP/unit.log" 2>&1); then
        unit_result="$(grep -E '^test result:' "$TMP/unit.log" | tail -1)"
        echo "PASS     $unit_result"
    else
        tail -30 "$TMP/unit.log" >&2
        echo "FAIL     unit tests"
        fail=$((fail + 1))
        unit_result="failed"
    fi

    echo "==> Core contract"
    if HUST_RUST_BIN="$HUST_RUST" "$ROOT/tests/core_contract.sh" > "$TMP/contract.log" 2>&1; then
        echo "PASS     core_contract.sh"
    else
        cat "$TMP/contract.log" >&2
        echo "FAIL     core_contract.sh"
        fail=$((fail + 1))
    fi

    echo "==> Runtime samples"
    for rel in "${RUNTIME_TESTS[@]}"; do
        target="$ROOT/tests/$rel"
        if [ -d "$target" ]; then
            target="$(copy_project "$rel")"
        fi
        if timeout 60 "$HUST_RUST" run "$target" > "$TMP/run.log" 2>&1; then
            runtime_pass=$((runtime_pass + 1))
        else
            echo "FAIL     run $rel"
            tail -5 "$TMP/run.log"
            fail=$((fail + 1))
        fi
    done
fi

echo
echo "=== Hust-Rust test summary ==="
printf "Pass:                %d\n" "$pass"
printf "Negative expected:   %d\n" "$neg_ok"
printf "Runtime:             %d\n" "$runtime_pass"
printf "Unit tests:          %s\n" "${unit_result#test result: }"
printf "Known failures:      %d\n" "$known"
printf "Newly fixed:         %d\n" "$fixed"
printf "Unexpected failures: %d\n" "$fail"

if [ "$fixed" -gt 0 ]; then
    echo
    echo "Note: some documented failures now pass. Update the known-failure lists."
fi

if [ "$fail" -ne 0 ]; then
    echo "Result: FAIL"
    exit 1
fi
if [ "$STRICT" -eq 1 ] && [ "$known" -ne 0 ]; then
    echo "Result: FAIL (--strict, $known known failure(s) remain)"
    exit 1
fi
if [ "$known" -gt 0 ]; then
    echo "Result: PASS (known baseline unchanged)"
else
    echo "Result: PASS"
fi
