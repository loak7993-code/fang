# Learn Fang in 15 minutes

Never programmed before? Run `fang learn` first: it is an interactive
course in your terminal that starts from zero and checks every answer.
Then come back here for the full language.

Fang reads like Python and runs as native code. This tutorial covers the
whole language. Type the examples into a file called `intro.fang` and run
them with `fang run intro.fang` as you go. You can also paste single lines
into `fang repl` and see them run immediately.

Every example below runs exactly as printed.

## 1. Values

A Fang program is just lines in a file. The smallest program is one line:

```fang
print("hello")
```

Run it with `fang run hello.fang`. No function wrappers, no semicolons,
no imports. Fang has four primitive types: `int`, `float`, `bool`, and
`str`.

```fang
print(42)
print(3.5)
print(true)
print("hello")
```

Numbers, booleans, and strings print themselves. Division follows the
operands: `7 / 2` is `3` (int), `7.0 / 2` is `3.5` (float). That is the
biggest difference from Python and it is deliberate: no hidden type
changes in a compiled language.

```fang
fn main():
  print(7 / 2)
  print(float(7) / 2)
```

## 2. Names

Assigning to a new name creates it. `let` works too and means the same
thing.

```fang
x = 10
x = x + 5
print(x)
```

A name keeps its type. Assigning a different type is a compile error that
tells you both types.

```fang-expect-error
x = 1
x = 1.5
```

That program does not compile. Fang says `x is int; assigning float is not
allowed`. Convert the value or use a new name.

## 3. Functions

Define with `fn`. Parameter types are optional — Fang infers them from
how you use them. `fn main():` is the entry point; files with plain
top-level lines do not need one.

```fang
fn add(a, b):
  return a + b

print(add(2, 3))
```

A function that calls itself needs its return type declared, so the
compiler can check the recursion:

```fang
fn fib(n: int) -> int:
  if n < 2:
    return n
  return fib(n - 1) + fib(n - 2)

fn main():
  print(fib(10))
```

## 4. Control flow

Conditions must be booleans. There is no truthiness: an empty list is
`len(xs) == 0`, not just `xs`.

```fang
fn main():
  let n = 15
  if n % 15 == 0:
    print("fizzbuzz")
  elif n % 3 == 0:
    print("fizz")
  elif n % 5 == 0:
    print("buzz")
  else:
    print(n)
```

Loops: `while`, range `for`, and collection `for`.

```fang
fn main():
  let i = 0
  while i < 3:
    i = i + 1
  for j in 0..5:
    print(j)
  for k in 0..=3:
    print(k)
```

`0..5` excludes the end, `0..=3` includes it. Use `break` and `continue`
inside loops.

## 5. Collections

`list` and `dict` are built in. They are typed by what you put in them.

```fang
fn main():
  let xs = [4, 1, 3]
  xs.push(2)
  xs.sort()
  print(xs[0], xs[-1], len(xs))
  print(len(xs[1:3]))
```

An empty list needs its type written out, because there is nothing to
infer from:

```fang
fn main():
  let ys: list[int] = []
  ys.push(1)
  print(len(ys))
```

Dicts map `str` or `int` keys to values:

```fang
fn main():
  let ages = {"panda": 30, "fang": 1}
  ages.set("nova", 5)
  print(ages.get("panda", 0))
  print(ages.contains("fang"))
  for name, age in ages.items():
    print(f"{name} is {age}")
```

Indexing a list with `xs[i]` gives one element. Negative indexes count
from the end. Slices `xs[1:3]` give a new list.

## 6. Strings and f-strings

Strings are immutable. Build text with f-strings, which accept any
printable expression inside braces:

```fang
fn main():
  let name = "fang"
  let version = 1
  print(f"{name} v{version}, {1 + 2} tests")
```

String methods: `upper`, `lower`, `strip`, `split`, `join`, `replace`,
`contains`, `find`, `starts_with`, `ends_with`, `to_int`, `to_float`.

```fang
fn main():
  let parts = "a,b,c".split(",")
  print(len(parts))
  print("-".join(parts))
  print("42".to_int() + 1)
```

## 7. Structs and methods

A struct is a fixed set of named fields, passed by value.

```fang
struct Point:
  x: float
  y: float

fn Point.length(self) -> float:
  return (self.x * self.x + self.y * self.y) ** 0.5

fn main():
  let p = Point(3.0, 4.0)
  print(p.length())
  print(p.x)
```

Construct with the struct name like a function. Methods are functions
whose first parameter is `self`; they have no runtime cost over a plain
function call.

## 8. Functions as values, comprehensions, and swaps

A function is a value. Write one inline with `fn(...)` expressions; it
closes over the locals it reads (captured by value when created):

```fang
fn main():
  let base = 100
  let addbase = fn(x: int) -> int: return base + x
  print(addbase(5))
```

Name a function and pass it around the same way:

```fang
fn double(x: int) -> int:
  return x * 2

fn main():
  let d = double
  print(d(21))
```

Lists understand `map`, `filter`, and `reduce` with any function value:

```fang
fn double(x: int) -> int:
  return x * 2

fn main():
  let xs = [1, 2, 3, 4, 5, 6]
  print(xs.map(double)[0])
  print(len(xs.filter(fn(x: int) -> bool: return x % 2 == 0)))
  print(xs.reduce(fn(acc: int, x: int) -> int: return acc + x, 0))
```

Comprehensions build lists from lists, dict keys, or ranges:

```fang
fn main():
  let xs = [1, 2, 3, 4, 5, 6]
  let big = [x * 10 for x in xs if x > 3]
  print(len(big), big[0])
  let words = [f"n{x}" for x in xs]
  print(words[2])
```

Two binders iterate pairs, so dict items compose with comprehensions:

```fang
fn main():
  let ages = {"panda": 30, "fang": 1}
  let pairs = [f"{k}={v}" for k, v in ages.items()]
  print(pairs[0])
```

Swap (and any two-target tuple assignment) with one line:

```fang
fn main():
  let a = 1
  let b = 2
  a, b = b, a
  print(a, b)
```

Parameters can carry defaults, evaluated at the call site:

```fang
fn greet(name: str, greeting: str = "hello") -> str:
  return f"{greeting}, {name}"

fn main():
  print(greet("fang"))
  print(greet("panda", "yo"))
```

## 9. Errors

Runtime failures (bad index, bad division, failed assert) stop the
program with a message and the file and line. For expected failures,
`raise` a value and catch it with `try`/`except`:

```fang
fn main():
  try:
    raise "the disk is full"
  except e:
    print(f"caught: {e}")
```

## 10. Python libraries from Fang

Any Python library, imported and called from Fang. Build with
`fang run x.fang --python`:

```fang
import py

fn main():
  let math = py.import_module("math")
  let sqrt = py.attr(math, "sqrt")
  print(py.to_float(py.call(sqrt, [py.to_py_f(2.0)])))
```

## 11. Modules and packages

Standard modules load with `import` and are used by qualified name:

```fang
import math
import str

fn main():
  print(math.sqrt(2.0))
  print("HI".lower())
```

Your own files import by name when they sit next to the entry file:
`import utils` reads `utils.fang`. Install someone's package from a git
repository:

```
fang add github.com/someone/mypkg
```

That clones into `fang_packages/` and pins the commit in `fang.lock`. Then
`import mypkg` just works. Publishing a package means pushing a git
repository with `mypkg.fang` at its root.

## 12. Talking to C

Declare any C function and call it. This is how Fang reaches the whole C
ecosystem: GUI toolkits, databases, syscalls, kernel interfaces.

```fang
extern fn sqrt(x: float) -> float from "math.h"

fn main():
  print(sqrt(2.0))
```

Types map directly: `int` to `long long`, `float` to `double`,
`bool` to `bool`, `ptr` to `void*`. Convert between pointers and
integers with `cast`, and get a C string with `s.to_cstr()`.
Variadic C functions are not supported in v0.1.

## 13. Inline assembly and freestanding builds

For kernels and bare-metal targets:

```fang
fn main():
  asm("nop")
```

Build without the runtime, libc, or any dependency:

```
fang build kernel.fang --freestanding --target x86_64-elf
```

Freestanding programs use the value types (`int`, `float`, `bool`,
`ptr`, structs, `asm`); heap collections need the runtime.

## Differences from Python, in one list

- `let` is optional; assigning to a new name creates it
- parameter types are optional; unannotated parameters infer from
  how the body uses them (defaulting to int)
- `int / int` truncates; write `float(a) / b` for real division
- `if xs:` works for lists, dicts, and strings (nonempty is true);
  other conditions must be `bool`
- no classes and no exceptions hierarchy; `try`/`except` with plain
  values; closures capture by value, not by reference
- tuples exist for multiple returns and dict pairs, nothing else
- recursive functions declare their return type

That is the whole language. The standard library reference is in
`STDLIB.md`, and the full grammar is in `SPEC.md`.
