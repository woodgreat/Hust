#!/bin/bash
# Hust-C test runner
# Modes:
#   .expected      -> translate, compile, run, compare stdout
#   .expected.err  -> translate must fail and stderr must contain the text
#   neither        -> translate and compile only
#
# Usage:
#   ./tests/run_tests.sh          # m1..m4 + c_specific
#   ./tests/run_tests.sh m1
#   ./tests/run_tests.sh m2

set -u

TEST_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(dirname "$TEST_DIR")"
HUST_C="$ROOT/hust_c"
OUT_DIR="$TEST_DIR/output"
INCLUDE_DIR="$ROOT/include"

MILESTONE="${1:-all}"
case "$MILESTONE" in
    all) SEARCH_DIRS="m1 m2 m3 m4 c_specific" ;;
    m1|m2|m3|m4|c_specific) SEARCH_DIRS="$MILESTONE" ;;
    *) echo "Usage: $0 [m1|m2|m3|m4|c_specific|all]"; exit 2 ;;
esac

if [ ! -x "$HUST_C" ]; then
    echo "Error: $HUST_C not found. Run 'make' first."
    exit 2
fi

mkdir -p "$OUT_DIR"
rm -rf "$OUT_DIR"/*

total=0
passed=0
failed=0
failed_list=""

fail_test() {
    local name="$1"
    local reason="$2"
    failed=$((failed + 1))
    failed_list="$failed_list\n  $name: $reason"
    printf "FAIL %-52s %s\n" "$name" "$reason"
}

run_one() {
    local src="$1"
    local rel="${src#$TEST_DIR/}"
    local name="${rel%.hust}"
    local base
    base="$(echo "$name" | tr '/' '_')"
    local c_file="$OUT_DIR/${base}.c"
    local exe_file="$OUT_DIR/${base}.exe"
    local tr_err="$OUT_DIR/${base}.translate.err"
    local cc_err="$OUT_DIR/${base}.cc.err"
    local run_out="$OUT_DIR/${base}.stdout"
    local run_err="$OUT_DIR/${base}.run.err"
    local expected="${src%.hust}.expected"
    local expected_err="${src%.hust}.expected.err"
    local src_dir
    src_dir="$(dirname "$src")"
    local src_base
    src_base="$(basename "$src")"

    # Project directories: only main.hust is an entry; module files are loaded by it.
    if [ -f "$src_dir/settings.config" ] && [ "$src_base" != "main.hust" ]; then
        return
    fi

    total=$((total + 1))
    local project_flag=""
    if [ -f "$src_dir/settings.config" ]; then
        project_flag="--project"
    fi

    # 1. Translate
    if ! "$HUST_C" "$src" $project_flag -o "$c_file" > /dev/null 2> "$tr_err"; then
        if [ -f "$expected_err" ]; then
            local needle
            needle="$(head -n 1 "$expected_err")"
            if grep -Fq -- "$needle" "$tr_err"; then
                passed=$((passed + 1))
                printf "PASS %-52s negative\n" "$name"
            else
                fail_test "$name" "negative stderr mismatch"
            fi
            return
        fi
        fail_test "$name" "translate failed"
        return
    fi

    # Negative test must not translate successfully.
    if [ -f "$expected_err" ]; then
        fail_test "$name" "expected transpile error, got success"
        return
    fi

    # Positive tests must not emit translator diagnostics.
    if [ -s "$tr_err" ]; then
        fail_test "$name" "translator stderr not empty"
        return
    fi

    # 2. Compile
    local math_flag=""
    if grep -q "C\.math" "$src"; then
        math_flag="-lm"
    fi
    if ! gcc -std=c99 -I"$INCLUDE_DIR" "$c_file" -o "$exe_file" $math_flag 2> "$cc_err"; then
        fail_test "$name" "compile failed"
        return
    fi

    # Compile-only test
    if [ ! -f "$expected" ]; then
        passed=$((passed + 1))
        printf "PASS %-52s compile-only\n" "$name"
        return
    fi

    # 3. Run
    if ! timeout 5 "$exe_file" > "$run_out" 2> "$run_err"; then
        fail_test "$name" "run failed/timeout"
        return
    fi
    if [ -s "$run_err" ]; then
        fail_test "$name" "runtime stderr not empty"
        return
    fi

    # 4. Compare
    if diff -u "$expected" "$run_out" > /dev/null 2>&1; then
        passed=$((passed + 1))
        printf "PASS %-52s run\n" "$name"
    else
        fail_test "$name" "output mismatch"
    fi
}

echo "=== Hust-C Test Group ($MILESTONE) ==="
echo ""

for d in $SEARCH_DIRS; do
    [ -d "$TEST_DIR/$d" ] || continue
    echo "--- $d ---"
    while IFS= read -r src; do
        run_one "$src"
    done < <(find "$TEST_DIR/$d" -name "*.hust" -type f | sort)
    echo ""
done

echo "=== Summary ==="
echo "Total:   $total"
echo "Passed:  $passed"
echo "Failed:  $failed"
if [ "$failed" -gt 0 ]; then
    echo ""
    echo "Failures:"
    printf "%b\n" "$failed_list"
    exit 1
fi

if [ "$total" -eq 0 ]; then
    echo "No tests found."
    exit 1
fi

exit 0
