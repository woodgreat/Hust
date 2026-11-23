#!/usr/bin/env python3
"""Generate the Hust-C target test group (M1 and M2).

The generated tests are language-level targets. Tests without .expected are
compile-only; tests with .expected are compiled, run, and compared.
"""

from pathlib import Path

BASE = Path(__file__).resolve().parent


def write_test(rel, source, expected=None, expected_err=None):
    path = BASE / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(source.strip() + "\n", encoding="utf-8")
    if expected is not None:
        path.with_suffix(".expected").write_text(expected, encoding="utf-8")
    if expected_err is not None:
        path.with_suffix(".expected.err").write_text(expected_err.strip() + "\n", encoding="utf-8")


# ---------------------------------------------------------------- M1 pure

TYPES = {
    "i8": "12",
    "i16": "1234",
    "i32": "42",
    "i64": "1000000",
    "u8": "12",
    "u16": "1234",
    "u32": "42",
    "u64": "1000000",
    "f32": "3.5",
    "f64": "2.5",
    "bool": "true",
    "char": "'A'",
}

for name, literal in TYPES.items():
    write_test(
        f"m1/pure/types/{name}_decl.hust",
        f"""
void main() {{
    {name} x = {literal};
    {name} y = x;
}}
""",
    )

EXPRESSIONS = {
    "arithmetic": "i32 a = 1 + 2 * 3; i32 b = (1 + 2) * 3;",
    "bitwise": "i32 a = (5 & 3) | (8 >> 1); i32 b = ~a;",
    "comparison": "bool a = (1 < 2) && (3 != 4); bool b = (1 >= 2) || false;",
    "compound_assign": "i32 a = 1; a += 2; a *= 3; a -= 1;",
    "unary": "i32 a = -5; i32 b = +3; bool c = !false;",
    "cast": "f32 f = 3.9; i32 a = (i32)f; f64 d = (f64)a;",
    "const": "const i32 N = 5; i32 a = N;",
    "precedence": "i32 a = 1 + 2 * 3 - 4 / 2;",
}

def body_lines(body):
    parts = [p.strip() for p in body.split(';') if p.strip()]
    return "\n".join(f"    {p};" for p in parts)

for name, body in EXPRESSIONS.items():
    write_test(
        f"m1/pure/expressions/{name}.hust",
        f"void main() {{\n{body_lines(body)}\n}}",
    )

FUNCTIONS = {
    "no_params": """
i32 answer() {
    return 42;
}
void main() {
    i32 a = answer();
}
""",
    "add": """
i32 add(i32 a, i32 b) {
    return a + b;
}
void main() {
    i32 a = add(1, 2);
}
""",
    "void_return": """
void noop() {
}
void main() {
    noop();
}
""",
    "recursion": """
i32 fact(i32 n) {
    if (n <= 1) {
        return 1;
    }
    return n * fact(n - 1);
}
void main() {
    i32 a = fact(5);
}
""",
    "multi_param": """
i32 sum3(i32 a, i32 b, i32 c) {
    return a + b + c;
}
void main() {
    i32 a = sum3(1, 2, 3);
}
""",
    "public": """
public i32 add(i32 a, i32 b) {
    return a + b;
}
void main() {
    i32 a = add(1, 2);
}
""",
}

for name, src in FUNCTIONS.items():
    write_test(f"m1/pure/functions/{name}.hust", src)

CONTROL_FLOW = {
    "if_basic": """
void main() {
    i32 x = 1;
    if (x > 0) {
        x = 2;
    }
}
""",
    "if_else": """
void main() {
    i32 x = 1;
    if (x > 0) {
        x = 2;
    } else {
        x = 3;
    }
}
""",
    "else_if_chain": """
void main() {
    i32 x = 1;
    if (x > 2) {
        x = 3;
    } else if (x > 0) {
        x = 4;
    } else {
        x = 5;
    }
}
""",
    "while_loop": """
void main() {
    i32 i = 0;
    while (i < 3) {
        i++;
    }
}
""",
    "for_basic": """
void main() {
    for (i32 i = 0; i < 3; i++) {
        i = i;
    }
}
""",
    "for_untyped": """
void main() {
    i32 i;
    for (i = 0; i < 3; i++) {
        i = i;
    }
}
""",
    "for_block_update": """
void main() {
    for (i32 i = 0; i < 3; { i++; }) {
        i = i;
    }
}
""",
    "for_prefix": """
void main() {
    for (i32 i = 0; i < 3; ++i) {
        i = i;
    }
}
""",
    "for_postfix": """
void main() {
    for (i32 i = 0; i < 3; i++) {
        i = i;
    }
}
""",
    "nested_for": """
void main() {
    for (i32 i = 0; i < 2; i++) {
        for (i32 j = 0; j < 2; j++) {
            i = i;
        }
    }
}
""",
    "break": """
void main() {
    for (i32 i = 0; i < 3; i++) {
        if (i == 1) {
            break;
        }
    }
}
""",
    "continue": """
void main() {
    for (i32 i = 0; i < 3; i++) {
        if (i == 1) {
            continue;
        }
    }
}
""",
    "nested_continue": """
void main() {
    for (i32 i = 0; i < 2; i++) {
        for (i32 j = 0; j < 2; j++) {
            if (j == 0) {
                continue;
            }
        }
    }
}
""",
    "single_statement_for": """
void main() {
    for (i32 i = 0; i < 3; i++) i = i;
}
""",
    "pass": """
void main() {
    pass;
}
""",
    "block_scope": """
void main() {
    {
        i32 x = 1;
        x = 2;
    }
}
""",
}

for name, src in CONTROL_FLOW.items():
    write_test(f"m1/pure/control_flow/{name}.hust", src)

# ---------------------------------------------------------------- M1 C modules

C_STDIO = {
    "hello": ('printf("hello\\n");', "hello\n"),
    "int": ('printf("%d\\n", 42);', "42\n"),
    "int_var": ("i32 x = 7; printf(\"%d\\n\", x);", "7\n"),
    "float": ("f32 f = 3.5; printf(\"%.1f\\n\", f);", "3.5\n"),
    "two_ints": ('printf("%d %d\\n", 1, 2);', "1 2\n"),
    "char": ("char c = 'A'; printf(\"%c\\n\", c);", "A\n"),
    "bool": ("bool b = true; printf(\"%d\\n\", b);", "1\n"),
    "negative": ('printf("%d\\n", -5);', "-5\n"),
    "escape": ('printf("a\\tb\\n");', "a\tb\n"),
    "literal": ('printf("literal\\n");', "literal\n"),
}

for name, (body, expected) in C_STDIO.items():
    write_test(
        f"m1/c_stdio/{name}.hust",
        f"use C.stdio.*;\nvoid main() {{\n    {body}\n}}",
        expected,
    )

C_MATH = {
    "sqrt": ("f64 x = sqrt(16.0);", "4.0\n"),
    "pow": ("f64 x = pow(2.0, 3.0);", "8.0\n"),
    "fabs": ("f64 x = fabs(-3.5);", "3.5\n"),
    "floor": ("f64 x = floor(3.9);", "3.0\n"),
    "ceil": ("f64 x = ceil(3.1);", "4.0\n"),
    "combined": ("f64 x = sqrt(9.0) + pow(2.0, 2.0);", "7.0\n"),
    "sin_zero": ("f64 x = sin(0.0);", "0.0\n"),
    "cos_zero": ("f64 x = cos(0.0);", "1.0\n"),
}

for name, (body, expected) in C_MATH.items():
    write_test(
        f"m1/c_math/{name}.hust",
        "use C.stdio.*;\n"
        "use C.math.*;\n"
        "void main() {\n"
        f"    {body}\n"
        '    printf("%.1f\\n", x);\n'
        "}",
        expected,
    )

# ---------------------------------------------------------------- M2 arrays

ARRAYS = {
    "static_1d_init": (
        "i32[3] a = {1, 2, 3}; printf(\"%d\\n\", a[0] + a[1] + a[2]);",
        "6\n",
    ),
    "static_1d_zero": (
        "i32[3] a = {0}; a[1] = 5; printf(\"%d\\n\", a[0] + a[1] + a[2]);",
        "5\n",
    ),
    "static_1d_noinit": (
        "i32[3] a; a[0] = 1; a[1] = 2; a[2] = 3; printf(\"%d\\n\", a[0] + a[1] + a[2]);",
        "6\n",
    ),
    "static_2d_init": (
        "i32[2][2] a = {{1, 2}, {3, 4}}; printf(\"%d\\n\", a[0][0] + a[1][1]);",
        "5\n",
    ),
    "static_2d_zero": (
        "i32[2][2] a = {0}; a[0][1] = 3; a[1][0] = 4; printf(\"%d\\n\", a[0][1] + a[1][0]);",
        "7\n",
    ),
    "static_2d_index": (
        "i32[2][2] a = {0}; a[1][1] = 9; printf(\"%d\\n\", a[1][1]);",
        "9\n",
    ),
    "static_3d_zero": (
        "i32[2][2][2] a = {0}; a[0][0][0] = 1; a[1][1][1] = 2; "
        "printf(\"%d\\n\", a[0][0][0] + a[1][1][1]);",
        "3\n",
    ),
    "static_4d_zero": (
        "i32[2][2][2][2] a = {0}; a[1][1][1][1] = 5; printf(\"%d\\n\", a[1][1][1][1]);",
        "5\n",
    ),
    "static_nd_sum": (
        "i32[2][2][2] a = {0}; "
        "a[0][0][0]=1; a[0][0][1]=1; a[0][1][0]=1; a[0][1][1]=1; "
        "a[1][0][0]=1; a[1][0][1]=1; a[1][1][0]=1; a[1][1][1]=1; "
        "i32 s = 0; "
        "for (i32 i = 0; i < 2; i++) { for (i32 j = 0; j < 2; j++) { "
        "for (i32 k = 0; k < 2; k++) { s = s + a[i][j][k]; } } } "
        "printf(\"%d\\n\", s);",
        "8\n",
    ),
    "const_dim": (
        "const i32 N = 2; i32[N][N] a = {0}; a[1][1] = 6; printf(\"%d\\n\", a[1][1]);",
        "6\n",
    ),
    "dynamic_1d_push": (
        "i32[] a; a.push(1); a.push(2); i32 n = (i32)a.len(); "
        "printf(\"%d %d\\n\", n, a[1]);",
        "2 2\n",
    ),
    "dynamic_1d_pop": (
        "i32[] a; a.push(1); a.push(2); a.pop(); i32 n = (i32)a.len(); "
        "printf(\"%d %d\\n\", n, a[0]);",
        "1 1\n",
    ),
    "dynamic_1d_write": (
        "i32[] a; a.push(1); a[0] = 5; printf(\"%d\\n\", a[0]);",
        "5\n",
    ),
    "dynamic_2d_rows": (
        "i32[][] c; c.push([]); c.push([]); c[0].push(7); c[1].push(8); "
        "i32 n = (i32)c.len(); printf(\"%d %d %d\\n\", n, c[0][0], c[1][0]);",
        "2 7 8\n",
    ),
    "dynamic_2d_write": (
        "i32[][] c; c.push([]); c[0].push(1); c[0][0] = 9; printf(\"%d\\n\", c[0][0]);",
        "9\n",
    ),
    "dynamic_2d_pop": (
        "i32[][] c; c.push([]); c[0].push(5); c[0].pop(); "
        "i32 n = (i32)c[0].len(); printf(\"%d\\n\", n);",
        "0\n",
    ),
    "dynamic_3d": (
        "i32[][][] c; c.push([]); c[0].push([]); c[0][0].push(4); "
        "printf(\"%d\\n\", c[0][0][0]);",
        "4\n",
    ),
    "array_literal_dynamic": (
        "i32[] b = {10, 20, 30}; i32 n = (i32)b.len(); "
        "printf(\"%d %d\\n\", n, b[1]);",
        "3 20\n",
    ),
    "array_param_static": (
        "i32 sum(i32[3] a) { return a[0] + a[1] + a[2]; } "
        "i32[3] b = {1, 2, 3}; printf(\"%d\\n\", sum(b));",
        "6\n",
    ),
    "array_param_dynamic": (
        "i32 sum_dyn(i32[] a) { i32 s = 0; "
        "for (i32 i = 0; i < a.len(); i++) { s = s + a[i]; } return s; } "
        "i32[] b = {1, 2, 3}; printf(\"%d\\n\", sum_dyn(b));",
        "6\n",
    ),
    "slice": (
        "i32[5] a = {1, 2, 3, 4, 5}; i32[] b = a[0..3]; "
        "i32 n = (i32)b.len(); printf(\"%d %d %d\\n\", n, b[0], b[2]);",
        "3 1 3\n",
    ),
    "replaceit": (
        "i32[] a; a.push(1); a.push(2); a.replaceit(1, 9); "
        "printf(\"%d\\n\", a[1]);",
        "9\n",
    ),
    "array_return": (
        "i32[3] make() { i32[3] a = {1, 2, 3}; return a; } "
        "i32[3] b = make(); printf(\"%d\\n\", b[0] + b[1] + b[2]);",
        "6\n",
    ),
}

def wrap_main(body):
    return "use C.stdio.*;\nvoid main() {\n" + body_lines(body) + "\n}"

for name, (body, expected) in ARRAYS.items():
    if name == "array_param_static":
        src = """
use C.stdio.*;
i32 sum(i32[3] a) {
    return a[0] + a[1] + a[2];
}
void main() {
    i32[3] b = {1, 2, 3};
    printf("%d\\n", sum(b));
}
"""
    elif name == "array_param_dynamic":
        src = """
use C.stdio.*;
i32 sum_dyn(i32[] a) {
    i32 s = 0;
    for (i32 i = 0; i < a.len(); i++) {
        s = s + a[i];
    }
    return s;
}
void main() {
    i32[] b = {1, 2, 3};
    printf("%d\\n", sum_dyn(b));
}
"""
    elif name == "array_return":
        src = """
use C.stdio.*;
i32[3] make() {
    i32[3] a = {1, 2, 3};
    return a;
}
void main() {
    i32[3] b = make();
    printf("%d\\n", b[0] + b[1] + b[2]);
}
"""
    elif name == "static_nd_sum":
        src = """
use C.stdio.*;
void main() {
    i32[2][2][2] a = {0};
    a[0][0][0] = 1;
    a[0][0][1] = 1;
    a[0][1][0] = 1;
    a[0][1][1] = 1;
    a[1][0][0] = 1;
    a[1][0][1] = 1;
    a[1][1][0] = 1;
    a[1][1][1] = 1;
    i32 s = 0;
    for (i32 i = 0; i < 2; i++) {
        for (i32 j = 0; j < 2; j++) {
            for (i32 k = 0; k < 2; k++) {
                s = s + a[i][j][k];
            }
        }
    }
    printf("%d\\n", s);
}
"""
    else:
        src = wrap_main(body)
    write_test(f"m2/arrays/{name}.hust", src, expected)

# ---------------------------------------------------------------- M2 strings

STRINGS = {
    "literal_len": (
        'String s = "Hello"; i32 n = (i32)s.len(); printf("%d\\n", n);',
        "5\n",
    ),
    "empty_len": (
        'String s = ""; i32 n = (i32)s.len(); printf("%d\\n", n);',
        "0\n",
    ),
    "concat_len": (
        'String s = "Hi"; String t = s + "!"; i32 n = (i32)t.len(); printf("%d\\n", n);',
        "3\n",
    ),
    "concat_multi": (
        'String t = "a" + "b" + "c"; i32 n = (i32)t.len(); printf("%d\\n", n);',
        "3\n",
    ),
    "compare_eq": (
        'String s = "Hi"; bool b = s == "Hi"; printf("%d\\n", b);',
        "1\n",
    ),
    "compare_ne": (
        'String s = "Hi"; bool b = s != "Bye"; printf("%d\\n", b);',
        "1\n",
    ),
    "param": (
        "i32 len_of(String s) { return (i32)s.len(); } "
        'String s = "Hello"; printf("%d\\n", len_of(s));',
        "5\n",
    ),
    "return": (
        'String greet() { return "Hi"; } String s = greet(); '
        "i32 n = (i32)s.len(); printf(\"%d\\n\", n);",
        "2\n",
    ),
    "object_array": (
        'String[] a; a.push("x"); a.push("yy"); i32 n = (i32)a.len(); '
        "i32 m = (i32)a[1].len(); printf(\"%d %d\\n\", n, m);",
        "2 2\n",
    ),
    "compare_concat": (
        'String s = "a" + "b"; bool b = s == "ab"; printf("%d\\n", b);',
        "1\n",
    ),
}

for name, (body, expected) in STRINGS.items():
    if name == "param":
        src = """
use C.stdio.*;
i32 len_of(String s) {
    return (i32)s.len();
}
void main() {
    String s = "Hello";
    printf("%d\\n", len_of(s));
}
"""
    elif name == "return":
        src = """
use C.stdio.*;
String greet() {
    return "Hi";
}
void main() {
    String s = greet();
    i32 n = (i32)s.len();
    printf("%d\\n", n);
}
"""
    else:
        src = wrap_main(body)
    write_test(f"m2/strings/{name}.hust", src, expected)

print("Generated M1 and M2 test files.")
