# SDD ledger — plan: /root/.opencode/plan/leo-language-plan.md

Executor: inline (executing-plans skill). Repo: /root/leo.

Pre-flight: the pipeline is strictly sequential (lexer -> parser -> sema ->
codegen -> CLI). Each milestone consumes the previous phase's public API as
it is defined here; no cross-task interface conflicts found. Builder-review
forks are pre-settled in plan section 13.

Task M1: complete (tests 23 -> lexer, fixtures, CLI skeleton)
Task M2: complete (parser full grammar, extern fn items, formatter fixpoint property, comment preservation; tests 41)
  Ruling: let gains an optional type annotation (`let xs: list[int] = []`)
  because empty collection literals are otherwise untypable without
  cross-function inference. Plan grammar updated in SPEC later. Cost if
  wrong: one grammar production.
  Ruling: trailing comments reflow to their own statement line; comments
  never dedent. Documented in leo fmt help later.
  Ruling: extern `str` params map to leo_str* (runtime native); C char*
  interop goes through ptr + cast + s.to_cstr() bridge. SPEC documents.

Task M3: complete (sema: inference, method sugar, instantiation records; 66 tests)

  Ruling (Panda, mid-build): "C-like" means C's reach (kernels, OS,
  desktop, mobile), not C syntax. Changes: (1) freestanding build mode,
  --freestanding --target triple, value-types only, -nostdlib, no
  runtime; (2) asm("...") statement in v0.1, __asm__ volatile passthrough;
  (3) cross-compile target passthrough to cc. The transpile-to-C
  architecture already enables all three. Cost if wrong: asm passthrough
  is raw by design; freestanding has no heap types (documented, like C).

Task M4: complete. leo run/build end-to-end verified: structs+methods,
recursion, list/dict/tuple stamps, f-strings, try/except (setjmp), release
and freestanding (-nostdlib, no runtime, value types only), asm passthrough.
  Ruling: function-level cleanup via a single __exit label + flat locals
  registry + zero-init frame, instead of per-block cleanup. Per-block
  cleanup double-frees on merged control flow after conditional returns.
  Cost: locals live until function exit; acceptable, documented.
  Ruling: raise leaks the raising frame's RC locals (longjmp skips cleanup).
  Documented limitation; the v0.2 design is a frame-unwinder walk.

Task M5: complete (math, str, io, os, fs, time in Leo over externs; embedded
in the binary; conformance harness with expected outputs).
  Ruling: self-recursive functions must declare their return type
  (fn fib(n: int) -> int). Undeclared recursion has no inferable type;
  the compile error teaches the rule. Non-recursive functions infer fully.
  Ruling: externs keep mangled registry keys but call and prototype by their
  raw C symbol; prototypes are not emitted at all since the header declares
  them.

Task M8: complete (json pure Leo, net TCP/HTTP, examples, benchmarks
measured: fib 0.009s vs py 0.635s vs C 0.010s; float()/int() builtins)
Task M9: complete (README, LEARN 18/18 blocks verified, SPEC, STDLIB,
CHANGELOG, LICENSE, install.sh, CI, logo, docs page: overflow clean at
390/768/1440, monochrome + one accent)
Fresh checkout verified: clone -> build 18.6s -> 67 tests green ->
learn blocks green -> benchmarks run -> cargo install works.
Final review: dispatched to fresh-context reviewer.
