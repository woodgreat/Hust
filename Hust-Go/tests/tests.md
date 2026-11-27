# Hust-Go Test Suite

> Version: 0.1.0.20261127084900
> Date: 2026-11-27

## Test Groups

| Group | Description | Count | Status |
|-------|-------------|-------|--------|
| m1 | Basic types, variables, control flow | 4 | 🚧 In Progress |
| m2 | Arrays, strings | 0 | ⏳ Pending |
| m3 | Classes, inheritance, interfaces | 0 | ⏳ Pending |
| m4 | Enums, modules | 0 | ⏳ Pending |
| negative | Error cases | 5 | 🚧 In Progress |

## Test Files

### m1 - Basic Tests

| File | Description | Status |
|------|-------------|--------|
| hello.hust | Hello World | 🚧 |
| variables.hust | Variable declarations | 🚧 |
| if_else.hust | If-else control flow | 🚧 |
| for_loop.hust | For loop | 🚧 |

### negative - Error Cases

| File | Description | Expected Error |
|------|-------------|----------------|
| rust_dialect.hust | Rust dialect import | Unsupported dialect |
| c_dialect.hust | C dialect import | Unsupported dialect |
| mixed_dialect.hust | Mixed dialects | Unsupported dialect |
| type_mismatch.hust | Type mismatch | Type mismatch error |
| undefined_function.hust | Undefined function | Undefined function error |

## Known Issues

None yet.

## Test Runner

```bash
cd tests
./run_tests.sh
```

## Adding New Tests

1. Create `.hust` file in appropriate group directory
2. For positive tests: create `.expected` file with expected output
3. For negative tests: create `.expected.err` file with expected error message
4. Run `./run_tests.sh` to verify
