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

### M1 基础、控制流与函数（35）

| 覆盖点 | 代表用例 |
|--------|----------|
| 全部数值/布尔/String 类型 | `m1/types.hust` |
| 无符号与浮点宽度 | `m1/unsigned_types.hust`, `m1/float_types.hust` |
| char 字面量与隐式整数转换 | `m1/char_literals.hust` |
| 布尔逻辑 | `m1/bool_logic.hust` |
| 常量与常量表达式 | `m1/constants.hust`, `m1/const_expr.hust` |
| 多声明符 | `m1/multi_declarators.hust` |
| 算术/比较/逻辑/位/三元 | `m1/operators.hust`, `m1/bitwise_ops.hust`, `m1/ternary_chain.hust` |
| 复合赋值、前后缀自增 | `m1/assignment_ops.hust`, `m1/increment.hust` |
| if / else if / else / 嵌套 if | `m1/if_else.hust`, `m1/nested_if.hust` |
| while / for / do-while | `m1/while_loop.hust`, `m1/for_loop.hust`, `m1/do_while.hust` |
| break / continue | `m1/while_break_continue.hust`, `m1/for_continue.hust` |
| switch / case / default | `m1/switch.hust` |
| 函数、参数、返回值、递归 | `m1/function_params.hust`, `m1/recursion_fib.hust` |
| 前向调用 / 嵌套调用 | `m1/functions.hust`, `m1/nested_calls.hust` |
| 提前返回 | `m1/early_return.hust` |
| 块作用域 | `m1/scope.hust`, `m1/scope_inner.hust`, `m1/nested_blocks.hust` |
| 注释保护 / 字符串转义 | `m1/comments.hust`, `m1/string_escape.hust` |
| 空 main / Hello World | `m1/empty_main.hust`, `m1/hello.hust` |

### M2 数组与字符串（28）

| 覆盖点 | 代表用例 |
|--------|----------|
| 数组字面量、索引、长度、写回、遍历 | `m2/array_literal.hust`, `m2/array_write.hust`, `m2/array_sum.hust` |
| 固定长度 `new i32[N]` | `m2/array_basic.hust` |
| f64[] / bool[] / String[] | `m2/array_f64.hust`, `m2/array_bool.hust`, `m2/array_literal_string.hust` |
| 空数组 | `m2/array_empty.hust` |
| 二维 jagged、行列长度、遍历 | `m2/array_2d.hust`, `m2/array_jagged.hust`, `m2/array_2d_iter.hust` |
| 数组参数 / 数组返回 | `m2/array_param.hust`, `m2/array_return.hust` |
| 对象数组与字段 | `m2/array_class_fields.hust` |
| 字符串拼接（含数字/浮点） | `m2/string_basic.hust`, `m2/string_concat.hust`, `m2/string_number_concat.hust` |
| length / charAt / substring | `m2/string_methods.hust`, `m2/string_substring.hust` |
| 大小写 / trim / replace | `m2/string_case.hust`, `m2/string_trim_compare.hust`, `m2/string_replace.hust` |
| contains / startsWith / endsWith / indexOf | `m2/string_methods.hust`, `m2/string_indexof.hust` |
| split 与字符串数组 | `m2/string_split.hust`, `m2/array_literal_string.hust` |
| 空字符串 | `m2/string_empty.hust` |

### M3 类、继承与接口（26）

| 覆盖点 | 代表用例 |
|--------|----------|
| 字段、默认构造、方法、`this` | `m3/class_basic.hust` |
| 带参构造、getter | `m3/class_constructor.hust`, `m3/class_methods.hust` |
| 构造中调用方法 / 循环初始化 | `m3/class_this_method.hust`, `m3/class_ctor_loop.hust` |
| 类字段数组 | `m3/class_field_array.hust` |
| 常量字段 | `m3/class_const_field.hust` |
| 字符串字段方法 | `m3/class_string_field.hust` |
| 多级继承 / 构造 / 覆盖链 | `m3/class_chain.hust`, `m3/class_inheritance_ctor.hust`, `m3/class_override_chain.hust` |
| 接口基础 / 多接口 / 形状接口 | `m3/interface_basic.hust`, `m3/interface_multiple.hust`, `m3/shape_interface.hust` |
| 类型化接口 + 接口变量多态 | `m3/interface_typed.hust`, `m3/interface_polymorphism.hust` |
| 接口方法参数 | `m3/interface_param.hust` |
| 对象数组 / 对象数组参数 | `m3/class_array.hust`, `m3/class_array_param.hust` |
| 函数返回对象 | `m3/class_return.hust` |
| null 引用判断 | `m3/null_reference.hust`, `m3/null_method.hust` |
| 类方法中的循环/条件 | `m3/class_counter_loop.hust` |

### M4 枚举与工程模式（12）

| 覆盖点 | 代表用例 |
|--------|----------|
| 枚举声明/打印/比较 | `m4/enum_basic.hust`, `m4/enum_values.hust`, `m4/enum_compare.hust` |
| 枚举数组 | `m4/enum_array.hust` |
| 枚举作参数/返回值 | `m4/enum_function.hust`, `m4/enum_param.hust`, `m4/enum_return.hust` |
| switch on enum | `m4/enum_switch.hust` |
| 类字段使用枚举 | `m4/enum_in_class.hust` |
| switch on String / 嵌套 switch | `m4/switch_string.hust`, `m4/switch_nested.hust` |
| `settings.config` + `--project` | `m4/project_mode/` |

### negative（11）

| 场景 | 用例 | 期望 |
|------|------|------|
| RUST.* / C.* / GO.* 方言 | `negative/rust_dialect.hust`, `negative/c_dialect.hust`, `negative/go_dialect.hust` | 明确拒绝 |
| 混合方言 | `negative/mixed_dialect.hust` | 明确拒绝 |
| `println!` Rust 宏 | `negative/println_macro.hust` | 教学式错误 |
| `HUST.*` 未实现 | `negative/hust_import.hust` | 明确提示 |
| 类型不匹配 | `negative/type_mismatch.hust` | 编译前报错 |
| 未定义函数 | `negative/undefined_function.hust` | 编译前报错 |
| 未知类型 | `negative/unknown_type.hust` | `Unknown type: X` |
| 参数个数错误 | `negative/wrong_arity.hust` | `Wrong number of arguments...` |
| 除零 | `negative/division_by_zero.hust` | `Division by zero` |

## 3. 当前基线

```
Total:    112
Passed:   112
Failed:   0
Negative: 11/11
```

核心契约：`PASS: Hust-CSharp core contract`。

## 4. 已知边界

- 数组使用 C# 原生数组：支持字面量、索引、`len()`、二维 jagged，不支持 `push/pop/insert/removeAt`。
- 基类多态调用未生成 `virtual/override`；当前覆盖测试使用具体类型或接口变量。
- C# 不允许局部变量遮蔽（CS0136），Hust 的 shadowing 用例需改用不同变量名。
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
