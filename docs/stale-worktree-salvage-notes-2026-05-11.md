# Stale Worktree Salvage Notes - 2026-05-11

These notes preserve the useful ideas from two stale `.claude/worktrees`
checkouts before pruning them. The worktrees were about six weeks old and were
not merge-ready against current `main`.

## `interesting-wilbur`

Source state:

- Branch head was already merged.
- Remaining local edits touched `src/algorithms/absence_slice.rs`, `src/ast.rs`,
  and `src/languages/mod.rs`.
- Diff size was 161 insertions and 42 deletions.

Potential salvage:

- Replace substring matching in AbsenceSlice paired-operation detection with
  AST call-expression matching where possible.
- Normalize paired-operation patterns such as `malloc(`, `.lock(`, and
  `.close()` down to callee names, then match exact callees or method suffixes.
- Avoid partial-name false positives such as matching `malloc_wrapper` as
  `malloc`.
- Add a scope-awareness concept for variable-reference collection so nested
  functions, lambdas, closures, or methods that redefine the same variable name
  do not emit references for the outer binding.

Current-state caveat:

- Current `main` already has newer scoped-reference helpers such as
  `find_variable_references_scoped`, plus newer call extraction APIs. Any future
  implementation should be rebuilt against those current APIs instead of
  replaying the stale patch.

Suggested future task:

- Add focused tests for AbsenceSlice false positives, including call names like
  `malloc_wrapper`, method calls like `mutex.lock()`, and paired calls inside
  nested functions that should not satisfy an outer function's cleanup contract.

## `tender-lichterman`

Source state:

- Branch head was already merged.
- Remaining local work was an untracked C fixture bundle:
  `tests/c_fixtures_test.rs` plus eight fixture directories under
  `tests/fixtures/c/`.
- The old test file used a monolithic integration-test style that does not match
  the current organized `tests/lang/c/` and `tests/algo/` layout.

Potential salvage:

- `heartbleed`: unchecked heartbeat payload length flowing into allocation and
  copy operations.
- `buffer_overflow`: DNS-style label length flowing into `memcpy`.
- `command_injection`: `getenv()` data reaching `system()`, with provenance as
  environment input.
- `use_after_free`: pointer allocated, freed on an error path, then reused.
- `missing_unlock`: lock acquisition without the matching release on relevant
  paths.
- `dma_leak`: DMA allocation or buffer ownership requiring release coverage.
- `isr_race`: interrupt handler or async callback race around shared state.
- `config_rollback`: configuration update paths that should retain rollback or
  full-flow context.

Current-state caveat:

- Current `main` already has a much richer C fixture suite and newer test
  organization. These scenarios should be ported selectively only if they add
  coverage not already represented by the current `tests/lang/c/` fixtures.

Suggested future task:

- Compare the eight fixture themes against current `tests/lang/c/cve_fixture_test.rs`
  and add only missing high-value scenarios in the current test layout.
