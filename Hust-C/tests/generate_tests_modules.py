#!/usr/bin/env python3
"""Generate project-mode user module tests."""

from pathlib import Path

BASE = Path(__file__).resolve().parent
ROOT = BASE / "m4" / "modules_user"

SETTINGS = """
[project]
name = {name}
core = c
entry = main.hust

[modules]
paths = .
"""


def write_case(name, files, expected):
    d = ROOT / name
    d.mkdir(parents=True, exist_ok=True)
    (d / "settings.config").write_text(SETTINGS.format(name=name).strip() + "\n", encoding="utf-8")
    for filename, source in files.items():
        (d / filename).write_text(source.strip() + "\n", encoding="utf-8")
    (d / "main.expected").write_text(expected, encoding="utf-8")


MATH_ADD = """
public i32 add(i32 a, i32 b) {
    return a + b;
}
"""

write_case(
    "basic_use",
    {
        "math.hust": MATH_ADD,
        "main.hust": """
use C.stdio.*;
use math.*;

void main() {
    printf("%d\\n", add(2, 3));
}
""",
    },
    "5\n",
)

write_case(
    "prefix_use",
    {
        "math.hust": MATH_ADD,
        "main.hust": """
use C.stdio.*;
use math;

void main() {
    printf("%d\\n", math.add(2, 3));
}
""",
    },
    "5\n",
)

write_case(
    "specific_use",
    {
        "math.hust": """
public i32 square(i32 x) {
    return x * x;
}
""",
        "main.hust": """
use C.stdio.*;
use math.square;

void main() {
    printf("%d\\n", square(4));
}
""",
    },
    "16\n",
)

write_case(
    "class_use",
    {
        "shapes.hust": """
public class Point {
    i32 x;
    i32 y;

    public Point(i32 x, i32 y) {
        this.x = x;
        this.y = y;
    }

    public i32 sum() {
        return this.x + this.y;
    }
}
""",
        "main.hust": """
use C.stdio.*;
use shapes.*;

void main() {
    Point p = new Point(1, 2);
    printf("%d\\n", p.sum());
}
""",
    },
    "3\n",
)

write_case(
    "namespace_use",
    {
        "math.hust": """
namespace math;

public i32 add(i32 a, i32 b) {
    return a + b;
}
""",
        "main.hust": """
use C.stdio.*;
use math;

void main() {
    printf("%d\\n", math.add(3, 4));
}
""",
    },
    "7\n",
)

write_case(
    "dependency_chain",
    {
        "math.hust": MATH_ADD,
        "calc.hust": """
use math.*;

public i32 twice_sum(i32 a, i32 b) {
    return add(a, b) * 2;
}
""",
        "main.hust": """
use C.stdio.*;
use calc.*;

void main() {
    printf("%d\\n", twice_sum(1, 2));
}
""",
    },
    "6\n",
)

write_case(
    "alias_use",
    {
        "math.hust": MATH_ADD,
        "main.hust": """
use C.stdio.*;
use math as m;

void main() {
    printf("%d\\n", m.add(5, 6));
}
""",
    },
    "11\n",
)

print("Generated user module project tests.")
