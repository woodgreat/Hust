# Hust-CSharp Tests

> 版本：0.1.2.20261128125000
> 更新时间：2026-11-28

## 1. 运行方式

```bash
cd tests

# 全量：转译 -> dotnet build -> 运行 -> 精确比对 .expected
./run_tests.sh

# 核心契约：info / transpile / build(release,debug) / 方言错误 / 工程模式
./core_contract.sh
```

`run_tests.sh` 会先真实构建核心，再对每个正向用例：

1. `hust_csharp transpile`；
2. 生成临时 `test.csproj` 并 `dotnet build`；
3. 运行生成的 DLL；
4. 与 `<test>.expected` 逐字节比对。

负向用例必须转译失败，且 stderr 包含 `<test>.expected.err` 的第一行。

## 2. 测试矩阵

### M1 基础、控制流与函数（15）

| 覆盖点 | 用例 |
|--------|------|
| 全部数值/布尔/String 类型 | `m1/types.hust` |
| 常量与常量表达式 | `m1/constants.hust` |
| 算术/比较/逻辑/位/三元 | `m1/operators.hust` |
| 复合赋值 | `m1/assignment_ops.hust` |
| 前后缀自增自减 | `m1/increment.hust` |
| if / else if / else | `m1/if_else.hust` |
| while | `m1/while_loop.hust` |
| for | `m1/for_loop.hust` |
| do-while | `m1/do_while.hust` |
| switch / case / default | `m1/switch.hust` |
| 变量与算术 | `m1/variables.hust` |
| 顶层函数、返回值、递归 | `m1/functions.hust` |
| 嵌套作用域 | `m1/scope.hust` |
| 注释保护 | `m1/comments.hust` |
| Hello World | `m1/hello.hust` |

### M2 数组与字符串（8）

| 覆盖点 | 用例 |
|--------|------|
| 数组声明、索引、长度、遍历、写回 | `m2/array_literal.hust` |
| 二维数组字面量、行列长度、遍历 | `m2/array_2d.hust` |
| 固定长度数组 `new i32[N]` | `m2/array_basic.hust` |
| String[] / bool[] | `m2/array_types.hust` |
| 字符串拼接（含数字） | `m2/string_basic.hust`, `m2/string_concat.hust` |
| length/大小写/trim/contains/startsWith/endsWith/indexOf/substring/replace/split | `m2/string_methods.hust` |
| 字符串比较与 charAt | `m2/string_compare.hust` |

### M3 类、继承与接口（11）

| 覆盖点 | 用例 |
|--------|------|
| 字段、默认构造、方法、`this` | `m3/class_basic.hust` |
| 带参构造、getter | `m3/class_constructor.hust` |
| 状态修改方法、i32 返回值 | `m3/class_methods.hust` |
| extends 与方法覆盖 | `m3/inheritance.hust`, `m3/inheritance_override.hust` |
| implements + 接口变量多态 | `m3/interface_polymorphism.hust` |
| 多接口实现 | `m3/interface_multiple.hust` |
| 形状接口（无类型参数） | `m3/shape_interface.hust` |
| 接口基础 | `m3/interface_basic.hust` |
| 对象数组 | `m3/class_array.hust` |
| null 引用判断 | `m3/null_reference.hust` |

### M4 枚举与工程模式（5）

| 覆盖点 | 用例 |
|--------|------|
| 枚举声明与比较 | `m4/enum_basic.hust` |
| switch on enum | `m4/enum_switch.hust` |
| 枚举作参数/返回值 | `m4/enum_function.hust` |
| 类字段使用枚举 | `m4/enum_in_class.hust` |
| `settings.config` + `--project` | `m4/project_mode/` |

### negative（8）

| 场景 | 用例 | 期望 |
|------|------|------|
| RUST.* 方言 | `negative/rust_dialect.hust` | 明确拒绝 |
| C.* 方言 | `negative/c_dialect.hust` | 明确拒绝 |
| GO.* 方言 | `negative/go_dialect.hust` | 明确拒绝 |
| 混合方言 | `negative/mixed_dialect.hust` | 明确拒绝 |
| `println!` Rust 宏 | `negative/println_macro.hust` | 教学式错误 |
| `HUST.*` 未实现 | `negative/hust_import.hust` | 明确提示 |
| 类型不匹配 | `negative/type_mismatch.hust` | 编译前报错 |
| 未定义函数 | `negative/undefined_function.hust` | 编译前报错 |

## 3. 当前基线

```
Total:   47
Passed:  47
Failed:  0
```

核心契约：`PASS: Hust-CSharp core contract`。

## 4. 已知边界

- 数组使用 C# 原生数组：支持字面量、索引、`len()`、二维 jagged，不支持 `push/pop/insert/removeAt`。
- 基类多态调用未生成 `virtual/override`；当前覆盖测试使用具体类型或接口变量。
- 未实现用户模块/多文件工程；`--project` 支持单入口 `settings.config`。
- `bool` 输出为 C# 风格 `True/False`。
- `HUST.*` 跨核标准库未实现。

## 5. 新增用例

1. 在对应 `m1`–`m4` 目录创建 `.hust`；
2. 正向：创建同名 `.expected`（精确 stdout）；
3. 负向：放到 `negative/`，创建 `.expected.err`（第一行关键字）；
4. 运行 `./run_tests.sh`。

## 6. 相关文档

- [C# 核专用说明](../../../Hust-Docs/doc/core_csharp_write_guide_dg_cpt3.md)
- [语言核测试指引](../../../Hust-Docs/doc/core_testing_guide.md)
- [核心开发手册](../../../Hust-Docs/doc/core_development_playbook.md)
