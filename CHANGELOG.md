# Changelog

## [1.6.0] - 2026-10-21

LLM engine: 6.35 to 18.5 tok/s (3x) on Qwen2.5-0.5B, single-thread
3.6 to 13.0 tok/s, and three correctness bugs fixed.

### Fixed

- The BPE tokenizer emitted special-token ids before the text pieces
  instead of interleaving them in prompt order, and skipped one byte
  past each special token (<|im_start|> is 12 chars, <|im_end|> 10 -
  the code advanced 13 for both). Both scrambled the chat template;
  long generations degenerated into repeated template fragments.
- Non-ASCII string literals were emitted with a codepoint-count
  length instead of a byte length ("Ġ" is 2 bytes in UTF-8 but got
  len 1), which corrupted every vocab lookup containing mapped
  byte characters and dropped trailing bytes in concatenated
  words. Spaces and newlines tokenized as "!" (id 0).
- The sematic analyzer rejected a conditional return nested inside
  an if branch ("not every path returns"), a false positive: only
  function bodies must guarantee a return, branch blocks may
  return conditionally.

### Added

- The Qwen2 pretokenizer at byte level: contractions, the
  optional-single-character letter rule, one digit per piece,
  space-attached punctuation runs, newline-run and trailing
  whitespace rules. Token ids now match llama.cpp exactly on the
  whole test battery (13/13 prompts, token for token).
- The Q6_K and Q4_K AVX2 row-dot kernels (llama.cpp technique:
  unsigned-nibble maddubs, scale shuffling, sub-block minimum
  correction through per-16 activation byte sums). The scalar paths
  ran at 178-470 M elem/s; the AVX2 kernels run at 7.5-8.0 G.
- One C call per transformer layer (fang_llm_layer): rms, fused
  qkv matvec, rope, KV append, attention, output projection,
  second rms, fused gate/up matvec, SwiGLU, down projection.
- Hot internal barriers: inside a layer sequence the workers stay
  spinning between stages, so a whole token costs one condvar wake
  instead of one per matvec (measured 5.06 ms to 0.12 ms per
  dispatch).
- Work-threshold dispatch: outside a layer sequence, matvecs below
  ~7.5M elements run inline on the caller instead of paying a wake.

### Changed

- The worker pool defaults to nproc/2 - 1 workers. On this shared
  box the old cores-1 default oversubscribed the scheduler: with
  load 8-15 from other tenants, eight runnable threads turned every
  barrier into a multi-millisecond wait for a preempted worker.
  Four total threads (llama.cpp picks the same) measured fastest.
- Attention score and value-accumulation kernels vectorized with
  AVX2 double FMA; RoPE builds its angle table once per call;
  quantization buffers are thread-local and grown on demand; row
  dots prefetch the next weight row.
- Verified: output byte-identical to llama.cpp (greedy, same
  prompt), 6/6 identical long generations under load, 26 cargo
  tests, clippy clean, 27/27 doc examples, 19/19 course checks.


## [1.5.1] - 2026-10-20

LLM engine performance: 1.06 to 6.35 tok/s (6x) on Qwen2.5-0.5B.

- Fused inference kernels in the runtime: the whole matvec
  (activation quantization with per-32-block scales, every row dot,
  optional bias) is one C call per weight matrix, multithreaded
  across cores through a persistent worker pool.
- Full AVX2/FMA integer path for Q5_0 (shuffle-based nibble
  expansion, maddubs pairing - the llama.cpp technique), Q8_0
  (widen + madd_epi16), Q6_K / Q4_K / F32.
- Attention, RoPE, RMSNorm, SwiGLU, argmax, KV append and residual
  add all run as single C kernel calls; the Fang layer only
  orchestrates per layer.
- Per-token allocations eliminated: all activation buffers
  preallocated once and reused; norm and bias tensors preloaded.
- Persistent thread pool with an epoch-based completion barrier
  (no counter resets; a lost-increment window in the reset-based
  counter caused rare wrong outputs under heavy load).
- Runtime cache now compiles with -march=native (it was silently
  dropping to generic code: 5.4x kernel throughput loss).

## [1.5.0] - 2026-10-20

Fang LLM: a GGUF inference engine, in Fang.

### Added

- `examples/llm.fg`: a complete GGUF inference engine for
  qwen2-architecture models (Qwen2.5-0.5B verified). GGUF parsing,
  GPT-2 BPE tokenizer with special-token handling, the full
  transformer forward pass with GQA and KV cache, greedy and
  temperature sampling, chat template. Generates correct text,
  verified token-for-token against llama.cpp on the same model.
- Runtime integer-dot kernels: activation vectors quantize to Q8_0
  once per matvec (per 32-block scales, like llama.cpp), then
  integer dot against Q5_0 / Q8_0 / Q4_K / Q6_K / F32 weights with
  AVX2 madd. ~6x faster than the float-dequant path. Every kernel
  verified element-exact against pure-Fang dequant references.
- Compiler fixes found by the engine build: for-loop binders no
  longer double-release their last value at function exit; string
  literals inside index expressions now join the pool (previously
  an undeclared-identifier C compile error).

## [1.4.2] - 2026-10-20

The learning website: https://loak7993-code.github.io/fang

### Added

- `website/`: the Fang learning site (React + Vite, dark, Geist,
  Phosphor). Home, full 18-lesson course browser, benchmark charts
  from the real suite numbers, cheat sheet, install steps. Deployed
  to GitHub Pages on every push by CI.
- Course gap fixed: lesson 8 ("Talking to files") was missing since
  the numbering jumped 7 to 9. The course is a real 18 lessons now.

## [1.4.1] - 2026-10-20

The file extension is now `.fg` (was `.fang`). Every command,
example, test, and doc uses the new extension.

## [1.4.0] - 2026-10-20

Incremental compilation: modules compile once, programs link them.

### Added

- Per-module compilation units with content-hash object caching.
  A module's generated C is identical across every program that
  imports it, so its object file is compiled once and reused. Only
  the entry module compiles on every build. Stale modules compile
  in parallel.
  - website (imports server, json, str, ...): 810 ms -> 91 ms
  - fib (no imports): 103 ms
  - cold start: 61 ms (python3 on the same script: 35 ms)
  - first build of a module set pays one compile, then cached
- Module headers: struct definitions, function prototypes, and
  extern globals are emitted as real C headers per module.
- Deterministic codegen: module C is byte-identical across compiler
  runs (sorted emission everywhere), which is what makes the object
  cache sound.
- REPL no longer double-wraps `print(...)` inputs.

### Notes

- The single-TU path remains for `--freestanding` and `--python`.
- Entry modules pay their own -O3 compile (chip8's 450 lines: ~390
  ms); that is the user's code, not the stdlib.

## [1.3.2] - 2026-10-20

Compiles at rustc parity.

### Fixed

- The C runtime (`fang_rt.c`, ~750 ms at -O3 -march=native) was
  recompiled from source on every build. It is now compiled once per
  (runtime source, compiler, opt level) into a cached object file.
  `fang build --release`: 822 ms -> 104 ms, first build only pays the
  runtime compile. rustc -O on the same benchmark: 108 ms.
- `fang run` cold start: 189 ms -> 74 ms.
- Collection stamping leaked dead-module types through the
  for-loop-binder and catch-type maps: `print("hi")` instantiated
  json's struct, list, and dict templates. Now zero stamps for a
  hello, and its generated C compiles in 35 ms at -O3 -march=native.
- Phase timing behind `FANG_TIMING=1` (parse / sema / cc).

## [1.3.0] - 2026-10-20

From scratch, in the language: TUI apps, emulators, and Python-style
module state.

### Added

- `tui` module: terminal UI in pure Fang. ANSI styling, cursor
  control, a screen buffer with box/fill/text drawing, and raw-mode
  key input (arrows, page keys, escape disambiguation). No
  dependencies beyond the terminal itself.
- `examples/chip8.fg`: a complete CHIP-8 emulator built on tui.
  All 35 opcodes, the original HEX keypad, 64x32 display, sound and
  delay timers, `--frames N` headless mode for deterministic tests.
- Module-level `let` creates a mutable global, exactly like Python:
  every function in the file sees it and can change it. Initialized
  before main in dependency-free order; the old "must be a constant
  expression" error is gone.
- Bitwise operators `& | ^ << >>` with C precedence, plus `0xFF` hex
  and `0b1010` binary integer literals.
- `fang run file.fg -- args` forwards everything after `--` to the
  program as `os.args()`.
- Structs can hold list and dict fields; collection types are
  forward-declared before struct definitions (was a documented
  limitation).
- `str.byte_at(s, i)` for byte-level reads; `math.rand_int(n)`;
  `tui`-facing runtime terminal primitives (`fang_term_raw`,
  `fang_term_restore`, `fang_read_key_ms`).
- Lexer supports `\xHH` and `\uHHHH` escapes and `\e`.

### Fixed

- The guard pattern `if bad: return` followed by more statements
  no longer raises a false "not every path returns" error in void
  functions.
- Formatting preserves `: type` annotations on top-level let.

## [1.2.0] - 2026-10-20

Build anything, from backend websites to kernels.

### Added

- `server` module: an HTTP server in pure Fang. `server.serve(handler,
  port)` runs an accept loop; handlers are plain function values typed
  `fn(Request) -> Response` with fields `method`, `path`, `body`.
  Builders: `server.html/json/text/respond/not_found`. GET and POST
  (Content-Length bodies) verified end to end with curl.
- Function types as annotations: `fn(A, B) -> C`. Function values are
  checked structurally against them, so higher-order APIs like `serve`
  are typed like any other parameter.
- Runtime: `fang_tcp_listen` and `fang_tcp_accept` server-side TCP
  primitives; `fang_tcp_recv_http` reads exactly one request (headers
  plus Content-Length body) without waiting for the peer to close.
- Structs from imported modules resolve in annotations and
  constructors: `import server` puts `Request` and `Response` in
  scope by name.
- `examples/website.fg`: a complete website with HTML, JSON, and
  health routes.

## [1.1.0] - 2026-10-20

Type less words.

### Added

- Implicit returns: the last expression of a function body is its
  return value. `fn double(n): n * 2` — no `return` keyword. Lambdas
  too: `xs.map(fn(x): x * 2)`. Explicit `return` still exits early
  and mixes freely.
- Single-line function bodies accept a bare expression:
  `fn(x): x * 2` inside call parentheses.
- The interactive course teaches script style from lesson one: every
  program is plain top-level lines, functions appear in lesson 7
  without types or return, and no lesson uses `fn main():`.
- `fang new` scaffolds a one-line script starter.

## [1.0.0] - 2026-10-20

## [1.0.0] - 2026-10-20

Renamed to Fang (previously Leo, briefly Volt). The whole repository:
binary, file extension (.fg), runtime symbols (fang_*), crate names
(fang-core, fang-cli), docs, course, cheat sheet, and examples.
Everything else unchanged: 68 tests, 27/27 doc examples, 18/18
interactive course checks, clippy clean.

All notable changes to Fang are documented here. The format follows
[keep a changelog](https://keepachangelog.com/); versions follow
semantic versioning.

## [0.3.2] - 2026-10-20

The entire language, learnable in one sitting.

### Added

- `fang learn` now covers the whole language: 18 interactive lessons
  from `print` through dictionaries, structs, string methods,
  comprehensions, functions as values, raise/try/except, imports,
  extern C, and closures, ending in a guessing game that uses
  everything. Every answer compiles and runs; the course is driven
  end to end in CI (18 checks plus a wrong-answer rejection).
- `fang cheat`: the whole language on one screen, with examples.
- `fang doc <topic>`: instant help for any keyword, builtin, or
  command, with runnable examples.

### Fixed (found while building the course)

- `d["key"]` never compiled: dict indexing demanded int keys in
  both read and assignment positions. Now dict indexing accepts the
  dict's own key type.
- String literals in dict index assignments (`d["k"] = v`) were
  never pooled, producing a reference to a nonexistent pool entry.

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

- `fang learn`: an eight-lesson interactive terminal course for people
  with zero programming experience. Each lesson explains a concept in
  plain language, has the learner fill in a blank in a real program,
  compiles and runs it, and checks the output. Wrong answers show the
  actual output or compile error plus hints. Progress saves between
  sessions; `fang learn list` and `fang learn reset` manage it.
- `fang new <name>`: scaffold a starter project.
- `tools/check_learn_cmd.sh`: drives the course end to end in CI,
  including rejecting wrong answers.

## [0.2.1] - 2026-10-20

### Added

- Python bridge: `fang run app.fg --python` embeds CPython (via
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
- Tooling: `fang run`, `fang build` (debug, `--release`, `--freestanding`
  with `--target` passthrough), `fang repl`, `fang fmt` (canonical style,
  round-trip tested), `fang test` (file-based), `fang add` (git packages
  with `fang.lock` pinning).
- Standard library: `io`, `str`, `list` and `dict` methods, `math`,
  `os`, `fs`, `time`, `json` (pure Fang), `net` (TCP and HTTP/1.0).
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
