#!/bin/bash
# Hust-Go Test Runner
#
# Every positive test is transpiled, compiled with the Go toolchain, executed,
# and compared byte-for-byte with its .expected file. Negative tests must fail
# to transpile and match the first line of their .expected.err file.
#
# Usage:
#   ./run_tests.sh            # full: transpile + build + run + compare
#   ./run_tests.sh --quick    # transpile only
#   ./run_tests.sh --strict   # same as default; kept for parity with other cores

set -u

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CORE_DIR="$(dirname "$SCRIPT_DIR")"
CORE="$CORE_DIR/hust_go"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

TOTAL=0
PASSED=0
FAILED=0
NEG_TOTAL=0
NEG_PASSED=0

MODE="full"
case "${1:-}" in
    --quick) MODE="quick" ;;
    --strict) MODE="full" ;;
    "") ;;
    *) echo "unknown option: $1" >&2; exit 2 ;;
esac

GO_BIN="${GO_BIN:-}"
if [ -z "$GO_BIN" ] || [ ! -x "$GO_BIN" ]; then
    if command -v go >/dev/null 2>&1; then
        GO_BIN="$(command -v go)"
    elif [ -x "$HOME/go-sdk/go/bin/go" ]; then
        GO_BIN="$HOME/go-sdk/go/bin/go"
    else
        echo -e "${RED}go toolchain not found${NC}" >&2
        exit 1
    fi
fi

echo "=== Building Hust-Go core ==="
if ! (cd "$CORE_DIR" && "$GO_BIN" build -o "$CORE" ./src); then
    echo -e "${RED}Failed to build Hust-Go core${NC}"
    exit 1
fi
echo -e "${GREEN}Core built successfully${NC}"
echo ""

fail() {
    local name="$1"
    shift
    echo -e "${RED}FAIL${NC} $name ($*)"
    FAILED=$((FAILED + 1))
}

run_positive() {
    local test_file="$1"
    local test_name
    test_name="$(basename "$test_file" .hust)"
    local test_dir
    test_dir="$(dirname "$test_file")"
    local expected_file="${test_file%.hust}.expected"
    local project_flag=""

    if [ -f "$test_dir/settings.config" ]; then
        project_flag="--project"
    fi

    TOTAL=$((TOTAL + 1))

    if [ ! -f "$expected_file" ]; then
        fail "$test_name" "missing .expected"
        return
    fi

    local temp_dir
    temp_dir="$(mktemp -d)"
    local out_dir="$temp_dir/out"
    mkdir -p "$out_dir"

    if [ "$MODE" = "quick" ]; then
        # shellcheck disable=SC2086
        "$CORE" transpile "$test_file" $project_flag --out-dir "$out_dir" \
            >"$temp_dir/stdout" 2>"$temp_dir/stderr"
        local exit_code=$?
        if [ $exit_code -ne 0 ]; then
            fail "$test_name" "transpile exit $exit_code"
            sed -n '1,5p' "$temp_dir/stderr"
        elif [ -s "$temp_dir/stderr" ]; then
            fail "$test_name" "unexpected translator diagnostics"
            sed -n '1,5p' "$temp_dir/stderr"
        else
            echo -e "${GREEN}PASS${NC} $test_name (transpile)"
            PASSED=$((PASSED + 1))
        fi
        rm -rf "$temp_dir"
        return
    fi

    # shellcheck disable=SC2086
    "$CORE" build "$test_file" $project_flag --out-dir "$out_dir" \
        >"$temp_dir/stdout" 2>"$temp_dir/stderr"
    local build_exit=$?

    if [ $build_exit -ne 0 ]; then
        fail "$test_name" "build exit $build_exit"
        sed -n '1,8p' "$temp_dir/stderr"
        rm -rf "$temp_dir"
        return
    fi

    if [ -s "$temp_dir/stderr" ]; then
        fail "$test_name" "unexpected diagnostics"
        sed -n '1,5p' "$temp_dir/stderr"
        rm -rf "$temp_dir"
        return
    fi

    local exe
    exe="$(tail -n 1 "$temp_dir/stdout")"
    case "$exe" in
        executable=*) exe="${exe#executable=}" ;;
        *)
            fail "$test_name" "build did not report executable"
            rm -rf "$temp_dir"
            return
            ;;
    esac

    if [ ! -x "$exe" ]; then
        fail "$test_name" "missing executable: $exe"
        rm -rf "$temp_dir"
        return
    fi

    timeout 15 "$exe" >"$temp_dir/actual" 2>"$temp_dir/runerr"
    local run_exit=$?
    if [ $run_exit -ne 0 ]; then
        fail "$test_name" "runtime exit $run_exit"
        sed -n '1,5p' "$temp_dir/runerr"
        rm -rf "$temp_dir"
        return
    fi

    if ! diff -u "$expected_file" "$temp_dir/actual" >"$temp_dir/diff"; then
        fail "$test_name" "output mismatch"
        sed -n '1,20p' "$temp_dir/diff"
        rm -rf "$temp_dir"
        return
    fi

    echo -e "${GREEN}PASS${NC} $test_name"
    PASSED=$((PASSED + 1))
    rm -rf "$temp_dir"
}

run_negative() {
    local test_file="$1"
    local test_name
    test_name="$(basename "$test_file" .hust)"
    local expected_err_file="${test_file%.hust}.expected.err"

    NEG_TOTAL=$((NEG_TOTAL + 1))

    if [ ! -f "$expected_err_file" ]; then
        echo -e "${RED}FAIL${NC} $test_name (missing .expected.err)"
        FAILED=$((FAILED + 1))
        return
    fi

    local temp_dir
    temp_dir="$(mktemp -d)"

    "$CORE" transpile "$test_file" --out-dir "$temp_dir/out" \
        >"$temp_dir/stdout" 2>"$temp_dir/stderr"
    local exit_code=$?
    local expected
    expected="$(head -n 1 "$expected_err_file")"

    if [ $exit_code -eq 0 ]; then
        echo -e "${RED}FAIL${NC} $test_name (expected failure, got success)"
        FAILED=$((FAILED + 1))
    elif ! grep -qF "$expected" "$temp_dir/stderr"; then
        echo -e "${RED}FAIL${NC} $test_name (wrong error message)"
        echo "  Expected: $expected"
        sed -n '1,5p' "$temp_dir/stderr"
        FAILED=$((FAILED + 1))
    else
        echo -e "${GREEN}PASS${NC} $test_name (expected failure)"
        NEG_PASSED=$((NEG_PASSED + 1))
    fi

    rm -rf "$temp_dir"
}

for group in m1 m2 m3 m4; do
    group_dir="$SCRIPT_DIR/$group"
    [ -d "$group_dir" ] || continue
    echo "=== Testing $group ==="
    while IFS= read -r test_file; do
        run_positive "$test_file"
    done < <(find "$group_dir" -name '*.hust' | sort)
    echo ""
done

echo "=== Testing negative ==="
while IFS= read -r test_file; do
    run_negative "$test_file"
done < <(find "$SCRIPT_DIR/negative" -name '*.hust' | sort)
echo ""

echo "=== Test Summary ==="
echo "Total:    $TOTAL"
echo -e "Passed:   ${GREEN}$PASSED${NC}"
echo -e "Failed:   ${RED}$FAILED${NC}"
echo "Negative: $NEG_PASSED/$NEG_TOTAL"

if [ $FAILED -eq 0 ] && [ $NEG_PASSED -eq $NEG_TOTAL ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
fi

echo -e "${RED}Some tests failed!${NC}"
exit 1
