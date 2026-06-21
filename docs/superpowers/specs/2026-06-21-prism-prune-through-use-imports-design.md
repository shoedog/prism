# Prune through `use`/re-export imports — design (scope-graph recovery slice A)

**Status:** design-of-record (approved 2026-06-21), PLAN-READY
**Area:** `src/resolution.rs` (`leading_segment_binds_directly`)
**Predecessor:** the merged scope-graph precision-recovery feature (PR #121) — this is its deferred **(A)** follow-up (the `②B` "direct-binding only" relaxation).
**Branch:** `prune-through-use-imports` (off `main`, which has #121).

## 1. Problem

PR #121 shipped the `ScopeResolution` disproof predicate, which recovers a same-name
owner-key collision (`CliTest::with_file()` where two distinct `CliTest` types exist)
to a single Exact **only when the leading type binds directly** — decision ②B,
`leading_segment_binds_directly` (`resolution.rs:1430`): the single visible `NS_TYPE`
binding must be a non-glob `BindTarget::Resolved(Target::Item)`. A **`Pending`
binding** — i.e. the type reached through a `use`/re-export (`use ty::CliTest;`) —
returns `false` → keep-all → no recovery.

Real code imports types via `use`, so on real corpora the prune-eligible collisions
sit at NameOnly. Measured (post-#121 `recovery_typepath.failopen_demote`, the
`use`-imported owner-`::` collisions): **ruff ~1,586, ripgrep 161, prism 15**. This
slice recovers those by pruning through a `use` chain that resolves unambiguously.

## 2. Goal / non-goals

**Goal:** when the leading type segment is reached through a **single `use`/re-export
chain that resolves to exactly one in-repo `Item`**, treat it as directly bound so the
prune proceeds — recovering the `use`-imported collisions (`failopen_demote` →
`singleton`). Recall-safe: never prune on an ambiguous/external/unresolvable import.

**Non-goals:**
- The heavy original (A) framing — populating serialized `Candidate.provenance` with
  via-re-export markers (engine change + `CACHE_VERSION` bump). **Not needed** (below).
- Pruning through **glob** imports (`use b::*`) — stays keep-all (the `Provenance`/
  glob slice is still deferred, §9).
- Touching ①C (block-local shadow keep-all) or any other predicate.

## 3. The change (one gate, no engine/cache change)

The disproof body (`ScopeResolution::disproves`, `resolution.rs:1327`) already does
the right thing for `Pending` in its **final step**: after the ②B gate it resolves the
full callable path via `rust_graph_qualified_callable_edge` + `graph_target_ids`
(`:1373-1380`), which **fold `use`-imports through the engine** — so for
`use ty::CliTest; CliTest::with_file()` the id-set is `{ty::CliTest::with_file}` and
the other `CliTest`'s method is correctly disproved. The **only** thing stopping the
`Pending` case from reaching that step is the ②B gate.

So the change is entirely in `leading_segment_binds_directly` (`resolution.rs:1384`),
at the final match (`:1430-1433`):

```rust
// BEFORE
match visible.as_slice() {
    [b] => matches!(&b.target, BindTarget::Resolved(Target::Item { .. })),
    _ => false,
}

// AFTER
match visible.as_slice() {
    [b] => match &b.target {
        // Directly bound in-repo type — unchanged (②B).
        BindTarget::Resolved(Target::Item { .. }) => true,
        // (A) Slice: a single `use`/re-export chain. Fold it via the engine; prune
        // only if it resolves UNAMBIGUOUSLY to one in-repo `Item` (Rust `use`
        // resolution is deterministic). Ambiguous / poisoned / unresolved / external
        // -> keep-all (the engine returns those statuses; we do NOT prune).
        BindTarget::Pending(..) => {
            engine_resolves_to_single_in_repo_item(graph, &q, &policy)
        }
        _ => false,
    },
    _ => false,
}
```

`engine_resolves_to_single_in_repo_item` is a small local helper: call
`crate::name_resolution::engine::resolve(graph, &q, &policy)` (`engine.rs:40`; the
`engine` module is already used by `resolution.rs`, which imports `resolve_path` — add
the `resolve` import) and return `true` iff the result is a successful
resolution to exactly one `Target::Item` whose defining scope is **in-repo** (maps to a
known `FileId` via the graph), `false` for `Ambiguous`/`Poisoned`/`Unresolved`/
`External`/multiple. No `engine` change, no `Provenance` change, **no `CACHE_VERSION`
bump** (nothing serialized changes — this is a resolution-time predicate decision).

## 4. Soundness / recall-safety

Prune-on-`Pending` fires **only** when the engine resolves the leading type to one
in-repo `Item`. Rust `use` resolution is deterministic, so `use ty::CliTest;
CliTest::with_file()` means `ty::CliTest` unambiguously → disproving the other
same-named `CliTest`'s method is sound, and the final-step callable resolution
(`:1373`) independently follows the **same** `use` to the same target (consistent).

Every uncertain shape keeps-all (returns `false` → no disproof → full pool retained at
NameOnly), so this change can only **add** recoveries, never drop a real edge:
- **Glob** import (`use b::*`) bringing the type → engine `Ambiguous`/`Poison` (or the
  binding isn't a single visible `Pending`) → keep-all.
- **Ambiguous** (two `use`s of the same name) → engine `Ambiguous` → keep-all.
- **Poisoned** (broken/macro re-export chain) → `Poison` → keep-all.
- **Unresolved** / **External** (resolves outside the repo) → keep-all (can't prune to
  a target we don't own).
- **②C/edition guard and ①C block-local-shadow keep-all** compose unchanged upstream
  (`:1337`, `:1353`).

## 5. Components

Single focused unit; no new files.
- `src/resolution.rs` — the `leading_segment_binds_directly` `Pending` arm + the
  `engine_resolves_to_single_in_repo_item` local helper. Sole production change.

## 6. Testing (TDD)

`tests/integration/resolution_test.rs` (use the `build_rust_complete` / `build`
convention helpers + `resolve_call_site_full` per the existing `scope_graph_*`
fixtures):
1. **Headline recovery (red→green):** `mod ty; mod ru;` each defining `struct CliTest`
   with `with_file`, a caller `use crate::ty::CliTest; CliTest::with_file();` →
   **resolves to a single Exact** (`ty`'s); currently keep-all (2 NameOnly).
2. **External `use` keeps-all:** `use some_external::CliTest; CliTest::with_file();`
   (the type resolves outside the repo) → keep-all (full pool, NameOnly), not dropped.
3. **Glob import keeps-all:** `use crate::ru::*; CliTest::with_file();` → keep-all.
4. **Ambiguous `use` keeps-all:** two `use`s bringing same-named `CliTest` → keep-all.
5. **Direct binding unchanged:** the existing ②B direct-binding recovery still Exact.
6. **Drop invariants + ①C/②B keep-all fixtures unchanged** (re-run
   `resolution_test::` + `tests/ast cpg_test::` — the trait-CHA decline must still hold).

## 7. Acceptance

- `cargo build --release`; `cd eval && uv run tier-a --matrix-only --allow-stale-sut`
  (cache auto-fresh — no `CACHE_VERSION` bump, so clear the nav cache or `--no-cache`
  the call-stats reads).
- **Recovery signal** (`prism nav --no-cache call-stats`, ruff/prism/ripgrep):
  `recovery_typepath.singleton` **rises** and `failopen_demote` **falls** (the
  `use`-imported collisions move from demote → singleton); `kind_exact[qualified_owner]`
  rises, `kind_nameonly[qualified_owner]` falls. Report the per-anchor delta.
- **Recall-safety:** matrix **0 regression**; a `--corpus ruff` M2 (ruff is pinned →
  valid, regression-classified) — **0 regression** (the same clean validator #121 used).
- Independent codex (gpt-5.5, xhigh) spec/plan/diff reviews per the established loop.

## 8. Risks

- **Pruning on a mis-resolved `use`.** Mitigated by requiring a *single in-repo `Item`*
  result and keeping-all on every other engine status. The final-step callable
  resolution follows the same `use`, so the two are consistent. The `--corpus ruff` M2
  recall gate guards it on the real corpus.
- **Perf.** One extra `engine::resolve` per prune-eligible owner-`::` collision site
  (a small set — ~1.6k on ruff out of 170k call sites); negligible.
- **Engine status taxonomy drift.** The helper must treat *anything other than* a
  single in-repo `Item` as keep-all (fail-closed); pin with the external/ambiguous/
  poison fixtures so a future engine change can't silently start pruning.

## 9. Out of scope / future

- **Glob / `Provenance` markers** (prune through `use b::*` and re-export facades that
  the engine can't fold to one item) — the heavier slice, still deferred.
- Other disproof-seam predicates (arity, receiver-type, per-crate authoritativeness).
