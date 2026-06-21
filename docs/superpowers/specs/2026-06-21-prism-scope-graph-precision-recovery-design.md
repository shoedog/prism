# Scope-graph precision recovery — Rust-scoped completeness + disproof-pruning seam

**Status:** design-of-record (approved 2026-06-21), PLAN-READY
**Area:** `src/repo_loader.rs`, `src/resolution.rs`, new `src/resolution_disproof.rs`, `src/cpg_cache.rs`
**Predecessor:** [`2026-06-20-prism-owner-collision-demote-design.md`](2026-06-20-prism-owner-collision-demote-design.md) (#120, shipped the same-name-collision demote-not-drop; this is its §14 recovery path)
**Companion analysis:** [`docs/owner-key-identity-analysis-2026-06-20.md`](../../owner-key-identity-analysis-2026-06-20.md)

## 1. Goal & premises

#120 demotes same-name owner-key collisions to NameOnly (recall-safe), but does not
*correct* them — on ruff, ~17,552 `qualified_owner` edges sit at NameOnly because the
scope graph that could disambiguate them is never built (ruff fails the whole-repo
completeness gate on irrelevant Python files). This change **recovers those toward
single Exact** by (1) building the Rust scope graph on such repos and (2) routing
owner-keyed resolution through a **sound candidate-elimination (disproof) primitive**
that resolves to one Exact where it can, prunes the demote pool where it partially
can, and falls open to #120's full demote where it cannot.

Two standing premises govern every decision:

- **P1 — recall-safety.** Never drop a real edge. A candidate is eliminated only when
  *proven* not the target; uncertainty keeps it (NameOnly). A wrong drop is worse
  than a wrong demote.
- **P2 — recovery is upstream (§14).** #120's demote stays the *terminal* floor. This
  change adds disproof strictly *before* the demote; it never un-demotes in place and
  never mutates cached/receiver/index state in a way that blocks a later Exact.

Success: ruff `kind_nameonly[qualified_owner]` drops materially (recovers to
`kind_exact` or smaller pruned NameOnly sets), the `shadow_typepath_narrow` counter
flips from `failopen_no_graph` → `singleton`, **with zero recall regression**.

## 2. Component 1 — Rust-scoped completeness gate (the lever)

### Problem
`scope_graph_build_inputs` (`repo_loader.rs:170`) sets `complete =
has_complete_file_coverage(root, files)`, where coverage compares the parsed `files`
against **every** supported source file under root (`has_complete_file_coverage`,
`:215`). The loader drops files for `SkipReason::{NotUtf8 (repo_loader.rs:511),
Unreadable, Unsupported, TooLarge}`, but the "expected" set
(`collect_supported_source_paths`) only filters by path + 2MB cap — so a non-UTF-8 or
otherwise-skipped *supported* file makes `actual != expected` → `complete=false`.
Measured on ruff: **2,951 `.py` vs 1,874 `.rs`; 10+ non-UTF-8 `.py` (lint fixtures);
0 non-UTF-8 `.rs`; 0 oversized files.** ruff fails completeness purely on Python.

### Change
The Rust scope graph is **Rust-only**: `populate_scope_graph` → `populate_rust`, and
`populate_method_identity_indices` skips non-Rust (`if !matches!(parsed.language,
Rust) continue`). Both gate on `complete`. So `complete` becomes **Rust-file
coverage**: every `.rs` file under root that `collect_supported_source_paths` would
list is present in `files`. Non-Rust skips no longer block it.

Implementation: add `has_complete_rust_coverage(root, files)` (mirror
`has_complete_file_coverage` but restrict both expected and actual to
`Language::from_path(p) == Some(Rust)`), and use it for the scope-graph `complete`
flag in `scope_graph_build_inputs`. `has_complete_file_coverage` has exactly one
caller (`repo_loader.rs:170`, the scope-graph path), so it is **replaced** by the
Rust-scoped helper (no other consumer depends on all-language coverage).

### Soundness
Only `.rs` files define in-repo Rust types/modules, so Rust-file completeness is the
correct authoritativeness condition for in-repo Rust resolution. External-crate types
are already resolved as `External` (not in-repo); macro- and `build.rs`-generated
code is invisible to tree-sitter regardless of coverage (a pre-existing,
completeness-independent limitation). A repo that skips its *own* `.rs` (oversized or
non-UTF-8 Rust — 0 in ruff) still yields `complete=false` and no scope graph; that is
the deferred per-crate case (§7).

*Scoped to Rust because Rust is the only active scope resolver
(`rust_scope_graph_resolution` is Rust-gated, `resolution.rs:680`). When other-language
resolution lands, the coverage check extends per-language; the helper is named and
shaped to make that a one-line generalization.*

## 3. Component 2 — the disproof seam (the reusable primitive)

New module `src/resolution_disproof.rs`. A **disproof predicate proves a candidate is
not the callee at a site**; it must be sound — return "not disproved" whenever
uncertain. The seam composes predicates by **intersection**: a candidate survives
unless *some* predicate disproves it.

```rust
/// Context a predicate may consult (scope graph, method index, etc.). Read-only.
pub struct DisproofCx<'a> { /* &CallGraph, &ScopeGraph, resolved scope, … */ }

/// SOUND iff `disproves` returns true ONLY when it can prove `cand` is not the
/// target at `site`. Recall-safe by construction: adding predicates can only shrink
/// the surviving set, never wrongly drop the true target.
pub trait DisproofPredicate {
    fn disproves(&self, cand: &FunctionId, site: &CallSite, cx: &DisproofCx<'_>) -> bool;
}

/// Prune `pool` to candidates no predicate disproves. If pruning would empty a
/// non-empty pool, return the ORIGINAL pool — a disproof that eliminates everything
/// is treated as no-confidence (P1), never a drop.
pub fn prune<'a>(
    pool: Vec<&'a FunctionId>,
    site: &CallSite,
    cx: &DisproofCx<'_>,
    preds: &[&dyn DisproofPredicate],
) -> Vec<&'a FunctionId>;
```

### This slice ships exactly one predicate — `ScopeResolution`
Given the call site and the (authoritative) scope graph, it resolves the call's owner
type-path through the scope graph (reusing the existing
`graph_callable_edge`/`rust_graph_qualified_callable_edge` + the binding→`Target`→`ids`
match in `graph_target_resolution`, `resolution.rs:559`). If the path resolves to a
concrete in-repo defining scope, yielding a matched id-set `ids`:
- it **disproves** any candidate **not in `ids`** (Rust name resolution is
  deterministic — at that site `CliTest` *is* `ruff::CliTest`, so `ty::CliTest` is
  provably not the target).

If the path does **not** resolve to a single concrete in-repo scope — including a
**glob/ambiguous/re-export resolution** — it disproves **nothing** (keep all). Glob
and re-export resolutions are treated as "not a pin" (P1); a safe refinement to prune
under specific glob/re-export shapes is a future predicate, not this slice.

Under P1's completeness premise (§2), eliminating non-`ids` candidates is sound.

The seam is the extensibility deliverable: future precision-recovery becomes new
predicates (reachability, arity, receiver-type, trait-bound — §7) composed into the
same intersection, each small, sound, and independently tested. It also gives a home
to later consolidate today's ad-hoc per-candidate filters (`combine_kind` arity/self
at `resolution.rs:503`; `owner_lookup_in_modules` module-segment narrowing at `:644`)
— **not done in this slice** (YAGNI).

## 4. Component 3 — integration: prune → Exact / pruned-demote, fail-open

At the Rust scope path in `resolve_call_site_full` (`resolution.rs:679-690`), when the
graph is present and `rust_authoritative_scope` yields a scope: fetch the bare-name
pool for the call's `(owner, method)` (the `methods[(owner, method)]` set), run
`prune` with the `ScopeResolution` predicate, then decide:

| pruned pool | result | confidence |
|---|---|---|
| **1** | `exact(pool, QualifiedOwner)` — **recovery** | Exact (1.0) |
| **>1** (type resolved, but it owns e.g. inherent + trait `with_file`, or cfg variants) | `demoted(pool, QualifiedOwner)` — **pruned NameOnly (2, not the bare 5)** | NameOnly |
| **unchanged from bare** (path did not resolve) | **fall through** to the existing `::`-split → `owner_lookup_in_modules` → #120 full-pool demote | NameOnly |

Two behavior changes vs today:
1. **`graph_target_resolution` returns its `ids` set**: `1 → exact(ids, …)` (today),
   **`>1 → demoted(ids, QualifiedOwner)`** (today returns `None` at `:599`). The kind
   for the `::`-qualified case stays `QualifiedOwner`; the unqualified `LocalDef`/
   `FreeSingle` arms (`:602-607`) keep returning only on a singleton (a `>1`
   unqualified free-fn set is out of scope here — it routes through the existing
   free-fn rungs).
2. **Fail-open at `resolution.rs:687`**: when `rust_scope_graph_resolution` returns
   `None`, **fall through to the bare path** instead of `dropped(UnknownName)`. This
   is what makes enabling the scope graph recall-safe — the pre-gate `modfx` drop
   (`unresolved_unknown_name=1`) becomes a NameOnly demote. The non-scope-graph path
   (most repos) is untouched; #120's full demote remains the floor.

The demote arm reuses #120's `demoted(...)` (confidence-only; kind preserved so any
downstream relabel rides through — though the scope path's `QualifiedOwner` is
terminal here).

## 5. File structure

- `src/repo_loader.rs` — `has_complete_rust_coverage` + wire it into
  `scope_graph_build_inputs` (Component 1).
- `src/resolution_disproof.rs` (**new**) — `DisproofPredicate`, `DisproofCx`, `prune`,
  `ScopeResolution` (Component 2). One focused unit; no other responsibility.
- `src/resolution.rs` — fetch bare pool + `prune` at the scope path;
  `graph_target_resolution` `1→Exact / >1→demoted`; fail-open at `:687` (Component 3).
- `src/cpg_cache.rs` — `CACHE_VERSION` bump (§6).
- Tests: `tests/integration/resolution_test.rs` (seam, fail-open, recovery),
  a coverage test for Component 1, the Tier-A acceptance.

## 6. Cache

**`CACHE_VERSION` bump required.** `ScopeGraph` is serialized in the CPG cache
(`cpg_cache.rs:52`, v12 "Rust ScopeGraph stored in CallGraph"; `ScopeGraph` derives
`Serialize`/`Deserialize`). Building it for repos that previously had `None` changes
the cached bytes, and the resolution behavior changes, so bump `CACHE_VERSION`
(currently 15, `cpg_cache.rs:57`) — unlike #120 (which was confidence-only, no bump).

## 7. Testing & acceptance

### Unit / integration (`tests/integration/resolution_test.rs`, TDD)
1. **C1 coverage:** a fixture root with an unparseable/non-UTF-8 `.py` plus clean
   `.rs` → scope graph builds (`complete=true`, `methods_by_scope` populated); a
   fixture skipping a `.rs` (simulated) → `complete=false` (unchanged).
2. **Seam — resolves to 1:** two crates each defining `CliTest::with_file`, a call
   `CliTest::with_file()` in one crate with the scope graph present → **1 candidate,
   Exact** (the headline recovery; the ruff case in miniature).
3. **Seam — pruned to >1:** the resolved type owns both an inherent and a trait
   `with_file` (plus a same-named type in another module) → **2 candidates, NameOnly**
   (the other module's type disproved; the inherent/trait pair kept).
4. **Seam — no resolution → keep all:** a call whose type path the scope graph can't
   resolve → **full bare pool, NameOnly** (not dropped).
5. **Seam — glob/re-export → keep all:** a glob-imported same-name type → predicate
   keeps all (NameOnly), does not pin.
6. **Fail-open:** a `::` call the authoritative scope path can't resolve → NameOnly
   demote, **not** `UnknownName` drop.

### Tier-A acceptance gate
- `cargo build --release`; `cd eval && uv run tier-a --matrix-only --allow-stale-sut`
  then `--quick` (clear the nav cache first — §6/cache).
- **Recovery signal** (`prism nav --no-cache call-stats`, ruff/prism/ripgrep): ruff
  `kind_nameonly[qualified_owner]` **drops** with a matching rise in
  `kind_exact[qualified_owner]` (and/or smaller pruned NameOnly sets);
  `shadow_typepath_narrow` flips `failopen_no_graph` → `singleton` (the forward
  instrument #120 left). Report a per-anchor recovered-Exact count.
- **Recall-safety:** matrix **0 regression**; `--quick` M2 **fn = 0** (no dropped
  edges), fp not increased. If recall drops on any anchor → STOP (a prune dropped a
  real edge; tighten the predicate's soundness). Paste before/after into the PR.
- Independent codex (gpt-5.5, xhigh) spec/plan/diff reviews per the established loop.

## 8. Risks

- **Pruning soundness (the dominant risk).** If the scope graph mis-resolves a path
  (re-exports, glob imports, macro-generated `use`), the `ScopeResolution` predicate
  could disprove the true candidate → a dropped real edge (P1 violation). Mitigations:
  the completeness premise (§2), "uncertain → keep" (§3), fail-open (§4), and the
  Tier-A recall gate. **Glob/re-export is the explicit watch-area** — the predicate
  treats them as "not a pin" (keep all) until a safe refinement is designed.
- **Perf.** Building the scope graph for ruff (1,874 `.rs`) adds build time; expected
  acceptable post-S1, measured in the gate. A regression beyond a small budget is a
  blocker for the (A) default-on behavior.
- **Cache churn.** The bump invalidates all caches once; acceptable.

## 9. Out of scope / future slices

Each plugs into the §3 seam or extends §2 — none in this slice:
- **Reachability/visibility predicate** (Approach 2): crate-dep-graph + import-edge
  pruning when the path doesn't fully resolve (uses `manifest_hashes`).
- **(B) per-crate authoritativeness**: a scope is authoritative iff its crate's `.rs`
  are present; handles partial Rust coverage (skipped own `.rs`).
- **arity / receiver-type / trait-bound predicates**; consolidating the ad-hoc
  filters (§3) into the seam.
- **Other-language scope resolution** (generalize §2 coverage per-language).
- **Safe glob/re-export pruning** (refine the `ScopeResolution` predicate).
