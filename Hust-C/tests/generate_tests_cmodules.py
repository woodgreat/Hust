#!/usr/bin/env python3
"""Generate C.stdlib, C.string and C-specific tests."""

from pathlib import Path

BASE = Path(__file__).resolve().parent


def write_test(rel, source, expected=None):
    path = BASE / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(source.strip() + "\n", encoding="utf-8")
    if expected is not None:
        path.with_suffix(".expected").write_text(expected, encoding="utf-8")


C_STDLIB = {
    "malloc_free": (
        """
use C.stdio.*;
use C.stdlib.*;
void main() {
    i32* p = malloc(sizeof(i32));
    *p = 42;
    printf("%d\\n", *p);
    free(p);
}
""",
        "42\n",
    ),
    "malloc_array": (
        """
use C.stdio.*;
use C.stdlib.*;
void main() {
    i32* a = malloc(3 * sizeof(i32));
    a[0] = 1;
    a[1] = 2;
    a[2] = 3;
    printf("%d\\n", a[0] + a[1] + a[2]);
    free(a);
}
""",
        "6\n",
    ),
    "calloc_array": (
        """
use C.stdio.*;
use C.stdlib.*;
void main() {
    i32* a = calloc(3, sizeof(i32));
    a[1] = 5;
    printf("%d\\n", a[0] + a[1] + a[2]);
    free(a);
}
""",
        "5\n",
    ),
    "realloc_array": (
        """
use C.stdio.*;
use C.stdlib.*;
void main() {
    i32* a = malloc(2 * sizeof(i32));
    a[0] = 1;
    a = realloc(a, 4 * sizeof(i32));
    a[3] = 4;
    printf("%d\\n", a[0] + a[3]);
    free(a);
}
""",
        "5\n",
    ),
    "null_check": (
        """
use C.stdio.*;
use C.stdlib.*;
void main() {
    i32* p = null;
    if (p == null) {
        printf("null\\n");
    }
}
""",
        "null\n",
    ),
    "free_null": (
        """
use C.stdio.*;
use C.stdlib.*;
void main() {
    i32* p = null;
    free(p);
    printf("ok\\n");
}
""",
        "ok\n",
    ),
    "malloc_float": (
        """
use C.stdio.*;
use C.stdlib.*;
void main() {
    f64* p = malloc(sizeof(f64));
    *p = 2.5;
    printf("%.1f\\n", *p);
    free(p);
}
""",
        "2.5\n",
    ),
    "sizeof_types": (
        """
use C.stdio.*;
void main() {
    printf("%d %d\\n", (i32)sizeof(i32), (i32)sizeof(i64));
}
""",
        "4 8\n",
    ),
}

for name, (src, expected) in C_STDLIB.items():
    write_test(f"m1/c_stdlib/{name}.hust", src, expected)

C_STRING = {
    "strlen_literal": (
        """
use C.stdio.*;
use C.string.*;
void main() {
    i32 n = (i32)strlen("hello");
    printf("%d\\n", n);
}
""",
        "5\n",
    ),
    "strcpy_strcat": (
        """
use C.stdio.*;
use C.string.*;
void main() {
    char dest[32];
    strcpy(dest, "hi");
    strcat(dest, "!");
    printf("%s\\n", dest);
}
""",
        "hi!\n",
    ),
    "strcmp_equal": (
        """
use C.stdio.*;
use C.string.*;
void main() {
    i32 r = strcmp("a", "a");
    printf("%d\\n", r);
}
""",
        "0\n",
    ),
    "strcmp_diff": (
        """
use C.stdio.*;
use C.string.*;
void main() {
    i32 r = strcmp("a", "b");
    printf("%d\\n", r < 0);
}
""",
        "1\n",
    ),
    "memcpy_test": (
        """
use C.stdio.*;
use C.string.*;
void main() {
    char a[8];
    char b[8];
    strcpy(a, "abc");
    memcpy(b, a, 4);
    printf("%s\\n", b);
}
""",
        "abc\n",
    ),
    "memset_test": (
        """
use C.stdio.*;
use C.string.*;
void main() {
    char a[8];
    memset(a, 'x', 3);
    a[3] = '\\0';
    printf("%s\\n", a);
}
""",
        "xxx\n",
    ),
    "strlen_var": (
        """
use C.stdio.*;
use C.string.*;
void main() {
    char s[16];
    strcpy(s, "hust");
    i32 n = (i32)strlen(s);
    printf("%d\\n", n);
}
""",
        "4\n",
    ),
    "strcat_chain": (
        """
use C.stdio.*;
use C.string.*;
void main() {
    char s[32];
    strcpy(s, "a");
    strcat(s, "b");
    strcat(s, "c");
    printf("%s\\n", s);
}
""",
        "abc\n",
    ),
    "strcmp_gt": (
        """
use C.stdio.*;
use C.string.*;
void main() {
    printf("%d\\n", strcmp("b", "a") > 0);
}
""",
        "1\n",
    ),
    "memcpy_strlen": (
        """
use C.stdio.*;
use C.string.*;
void main() {
    char src[8];
    char dst[8];
    strcpy(src, "xyz");
    memcpy(dst, src, 4);
    printf("%d\\n", (i32)strlen(dst));
}
""",
        "3\n",
    ),
}

for name, (src, expected) in C_STRING.items():
    write_test(f"m1/c_string/{name}.hust", src, expected)

C_SPECIFIC = {
    "pointer_decl": (
        """
use C.stdio.*;
void main() {
    i32 x = 42;
    i32* p = &x;
    printf("%d\\n", *p);
}
""",
        "42\n",
    ),
    "pointer_write": (
        """
use C.stdio.*;
void main() {
    i32 x = 1;
    i32* p = &x;
    *p = 5;
    printf("%d\\n", x);
}
""",
        "5\n",
    ),
    "pointer_arith": (
        """
use C.stdio.*;
void main() {
    i32 a[3] = {1, 2, 3};
    i32* p = a;
    printf("%d %d\\n", *p, *(p + 1));
}
""",
        "1 2\n",
    ),
    "pointer_to_pointer": (
        """
use C.stdio.*;
void main() {
    i32 x = 7;
    i32* p = &x;
    i32** pp = &p;
    printf("%d\\n", **pp);
}
""",
        "7\n",
    ),
    "null_pointer": (
        """
use C.stdio.*;
void main() {
    i32* p = null;
    if (p == null) {
        printf("null\\n");
    }
}
""",
        "null\n",
    ),
    "sizeof_type": (
        """
use C.stdio.*;
void main() {
    printf("%d\\n", (i32)sizeof(i32));
}
""",
        "4\n",
    ),
    "c_style_array": (
        """
use C.stdio.*;
void main() {
    i32 a[3] = {4, 5, 6};
    printf("%d\\n", a[0] + a[1] + a[2]);
}
""",
        "15\n",
    ),
    "pointer_param": (
        """
use C.stdio.*;
i32 sum(i32* a, i32 n) {
    i32 s = 0;
    for (i32 i = 0; i < n; i++) {
        s = s + a[i];
    }
    return s;
}
void main() {
    i32 a[3] = {1, 2, 3};
    printf("%d\\n", sum(a, 3));
}
""",
        "6\n",
    ),
}

for name, (src, expected) in C_SPECIFIC.items():
    write_test(f"c_specific/{name}.hust", src, expected)

print("Generated C.stdlib, C.string and C-specific tests.")
