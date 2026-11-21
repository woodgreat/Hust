#!/bin/bash
# Hust-C Test Runner
# Runs all .hust test files and reports results

set -e

TEST_DIR="$(cd "$(dirname "$0")" && pwd)"
HUST_C="$(dirname "$TEST_DIR")/hust_c"
OUTPUT_DIR="$TEST_DIR/output"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Counters
total=0
passed=0
failed=0
skipped=0

# Print header
echo "=== Hust-C Test Suite ==="
echo ""

# Function to run a single test
run_test() {
    local test_file="$1"
    local test_name=$(basename "$test_file" .hust)
    local category=$(basename "$(dirname "$test_file")")
    local output_file="$OUTPUT_DIR/${category}_${test_name}.c"
    
    total=$((total + 1))
    
    printf "Running %-40s" "[$category] $test_name..."
    
    # Check if hust_c binary exists
    if [ ! -f "$HUST_C" ]; then
        echo -e "${YELLOW}SKIP${NC} (hust_c not found)"
        skipped=$((skipped + 1))
        return
    fi
    
    # Run the transpiler
    if "$HUST_C" "$test_file" -o "$output_file" 2>/dev/null; then
        # Check if output file was created and is not empty
        if [ -s "$output_file" ]; then
            echo -e "${GREEN}PASS${NC}"
            passed=$((passed + 1))
        else
            echo -e "${RED}FAIL${NC} (empty output)"
            failed=$((failed + 1))
        fi
    else
        echo -e "${RED}FAIL${NC} (transpilation error)"
        failed=$((failed + 1))
    fi
}

# Find and run all .hust files
echo "--- Basic Tests ---"
for f in "$TEST_DIR"/basic/*.hust; do
    [ -f "$f" ] && run_test "$f"
done

echo ""
echo "--- Control Flow Tests ---"
for f in "$TEST_DIR"/control_flow/*.hust; do
    [ -f "$f" ] && run_test "$f"
done

echo ""
echo "--- Array Tests ---"
for f in "$TEST_DIR"/array/*.hust; do
    [ -f "$f" ] && run_test "$f"
done

echo ""
echo "--- String Tests ---"
for f in "$TEST_DIR"/string/*.hust; do
    [ -f "$f" ] && run_test "$f"
done

echo ""
echo "--- Class Tests ---"
for f in "$TEST_DIR"/class/*.hust; do
    [ -f "$f" ] && run_test "$f"
done

echo ""
echo "--- Pointer Tests ---"
for f in "$TEST_DIR"/pointer/*.hust; do
    [ -f "$f" ] && run_test "$f"
done

echo ""
echo "--- Memory Tests ---"
for f in "$TEST_DIR"/memory/*.hust; do
    [ -f "$f" ] && run_test "$f"
done

echo ""
echo "--- C Library Tests ---"
for f in "$TEST_DIR"/c_library/*.hust; do
    [ -f "$f" ] && run_test "$f"
done

# Print summary
echo ""
echo "=== Summary ==="
echo -e "Total:   $total"
echo -e "Passed:  ${GREEN}$passed${NC}"
echo -e "Failed:  ${RED}$failed${NC}"
echo -e "Skipped: ${YELLOW}$skipped${NC}"

# Exit with error if any tests failed
if [ $failed -gt 0 ]; then
    exit 1
fi

exit 0
