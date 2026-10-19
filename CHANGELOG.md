# Changelog

All notable changes to Leo are documented here. The format follows
[keep a changelog](https://keepachangelog.com/); versions follow
semantic versioning.

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
- Sorts are type-specialized inlined introsorts; integer lists use LSD
  radix sort over sign-biased keys (2x over the previous qsort path).
- Benchmarks now include Rust; no benchmark where Rust is meaningfully
  faster, and integer dict ops beat Rust's default hasher by 2x.

### Known limitations

See the README list. The headline ones: no closures, no user generics,
no threads; reference cycles and raise paths leak; no TLS; no
variadic externs; no MSVC.
