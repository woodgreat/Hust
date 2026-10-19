# Hust 测试文件清单

> 用途：跑全量测试后对照本表分辨结果。
> 命名约定：`expect_fail_*` 为**负向测试**，编译失败才是预期结果，不算 bug。

## 一、负向测试（预期失败）

| 文件 | 测试目的 |
|------|----------|
| `expect_fail_self_deprecated.hust` | 已弃用的 `self` 关键字必须被拒绝，并给出友好提示 |

## 二、类与面向对象

| 文件 | 测试目的 |
|------|----------|
| `access_control_test.hust` | 类、字段、方法的可见性控制（public/private）|
| `class_simple.hust` | 最简类定义与实例化 |
| `class_test.hust` | 类 + 接口 + 方法基本组合 |
| `class_v2.hust` | 简化方法写法（含 snake_case 方法名）|
| `inheritance_test.hust` | 单层继承 + 父类成员访问 `d.Animal.x` |
| `inheritance_multi.hust` | 多级继承（Animal → Dog → Puppy，r24）|
| `interface_test.hust` | 接口定义与实现 |
| `nested_class_test.hust` | 嵌套类 `Outer.Inner` 扁平化与 `this` 改写（B22）|
| `implicit_ctor_test.hust` | 隐式空构造函数（2026-09-29 决议）|
| `init_user_responsibility.hust` | 原则验证：声明永不自动置 0 |
| `test_readonly.hust` | `readonly` 方法修饰符（V0.2.6，`&self`）|
| `test_this_supported.hust` | `this` 关键字正常使用 |
| `test_object_dynamic_array.hust` | 对象类型动态数组（`Group[]` 等）|
| `super_string_test.hust` | `super()` 调用点字符串字面量 |
| `private_field_test.hust` | 私有字段内部访问 |
| `private_field_external_test.hust` | 外部访问私有字段（应报错）|
| `public_private_test.hust` | public / private 字段对比 |

## 三、语法与表达式

| 文件 | 测试目的 |
|------|----------|
| `bitwise_test.hust` | 位运算 `& | ^ << >>` |
| `char_verify_test.hust` | char ↔ i32 转换（`u8 as char`、`char::from_u32`）|
| `control_flow.hust` | if / else / while / for 控制流 |
| `function_return.hust` | 函数返回值 |
| `brace_style.hust` | 大括号换行风格（Allman）|
| `type_cast_test.hust` | 类型转换 `(f32)sum` |
| `string_test.hust` | String 基本操作 |
| `slice_test.hust` | 切片语义 |
| `pass_test.hust` | `pass` 空语句 |
| `nested_for_test.hust` | 嵌套 for 循环 |
| `recursion_test.hust` | 递归（Go 引擎可行性验证）|

## 四、数组

| 文件 | 测试目的 |
|------|----------|
| `array_test.hust` | 定长数组声明、求和 |
| `all_arrays.hust` | 定长数组 / 动态数组 / 多维数组综合 |
| `array_2d_test.hust` | 2D 数组（Go 棋盘表示）|
| `multi_dim_4.hust` | 4 维数组（2^4 = 16 元素）|
| `multi_dim_6.hust` | 6 维数组（64 元素）|
| `multi_dim_8.hust` | 8 维数组（256 元素）|
| `multi_dim_static_3.hust` | 3 维静态数组 |
| `multi_dim_static_5.hust` | 5 维静态数组 |
| `multi_dim_static_7.hust` | 7 维静态数组 |
| `multi_dim_static_13.hust` | 13 维静态数组（深度压力测试）|
| `dynamic_2d_lifecycle.hust` | 动态 2D 数组 `i32[][]` 生命周期 |
| `dynamic_3d.hust` | 动态 3D 数组 `i32[][][]` |
| `mut_array_test.hust` | `needs_mut` 能否识别数组下标赋值 |
| `mut_19x19_test.hust` | 19x19 数组 + bool 返回值 |
| `mut_bool_test.hust` | `needs_mut` 与 bool 返回类型 |
| `mut_extra_field_test.hust` | 额外字段 + 方法调用下的 `&mut self` 判定 |
| `verify_b3_array_param.hust` | B3：数组作为函数参数 |
| `verify_b7_array_field.hust` | B7：`i32[]` 类字段与初始化 |
| `verify_b8_large_array.hust` | B8：大于 64 元素的数组 Default 实现 |
| `verify_const_array_dim.hust` | 同文件内 const 作数组维度 |

## 五、for 循环与代码块更新

| 文件 | 测试目的 |
|------|----------|
| `simple_for.hust` | 最简 for 循环 |
| `simple_block_test.hust` | 单语句循环体 |
| `simple_block_update.hust` | 单语句更新块 `{ i++ }` |
| `single_op_block.hust` | 单个操作符的更新块 |
| `dual_op_block.hust` | 两个操作符的更新块 `{ count++; other--; }` |
| `multi_statement_update.hust` | 多语句更新块 |
| `block_update.hust` | 代码块形式更新 |
| `block_test.hust` | 嵌套代码块作用域 |
| `complex_block_update.hust` | 复杂多语句更新块 |
| `complex_for_test.hust` | 复杂循环条件 |
| `for_block_test.hust` | 多条语句的循环体 |
| `for_update_test.hust` | 数组遍历 + 更新表达式 |
| `debug_for.hust` | for 循环调试用例 |
| `debug_block_update.hust` | 更新块调试用例 |
| `compare_prefix_postfix.hust` | `++i` 与 `i++` 差异 |
| `prefix_for.hust` | 前缀自增循环 |
| `untyped_for_header.hust` | 无类型 for 头 |
| `verify_block_support.hust` | 循环体 + `println!` 宏保护（B21）|
| `verify_a1_continue.hust` | A1：for + continue 死循环验证 |
| `verify_a2_ctor.hust` | A2：构造函数静默丢语句验证 |

## 六、模块与 use

| 文件 | 测试目的 |
|------|----------|
| `verify_b1_use.hust` | B1：system namespace `use` 语句 |
| `verify_b9_module_var.hust` | B9：模块级变量（只读）|
| `unrecognized_use_test.hust` | 无法识别的 use 路径教学式报错 |
| `e2e_rust_import_test.hust` | RUST 系统命名空间导入端到端 |

## 七、所有权、克隆与借用

| 文件 | 测试目的 |
|------|----------|
| `verify_b5_clone.hust` | B5：类默认 `.clone()` |
| `verify_b5_moveit.hust` | B5：`.moveit()` 显式移动 |
| `verify_b5_shape.hust` | B5：形状接口 + class 传参 |
| `verify_b5_shape2.hust` | B5：blank 名称与实现类参数一致的写法 |
| `verify_b6_mut_self.hust` | B6：类方法 `&mut self` 支持 |
| `verify_b13_ownership.hust` | B13：所有权 / move 语义 |
| `verify_clone_default.hust` | 类是否默认支持 `.clone()` |

## 八、接口与形状（shape）

| 文件 | 测试目的 |
|------|----------|
| `verify_shape_step1.hust` | Step 1：单参数接口，无返回变量名 |
| `verify_shape_step2.hust` | Step 2：双参数 + 返回变量名 |
| `verify_shape_step3.hust` | Step 3：两个实现类 |
| `verify_shape_step3b.hust` | Step 3b：两个实现类 + 参数名匹配 |
| `verify_shape_retvar.hust` | 接口返回变量名 |
| `verify_shape_params_retvar.hust` | 多参数 + 返回变量名 |
| `verify_r1_replaceit.hust` | `.replaceit()` 替换方法 |

## 九、其他验证与示例

| 文件 | 测试目的 |
|------|----------|
| `verify_b2_class_return.hust` | B2：类作为返回类型 |
| `verify_b4_usize.hust` | B4：usize 变量声明 |
| `verify_b11_ctor_self_method.hust` | B11：构造函数内调用自身方法 |
| `bug_fixes_test.hust` | 龙小虾报告的一批 bug 回归 |
| `extract_test.hust` | `extract_brace_content` 工具函数 |
| `go_board_class.hust` | Go 棋盘类版本 |
| `go_board_minimal.hust` | Go 棋盘最简版本 |
| `go_board_with_get.hust` | Go 棋盘 + `get()` 方法 |
| `go_board_with_print.hust` | Go 棋盘 + `print_board()` 方法 |
| `simple_sort.hust` | 简单排序算法 |
| `yanghui.hust` | 杨辉三角 |
| `demo.hust` | Hust V0.1 演示程序 |
| `main.hust` | 最小可编译程序（hello）|
| `sample.hust` | 最小示例（hello）|
| `clean_test.hust` | 简单循环冒烟测试 |

## 十、全量测试方法

```bash
cd Hust-Rust
for f in tests/*.hust; do
  if cargo run -q -- build "$f" 2>&1 | grep -q 'Build complete'; then
    echo "PASS $f"
  else
    echo "FAIL $f"
  fi
done
```

判定规则：
- `tests/expect_fail_*` 出现 FAIL → 正确
- `tests/expect_fail_*` 出现 PASS → **真事故**，校验失效
- 其他文件出现 FAIL → 真 bug
