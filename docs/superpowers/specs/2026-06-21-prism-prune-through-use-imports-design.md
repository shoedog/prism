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
sit at NameOnly. Measured (post-#121 `recovery_typepath.failopen_demote`): **ruff
~1,586, ripgrep 161, prism 15**. This slice recovers those by pruning through a
`use` chain that resolves unambiguously.

**These are UPPER bounds, not the realized buy.** `failopen_demote`
(`queries.rs:96`/`:152`) counts every owner-`::` `T::m` site where the predicate
*proved nothing* **and** the bare `(owner, method)` pool collides — which is
broader than "the leading segment is a single visible named in-repo `Pending`".
The realized recovery is the subset that additionally (a) has a single visible
`Pending` leading binding (not a glob edge, not 2+ ambiguous visible bindings) and
(b) resolves that import to one scope-bearing in-repo `Item`. Expect ~70–85% of
the upper bound, with the residue missed by: externals/unmodeled crates,
glob-in-chain poison, cfg `ResolvedSet`, type aliases / non-scope-bearing items
(`owns: None`), and final-step callable id-set misses. §7 measures the **realized
delta** (the `singleton` rise / `failopen_demote` fall), not this ceiling.

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
        // (A) Slice: a single `use`/re-export chain. Fold THIS binding's own
        // import path via the engine; prune only if it resolves UNAMBIGUOUSLY to
        // one scope-bearing in-repo `Item` (Rust `use` resolution is
        // deterministic). Ambiguous / poisoned / unresolved / `Target::External`
        // / a non-scope-bearing item / multiple -> keep-all (we do NOT prune).
        BindTarget::Pending(path, anchor) => pending_resolves_to_single_in_repo_item(
            graph, path, anchor, b.scope, b.ns, &q.at, &policy,
        ),
        _ => false,
    },
    _ => false,
}
```

`pending_resolves_to_single_in_repo_item` is a small local helper. **Chase the
exact visible `Pending` binding** (not a fresh bare `engine::resolve` from the
call scope — for a non-bare anchor like `crate::Foo::m` a bare lookup and the
binding's own anchored path can inspect different roots, so re-running the
binding's own path is what is sound and is what makes it consistent with the
final callable step). It reuses the EXISTING `resolve_path` import
(`resolution.rs:7` — no new `engine::resolve` import) exactly as the final
callable step at `:1554` does, but for the leading **type** segment:

```rust
// path/anchor come from `Pending(path, anchor)`; from = b.scope (the re-export
// author's scope); final-segment ns = b.ns (NS_TYPE for the type binding);
// anchor_ns = NS_TYPE for the scope-bearing prefix segments. resolve_path's
// signature (engine.rs:54) is:
//   resolve_path(graph, path, ns, anchor, from, anchor_ns, at, policy) -> Resolution
let res = resolve_path(graph, path, b_ns, anchor, b_scope, NS_TYPE, at, policy);
matches!(
    (res.status, res.candidates.as_slice()),
    (
        ResStatus::Resolved,
        [Candidate { target: Target::Item { owns: Some(scope), .. }, .. }],
    ) if graph_file_for_scope(graph, *scope).is_some()  // scope maps to an in-repo FileId
)
```

Return `true` iff the engine yields **`ResStatus::Resolved` with exactly one
candidate that is a `Target::Item { owns: Some(scope), .. }`** whose defining
`scope` maps to a known in-repo `FileId` (via the existing `graph_file_for_scope`,
`resolution.rs:1615`). Everything else is keep-all: `ResolvedSet` / `Ambiguous` /
`Poisoned` / `Unresolved`, a `Target::External` / `Target::Local` candidate, an
`owns: None` item (a type alias / non-scope-bearing item — which could not anchor
a `T::m` prune anyway), or >1 candidate. Note there is **no `External` engine
status**: externals surface as a `Target::External` *candidate target* under a
`Resolved` result (`types.rs`: `ResStatus` = `Resolved`/`ResolvedSet`/`Ambiguous`/
`Poisoned`/`Unresolved`; `Target` = `Scope`/`Item`/`Local`/`External`), so the
helper inspects the candidate **target**, not a status. No `engine` change, no
`Provenance` change, **no `CACHE_VERSION` bump** (nothing serialized changes —
this is a resolution-time predicate decision).

## 4. Soundness / recall-safety

Prune-on-`Pending` fires **only** when the engine resolves the leading type's own
import path to one scope-bearing in-repo `Item`. Rust `use` resolution is
deterministic, so `use ty::CliTest; CliTest::with_file()` means `ty::CliTest`
unambiguously → disproving the other same-named `CliTest`'s method is sound.

The helper resolves the **binding's own anchored path** (the same `resolve_path`
call shape the final-step callable resolution at `:1554` uses), so the two follow
the same `use` chain. The prune target is **not** load-bearing on the helper,
though: the actual disproof id-set always comes from the final step
(`:1373-1380`), and the gate's only job is to **decline** (keep-all) on any
non-single-in-repo import. (Re-running the binding's own anchored path — rather
than a bare `engine::resolve` from the call scope — is precisely what keeps the
gate consistent with that final step: for a non-bare anchor like `crate::Foo::m`
a bare lookup and the binding's anchored path could inspect different roots, so
the earlier "two resolutions are literally consistent" framing was too strong.)

Every uncertain shape keeps-all (returns `false` → no disproof → full pool retained at
NameOnly), so this change can only **add** recoveries, never drop a real edge:
- **Glob** import (`use b::*`) bringing the type → engine `Poisoned` (or the
  binding isn't a single visible `Pending`) → keep-all.
- **Ambiguous** (the binding's import path resolves to >1 candidate / two `use`s of
  the same name surfacing as 2 visible bindings, not `[b]`) → `Ambiguous` → keep-all.
- **Poisoned** (broken/macro re-export chain) → `Poisoned` → keep-all.
- **Unresolved** / **external** (the import resolves outside the repo: a
  `Target::External` candidate, or `Unresolved`/`Poisoned`) → keep-all (can't prune
  to a target we don't own). Note "external" is a candidate **target**, not a status.
- **②C/edition guard and ①C block-local-shadow keep-all** compose unchanged upstream
  (`:1337`, `:1353`).

## 5. Components

Single focused unit; no new files.
- `src/resolution.rs` — the `leading_segment_binds_directly` `Pending` arm + the
  `pending_resolves_to_single_in_repo_item` local helper (reuses the existing
  `resolve_path` import + `graph_file_for_scope`). Sole production change.

## 6. Testing (TDD)

`tests/integration/resolution_test.rs` (use the `build_rust_complete` / `build`
convention helpers + `resolve_call_site_full` per the existing `scope_graph_*`
fixtures). The existing `scope_graph_pending_import_alias_over_colliding_pool_keeps_all`
(`resolution_test.rs:1911` — `use crate::a::Foo; Foo::m()` over a colliding
`a::Foo`/`b::Foo` pool) is the headline red→green fixture: it currently asserts
keep-all (2 NameOnly) and **flips to a single Exact** under this change. Update it
in place (assert single Exact = `a::Foo::m`) rather than adding a duplicate.

Each fixture below must reach the `Pending` arm — i.e. the leading segment's
visible binding is a **single** `Pending` (`visible.as_slice()` is `[b]`); a
shape that surfaces 2 visible bindings is filtered out by the `_ => false` arm
*before* the helper and so does **not** pin the helper's branches:
1. **Headline recovery (red→green):** flip
   `scope_graph_pending_import_alias_over_colliding_pool_keeps_all` (above) to a
   single Exact. (Or, equivalently, a fresh `mod ty; mod ru;` each defining
   `struct CliTest`/`with_file`, caller `use crate::ty::CliTest; CliTest::with_file();`
   → single Exact = `ty`'s.)
2. **External alias keeps-all (`Target::External` branch):** a **single-segment**
   external alias — `use some_external::CliTest as CliTest;` (or a path whose crate
   root is not in-repo) → the `Pending` chain resolves to a `Target::External`
   candidate (or `Unresolved`/`Poisoned`) → helper returns `false` → keep-all (full
   pool, NameOnly), not dropped. (A multi-segment `use some_external::a::CliTest`
   tends to surface as `Unresolved`/`Poisoned`; use the one-segment form to actually
   exercise the `Target::External` candidate branch.)
3. **Glob import keeps-all:** `use crate::ru::*; CliTest::with_file();` → the glob
   is an `Edge` not a `Binding`, so there is no single visible `Pending` for the
   leading segment → keep-all (and a glob *in the chain* poisons, also keep-all).
4. **Ambiguous re-export keeps-all (`Ambiguous` branch):** a **single** visible
   `Pending` (`use crate::facade::CliTest;`) whose target module `facade`
   re-exports `CliTest` ambiguously (e.g. `pub use crate::ty::CliTest;` +
   `pub use crate::ru::CliTest;` in `facade`) → the binding's import path resolves
   `Ambiguous` → helper `false` → keep-all. (This pins the ambiguous branch *via*
   the `Pending` arm; the bare "two top-level `use`s of `CliTest`" shape instead
   yields 2 visible bindings → `_ => false` before the helper, so it does not pin
   this branch.)
5. **Direct binding unchanged:** the existing ②B direct-binding recovery
   (`scope_graph_two_crate_owner_collision_recovers_to_single_exact`,
   `resolution_test.rs:1692`) still Exact.
6. **Drop invariants + ①C/②B keep-all fixtures unchanged** (re-run
   `resolution_test::` + `tests/ast cpg_test::` — the trait-CHA decline must still
   hold; the ①C block-local `use`/glob/macro-shadow keep-all fixtures
   (`scope_graph_block_local_*`) are unaffected because ①C fires upstream of ②B).

## 7. Acceptance

- `cargo build --release`; `cd eval && uv run tier-a --matrix-only --allow-stale-sut`
  (cache auto-fresh — no `CACHE_VERSION` bump, so clear the nav cache or `--no-cache`
  the call-stats reads).
- **Recovery signal** (`prism nav --no-cache call-stats`, ruff/prism/ripgrep):
  `recovery_typepath.singleton` **rises** and `failopen_demote` **falls** (the
  `use`-imported collisions move from demote → singleton); `kind_exact[qualified_owner]`
  rises, `kind_nameonly[qualified_owner]` falls. Report the **realized per-anchor
  delta** (the measured `singleton` rise / `failopen_demote` fall) — NOT the §1
  upper bound; a realized delta below the ceiling is expected (per the §1 misses)
  and is not a regression.
- **Recall-safety:** matrix **0 regression**; a `--corpus ruff` M2 (ruff is pinned →
  valid, regression-classified) — **0 regression** (the same clean validator #121 used).
- Independent codex (gpt-5.5, xhigh) spec/plan/diff reviews per the established loop.

## 8. Risks

- **Pruning on a mis-resolved `use`.** Mitigated by requiring a *single
  scope-bearing in-repo `Item`* (`owns: Some(scope)` mapping to an in-repo
  `FileId`) and keeping-all on every other shape (`ResolvedSet`/`Ambiguous`/
  `Poisoned`/`Unresolved`, a `Target::External`/`Target::Local` candidate, `owns:
  None`, or >1). The gate resolves the binding's **own anchored path** via
  `resolve_path` (the same call shape as the final callable step), so it follows
  the same `use` chain; and the gate is decline-only — the actual disproof id-set
  comes from the final step (`:1373-1380`), never from the gate. The `--corpus
  ruff` M2 recall gate guards it on the real corpus.
- **Perf.** One extra `resolve_path` per prune-eligible owner-`::` collision site
  (a small set — ~1.6k on ruff out of 170k call sites); negligible.
- **Engine taxonomy drift.** The helper must treat *anything other than* a single
  scope-bearing in-repo `Item` candidate (under `ResStatus::Resolved`) as keep-all
  (fail-closed) — inspecting the candidate **target**, not a status (there is no
  `External` status); pin with the external/ambiguous/poison fixtures so a future
  engine change can't silently start pruning.

## 9. Out of scope / future

- **Glob / `Provenance` markers** (prune through `use b::*` and re-export facades that
  the engine can't fold to one item) — the heavier slice, still deferred.
- Other disproof-seam predicates (arity, receiver-type, per-crate authoritativeness).
