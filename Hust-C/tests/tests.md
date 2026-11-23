# Hust-C 测试组规范

> 状态：目标测试组，分批落地
> 日期：2026-11-22
> 对标：`Hust-Rust/tests/tests.md`（134 个活跃用例，10 大类）
> 目标：C 组覆盖只多不少，最终不少于 170 个用例

---

## 1. 目的

建立一套独立于 Hust-Rust 的 C 测试组，用来评估 Hust-C 的完整性，而不是复用 Rust 的测试。

判定标准比 Rust 组更严：

1. 方言检查通过
2. 转译成功，且转译器 stderr 为空
3. 生成的 C 能通过 `gcc -std=c99`
4. 可运行用例能运行，stdout 与 `.expected` 精确匹配
5. 负向用例必须按预期失败

Rust 组只检查 `Build complete`；C 组要求 compile + run + compare。

---

## 2. 方言与测试前提

- 纯 Hust：无系统导入，只能使用语言级逻辑；测试为 compile-only。
- C 方言：必须导入 `C.*`；M1 只支持 `use C.stdio.*;` 形式。
- Rust 方言代码（`println!`、`RUST.*`、Rust 宏）不得出现在 C 组。
- `String` 是语言级类型；String 测试属于语言组，不属于 `C.string`。
- 动态数组按 r18：`i32[] a; a.push(...)`；禁止 `new i32[...]`。
- C 专属语法（指针、C 风格数组、`struct`、`null`、`sizeof`）是否合法待 owner 确认，相关用例暂放 `c_specific/`，不纳入 M1-M4。

---

## 3. 目录结构

```text
tests/
├── m1/                     # M1 可编译基线
│   ├── pure/               # 无导入，compile-only
│   │   ├── types/
│   │   ├── expressions/
│   │   ├── functions/
│   │   └── control_flow/
│   ├── c_stdio/            # use C.stdio.*;
│   ├── c_stdlib/           # use C.stdlib.*;
│   ├── c_string/           # use C.string.*;
│   └── c_math/             # use C.math.*;
├── m2/                     # M2 运行时与数组
│   ├── arrays/
│   └── strings/
├── m3/                     # M3 面向对象
│   ├── classes/
│   ├── inheritance/
│   └── interfaces/
├── m4/                     # M4 其余语言特性
│   ├── enum_match/
│   ├── modules/
│   └── negative/
├── c_specific/             # C 方言专属，待语法边界确认
├── legacy/                 # 10 个旧种子，不纳入基线，改写后迁出
├── run_tests.sh
└── tests.md
```

命名：

- `m1/pure/types/i8_decl.hust`
- `m1/c_stdio/printf_int.hust`
- `m2/arrays/static_2d_init.hust`
- `m3/classes/ctor_basic.hust`
- `m4/negative/rust_macro_rejected.hust`

每个可运行用例配同名 `.expected`；compile-only 用例不配；负向用例配 `.expected.err`（stderr 必须包含的文本）。

---

## 4. 用例清单（目标 170）

### M1：可编译基线（78）

| 目录 | 数量 | 内容 |
|------|------|------|
| `m1/pure/types` | 12 | i8/i16/i32/i64/u8/u16/u32/u64/f32/f64/bool/char 声明与赋值 |
| `m1/pure/expressions` | 8 | 算术优先级、括号、位运算、比较、逻辑、复合赋值、一元运算、显式转换 |
| `m1/pure/functions` | 6 | 无参、带参、返回值、递归、多参数、public |
| `m1/pure/control_flow` | 16 | if/else、else-if、while、for 各形态、嵌套、break、continue、单语句体 |
| `m1/c_stdio` | 10 | printf 字符串/整数/浮点/多参数/转义/换行/负数/字符/布尔/组合 |
| `m1/c_stdlib` | 8 | malloc/free、calloc、realloc、数组分配、结构分配、失败检查 |
| `m1/c_string` | 10 | strlen/strcpy/strcat/strcmp/memcpy/memset/组合 |
| `m1/c_math` | 8 | sqrt/pow/fabs/floor/ceil/min/max/组合 |

### M2：运行时与数组（32）

| 目录 | 数量 | 内容 |
|------|------|------|
| `m2/arrays` | 22 | 静态 1D/2D/3D/N-D、`{0}`、无初始化、动态 1D push/pop/len/index、动态 2D/3D push([])、切片、数组参数/返回/字段、replaceit、const 维度 |
| `m2/strings` | 10 | 字面量、拼接、比较、len、方法、自动转换、参数、返回、对象数组、空串 |

### M3：面向对象（29）

| 目录 | 数量 | 内容 |
|------|------|------|
| `m3/classes` | 16 | 最简类、字段、方法、显式/隐式构造、构造语句、可见性、嵌套类、类返回、对象数组、clone、moveit、readonly、静态工厂 |
| `m3/inheritance` | 5 | 单层、多级、super()、父类字段/方法、上转型 |
| `m3/interfaces` | 8 | 形状接口、无类型参数、多实现、接口变量、动态分发、参数/返回、多态容器 |

### M4：其余语言特性（21）

| 目录 | 数量 | 内容 |
|------|------|------|
| `m4/enum_match` | 6 | enum 定义、全分支、非穷尽、块分支、enum 变量、match 默认分支 |
| `m4/modules` | 5 | C.stdio/C.stdlib/C.string/C.math 导入、未导入报错前置 |
| `m4/negative` | 10 | self 弃用、接口类型字、重复构造、RUST.* 拒绝、println! 拒绝、混核、缺导入、无效 C.* item、未实现 HUST.*、编译器 stderr 为空 |

### C 专属（10，待 Q1 确认）

指针声明/解引用/指针运算/二级指针/null/sizeof/裸 malloc 数组/C 风格数组/struct/入口签名。

### 合计

78 + 32 + 29 + 21 + 10 = **170**。

---

## 5. 与 Hust-Rust 10 类的对应

| Hust-Rust 类别 | Hust-C 对应 |
|----------------|-------------|
| 负向测试 | `m4/negative` + C 方言负向 |
| 类与 OOP | `m3/classes`、`m3/inheritance` |
| 语法与表达式 | `m1/pure/*`、`m2/strings` |
| 数组 | `m2/arrays` |
| for 循环与块更新 | `m1/pure/control_flow` |
| 模块与 use | `m4/modules` |
| 所有权/克隆/借用 | `m3/classes` 中的 clone/moveit；Rust 借用专属用例不迁移 |
| 接口与形状 | `m3/interfaces` |
| 其他验证与示例 | 分散到各语言组，另加算法用例 |
| 分组目录（enum/interface/module/use） | `m4/enum_match`、`m3/interfaces`、`m4/modules` |

Rust 专属的 `&mut self`、借用检查、`RUST.*` 导入不作为 C 用例；用 C 等价语义和 C 专属用例补足数量。

---

## 6. 运行方式

```bash
cd Hust-C
make
./tests/run_tests.sh          # 全部
./tests/run_tests.sh m1       # 只跑 M1
./tests/run_tests.sh m2
```

报告必须分别给出：

- 方言检查失败
- 转译失败
- 编译失败
- 运行失败
- 输出不匹配
- 通过

任何 DEBUG 输出、占位符残留、非空转译 stderr 都算失败。

---

## 7. 当前状态

- 旧 130 个用例：121 个逐字节复制自 Hust-Rust，120 个 Rust 方言复制品已删除。
- 10 个旧种子移至 `legacy/`，不纳入基线，待改写。
- 新测试组已生成 **171 个用例**：
  - M1：80 个（pure 42 + c_stdio 10 + c_math 8 + c_stdlib 8 + c_string 10 + 2 个多行 smoke）
  - M2：33 个（arrays 23 + strings 10）
  - M3：29 个（classes 16 + inheritance 5 + interfaces 8）
  - M4：21 个（enum_match 6 + modules 5 + negative 10）
  - C-specific：8 个（指针、C 风格数组、null、sizeof）
- 已超过 Hust-Rust 的 134 个活跃用例。
- 2026-11-22 晚基线：
  - M1：80/80 通过
  - M2：33/33 通过
  - M3：29/29 通过
  - M4：21/21 通过
  - C-specific：8/8 通过
  - 总计：171/171
- M1 已修复：DEBUG 输出、i8/i16/u8-u64 映射、函数调用被误判、C.* 通配导入、RUST.*/HUST.*/println! 方言错误、`int main`、pass、const、显式转换、for 块更新、单语句循环体、C.stdlib/C.string。
- M2 已修复：HustVec 运行时、静态 1D/N 维数组、动态 1D/2D/3D 数组、push/pop/len/index、replaceit、切片、const 维度、数组参数、HustString 运行时、String 声明/拼接/比较/长度/参数/返回、String 对象数组、数组返回（out-param 降级）。
- M3 已修复：类方法体解析、value 语义构造/隐式构造、字段/方法/可见性/readonly、clone/moveit、类返回、对象数组、继承、super()、父类字段/方法调用、接口 vtable/handle、接口变量/参数/返回/多态容器、嵌套类扁平化。
- M4 已修复：enum 类型注册与成员改写、match/switch/case/块分支、self 弃用、接口类型字、重复构造、先导入才能用 I/O 的负向检查。
- C-specific 暂定边界（待 owner 最终确认）：C 方言允许指针、C 风格数组、`null`、`sizeof`、C 库调用。
- 当前全部测试组通过。
- 下一步：扩展 HUST.* 与模块/命名空间，增加真实项目级用例。
