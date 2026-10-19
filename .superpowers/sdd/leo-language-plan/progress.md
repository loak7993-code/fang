# SDD ledger — plan: /root/.opencode/plan/leo-language-plan.md

Executor: inline (executing-plans skill). Repo: /root/leo.

Pre-flight: the pipeline is strictly sequential (lexer -> parser -> sema ->
codegen -> CLI). Each milestone consumes the previous phase's public API as
it is defined here; no cross-task interface conflicts found. Builder-review
forks are pre-settled in plan section 13.
