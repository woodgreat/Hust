# Hust-Go Test Suite

> Version: 0.1.1.20261128125000
> Date: 2026-11-28

## 1. 运行方式

```bash
cd tests

# 全量：转译 -> go build -> 运行 -> 精确比对 .expected
./run_tests.sh

# 只检查转译（不编译/运行）
./run_tests.sh --quick

# 核心契约：info / transpile / build(release,debug) / 方言错误 / 工程模式
./core_contract.sh
```

正向用例要求：转译 stderr 为空、`go build` 成功、运行输出与 `.expected` 逐字节一致。
负向用例要求：转译失败（exit 4），且 stderr 包含 `.expected.err` 的第一行。

## 2. 测试矩阵

### M1 基础、控制流与函数（50）

| 覆盖点 | 代表用例 |
|--------|----------|
| 全部数值/布尔/字符类型 | `m1/data_types.hust`, `m1/primitive_types.hust`, `m1/char_literals.hust` |
| 无符号类型运算 | `m1/unsigned_ops.hust` |
| 浮点运算与提升 | `m1/float_ops.hust` |
| 十六进制 / 二进制字面量 | `m1/hex_literals.hust` |
| 显式类型转换 r27 | `m1/casts.hust` |
| 变量、常量、多声明符 | `m1/variables.hust`, `m1/constants.hust`, `m1/multi_var.hust` |
| 算术/比较/逻辑/位运算 | `m1/arithmetic_ops.hust`, `m1/comparison_ops.hust`, `m1/logical_ops.hust`, `m1/bitwise_ops.hust` |
| 复合赋值、自增自减、前缀/后缀 | `m1/assignment_ops.hust`, `m1/increment.hust`, `m1/for_prefix.hust` |
| 三元表达式（含嵌套） | `m1/operators.hust`, `m1/ternary_chain.hust` |
| if / else if / else | `m1/if_else.hust`, `m1/nested_control.hust` |
| while / for / do-while | `m1/while_loop.hust`, `m1/for_loop.hust`, `m1/do_while.hust`, `m1/all_control_flow.hust` |
| break / continue / 提前 return | `m1/break_continue.hust`, `m1/early_return.hust` |
| switch / case | `m1/switch_case.hust` |
| 函数、前向引用、递归、嵌套调用 | `m1/functions.hust`, `m1/recursion.hust`, `m1/nested_calls.hust` |
| 作用域、块作用域 | `m1/scope.hust`, `m1/scope_test.hust` |
| 字符串转义 | `m1/string_escape.hust` |
| 注释、空 main | `m1/comments.hust`, `m1/empty_main.hust` |

### M2 数组与字符串（40）

| 覆盖点 | 代表用例 |
|--------|----------|
| 一维字面量、索引、写回、遍历 | `m2/array_basic.hust`, `m2/array_operations.hust` |
| 空数组与动态增长 | `m2/array_empty.hust` |
| 动态数组 push / pop / insert / removeAt / clear | `m2/array_dynamic.hust`, `m2/array_manipulation.hust` |
| 切片、拼接 | `m2/array_advanced.hust`, `m2/array_chunk.hust` |
| 数组参数与返回 | `m2/array_param.hust`, `m2/array_return.hust` |
| 浮点数组 | `m2/array_f64.hust` |
| 二维/三维/jagged | `m2/array_2d.hust`, `m2/array_3d.hust`, `m2/jagged_array.hust` |
| 矩阵运算 | `m2/matrix.hust`, `m2/matrix_ops.hust` |
| 排序/查找/统计/集合 | `m2/sorting_algorithms.hust`, `m2/array_search.hust`, `m2/array_stats.hust`, `m2/array_set_ops.hust` |
| map/filter/reduce/reverse/rotate/window | `m2/array_map.hust`, `m2/array_filter.hust`, `m2/array_reduce.hust`, `m2/array_reverse.hust`, `m2/array_rotate.hust`, `m2/array_window.hust` |
| 字符串拼接、比较、格式化 | `m2/string_basic.hust`, `m2/string_concat.hust`, `m2/string_format.hust` |
| len/charAt/substring/大小写/trim/contains/startsWith/endsWith/indexOf/replace/split | `m2/string_methods.hust`, `m2/string_operations.hust` |
| 字符串数组、字符串构建 | `m2/string_array.hust`, `m2/string_builder.hust` |

### M3 类、继承与接口（34）

| 覆盖点 | 代表用例 |
|--------|----------|
| 字段、构造、方法、`this` | `m3/class_basic.hust`, `m3/class_point.hust` |
| 对象数组、对象字段 | `m3/class_library.hust`, `m3/class_geometry.hust` |
| 类作为返回类型、对象数组参数 | `m3/class_return.hust` |
| extends / 方法覆盖 | `m3/class_inheritance.hust`, `m3/class_animals.hust`, `m3/class_vehicles.hust` |
| 多级继承 + 基类接口数组（虚调用） | `m3/class_shapes.hust` |
| implements / 接口变量多态 | `m3/interface_basic.hust`, `m3/interface_polymorphism.hust` |
| 接口参数与接口返回 | `m3/interface_param.hust` |
| 形状接口（无类型参数） | `m3/shape_interface.hust`, `m3/class_stack_queue.hust` |
| null 引用 | `m3/class_bst.hust`, `m3/class_linkedlist.hust`, `m3/class_employee.hust` |
| 数据结构综合 | `m3/class_hashmap.hust`, `m3/class_matrix.hust`, `m3/class_stack.hust`, `m3/class_queue.hust` |

### M4 枚举与工程模式（26）

| 覆盖点 | 代表用例 |
|--------|----------|
| 枚举声明、比较、打印 | `m4/enum_basic.hust`, `m4/enum_weekday.hust` |
| 枚举作参数/返回值 | `m4/enum_calculator.hust`, `m4/enum_calculator_ops.hust` |
| switch on enum | `m4/enum_switch.hust` |
| switch on String / 嵌套 switch | `m4/switch_string.hust`, `m4/switch_nested.hust` |
| 枚举状态机/调度/词法 | `m4/enum_state_machine.hust`, `m4/enum_traffic.hust`, `m4/enum_lexer.hust` |
| 枚举 + 类 | `m4/enum_class.hust`, `m4/enum_robot.hust` |
| `settings.config` + `--project` | `m4/project_mode/` |

### negative（26）

| 场景 | 代表用例 | 期望 |
|------|----------|------|
| RUST.* / C.* / 混合方言 | `negative/rust_dialect.hust`, `negative/c_dialect.hust`, `negative/mixed_dialect.hust` | 明确拒绝 |
| `println!` Rust 宏 | `negative/println_macro.hust` | 教学式错误 |
| 字符串/数字类型不匹配 | `negative/type_mismatch*.hust` | 编译前报错 |
| 未定义函数 | `negative/undefined_function*.hust` | 编译前报错 |
| 未定义方法 / 字符串方法 | `negative/undefined_method.hust`, `negative/undefined_string_method.hust` | 编译前报错 |
| 未定义变量 / 成员 | `negative/undefined_variable.hust`, `negative/undefined_member.hust` | 编译前报错 |
| 未知类型 | `negative/unknown_type.hust` | 编译前报错 |
| 参数个数错误（函数/方法/构造） | `negative/wrong_arity.hust`, `negative/method_arity.hust`, `negative/constructor_arity.hust` | 编译前报错 |
| 除零 | `negative/division_by_zero*.hust` | 编译前报错 |
| 常量缺少初值 | `negative/const_without_initializer.hust` | 编译前报错 |
| 不支持的 `new T[n]` | `negative/new_array_unsupported.hust` | 教学式错误 |
| `push()` 作为表达式 | `negative/push_as_expression.hust` | 编译前报错 |
| 无法推断空数组字面量 | `negative/array_literal_infer.hust` | 编译前报错 |
| 非法类型转换 | `negative/invalid_cast.hust`, `negative/invalid_cast_bool.hust` | 编译前报错 |

## 3. 当前基线

```
Total:    150
Passed:   150
Failed:   0
Negative: 26/26
```

核心契约：`PASS: Hust-Go core contract`。

## 4. 已知边界

- `super(...)` 未实现：继承采用字段扁平化 + 方法复制，子类构造函数直接给继承字段赋值。
- 类内 `const` 字段未实现（局部 const 已支持）。
- `new T[n]` 未实现，动态数组使用 `{}` + `push`。
- 声明的接口若作为变量类型使用，方法签名必须写明确的返回类型（形状接口的“无类型”仅适合具体类型调用）。
- 未实现多文件用户模块；`--project` 支持单入口 `settings.config`。
- 数组越界是运行期 panic，未做编译期边界检查。
- `HUST.*` 跨核标准库未实现。
- 结构体字段名与 Go 关键字冲突时自动加 `_`，源码层仍使用原名。

## 5. 新增用例

1. 在 `m1`–`m4` 创建 `.hust` 和同名 `.expected`；
2. 负向放到 `negative/`，创建 `.expected.err`（第一行关键字）；
3. 运行 `./run_tests.sh`，不允许未登记的已知失败。

## 6. 相关文档

- [Go 核专用说明](../../../Hust-Docs/doc/core_go_write_guide_dg_cpt4.md)
- [语言核测试指引](../../../Hust-Docs/doc/core_testing_guide.md)
- [核心开发手册](../../../Hust-Docs/doc/core_development_playbook.md)
