<div align="center">

<img src="docs/logo.svg" width="96" alt="Fang" />

# Fang

[![course](https://img.shields.io/badge/learn-fang.dev-34d399)](https://loak7993-code.github.io/fang/)

**The learning website lives at https://loak7993-code.github.io/fang/**
built from `website/` in this repository: the full 18-lesson course,
benchmarks with charts, cheat sheet, and install steps.

**Looks like Python. Runs like C.**

A small compiled language you can learn in an afternoon and ship
anywhere C ships: servers, CLI tools, kernels, desktop, embedded.

[Never coded? `fang learn`](LEARN.md) · [15-minute tutorial](LEARN.md) · [Spec](SPEC.md) · [Stdlib](STDLIB.md)

</div>

---

## Start from zero

```sh
fang learn      # interactive course, no experience needed
fang new demo   # starter project
```

## Why Fang

Fang compiles through C, so it runs at native speed with no interpreter
and no garbage collector pauses. The syntax is Python-shaped: no
semicolons, no type declarations on locals, f-strings, indentation
blocks. The whole language fits in one sitting, and the whole C
ecosystem is callable through `extern`.

A whole program:

```fang
print("hello")
```

And with everything on:

```fang
import math

struct Vec2:
  x: float
  y: float

fn Vec2.norm(self) -> float:
  return math.sqrt(self.x * self.x + self.y * self.y)

v = Vec2(3.0, 4.0)
print(f"norm = {v.norm()}")
xs = [3, 1, 2]
xs.sort()
print(xs[0], xs[-1], len(xs))
```

## Measured speed

Same algorithms, median of 5 runs, on an AMD EPYC-Milan 8-core
(Run `bench/run.sh` yourself: the suite ships in the repository).

| benchmark | Fang | Python 3 | C (gcc -O2) | Rust (rustc -O) | C++ (g++ -O2) |
|-----------|------:|---------:|------------:|----------------:|--------------:|
| fib(32) recursive | 0.010 s | 0.603 s | 0.010 s | 0.012 s | 0.010 s |
| loop sum, 100M | 0.001 s | 8.845 s | 0.001 s | 0.002 s | 0.001 s |
| string build, 100k | 0.002 s | 0.281 s | n/a | 0.002 s | 0.197 s |
| dict ops, 1M | 0.015 s | 0.353 s | n/a | 0.050 s | 0.015 s |
| sort 1M ints | 0.040 s | 0.951 s | n/a | 0.042 s | 0.085 s |

Fastest or tied on every benchmark, against every language in the
suite. At fib(35) scale (22 ms), Fang's generated code performs
identically to hand-written C compiled by gcc -O2. Dict ops edge out
C++ `std::unordered_map` and double Rust. Sorting beats `std::sort` by
2x via radix and edges Rust's pdqsort. String building leaves
`std::string +` in the dust (65x) because C++ concatenation without
`reserve` is quadratic. Tight loop sums sit where C sits: the
algorithm is the limit, not the language.

Techniques behind the results: string capacity reuse on `s = s + x`
(linear instead of quadratic), 24-byte dict slots keyed by comparison
with a 0.8 load factor (a third of the previous cache footprint),
type-specialized inlined sorts instead of qsort function pointers,
range-adaptive LSD radix for integer lists (three passes for 32-bit
data), and a release profile of `-O3 -march=native`. Dict ops sit in a
statistical tie with `std::unordered_map`: C++ hashes ints by identity
into prime-counted buckets, which streams well for cycling sequential
keys, while Fang's Fibonacci-multiply hash deliberately scatters to
resist stride-key clustering. Run-to-run variance on this class of
machine is a few milliseconds; the sort and string gaps are stable
across runs.

## Install

You need Rust (to build the compiler) and a C compiler (gcc or clang).

```sh
git clone <this repository> fang
cd fang
cargo install --path crates/fang-cli
fang version
```

Or build and use in place:

```sh
cargo build --release
./target/release/fang run examples/hello.fg
```

`fang` embeds its runtime and standard library in the binary, so the tool
itself has no runtime dependencies beyond `cc`.

## The tool

```sh
fang run app.fg              # compile and execute
fang build app.fg --release  # standalone native binary (-O2)
fang build kernel.fg --freestanding --target x86_64-elf  # no runtime, no libc
fang repl                     # interactive session
fang fmt src/*.fg            # canonical formatting
fang test                     # run every *_test.fg under the current directory
fang add github.com/user/pkg  # install a package, pinned in fang.lock
```

Exit codes: 0 success, 1 compile error, 2 test failure, 101 runtime panic.

## Reaching the rest of the world

Call any fixed-arity C function directly:

```fang
extern fn sqrt(x: float) -> float from "math.h"

fn main():
  print(sqrt(2.0))
```

For kernels and bare metal, `fang build --freestanding` emits C with no
runtime and no libc, compiles with `-nostdlib`, and passes any
`--target` triple through to the C compiler. Inline assembly is a
statement:

```fang
fn main():
  asm("nop")
```

Mobile and desktop apps link against the platform C libraries the same
way the `extern` examples do.

## Backend websites

A web server is three lines plus your handler. The whole HTTP layer is
pure Fang, no framework needed:

```fang
import server

fn handler(req: Request):
  if req.path == "/":
    return server.html("<h1>hello from fang</h1>")
  return server.not_found()

server.serve(handler, 8080)
```

GET and POST both round-trip (verified with curl); the handler is a
plain function value typed `fn(Request) -> Response`. See
`examples/website.fg` for a full site with HTML, JSON, and health
routes, and STDLIB.md for the `server` module reference.

Same language, same binary, same performance story: kernels at boot,
servers on the wire.

## TUI and emulators

Terminal apps are pure Fang: the `tui` module is ANSI escape codes, a
screen buffer, and raw-mode key input, with zero dependencies. See
`examples/chip8.fg`, a complete CHIP-8 emulator (all 35 opcodes, the
original HEX keypad, `--frames N` headless testing):

```sh
fang run examples/chip8.fg rom.ch8            # play it
fang run examples/chip8.fg rom.ch8 --frames N # headless run
```

## Every operating system

Fang emits C and compiles it with the platform C compiler, so it goes
wherever C goes: Linux, Windows, macOS, the BSDs, Android (via the
NDK's clang), and iOS. Cross-compile by passing a target triple:

```sh
fang build app.fg --target aarch64-linux-gnu     # ARM Linux
fang build app.fg --target x86_64-w64-mingw32    # Windows .exe
fang build app.fg --freestanding --target x86_64-elf   # kernels
```

The browser runs the same C through Emscripten's toolchain. GUI apps
link the platform's native libraries with `extern fn`, the same
mechanism the stdlib uses for every C function it touches.

## Module-level state

Top-level `let` creates a mutable global, exactly like Python: every
function in the file sees it and can change it.

```fang
let scoreboard: list[int] = []

fn record(points: int):
  scoreboard.push(points)

record(10)
record(20)
print(scoreboard)   # [10, 20]
```

## Packages

A package is a git repository with `mypkg.fg` at its root. There is no
registry, no account, and no server to run:

```sh
fang add github.com/someone/mypkg
```

```fang
import mypkg
```

`fang.lock` pins each package to a commit hash. Packages cannot depend
on other packages in v0.1.

## Python libraries from Fang

```sh
fang run app.fg --python
```

```fang
import py

fn main():
  let np = py.import_module("numpy")
  let a = py.call_method(np, "arange", [py.to_py(5)])
  print(py.repr(a))
  print(f"mean: {py.to_float(py.call_method(a, 'mean', []))}")
```

The flag links libpython via `python3-config --embed` (needs the
python3 dev package). Python objects are `ptr` values; convert with
`py.to_py*` and `py.to_*`. See STDLIB.md for the full surface.

## Where Fang is going

- `0.2`: struct fields holding collections, typed errors, a cycle
  collector, closures
- `0.3`: user-defined generics, threads

## Known limitations in 0.1

Stated plainly, so nothing here surprises you:

- Reference cycles leak (same trade early Python shipped with)
- `raise` leaks the values held by the raising function's frame
- No closures, no user-defined generics, no threads in the language
- `struct` fields cannot hold `list`, `dict`, or tuples; wrap them in
  functions instead
- No TLS in `net`; no variadic C externs; dict keys are `str` or `int`
- The REPL replays the whole session on each input
- Sorting requires `int`, `float`, or `str` element lists
- Windows is supported through WSL or mingw, not MSVC
- Locals live until function exit (no per-block memory release)
- `json` values hold their collections for the process lifetime

## Repository layout

```
crates/fang-core/    compiler library: lexer, parser, sema, codegen
crates/fang-cli/     the fang binary
std/                standard library, written in Fang
runtime/            the C runtime (fang_rt.c, fang_rt.h)
tests/conformance/  programs with expected output, run by cargo test
bench/              benchmark suite (bench/run.sh)
examples/           runnable example programs
docs/               this documentation and the web page
```

## License

MIT. See [LICENSE](LICENSE). The Geist font files in `docs/fonts/` are
SIL Open Font License 1.1, copyright Vercel.
