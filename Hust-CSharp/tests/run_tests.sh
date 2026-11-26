#!/bin/bash
# Hust-CSharp Test Runner
# Tests transpilation, compilation, and execution

set -u

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CORE_DIR="$(dirname "$SCRIPT_DIR")"
REPO_DIR="$(dirname "$CORE_DIR")"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
TOTAL=0
PASSED=0
FAILED=0
SKIPPED=0

# Test groups
GROUPS="m1 m2 m3 m4 negative"

# Build the core first
echo "=== Building Hust-CSharp Core ==="
cd "$CORE_DIR"
if ! dotnet build -c Release 2>&1 | tail -5; then
    echo -e "${RED}Failed to build Hust-CSharp core${NC}"
    exit 1
fi
echo -e "${GREEN}Core built successfully${NC}"
echo ""

# Find the executable
CORE_DLL="$CORE_DIR/bin/Release/net8.0/hust_csharp.dll"
if [ ! -f "$CORE_DLL" ]; then
    echo -e "${RED}Core DLL not found: $CORE_DLL${NC}"
    exit 1
fi

# Use dotnet to run the DLL
DOTNET_CMD="${DOTNET_CMD:-$HOME/.dotnet/dotnet}"
if [ ! -f "$DOTNET_CMD" ]; then
    DOTNET_CMD="dotnet"
fi
if ! command -v "$DOTNET_CMD" &> /dev/null; then
    echo -e "${RED}dotnet command not found${NC}"
    exit 1
fi

# Test function
run_test() {
    local test_file="$1"
    local test_name="$(basename "$test_file" .hust)"
    local test_dir="$(dirname "$test_file")"
    local expected_file="${test_file%.hust}.expected"
    local expected_err_file="${test_file%.hust}.expected.err"
    
    TOTAL=$((TOTAL + 1))
    
    # Create temp directory
    local temp_dir=$(mktemp -d)
    local out_dir="$temp_dir/out"
    mkdir -p "$out_dir"
    
    # Transpile
    local transpile_output
    transpile_output=$("$DOTNET_CMD" "$CORE_DLL" transpile "$test_file" --out-dir "$out_dir" 2>&1)
    local transpile_exit=$?
    
    if [ $transpile_exit -ne 0 ]; then
        if [ -f "$expected_err_file" ]; then
            # Expected to fail
            local expected_err=$(head -1 "$expected_err_file")
            if echo "$transpile_output" | grep -q "$expected_err"; then
                echo -e "${GREEN}PASS${NC} $test_name (expected failure)"
                PASSED=$((PASSED + 1))
            else
                echo -e "${RED}FAIL${NC} $test_name (wrong error message)"
                echo "  Expected: $expected_err"
                echo "  Got: $transpile_output"
                FAILED=$((FAILED + 1))
            fi
        else
            echo -e "${RED}FAIL${NC} $test_name (transpile failed)"
            echo "  $transpile_output"
            FAILED=$((FAILED + 1))
        fi
        rm -rf "$temp_dir"
        return
    fi
    
    # Check for unexpected transpile errors
    if [ -f "$expected_err_file" ]; then
        echo -e "${RED}FAIL${NC} $test_name (expected failure but succeeded)"
        FAILED=$((FAILED + 1))
        rm -rf "$temp_dir"
        return
    fi
    
    # Check transpile stderr is empty for positive tests
    if [ -n "$transpile_output" ]; then
        echo -e "${YELLOW}WARN${NC} $test_name (transpile produced output)"
    fi
    
    # Find generated C# file
    local cs_file=$(find "$out_dir" -name "*.cs" | head -1)
    if [ -z "$cs_file" ]; then
        echo -e "${RED}FAIL${NC} $test_name (no C# file generated)"
        FAILED=$((FAILED + 1))
        rm -rf "$temp_dir"
        return
    fi
    
    # Try to compile with dotnet (if available)
    if [ -f "$DOTNET_CMD" ] || command -v "$DOTNET_CMD" &> /dev/null; then
        # Create a minimal project file
        cat > "$out_dir/test.csproj" << 'EOF'
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
  </PropertyGroup>
</Project>
EOF
        
        local build_output
        build_output=$(cd "$out_dir" && "$DOTNET_CMD" build -c Release 2>&1)
        local build_exit=$?
        
        if [ $build_exit -ne 0 ]; then
            echo -e "${RED}FAIL${NC} $test_name (C# compilation failed)"
            echo "  $build_output" | head -5
            FAILED=$((FAILED + 1))
            rm -rf "$temp_dir"
            return
        fi
        
        # Run if expected output exists
        if [ -f "$expected_file" ]; then
            # Find the generated DLL (not the executable)
            local dll_file=$(find "$out_dir/bin" -name "*.dll" | grep -v "ref" | head -1)
            if [ -n "$dll_file" ]; then
                local run_output
                run_output=$("$DOTNET_CMD" "$dll_file" 2>&1)
                local expected_output=$(cat "$expected_file")
                
                if [ "$run_output" == "$expected_output" ]; then
                    echo -e "${GREEN}PASS${NC} $test_name"
                    PASSED=$((PASSED + 1))
                else
                    echo -e "${RED}FAIL${NC} $test_name (output mismatch)"
                    echo "  Expected: $expected_output"
                    echo "  Got: $run_output"
                    FAILED=$((FAILED + 1))
                fi
            else
                echo -e "${YELLOW}SKIP${NC} $test_name (no executable found)"
                SKIPPED=$((SKIPPED + 1))
            fi
        else
            echo -e "${GREEN}PASS${NC} $test_name (compile only)"
            PASSED=$((PASSED + 1))
        fi
    else
        echo -e "${YELLOW}SKIP${NC} $test_name (dotnet not available)"
        SKIPPED=$((SKIPPED + 1))
    fi
    
    rm -rf "$temp_dir"
}

# Run tests by group
for group in m1 m2 m3 m4 negative; do
    group_dir="$SCRIPT_DIR/$group"
    if [ ! -d "$group_dir" ]; then
        continue
    fi
    
    echo "=== Testing $group ==="
    
    # Find all .hust files
    for test_file in "$group_dir"/*.hust; do
        if [ -f "$test_file" ]; then
            run_test "$test_file"
        fi
    done
    
    echo ""
done

# Summary
echo "=== Test Summary ==="
echo "Total:   $TOTAL"
echo -e "Passed:  ${GREEN}$PASSED${NC}"
echo -e "Failed:  ${RED}$FAILED${NC}"
echo -e "Skipped: ${YELLOW}$SKIPPED${NC}"

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi
