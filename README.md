<div align="center">

<img src="docs/logo.svg" width="96" alt="Leo" />

# Leo

**Looks like Python. Runs like C.**

A small compiled language you can learn in an afternoon and ship
anywhere C ships: servers, CLI tools, kernels, desktop, embedded.

[Learn it in 15 minutes](LEARN.md) · [Language spec](SPEC.md) · [Standard library](STDLIB.md)

</div>

---

## Why Leo

Leo compiles through C, so it runs at native speed with no interpreter
and no garbage collector pauses. The syntax is Python-shaped: no
semicolons, no type declarations on locals, f-strings, indentation
blocks. The whole language fits in one sitting, and the whole C
ecosystem is callable through `extern`.

```leo
import math

struct Vec2:
  x: float
  y: float

fn Vec2.norm(self) -> float:
  return math.sqrt(self.x * self.x + self.y * self.y)

fn main():
  let v = Vec2(3.0, 4.0)
  print(f"norm = {v.norm()}")
  let xs = [3, 1, 2]
  xs.sort()
  print(xs[0], xs[-1], len(xs))
```

## Measured speed

Same algorithms, median of 5 runs, on an AMD EPYC-Milan 8-core
(Run `bench/run.sh` yourself: the suite ships in the repository).

| benchmark | Leo | Python 3 | C (gcc -O2) | Rust (rustc -O) |
|-----------|------:|---------:|------------:|----------------:|
| fib(32) recursive | 0.010 s | 0.617 s | 0.009 s | 0.013 s |
| loop sum, 100M | 0.001 s | 8.847 s | 0.002 s | 0.002 s |
| string build, 100k | 0.218 s | 0.255 s | n/a | 0.002 s |
| dict ops, 1M | 0.049 s | 0.356 s | n/a | 0.074 s |
| sort 1M ints | 0.178 s | 0.971 s | n/a | 0.064 s |

Arithmetic lands at C parity, alongside Rust. Integer-keyed dict ops
beat Rust's default-hasher HashMap. Two honest losses: `sort` (C qsort
through a function pointer versus Rust's inlined pdqsort, about 3x) and
string `+` (Leo always allocates a fresh string, so quadratic in the
number of concatenations; Rust reuses capacity. The builder behind
f-strings is linear; a `+` fast path is on the v0.2 list).

## Install

You need Rust (to build the compiler) and a C compiler (gcc or clang).

```sh
git clone <this repository> leo
cd leo
cargo install --path crates/leo-cli
leo version
```

Or build and use in place:

```sh
cargo build --release
./target/release/leo run examples/hello.leo
```

`leo` embeds its runtime and standard library in the binary, so the tool
itself has no runtime dependencies beyond `cc`.

## The tool

```sh
leo run app.leo              # compile and execute
leo build app.leo --release  # standalone native binary (-O2)
leo build kernel.leo --freestanding --target x86_64-elf  # no runtime, no libc
leo repl                     # interactive session
leo fmt src/*.leo            # canonical formatting
leo test                     # run every *_test.leo under the current directory
leo add github.com/user/pkg  # install a package, pinned in leo.lock
```

Exit codes: 0 success, 1 compile error, 2 test failure, 101 runtime panic.

## Reaching the rest of the world

Call any fixed-arity C function directly:

```leo
extern fn sqrt(x: float) -> float from "math.h"

fn main():
  print(sqrt(2.0))
```

For kernels and bare metal, `leo build --freestanding` emits C with no
runtime and no libc, compiles with `-nostdlib`, and passes any
`--target` triple through to the C compiler. Inline assembly is a
statement:

```leo
fn main():
  asm("nop")
```

Mobile and desktop apps link against the platform C libraries the same
way the `extern` examples do.

## Packages

A package is a git repository with `mypkg.leo` at its root. There is no
registry, no account, and no server to run:

```sh
leo add github.com/someone/mypkg
```

```leo
import mypkg
```

`leo.lock` pins each package to a commit hash. Packages cannot depend
on other packages in v0.1.

## Where Leo is going

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
crates/leo-core/    compiler library: lexer, parser, sema, codegen
crates/leo-cli/     the leo binary
std/                standard library, written in Leo
runtime/            the C runtime (leo_rt.c, leo_rt.h)
tests/conformance/  programs with expected output, run by cargo test
bench/              benchmark suite (bench/run.sh)
examples/           runnable example programs
docs/               this documentation and the web page
```

## License

MIT. See [LICENSE](LICENSE). The Geist font files in `docs/fonts/` are
SIL Open Font License 1.1, copyright Vercel.
