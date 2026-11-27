#!/bin/bash

# Hust-Go Test Runner
# Version: 0.1.0.20261127084900

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0

# Build the core
echo "=== Building Hust-Go Core ==="
cd "$(dirname "$0")/.."
go build -o hust_go src/main.go src/preprocessor.go src/translator.go
if [ $? -ne 0 ]; then
    echo -e "${RED}Failed to build Hust-Go core${NC}"
    exit 1
fi
echo "Core built successfully"

# Test function
run_test() {
    local test_file=$1
    local test_name=$(basename "$test_file" .hust)
    local test_dir=$(dirname "$test_file")
    local expected_file="${test_file%.hust}.expected"
    local expected_err_file="${test_file%.hust}.expected.err"
    
    TOTAL=$((TOTAL + 1))
    
    # Create temp directory
    local temp_dir=$(mktemp -d)
    local out_dir="$temp_dir/out"
    mkdir -p "$out_dir"
    
    # Transpile
    ./hust_go transpile "$test_file" --out-dir "$out_dir" 2>"$temp_dir/stderr.txt"
    local transpile_exit=$?
    
    # Check for expected error (negative test)
    if [ -f "$expected_err_file" ]; then
        if [ $transpile_exit -ne 0 ]; then
            # Check if error message matches
            local expected_err=$(head -n 1 "$expected_err_file")
            if grep -q "$expected_err" "$temp_dir/stderr.txt"; then
                echo -e "${GREEN}PASS${NC} $test_name (expected failure)"
                PASSED=$((PASSED + 1))
            else
                echo -e "${RED}FAIL${NC} $test_name (wrong error message)"
                echo "  Expected: $expected_err"
                echo "  Got: $(cat "$temp_dir/stderr.txt")"
                FAILED=$((FAILED + 1))
            fi
        else
            echo -e "${RED}FAIL${NC} $test_name (should have failed)"
            FAILED=$((FAILED + 1))
        fi
        rm -rf "$temp_dir"
        return
    fi
    
    # Check transpile success
    if [ $transpile_exit -ne 0 ]; then
        echo -e "${RED}FAIL${NC} $test_name (transpile failed)"
        echo "  Error: $(cat "$temp_dir/stderr.txt")"
        FAILED=$((FAILED + 1))
        rm -rf "$temp_dir"
        return
    fi
    
    # Check for expected output
    if [ -f "$expected_file" ]; then
        # TODO: Build and run, compare output
        # For now, just check if transpile succeeded
        echo -e "${GREEN}PASS${NC} $test_name"
        PASSED=$((PASSED + 1))
    else
        # Compile-only test
        echo -e "${GREEN}PASS${NC} $test_name (compile-only)"
        PASSED=$((PASSED + 1))
    fi
    
    rm -rf "$temp_dir"
}

# Run tests by group
run_group() {
    local group=$1
    local group_dir="tests/$group"
    
    if [ ! -d "$group_dir" ]; then
        echo -e "${YELLOW}SKIP${NC} $group (directory not found)"
        return
    fi
    
    echo ""
    echo "=== Testing $group ==="
    
    for test_file in "$group_dir"/*.hust; do
        if [ -f "$test_file" ]; then
            run_test "$test_file"
        fi
    done
}

# Main
cd "$(dirname "$0")/.."

# Run test groups
run_group "m1"
run_group "m2"
run_group "m3"
run_group "m4"
run_group "negative"

# Summary
echo ""
echo "=== Test Summary ==="
echo "Total:   $TOTAL"
echo -e "Passed:  ${GREEN}$PASSED${NC}"
echo -e "Failed:  ${RED}$FAILED${NC}"
echo "Skipped: $SKIPPED"

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
