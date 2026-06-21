# Scope-graph precision recovery — Rust-scoped completeness + disproof-pruning seam

**Status:** CHANGES-FOLDED (re-review) — all 5 first-round codex findings addressed;
both design decisions resolved (①C, ②B) and the re-review's two **contract refinements**
folded (①C broadened to glob/macro shadows; ②B keyed on binding shape, not provenance).
Pending a final codex re-confirm before plan.
A codex (gpt-5.5 xhigh) spec review returned CHANGES-REQUIRED (2 BLOCKER + 3 MAJOR).
Findings 2/4/5 are folded above (edition-uniformity guard; qualified-scoped fail-open;
recovery-signal replacement). Findings **1** (bare-anchor lexical scope for `T::m`
pruning — §8.1) and **3** (re-export provenance for the predicate contract — §8.2) were
escalated to the owner; both are now **RESOLVED** — the owner chose **(C)** for §8.1
(block-local-shadow keep-all) and **(B)** for §8.2 (direct-binding-only prune) on
2026-06-21, with each rejected option's (A) filed as a deferred precision follow-up
(§9). Their contracts are folded into the `ScopeResolution` predicate (§3). A codex
**re-review** of ①C/②B then returned CHANGES-REQUIRED with two refinements (1 BLOCKER +
1 MAJOR) that make those two contracts recall-safe/implementable **without** reopening
the chosen approach — both verified against source and folded into §3/§8.1/§8.2/§7:
①C now keeps-all on **any** block-local shadow source (exact `NS_TYPE` binding, **glob
edge**, or **macro wildcard poison**), and ②B proves directness from the
**`BindTarget::Resolved`-vs-`Pending` binding shape**, not the (empty-provenance)
`Candidate`.
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

Success: ruff `kind_nameonly[qualified_owner]` drops materially with a matching rise
in `kind_exact[qualified_owner]` (recoveries) and/or smaller pruned NameOnly sets,
**with zero recall regression**. (The §7 acceptance reads this `kind_nameonly` →
`kind_exact` delta directly; the legacy `shadow_typepath_narrow` counter is NOT a
usable signal here — see review MAJOR 5 / §7.)

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

**Edition-uniformity constraint (review BLOCKER 2 — folded as a documented gate, not
a new resolver feature).** `ScopeGraph` carries a single repo-wide `edition`
(`name_resolution/graph.rs:80`), and `parse_rust_crate_config` populates it
last-write-wins while iterating manifests (`repo_loader.rs:298-305`); edition drives
`UsePath`/`LeadingColon` anchoring (`rust_policy.rs:275,286`). Today this is a
pre-existing precision limitation on the *already-shipped* `crate::`/`super::`/`self::`
graph edges; it is harmless there because a mis-anchored path simply fails to resolve
(no edge change). **This change makes it a recall risk**: the `ScopeResolution`
predicate would *prune* on a path the wrong edition mis-anchored, and a mis-anchor
that still yields a single clean module-level target could disprove the true
candidate. Because P1 is paramount and per-crate edition is a deferred capability (§7
(B)), this slice **treats a non-uniform-edition workspace as non-authoritative for
disproof**: the `ScopeResolution` predicate disproves nothing (keep-all) whenever the
scope graph's edition was not uniform across the repo's manifests. Implementation:
`parse_rust_crate_config` records whether all parsed manifests agreed on one edition
(an `edition_uniform: bool` on the crate config / a flag carried onto `ScopeGraph`);
the predicate early-returns "not disproved" when it is false. ruff is a uniform-2021
workspace, so this is a guard for correctness, not a recovery loss there; lifting it
is the per-crate-edition future slice (§7). Pin with a mixed-edition fixture
(2015 crate + 2021 crate) asserting keep-all.

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
type-path through the scope graph and matches the resolved callable target to an id-set
`ids` (as `graph_target_resolution` does today, `resolution.rs:559`). But it does **not**
prove directness or shadow-freedom by reading the returned `Candidate` — those facts are
not recoverable from the candidate (see ②B/①C below). It inspects the **scope-graph
binding/edge shapes at the call site directly**. It **disproves a candidate only when
both** of the following hold (otherwise it keeps all — uncertain ⇒ no disproof, P1):

1. **Direct binding (decision ②B — §8.2).** The owner type-path's **leading type
   segment** must bind via a **direct `BindTarget::Resolved`** binding to an in-repo
   item (`types.rs:383-388` — `BindTarget` is exactly `Resolved(Target)` | `Pending`),
   inspected on the scope graph at the call-site scope. A leading segment that binds via
   `BindTarget::Pending` (a named `use`/`pub use` re-export chain — `items.rs:216,231`)
   or that resolves through a glob edge / is ambiguous, or whose directness cannot be
   proven from the binding shape, is **not** a direct binding and disproves nothing
   (keep all). This is keyed on the **binding shape, not the resolved `Candidate`**:
   today the engine folds both a direct `Resolved` hit and a chased-`Pending` hit into a
   `Candidate` with empty `provenance` (`engine.rs:187-193,213-220`; `Provenance` is an
   empty struct, `types.rs:438-441`), so a "direct-only" rule **cannot** be honored by
   reading the candidate — it must check the binding/edge that the leading type-path
   resolved through. Concretely: prove directness by re-resolving the leading type
   segment (the `Anchor` of `rust_call_path_anchor`'s first segment) and requiring it
   reach an in-repo `Item` definition through a **non-glob `BindTarget::Resolved`**
   binding, with no `Pending` hop on the way; anything else keeps all. (Reusing
   `rust_graph_qualified_callable_edge`/`graph_target_resolution` for the *final
   callable* `ids` match is fine — those just supply the id-set — but they are **not**
   the directness oracle.)
2. **No block-local shadow (decision ①C — §8.1).** The resolved type-path's **leading
   segment** must have **no potential block-local (lexical) shadow** of that exact
   identifier in scope at the call site. A bare-leading `T::m()` owner path anchors at
   the *enclosing module* (`rust_call_path_anchor` → `Anchor::bare()`, `resolution.rs:1267`;
   `rust_policy.rs:296-302` `AnchorKind::Bare` → `enclosing_module`), and member lookup
   has **no lexical fall-out** (`engine.rs:370-374`), so a block-local shadow is invisible
   and the engine can return a single *wrong-scope* `ids`. A block-local shadow takes
   **three shapes**, all recorded *without* an exact-ident binding for two of them — so an
   exact-ident-only scan is too narrow (review BLOCKER). The predicate **keeps all**
   (disproves nothing) when the call site's lexical scope chain (block/callable scopes up
   to, but not past, the enclosing module) contains **ANY** of:
   - **(a)** a visible **`NS_TYPE` `Binding`** for the exact leading ident (an explicit
     block-local `use crate::b::Foo;` or block-scoped type item — `items.rs:227-235`);
   - **(b)** a visible **block-local glob edge** (`use crate::b::*;` — recorded as an
     **unnamed `Edge`** with `BindTarget::Pending`, not an exact-ident binding —
     `items.rs:237-247`; "Globs are `Edge`s", `types.rs:416-417`), whose
     `glob_vis_range` covers the call-site byte;
   - **(c)** a covering **macro wildcard poison** in `NS_TYPE` (an item-position
     name-introducing macro — `walk/types.rs:403-418` adds `MacroWildcard` over
     `NS_VALUE/NS_TYPE/NS_MACRO`; `graph.rs:33-55` — glob-tier, shadows when no explicit
     binding claims the name) whose `range` covers the call-site byte.
   This is the conservative **"any local shadow source ⇒ keep-all"**: globs (b) and
   macros (c) introduce *unknowable* names, so the leading ident *could* bind locally
   even when no exact `NS_TYPE` binding exists. This is a cheap per-site scope scan over
   `graph.bindings` / `graph.edges` / `graph.macro_wildcards` — no
   name-resolution-engine change.

When both hold, it **disproves** any candidate **not in `ids`** (Rust name resolution is
deterministic — at that site `CliTest` *is* `ruff::CliTest`, so `ty::CliTest` is
provably not the target). Under P1's completeness premise (§2) and these two contracts,
eliminating non-`ids` candidates is sound.

True lexical first-segment resolution (replacing the conservative block-local-shadow
keep-all guard — (a)/(b)/(c) above — with a real lexical resolve that *knows* whether a
glob/macro actually introduced the leading ident) and re-export/glob `Provenance` markers
(enabling safe pruning through `pub use` facades and globs, replacing the binding-shape
direct-only test) are the deferred precision follow-ups (§9).

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
| **unchanged from bare** (path did not resolve) | **fall through** to #120's `owner_lookup_in_modules` demote floor (full-pool NameOnly), **bypassing the legacy stem heuristic** (review MAJOR 4 — see below) | NameOnly |

Two behavior changes vs today:
1. **`graph_target_resolution` returns its `ids` set**: `1 → exact(ids, …)` (today),
   **`>1 → demoted(ids, QualifiedOwner)`** (today returns `None` at `:599`). The kind
   for the `::`-qualified case stays `QualifiedOwner`; the unqualified `LocalDef`/
   `FreeSingle` arms (`:602-607`) keep returning only on a singleton (a `>1`
   unqualified free-fn set is out of scope here — it routes through the existing
   free-fn rungs).
2. **Fail-open at `resolution.rs:687`, scoped to the qualified-owner-demote path
   (review MAJOR 4)**: when `rust_scope_graph_resolution` returns `None`, fall
   through **only for `name.contains("::")` owner-path sites** — and then **only as
   far as #120's `owner_lookup_in_modules` demote floor**, NOT into the legacy
   `::`-split stem heuristic. A naïve "just fall through to the bare path" is
   **over-broad and breaks shipped recall-safety invariants** that the current tests
   pin; the fall-through MUST preserve all three:
   - **unqualified bare miss → still drops** (no legacy free-fn fan-out). Pinned by
     `rust_scope_graph_unqualified_declines_do_not_legacy_guess`
     (`tests/integration/resolution_test.rs:1528`) — an authoritative unresolved bare
     name asserts EMPTY; a blind fall-through would route it to the repo-wide
     free-fn rung (`resolution.rs:1063`) and resurrect the 2-edge guess.
   - **authoritative unresolved qualified path → still drops, NOT stem-guessed.**
     Pinned by `rust_scope_graph_qualified_paths_resolve_or_disable_legacy_stem`
     (negative case, `:1629`). **Verified concretely**: for `crate::missing::target`,
     a blind fall-through reaches the `::`-split at `resolution.rs:705`,
     `owner_lookup_in_modules("missing","target")` misses (module, not a type), then
     `functions.get("target")` + `file_stem == "missing"` (`:746-765`) mints a
     `StemSingle` **Exact** to `src/missing.rs` — exactly the legacy stem heuristic
     the test forbids. So the qualified fail-open must route to the
     `owner_lookup_in_modules` demote and stop there, never to the stem block.
   - **pending/unresolved import → still poisons + suppresses fan-out.** Pinned by
     `rust_scope_graph_authority_gate_and_poison_skip_legacy` (`:1674`).

   Concretely: the fail-open target is the `owner_lookup_in_modules(head, fn_name,
   module_segs)` arm (`resolution.rs:742`), which yields #120's full-pool NameOnly
   demote when the owner key has candidates and `None` (→ existing drop) otherwise.
   The pre-gate `modfx` drop (`unresolved_unknown_name=1`) thus becomes a NameOnly
   demote **for owner-keyed `::` sites only**. The non-scope-graph path (most repos)
   is untouched; #120's full demote remains the floor. New tests assert: (a) a
   colliding-owner `::` miss demotes (not drops); (b) the three invariants above are
   unchanged (regression-pin them inside the scope-graph-present build).

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
5. **Seam — ②B re-export facade → keep all:** the leading type-path segment binds via a
   named `pub use` facade (`BindTarget::Pending`) that resolves cleanly to a single
   target — predicate keeps all (NameOnly), does **not** pin (the directness test fails
   on the `Pending` hop even though the final callable resolves to one id). Pins ②B's
   binding-shape directness check (a candidate-level provenance read would wrongly pin).
6. **Seam — ①C block-local glob shadow → keep all (review re-review BLOCKER):** module
   level `use a::Foo;` + a function with **block-local `use b::*;`** and a `Foo::m()`
   call, where both `a::Foo` and `b::Foo` exist with `m`. The module-anchored resolution
   sees only `a::Foo`; the block-local glob (unnamed `Edge`, no exact `Foo` binding) is a
   potential shadow → predicate **keeps all** (NameOnly), does **not** prune to
   `a::Foo::m`. Pins guard shape (b). *(An exact-ident-only ①C would wrongly drop the
   `b::Foo::m` edge here — the BLOCKER's P1 violation.)*
7. **Seam — ①C macro-wildcard shadow → keep all:** a function with an item-position
   name-introducing macro invocation (poisons `NS_TYPE` over the trailing scope) and a
   bare `Foo::m()` after it → predicate keeps all (NameOnly). Pins guard shape (c).
8. **Fail-open:** a `::` call the authoritative scope path can't resolve → NameOnly
   demote, **not** `UnknownName` drop.

### Tier-A acceptance gate
- `cargo build --release`; `cd eval && uv run tier-a --matrix-only --allow-stale-sut`
  then `--quick` (clear the nav cache first — §6/cache).
- **Recovery signal** (`prism nav --no-cache call-stats`, ruff/prism/ripgrep): ruff
  `kind_nameonly[qualified_owner]` **drops** with a matching rise in
  `kind_exact[qualified_owner]` (and/or smaller pruned NameOnly sets). Report a
  per-anchor recovered-Exact count (the `kind_nameonly[qualified_owner]` delta plus
  the `kind_exact[qualified_owner]` delta between the #120 baseline and this branch).

  **The legacy `shadow_typepath_narrow` counter is NOT the signal (review MAJOR 5).**
  It only runs inside the `exact_kinds.len() >= 2` guard (`navigation/queries.rs:167`,
  `shadow_narrow_type_path` at `:51`), i.e. over **multi-target-Exact** sites. But
  #120 moved these collisions to NameOnly (so they have 0 Exact edges → guard never
  entered — pinned by `call_stats_demoted_collision_absent_from_shape_and_shadow`,
  `tests/cli/call_stats_test.rs:78`), and **this change's own recovery makes them
  *single*-Exact** (1 edge, still < 2) — so the shadow stays empty either way and
  cannot show `singleton`. The forward instrument must therefore be **a new
  recovery counter**, added in this slice: count, over NameOnly `qualified_owner`
  collision sites, how many the scope path now resolves to `singleton` /
  `pruned-multiple` / `failopen-unresolved` (reusing `shadow_narrow_type_path`'s
  resolve-and-classify body, but keyed off the demoted-NameOnly population rather
  than the `>=2 Exact` population). Wire it into `call_stats` alongside the existing
  histogram; the acceptance reads `singleton` rising from this new counter **and**
  the `kind_exact`/`kind_nameonly` deltas above.
- **Recall-safety:** matrix **0 regression**; `--quick` M2 **fn = 0** (no dropped
  edges), fp not increased. If recall drops on any anchor → STOP (a prune dropped a
  real edge; tighten the predicate's soundness). Paste before/after into the PR.
- Independent codex (gpt-5.5, xhigh) spec/plan/diff reviews per the established loop.

## 8. Risks

### 8.1 RESOLVED — owner chose (C) on 2026-06-21 — `T::m` bare-anchor scope is module-level, not lexical

**Resolution (owner, 2026-06-21): (C) block-local-shadow keep-all**, with (A) filed as
the deferred precision follow-up (§9). The `ScopeResolution` predicate disproves a
candidate only when the resolved type-path's leading segment has **no potential
block-local (lexical) shadow** of that exact identifier in scope at the call site. A
block-local shadow has **three shapes** (review re-review BLOCKER — an exact-ident-only
scan is too narrow because two of them are recorded *without* an exact-ident binding):
**(a)** an explicit `NS_TYPE` `Binding` for the exact leading ident (`items.rs:227-235`);
**(b)** a block-local **glob** edge `use crate::b::*;` — an **unnamed `Edge`** with
`BindTarget::Pending` (`items.rs:237-247`; "Globs are `Edge`s", `types.rs:416-417`); or
**(c)** a **macro wildcard poison** in `NS_TYPE` from an item-position name-introducing
macro (`walk/types.rs:403-418`; `graph.rs:33-55`). If **any** of the three is visible at
the call-site byte in the lexical chain up to the enclosing module, the predicate keeps
all (disproves nothing) — the conservative "any local shadow source ⇒ keep-all", because
a glob or macro could bind the leading ident locally even with no exact `NS_TYPE`
binding. This is a cheap per-site scan over `graph.bindings`/`graph.edges`/
`graph.macro_wildcards` — no name-resolution-engine change. Folded into the §3 predicate
contract.

**The underlying problem (verified true).** For a `T::m()` call, `rust_call_path_anchor`
anchors the leading type segment with `Anchor::bare()` (`resolution.rs:1267`).
`RustPolicy::anchor` maps `AnchorKind::Bare` to the **enclosing module**
(`rust_policy.rs:296-302`), and `resolve_path_guarded` then walks the path from that
module scope with **no lexical fall-out** (`engine.rs:332`, member lookup `:370-374`).
So a **block-local `use crate::b::Foo;`** (or block-local glob / macro wildcard —
recorded at the Block/Callable scope, `rust_populator/walk/items.rs:195-207,237`) is
**never consulted**: a module-level `use crate::a::Foo;` resolves cleanly to a single
`a::Foo`, and the engine returns one Resolved candidate. Today this is a latent
*precision* bug on the shipped `T::m` graph edge (it only ever picks *which single*
Exact); without the guard this change would weaponize it into a recall drop (disproving
the true `b::Foo::make` edge — a P1 violation), which (C) prevents.

**Rejected options (for the record):**
- **(A) True lexical first-segment resolution** — resolve the leading type segment with
  the lexical `resolve(..., NS_TYPE)` from the *actual* `from` scope (honoring
  block-local `use`/glob, propagating `Poisoned`), then continue member lookup. Most
  precise, but real engine work + a poison-propagation path; perturbs the shipped
  `T::m` edge's behavior. **Deferred to §9** as the precision-completing follow-up.
- **(B) Conservative keep-all for any bare-leading segment in a function with *any*
  block-local type binding/glob/macro** — recall-safe but blunts recovery for the common
  bare `T::m()` shape in functions with **unrelated** block-local imports. Superseded by
  (C), which is **ident-scoped**: (C) keeps-all only when the shadow source could bind
  *this* leading ident — an exact-ident `NS_TYPE` binding (a), or a glob/macro wildcard
  whose unknowable name-set *includes* this ident (b/c) — so an unrelated block-local
  `use std::fmt::Write;` next to a `Foo::m()` call does **not** block recovery, whereas a
  block-local `use other::*;` (which could introduce `Foo`) does. *(Re-review note: the
  re-review's BLOCKER broadened (C) to cover (b) glob and (c) macro shadows — those are
  the same shapes (B) would catch, but (C) catches them only against the leading ident's
  potential binding, not against any unrelated import.)*

### 8.2 RESOLVED — owner chose (B) on 2026-06-21 — the "re-export → keep all" predicate is not implementable as written

**Resolution (owner, 2026-06-21): (B) direct-binding-only prune**, with (A) filed as
the deferred follow-up (§9). The `ScopeResolution` predicate prunes **only on a direct
in-scope type binding** — the **leading type-path segment** must bind via a **direct
`BindTarget::Resolved`** binding to an in-repo item, with **no `Pending` hop** chased on
the way. A leading segment that binds via `BindTarget::Pending` (a named `use`/`pub use`
re-export — `items.rs:216,231`), through a glob edge, or ambiguously — or whose
directness cannot be proven from the binding shape — disproves nothing (keep all).

**Implementation recipe (review re-review MAJOR — the recipe, not the contract, was
imprecise).** Directness MUST be proven by inspecting the **binding shape** in the scope
graph, **NOT** by reading the resolved `Candidate`. `rust_graph_qualified_callable_edge`
(`resolution.rs:1219`) and `graph_target_resolution` (`:559`) expose only the **final
resolved callable target** and an id-set; they **cannot** prove the *owner type-path* was
reached by a direct binding rather than chased through `Pending`, because the engine
folds both a direct `BindTarget::Resolved` hit and a chased-`Pending` hit into a
`Candidate` with empty `provenance` (`engine.rs:187-193,213-220`; `Provenance` is an
empty struct — `types.rs:438-441`; `BindTarget` is exactly `Resolved`|`Pending` —
`types.rs:383-388`). So the predicate proves directness by **re-resolving the leading
type segment** and requiring it reach an in-repo `Item` through a non-glob
`BindTarget::Resolved` binding with no intervening `Pending`; it may still *reuse*
`graph_target_resolution`'s id-set for the final callable match, but that helper is **not**
the directness oracle. (B) is recall-safe and gives up recovery through re-export facades
(acceptable — facades are a minority of ruff's `qualified_owner` pool). Folded into the
§3 predicate contract.

**The underlying problem (verified true).** Named `use`/`pub use` is stored as
`BindTarget::Pending` (`rust_populator/walk/items.rs:216`); when the chain resolves, the
engine folds the chased target into an ordinary `Candidate` whose `provenance` is
`Default::default()` (`engine.rs:213-220`), and `Provenance` is an **empty struct**
(`name_resolution/types.rs:438-441`). So at the consumer there is **no signal** that a
resolved `Target` arrived via a re-export vs a direct type binding — a "re-export → keep
all" rule keyed on provenance cannot be honored for named `pub use` facades (which today
resolve to a single clean target and *look identical* to a direct hit). (B) sidesteps
this by keying the prune on the *direct vs folded-Pending* binding shape, which **is**
observable today (a direct `BindTarget::Resolved` `Item`-definition vs a chased-Pending
result), rather than on absent provenance. This is recall-safe and gives up recovery
through re-export facades (acceptable — facades are a minority of ruff's
`qualified_owner` pool).

**Rejected options (for the record):**
- **(A) Add resolution provenance** — have the Rust policy populate `Provenance` with a
  "via re-export / via glob" marker (the struct is reserved for exactly this, "Phase N:
  structured provenance"); keep-all whenever provenance is non-direct. Most principled
  and reusable (also unlocks safe glob/re-export pruning), but threading provenance
  through `resolve_rib`/`scope_member_lookup` is its own change with cache-byte impact.
  **Deferred to §9** as the follow-up that enables safe facade/glob pruning.
- **(C) Accept the residual as bounded and document it** — keep the as-written contract
  but note that named `pub use` resolves opaquely and rely on §8.1 + the Tier-A recall
  gate to catch any drop. Cheapest, weakest soundness guarantee. Superseded by (B)'s
  implementable direct-binding test.

### Other risks

- **Pruning soundness (the dominant risk).** If the scope graph mis-resolves a path
  (re-exports, glob imports, macro-generated `use`), the `ScopeResolution` predicate
  could disprove the true candidate → a dropped real edge (P1 violation). Mitigations:
  the completeness premise (§2), "uncertain → keep" (§3), fail-open (§4), the
  edition-uniformity guard (§2), the §3 predicate's two resolved contracts
  (①C block-local-shadow keep-all + ②B direct-binding-only prune — §8.1/§8.2), and the
  Tier-A recall gate. **Glob/re-export is the explicit watch-area** — the predicate
  treats them as "not a pin" (keep all) until a safe refinement is designed.
- **Perf.** Building the scope graph for ruff (1,874 `.rs`) adds build time; expected
  acceptable post-S1, measured in the gate. A regression beyond a small budget is a
  blocker for the (A) default-on behavior.
- **Cache churn.** The bump invalidates all caches once; acceptable.

## 9. Out of scope / future slices

Each plugs into the §3 seam or extends §2 — none in this slice:
- **(A) true lexical first-segment resolution for `T::m`** (§8.1 follow-up): resolve the
  leading type segment lexically from the *actual* `from` scope (honoring block-local
  `use`/glob, propagating `Poisoned`), then continue member lookup — **replaces the
  block-local-shadow keep-all guard** (decision ①C) with real lexical resolution, so
  shadowed bare `T::m()` sites recover instead of keeping all.
- **(A) re-export/glob `Provenance` markers** (§8.2 follow-up): populate `Provenance`
  with via-re-export / via-glob markers (the struct is reserved for exactly this), so
  the `ScopeResolution` predicate can **safely prune through `pub use` facades and
  globs** instead of keeping all on them (the recovery decision ②B's direct-binding-only
  rule forgoes). Also unlocks the safe-glob-pruning predicate below.
- **Reachability/visibility predicate** (Approach 2): crate-dep-graph + import-edge
  pruning when the path doesn't fully resolve (uses `manifest_hashes`).
- **(B) per-crate authoritativeness**: a scope is authoritative iff its crate's `.rs`
  are present; handles partial Rust coverage (skipped own `.rs`).
- **arity / receiver-type / trait-bound predicates**; consolidating the ad-hoc
  filters (§3) into the seam.
- **Other-language scope resolution** (generalize §2 coverage per-language).
- **Safe glob/re-export pruning** (refine the `ScopeResolution` predicate; depends on
  the `Provenance` markers above).
