# Changelog

All notable changes to Leo are documented here. The format follows
[keep a changelog](https://keepachangelog.com/); versions follow
semantic versioning.

## [0.3.1] - 2026-10-20

### Fixed (found in a dedicated bug hunt)

- Closure environments emitted an invalid C statement (missing
  semicolon) whenever a lambda captured a list, dict, or string: any
  such program failed to compile.
- Calls through zero-argument closures emitted a trailing comma:
  `f(c, )` instead of `f(c)`.
- Lambda parameters written without types inferred to int always,
  even when the body used them as strings (`prefix + name`).
  Parameter inference now runs inside function expressions, using
  literals, string methods, the declared return type, and known
  enclosing locals as constraints.
- `x, y = pair()` silently assigned zeros when `x` or `y` were new
  names: tuple assignment now declares missing targets with their
  part types.
- A multi-line function expression as the right side of an assignment
  (`risky = fn(n): ...`) consumed the statement's line terminator and
  failed to parse whatever came next.
- A `try`/`except` where both arms return was misreported as "not
  every path returns a value".
- Two lambdas with the same signature could not share a list
  (`list elements disagree: fn then fn`): function types now compare
  by signature.
- `d: dict[str, int] = {}` did not parse on bare assignment, leaving
  no way to annotate an empty collection without `let`. Typed
  assignment now works everywhere, and annotates empty collection
  literals.
- `str(x)` did not exist: converting a number or collection to a
  string required f-strings. It is now a builtin.
- Lambdas passed to `map`, `filter`, and `reduce` had their inferred
  signatures checked before the call could supply the expected types;
  they now adapt to the higher-order call site before failing.

## [0.3.0] - 2026-10-20

Simpler than Python for getting started.

### Added

- Top-level scripts: a file of plain lines runs directly; no
  `fn main()` required. `print("hello")` is a complete program.
- Plain assignment creates names; `let` is optional.
- Parameter types are optional: unannotated parameters infer from
  body usage (literals, string methods, declared return types),
  defaulting to int.
- `input("prompt")` is a global builtin with an optional prompt.
- `print` renders lists, dicts, and tuples directly (`[1, 2, 3]`).
- Conditions accept strings, lists, and dicts: nonempty is true
  (`if xs:`).

## [0.2.2] - 2026-10-20

### Added

- `leo learn`: an eight-lesson interactive terminal course for people
  with zero programming experience. Each lesson explains a concept in
  plain language, has the learner fill in a blank in a real program,
  compiles and runs it, and checks the output. Wrong answers show the
  actual output or compile error plus hints. Progress saves between
  sessions; `leo learn list` and `leo learn reset` manage it.
- `leo new <name>`: scaffold a starter project.
- `tools/check_learn_cmd.sh`: drives the course end to end in CI,
  including rejecting wrong answers.

## [0.2.1] - 2026-10-20

### Added

- Python bridge: `leo run app.leo --python` embeds CPython (via
  `python3-config --embed`) and the `py` stdlib module imports any
  Python library, calls functions and methods, evaluates Python source,
  and converts values in both directions. Third-party packages work:
  the shipped example calls numpy.

## [0.2.0] - 2026-10-20

### Added

- Function values: `fn(x: int) -> int: return x * 2` expressions with
  closures (capture by value), named functions as values, and statically
  typed calls through function values.
- Higher-order list methods: `map`, `filter`, `reduce`, accepting
  lambdas or named functions.
- List comprehensions: `[x * x for x in xs if x > 2]`, over lists, dict
  keys, and ranges, with optional conditions and two-binder pair
  destructuring (`for k, v in d.items()`).
- Tuple assignment: `a, b = b, a` swap and 2-tuple unpacking.
- Default parameters: `fn greet(name: str, greeting: str = "hello")`,
  evaluated at the call site.
- `dict.reserve(n)` for pre-sizing.
- Conformance suite now runs in `cargo test` (was missing from the
  workspace).

## [0.1.0] - 2026-10-19

First public build. The whole pipeline runs: lexer, parser, semantic
analysis with type inference, C code generation, native compilation.

### Added

- Language: `let`, `fn` with inferred returns, `struct` with methods,
  `if/elif/else`, `while`, `for` over ranges, lists, dicts, and dict
  items, `try/except` with `raise`, f-strings, slices, negative
  indexing, `cast` between ptr and RC types, `asm` statements, and
  `extern` C declarations.
- Types: `int`, `float`, `bool`, `str`, `ptr`, `list[T]`,
  `dict[K, V]`, tuples for multiple returns, nominal structs.
- Memory: reference counting with deterministic release at function
  exit. Documented leaks: cycles, raise paths, ptr-cast holders.
- Tooling: `leo run`, `leo build` (debug, `--release`, `--freestanding`
  with `--target` passthrough), `leo repl`, `leo fmt` (canonical style,
  round-trip tested), `leo test` (file-based), `leo add` (git packages
  with `leo.lock` pinning).
- Standard library: `io`, `str`, `list` and `dict` methods, `math`,
  `os`, `fs`, `time`, `json` (pure Leo), `net` (TCP and HTTP/1.0).
- Documentation: `README.md`, `LEARN.md` (every example executed by
  `tools/check_learn.sh`), `SPEC.md`, `STDLIB.md`.
- Testing: 67 unit and integration tests, a conformance suite of
  programs with pinned output, formatter fixpoint property, benchmark
  suite (`bench/run.sh`) against Python and C.

### Performance

- String `+` in the `s = s + x` assignment idiom reuses capacity: linear
  instead of quadratic (100x on the 100k build benchmark, now at Rust parity).
- Sorts are type-specialized inlined introsorts; integer lists use
  range-adaptive LSD radix sort over offset-biased keys: three passes
  for 32-bit data, six only when the range demands it (2.4x over the
  qsort path, ahead of Rust's pdqsort on large int lists).
- Release builds compile with `-O3 -march=native` (about 4% over -O2
  on collection-heavy code).
- Benchmarks include Rust and C++; no benchmark where either is
  meaningfully faster. Dict slots shrank from 32 to 24 bytes with a 0.8
  load factor (a third of the previous cache footprint), and dicts
  gained `reserve(n)`; dict ops now edge out C++ `std::unordered_map`
  and double Rust.

### Known limitations

See the README list. The headline ones: no closures, no user generics,
no threads; reference cycles and raise paths leak; no TLS; no
variadic externs; no MSVC.
