# Hust-CSharp Tests

## Test Groups

| Group | Description | Status |
|-------|-------------|--------|
| m1 | Basic types, variables, expressions, control flow | Ready |
| m2 | Arrays, strings | Ready |
| m3 | Classes, inheritance, interfaces | Ready |
| m4 | enum/match, modules, project mode | Ready |
| negative | Dialect errors, syntax errors | Ready |

## Running Tests

```bash
cd tests
./run_tests.sh
```

## Test Format

### Positive Tests

- `test.hust` - Hust source file
- `test.expected` - Expected stdout output

### Negative Tests

- `test.hust` - Hust source file (should fail)
- `test.expected.err` - Expected error message (first line)

## Current Test Cases

### m1 (Basic)

- `hello.hust` - Hello World
- `variables.hust` - Variable declaration and arithmetic
- `if_else.hust` - If-else statement
- `for_loop.hust` - For loop

### m2 (Arrays & Strings)

- `array_basic.hust` - Array declaration and access
- `string_basic.hust` - String concatenation

### m3 (Classes & OOP)

- `class_basic.hust` - Class declaration and methods
- `inheritance.hust` - Class inheritance

### m4 (Advanced)

- `enum_basic.hust` - Enum declaration and usage
- `project_mode/` - Project mode with settings.config

### negative (Error Cases)

- `rust_dialect.hust` - RUST.* import in C# core
- `c_dialect.hust` - C.* import in C# core
- `mixed_dialect.hust` - Multiple dialects
- `type_mismatch.hust` - Type mismatch error
- `undefined_function.hust` - Undefined function error

## Test Count

- m1: 4 tests
- m2: 2 tests
- m3: 2 tests
- m4: 2 tests
- negative: 5 tests
- **Total: 15 tests**

## Known Issues

- C# core transpilation is partially implemented
- Some advanced features not yet supported
- Project mode requires settings.config parsing

## Adding New Tests

1. Create `.hust` file in appropriate group directory
2. Add `.expected` file for positive tests
3. Add `.expected.err` file for negative tests
4. Run `./run_tests.sh` to verify
