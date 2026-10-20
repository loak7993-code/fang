You are a senior compiler engineer reviewing a complete language implementation.

Review the Fang programming language repository at /root/fang: a small
Python-shaped language that compiles to C. Read the plan it was built from at
/root/.opencode/plan/fang-language-plan.md (the spec authority) and the
execution ledger at /root/fang/.superpowers/sdd/fang-language-plan/progress.md
(rulings are recorded decisions, not bugs).

Focus areas, in order:
1. Correctness of the memory model in crates/fang-core/src/codegen.rs and
   runtime/fang_rt.h: reference counting, the single __exit label cleanup,
   the goto-exit structure, use-after-free in assignments, the RC-on-cast
   convention. Look for paths that double-free, leak unboundedly, or read
   freed memory.
2. Parser and sema soundness: span-based side tables in
   crates/fang-core/src/sema.rs (keys are expr span starts) can collide or
   miss; check f-string span shifting in crates/fang-core/src/parser.rs.
3. Runtime C correctness in runtime/fang_rt.c: dict macro tombstones, list
   macros, str ops, the try/except longjmp machinery.
4. Anything shipped that is dead code, a placeholder, or a fake claim in
   README.md / LEARN.md / STDLIB.md versus what the code actually does.

Verify claims by reading code, not by trusting comments. For each finding:
severity (Critical/Important/Minor), file:line, one-paragraph explanation,
and the concrete user-visible consequence. If the finding is in an area the
ledger documents as a known limitation, say so and judge whether the
documentation matches the actual behavior. End with a one-line verdict.
