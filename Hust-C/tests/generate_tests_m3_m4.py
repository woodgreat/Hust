#!/usr/bin/env python3
"""Generate M3 (OOP) and M4 (enum/match/modules/negative) tests."""

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


# ---------------------------------------------------------------- M3 classes

CLASSES = {
    "simple_fields": (
        """
use C.stdio.*;
class Point {
    i32 x;
    i32 y;
}
void main() {
    Point p = new Point();
    p.x = 1;
    p.y = 2;
    printf("%d\\n", p.x + p.y);
}
""",
        "3\n",
    ),
    "method_sum": (
        """
use C.stdio.*;
class Point {
    i32 x;
    i32 y;
    public i32 sum() {
        return this.x + this.y;
    }
}
void main() {
    Point p = new Point();
    p.x = 1;
    p.y = 2;
    printf("%d\\n", p.sum());
}
""",
        "3\n",
    ),
    "ctor_explicit": (
        """
use C.stdio.*;
class Point {
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
void main() {
    Point p = new Point(1, 2);
    printf("%d\\n", p.sum());
}
""",
        "3\n",
    ),
    "implicit_ctor": (
        """
use C.stdio.*;
class Point {
    i32 x;
    i32 y;
}
void main() {
    Point p = new Point();
    p.x = 4;
    p.y = 5;
    printf("%d\\n", p.x + p.y);
}
""",
        "9\n",
    ),
    "ctor_loop": (
        """
use C.stdio.*;
class Counter {
    i32 total;
    public Counter(i32 n) {
        this.total = 0;
        for (i32 i = 0; i < n; i++) {
            this.total = this.total + 1;
        }
    }
}
void main() {
    Counter c = new Counter(4);
    printf("%d\\n", c.total);
}
""",
        "4\n",
    ),
    "visibility_public": (
        """
use C.stdio.*;
class Box {
    public i32 value;
    public i32 get() {
        return this.value;
    }
}
void main() {
    Box b = new Box();
    b.value = 7;
    printf("%d\\n", b.get());
}
""",
        "7\n",
    ),
    "private_internal": (
        """
use C.stdio.*;
class Box {
    i32 value;
    public void set(i32 v) {
        this.value = v;
    }
    public i32 get() {
        return this.value;
    }
}
void main() {
    Box b = new Box();
    b.set(8);
    printf("%d\\n", b.get());
}
""",
        "8\n",
    ),
    "nested_class": (
        """
use C.stdio.*;
class Outer {
    class Inner {
        i32 value;
        public i32 get() {
            return this.value;
        }
    }
}
void main() {
    Outer.Inner i = new Outer.Inner();
    i.value = 6;
    printf("%d\\n", i.get());
}
""",
        "6\n",
    ),
    "class_return": (
        """
use C.stdio.*;
class Point {
    i32 x;
    public Point(i32 x) {
        this.x = x;
    }
}
Point make() {
    return new Point(5);
}
void main() {
    Point p = make();
    printf("%d\\n", p.x);
}
""",
        "5\n",
    ),
    "object_array": (
        """
use C.stdio.*;
class Point {
    i32 x;
    public Point(i32 x) {
        this.x = x;
    }
}
void main() {
    Point[] ps;
    ps.push(new Point(1));
    ps.push(new Point(2));
    printf("%d %d %d\\n", (i32)ps.len(), ps[0].x, ps[1].x);
}
""",
        "2 1 2\n",
    ),
    "clone_default": (
        """
use C.stdio.*;
class Point {
    i32 x;
    public Point(i32 x) {
        this.x = x;
    }
}
void main() {
    Point a = new Point(1);
    Point b = a;
    b.x = 2;
    printf("%d %d\\n", a.x, b.x);
}
""",
        "1 2\n",
    ),
    "moveit": (
        """
use C.stdio.*;
class Point {
    i32 x;
    public Point(i32 x) {
        this.x = x;
    }
}
void main() {
    Point a = new Point(1);
    Point b = a.moveit();
    printf("%d\\n", b.x);
}
""",
        "1\n",
    ),
    "readonly_method": (
        """
use C.stdio.*;
class Point {
    i32 x;
    public Point(i32 x) {
        this.x = x;
    }
    public readonly i32 get() {
        return this.x;
    }
}
void main() {
    Point p = new Point(3);
    printf("%d\\n", p.get());
}
""",
        "3\n",
    ),
    "method_params": (
        """
use C.stdio.*;
class Adder {
    i32 base;
    public Adder(i32 base) {
        this.base = base;
    }
    public i32 add(i32 v) {
        return this.base + v;
    }
}
void main() {
    Adder a = new Adder(10);
    printf("%d\\n", a.add(5));
}
""",
        "15\n",
    ),
    "field_array": (
        """
use C.stdio.*;
class Board {
    i32[3] cells;
}
void main() {
    Board b = new Board();
    b.cells[0] = 1;
    b.cells[1] = 2;
    b.cells[2] = 3;
    printf("%d\\n", b.cells[0] + b.cells[1] + b.cells[2]);
}
""",
        "6\n",
    ),
    "ctor_self_method": (
        """
use C.stdio.*;
class Level {
    i32 budget;
    public Level(i32 lv) {
        this.budget = this.calc(lv);
    }
    public i32 calc(i32 lv) {
        return lv * 100;
    }
}
void main() {
    Level l = new Level(3);
    printf("%d\\n", l.budget);
}
""",
        "300\n",
    ),
}

for name, (src, expected) in CLASSES.items():
    write_test(f"m3/classes/{name}.hust", src, expected)

# ---------------------------------------------------------------- M3 inheritance

INHERITANCE = {
    "single_field": (
        """
use C.stdio.*;
class Animal {
    i32 age;
}
class Dog extends Animal {
}
void main() {
    Dog d = new Dog();
    d.Animal.age = 5;
    printf("%d\\n", d.Animal.age);
}
""",
        "5\n",
    ),
    "parent_method": (
        """
use C.stdio.*;
class Animal {
    i32 age;
    public i32 get_age() {
        return this.age;
    }
}
class Dog extends Animal {
}
void main() {
    Dog d = new Dog();
    d.Animal.age = 6;
    printf("%d\\n", d.Animal.get_age());
}
""",
        "6\n",
    ),
    "child_method": (
        """
use C.stdio.*;
class Animal {
    i32 age;
    public void eat() {
    }
}
class Dog extends Animal {
    public i32 bark_volume() {
        return 9;
    }
}
void main() {
    Dog d = new Dog();
    printf("%d\\n", d.bark_volume());
}
""",
        "9\n",
    ),
    "super_ctor": (
        """
use C.stdio.*;
class Animal {
    i32 age;
    public Animal(i32 age) {
        this.age = age;
    }
}
class Dog extends Animal {
    public Dog() {
        super(4);
    }
}
void main() {
    Dog d = new Dog();
    printf("%d\\n", d.Animal.age);
}
""",
        "4\n",
    ),
    "parent_and_child": (
        """
use C.stdio.*;
class Animal {
    i32 age;
    public Animal(i32 age) {
        this.age = age;
    }
    public i32 get_age() {
        return this.age;
    }
}
class Dog extends Animal {
    i32 volume;
    public Dog(i32 age, i32 volume) {
        super(age);
        this.volume = volume;
    }
}
void main() {
    Dog d = new Dog(3, 8);
    printf("%d %d\\n", d.Animal.get_age(), d.volume);
}
""",
        "3 8\n",
    ),
}

for name, (src, expected) in INHERITANCE.items():
    write_test(f"m3/inheritance/{name}.hust", src, expected)

# ---------------------------------------------------------------- M3 interfaces

INTERFACES = {
    "shape_getter": (
        """
use C.stdio.*;
interface Getter {
    public value get();
}
class Const implements Getter {
    public Const() {
    }
    public i32 get() {
        return 7;
    }
}
void main() {
    Getter(i32) g = new Const();
    printf("%d\\n", g.get());
}
""",
        "7\n",
    ),
    "shape_consumer": (
        """
use C.stdio.*;
interface Consumer {
    public result consume(item);
}
class PlusOne implements Consumer {
    public PlusOne() {
    }
    public i32 consume(i32 item) {
        return item + 1;
    }
}
void main() {
    Consumer(i32, i32) c = new PlusOne();
    printf("%d\\n", c.consume(4));
}
""",
        "5\n",
    ),
    "two_impls": (
        """
use C.stdio.*;
interface Getter {
    public value get();
}
class Seven implements Getter {
    public Seven() {
    }
    public i32 get() {
        return 7;
    }
}
class Nine implements Getter {
    public Nine() {
    }
    public i32 get() {
        return 9;
    }
}
void main() {
    Getter(i32) g = new Seven();
    printf("%d\\n", g.get());
    g = new Nine();
    printf("%d\\n", g.get());
}
""",
        "7\n9\n",
    ),
    "interface_param": (
        """
use C.stdio.*;
interface Getter {
    public value get();
}
class Seven implements Getter {
    public Seven() {
    }
    public i32 get() {
        return 7;
    }
}
void feed(Getter(i32) g) {
    printf("%d\\n", g.get());
}
void main() {
    feed(new Seven());
}
""",
        "7\n",
    ),
    "interface_return": (
        """
use C.stdio.*;
interface Getter {
    public value get();
}
class Seven implements Getter {
    public Seven() {
    }
    public i32 get() {
        return 7;
    }
}
Getter(i32) make() {
    return new Seven();
}
void main() {
    Getter(i32) g = make();
    printf("%d\\n", g.get());
}
""",
        "7\n",
    ),
    "poly_array": (
        """
use C.stdio.*;
interface Getter {
    public value get();
}
class Seven implements Getter {
    public Seven() {
    }
    public i32 get() {
        return 7;
    }
}
class Nine implements Getter {
    public Nine() {
    }
    public i32 get() {
        return 9;
    }
}
void main() {
    Getter(i32)[] gs;
    gs.push(new Seven());
    gs.push(new Nine());
    for (i32 i = 0; i < gs.len(); i++) {
        printf("%d\\n", gs[i].get());
    }
}
""",
        "7\n9\n",
    ),
    "multi_params": (
        """
use C.stdio.*;
interface Adder {
    public result add(a, b);
}
class IntAdder implements Adder {
    public IntAdder() {
    }
    public i32 add(i32 a, i32 b) {
        return a + b;
    }
}
void main() {
    Adder(i32, i32, i32) a = new IntAdder();
    printf("%d\\n", a.add(3, 4));
}
""",
        "7\n",
    ),
    "dispatch": (
        """
use C.stdio.*;
interface Getter {
    public value get();
}
class Seven implements Getter {
    public Seven() {
    }
    public i32 get() {
        return 7;
    }
}
class Nine implements Getter {
    public Nine() {
    }
    public i32 get() {
        return 9;
    }
}
void main() {
    Getter(i32) g;
    g = new Seven();
    printf("%d\\n", g.get());
    g = new Nine();
    printf("%d\\n", g.get());
}
""",
        "7\n9\n",
    ),
}

for name, (src, expected) in INTERFACES.items():
    write_test(f"m3/interfaces/{name}.hust", src, expected)

# ---------------------------------------------------------------- M4 enum/match

ENUM_MATCH = {
    "enum_value": (
        """
use C.stdio.*;
enum Color {
    Red,
    Green,
    Blue
}
void main() {
    Color c = Color.Green;
    printf("%d\\n", c);
}
""",
        "1\n",
    ),
    "match_all": (
        """
use C.stdio.*;
enum Color {
    Red,
    Green,
    Blue
}
void main() {
    Color c = Color.Green;
    match c {
        Color.Red : printf("red\\n"),
        Color.Green : printf("green\\n"),
        Color.Blue : printf("blue\\n"),
        default : printf("other\\n")
    }
}
""",
        "green\n",
    ),
    "match_default": (
        """
use C.stdio.*;
enum Color {
    Red,
    Green
}
void main() {
    Color c = Color.Green;
    match c {
        Color.Red : printf("red\\n"),
        default : printf("other\\n")
    }
}
""",
        "other\n",
    ),
    "match_block": (
        """
use C.stdio.*;
enum Color {
    Red,
    Green
}
void main() {
    Color c = Color.Red;
    match c {
        Color.Red : {
            printf("red\\n");
        },
        Color.Green : {
            printf("green\\n");
        },
        default : {
            printf("other\\n");
        }
    }
}
""",
        "red\n",
    ),
    "enum_var": (
        """
use C.stdio.*;
enum Color {
    Red,
    Green,
    Blue
}
void main() {
    Color c = Color.Blue;
    Color d = c;
    printf("%d\\n", d);
}
""",
        "2\n",
    ),
    "match_two_values": (
        """
use C.stdio.*;
enum Color {
    Red,
    Green,
    Blue
}
void main() {
    Color a = Color.Red;
    Color b = Color.Blue;
    match a {
        Color.Red : printf("a=red\\n"),
        default : printf("a=other\\n")
    }
    match b {
        Color.Blue : printf("b=blue\\n"),
        default : printf("b=other\\n")
    }
}
""",
        "a=red\nb=blue\n",
    ),
}

for name, (src, expected) in ENUM_MATCH.items():
    write_test(f"m4/enum_match/{name}.hust", src, expected)

# ---------------------------------------------------------------- M4 modules

MODULES = {
    "c_stdio": (
        'use C.stdio.*;\nvoid main() {\n    printf("stdio\\n");\n}\n',
        "stdio\n",
    ),
    "c_math": (
        "use C.stdio.*;\nuse C.math.*;\nvoid main() {\n    printf(\"%.1f\\n\", sqrt(9.0));\n}\n",
        "3.0\n",
    ),
    "multiple": (
        "use C.stdio.*;\nuse C.math.*;\nvoid main() {\n    f64 x = pow(2.0, 4.0);\n    printf(\"%.1f\\n\", x);\n}\n",
        "16.0\n",
    ),
    "import_order": (
        "use C.math.*;\nuse C.stdio.*;\nvoid main() {\n    printf(\"%.1f\\n\", floor(2.9));\n}\n",
        "2.0\n",
    ),
    "c_stdlib_import": (
        "use C.stdlib.*;\nvoid main() {\n    pass;\n}\n",
        None,
    ),
}

for name, (src, expected) in MODULES.items():
    write_test(f"m4/modules/{name}.hust", src, expected)

# ---------------------------------------------------------------- M4 negative

NEGATIVE = {
    "self_deprecated": (
        "void main() {\n    self;\n}\n",
        "self",
    ),
    "shape_type_in_interface": (
        "interface Bad {\n    public void draw();\n}\nvoid main() {\n}\n",
        "interface",
    ),
    "duplicate_ctor": (
        """
class Point {
    i32 x;
    public Point() {
        this.x = 0;
    }
    public Point(i32 x) {
        this.x = x;
    }
}
void main() {
}
""",
        "constructor",
    ),
    "rust_import": (
        "use RUST.io.*;\nvoid main() {\n}\n",
        "RUST",
    ),
    "rust_macro": (
        'void main() {\n    println!("x");\n}\n',
        "println",
    ),
    "mixed_import": (
        "use C.stdio.*;\nuse RUST.io.*;\nvoid main() {\n}\n",
        "mixed",
    ),
    "missing_import_printf": (
        'void main() {\n    printf("x\\n");\n}\n',
        "C.stdio",
    ),
    "unknown_c_module": (
        "use C.unknown.*;\nvoid main() {\n}\n",
        "unknown",
    ),
    "hust_not_available": (
        "use HUST.math.*;\nvoid main() {\n}\n",
        "HUST",
    ),
    "improper_c_import": (
        "use C.stdio;\nvoid main() {\n}\n",
        "C.stdio",
    ),
}

for name, (src, needle) in NEGATIVE.items():
    write_test(f"m4/negative/{name}.hust", src, expected_err=needle)

print("Generated M3 and M4 test files.")
