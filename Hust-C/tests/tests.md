# Hust-C 测试文件清单

> Hust-C 转译器专属测试套件
> 与 Hust-Rust 测试分离，专注 C 语言特性

## 测试目录结构

```
tests/
├── basic/          # 基础语法测试
├── control_flow/   # 控制流测试
├── array/          # 数组测试
├── string/         # 字符串测试
├── class/          # 类测试
├── pointer/        # 指针测试
├── memory/         # 内存管理测试
├── c_library/      # C 库集成测试
└── run_tests.sh    # 测试运行脚本
```

## 一、基础语法 (basic/)

| 文件 | 测试目的 |
|------|----------|
| `variable_declarations.hust` | 各种类型变量声明（i32/i64/f32/f64/bool/char/String）|
| `function_definitions.hust` | 函数定义、参数、返回值 |

## 二、控制流 (control_flow/)

| 文件 | 测试目的 |
|------|----------|
| `if_while_for.hust` | if/else/while/for/break/continue |

## 三、数组 (array/)

| 文件 | 测试目的 |
|------|----------|
| `basic_arrays.hust` | 定长数组、动态数组、多维数组 |

## 四、字符串 (string/)

| 文件 | 测试目的 |
|------|----------|
| `basic_string.hust` | String 类型、拼接、比较、方法 |

## 五、类 (class/)

| 文件 | 测试目的 |
|------|----------|
| `rectangle.hust` | 类定义、字段、方法、构造函数 |

## 六、指针 (pointer/)

| 文件 | 测试目的 |
|------|----------|
| `basic_pointers.hust` | 指针声明、解引用、指针运算、动态分配 |

## 七、内存管理 (memory/)

| 文件 | 测试目的 |
|------|----------|
| `basic_memory.hust` | malloc/free/realloc、内存泄漏防护 |

## 八、C 库集成 (c_library/)

| 文件 | 测试目的 |
|------|----------|
| `basic_c_library.hust` | stdio/stdlib/string/math 函数调用 |

## 运行测试

```bash
cd Hust-C
make                    # 编译 hust_c
./tests/run_tests.sh    # 运行所有测试
```

## 与 Hust-Rust 测试的区别

| 特性 | Hust-Rust | Hust-C |
|------|-----------|--------|
| 系统命名空间 | `RUST.*` | `C.*` |
| 打印宏 | `println!` | `printf` |
| 内存管理 | 自动（GC/RAII）| 手动（malloc/free）|
| 指针 | 隐藏 | 显式 |
| C 库 | 通过 FFI | 直接调用 |

## 未来扩展

- [ ] 负向测试（expect_fail_*）
- [ ] 接口与形状测试
- [ ] 继承与多态测试
- [ ] 泛型测试
- [ ] 错误处理测试
