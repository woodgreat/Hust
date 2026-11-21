#!/bin/bash
# Hust-C Test Runner
# Runs all test cases and compares output with expected results

# Don't exit on error - we want to run all tests
# set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
HUST_C_BIN="$PROJECT_DIR/hust_c"
TEST_DIRS=("basic" "class" "array" "string" "control_flow")

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
PASSED=0
FAILED=0
SKIPPED=0

echo "=== Hust-C Test Suite ==="
echo ""

# Check if hust-c binary exists
if [ ! -f "$HUST_C_BIN" ]; then
    echo -e "${YELLOW}Building hust-c...${NC}"
    cd "$PROJECT_DIR"
    make
    echo ""
fi

# Run tests in each directory
for dir in "${TEST_DIRS[@]}"; do
    test_dir="$SCRIPT_DIR/$dir"
    
    if [ ! -d "$test_dir" ]; then
        continue
    fi
    
    echo "--- Testing: $dir ---"
    
    for test_file in "$test_dir"/*.hust; do
        if [ ! -f "$test_file" ]; then
            continue
        fi
        
        test_name=$(basename "$test_file" .hust)
        output_file="$test_dir/${test_name}.c"
        expected_file="$test_dir/${test_name}.expected.c"
        
        # Run transpiler
        if "$HUST_C_BIN" "$test_file" -o "$output_file" 2>/dev/null; then
            # Check if expected file exists
            if [ -f "$expected_file" ]; then
                # Compare output with expected
                if diff -q "$output_file" "$expected_file" > /dev/null 2>&1; then
                    echo -e "  ${GREEN}✓${NC} $test_name"
                    ((PASSED++))
                else
                    echo -e "  ${RED}✗${NC} $test_name (output differs)"
                    ((FAILED++))
                    
                    # Show diff if verbose
                    if [ "$1" == "-v" ] || [ "$1" == "--verbose" ]; then
                        echo "    Diff:"
                        diff "$output_file" "$expected_file" | head -20 | sed 's/^/    /'
                    fi
                fi
            else
                echo -e "  ${YELLOW}?${NC} $test_name (no expected file)"
                ((SKIPPED++))
            fi
        else
            echo -e "  ${RED}✗${NC} $test_name (transpile failed)"
            ((FAILED++))
        fi
    done
    
    echo ""
done

# Summary
echo "=== Summary ==="
echo -e "  ${GREEN}Passed:${NC}  $PASSED"
echo -e "  ${RED}Failed:${NC}  $FAILED"
echo -e "  ${YELLOW}Skipped:${NC} $SKIPPED"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed.${NC}"
    exit 1
fi
