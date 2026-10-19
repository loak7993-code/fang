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
