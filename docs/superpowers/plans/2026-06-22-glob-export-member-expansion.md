# Glob Re-Export Member Expansion — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL — implement this plan task-by-task with TDD
> (`superpowers:test-driven-development`): write the failing test, watch it fail, minimal code to
> pass, watch it pass, commit. Steps use checkbox (`- [ ]`) syntax.
>
> **The authoritative algorithm is the spec** —
> `docs/superpowers/specs/2026-06-22-glob-export-member-expansion-design.md` (codex xhigh SHIP, 3
> rounds). Read spec §3.2 (the `glob_lookup` algorithm), §3.3 (depth/cycle), §3.4 (tri-state
> visibility), §3.5 (telemetry). This plan gives the file/task/test breakdown; where it says "per
> spec §X", that section is the exact behavior.

**Goal:** Expand deferred Rust glob re-export edges (`pub use mod::*`) during name resolution,
bounded to two glob hops, so a queried name reachable through a facade resolves instead of poisoning —
never minting a wrong full-confidence edge, and instrumenting every fail-closed bail into a new
`glob_expand` call-stats histogram.

**Architecture:** One engine function (`glob_lookup`) changes behavior: a deferred glob is expanded
(resolve its target scope via the existing `resolve_path_guarded`, look the name up there via
`scope_member_lookup_probed`, recursing one more hop), gated by a depth bound, an extended cycle
guard, and a new tri-state edge-visibility policy hook, with every outcome recorded to a process-
global per-measurement counter. `resolve`/`resolve_path`/`CallGraph` signatures are UNCHANGED.

**Tech Stack:** Rust, the `prism::name_resolution` scope-graph engine, `petgraph`, `serde_json` (the
call-stats JSON), `std::sync::atomic` (the counters). Tests are real-source fixtures via the existing
`single_file_resolve` pattern + `build_wiring_test` multi-crate builds.

---

## File Structure

| File | Responsibility | Change |
|------|----------------|--------|
| `src/name_resolution/glob_stats.rs` | **(new)** process-global per-measurement telemetry: `GlobExpandStats` (8 `AtomicUsize`), per-bucket record helpers, `reset()`, `snapshot() -> GlobExpandSnapshot`, a `static GLOBAL`. | create |
| `src/name_resolution/mod.rs` | module wiring | add `pub mod glob_stats;` (or `pub(crate)`) |
| `src/name_resolution/engine.rs` | the resolution engine | `CycleGuard` gains glob state + RAII helper; `glob_lookup` rewritten to expand; `MAX_GLOB_DEPTH` const; `#[doc(hidden)] pub` stats entries (always-compiled so the integration-test crate can inject a local sink — `#[cfg(test)]` items are NOT visible across the integration-test boundary) | modify |
| `src/name_resolution/types.rs` | policy trait + types | `GlobEdgeVis` enum; `ResolutionPolicy::glob_edge_visible` (default `Visible`) | modify |
| `src/name_resolution/rust_policy.rs` | Rust policy | `vis_reaches` helper (factored from `visible()`); `glob_edge_visible` impl | modify |
| `src/navigation/queries.rs` | `call_stats` | reset at entry, snapshot after the loop, emit `glob_expand` | modify |
| `src/cpg_cache.rs` | cache version | `CACHE_VERSION` 18 → 19 + version test | modify |
| `tests/name_resolution/glob_expand_test.rs` | **(new)** the unit tests | create + `mod glob_expand_test;` in `main.rs` |
| `tests/name_resolution/build_wiring_test.rs` | multi-crate builds | add the glob-member-workspace fixture test | modify |
| `tests/integration/` | e2e | cross-crate facade collision recovers to one Exact | modify |

---

## Task 1: `glob_stats` telemetry module

**Files:**
- Create: `src/name_resolution/glob_stats.rs`
- Modify: `src/name_resolution/mod.rs` (add the module)
- Test: inline `#[cfg(test)] mod tests` in `glob_stats.rs`

- [ ] **Step 1: Write the failing test** (inline in `glob_stats.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn record_reset_snapshot_roundtrip() {
        let s = GlobExpandStats::default();
        s.record_resolved(1);
        s.record_resolved(2);
        s.record_depth_exceeded();
        s.record_cycle();
        s.record_external();
        s.record_multi_target();
        s.record_ambiguous();
        s.record_vis_unknown();
        let snap = s.snapshot();
        assert_eq!(snap.resolved_l1, 1);
        assert_eq!(snap.resolved_l2, 1);
        assert_eq!(snap.depth_exceeded, 1);
        assert_eq!(snap.cycle, 1);
        assert_eq!(snap.external, 1);
        assert_eq!(snap.multi_target, 1);
        assert_eq!(snap.ambiguous, 1);
        assert_eq!(snap.vis_unknown, 1);
        s.reset();
        assert_eq!(s.snapshot().resolved_l1, 0);
    }
}
```

- [ ] **Step 2: Run it, watch it fail** — `cargo test --lib name_resolution::glob_stats` → fails to compile (`GlobExpandStats` undefined).

- [ ] **Step 3: Implement the module**

```rust
//! Process-global, per-measurement telemetry for deferred-glob expansion
//! (spec §3.5). Reset at `call_stats` entry, snapshot after the re-resolution
//! pass. The counters are *expansion-event* counts, not final-edge counts — the
//! realized edge buy is read from `kind_exact`/`unresolved_unknown_name`.
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub struct GlobExpandStats {
    resolved_l1: AtomicUsize,
    resolved_l2: AtomicUsize,
    depth_exceeded: AtomicUsize,
    cycle: AtomicUsize,
    external: AtomicUsize,
    multi_target: AtomicUsize,
    ambiguous: AtomicUsize,
    vis_unknown: AtomicUsize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GlobExpandSnapshot {
    pub resolved_l1: usize,
    pub resolved_l2: usize,
    pub depth_exceeded: usize,
    pub cycle: usize,
    pub external: usize,
    pub multi_target: usize,
    pub ambiguous: usize,
    pub vis_unknown: usize,
}

impl GlobExpandStats {
    const fn z() -> AtomicUsize { AtomicUsize::new(0) }

    /// `depth` is the *current* glob_depth (1 for the first hop, 2 for the second).
    pub fn record_resolved(&self, depth: usize) {
        match depth {
            1 => &self.resolved_l1,
            _ => &self.resolved_l2, // depth==2 (cap); never called deeper
        }
        .fetch_add(1, Ordering::Relaxed);
    }
    pub fn record_depth_exceeded(&self) { self.depth_exceeded.fetch_add(1, Ordering::Relaxed); }
    pub fn record_cycle(&self) { self.cycle.fetch_add(1, Ordering::Relaxed); }
    pub fn record_external(&self) { self.external.fetch_add(1, Ordering::Relaxed); }
    pub fn record_multi_target(&self) { self.multi_target.fetch_add(1, Ordering::Relaxed); }
    pub fn record_ambiguous(&self) { self.ambiguous.fetch_add(1, Ordering::Relaxed); }
    pub fn record_vis_unknown(&self) { self.vis_unknown.fetch_add(1, Ordering::Relaxed); }

    pub fn reset(&self) {
        for a in [
            &self.resolved_l1, &self.resolved_l2, &self.depth_exceeded, &self.cycle,
            &self.external, &self.multi_target, &self.ambiguous, &self.vis_unknown,
        ] {
            a.store(0, Ordering::Relaxed);
        }
    }
    pub fn snapshot(&self) -> GlobExpandSnapshot {
        let g = |a: &AtomicUsize| a.load(Ordering::Relaxed);
        GlobExpandSnapshot {
            resolved_l1: g(&self.resolved_l1), resolved_l2: g(&self.resolved_l2),
            depth_exceeded: g(&self.depth_exceeded), cycle: g(&self.cycle),
            external: g(&self.external), multi_target: g(&self.multi_target),
            ambiguous: g(&self.ambiguous), vis_unknown: g(&self.vis_unknown),
        }
    }
}

/// The process-global sink used by production resolution (reset per `call_stats`
/// measurement). Tests inject a LOCAL `&GlobExpandStats` via the `#[cfg(test)]`
/// engine entries instead (spec §3.5 test isolation).
pub static GLOBAL: GlobExpandStats = GlobExpandStats {
    resolved_l1: GlobExpandStats::z(), resolved_l2: GlobExpandStats::z(),
    depth_exceeded: GlobExpandStats::z(), cycle: GlobExpandStats::z(),
    external: GlobExpandStats::z(), multi_target: GlobExpandStats::z(),
    ambiguous: GlobExpandStats::z(), vis_unknown: GlobExpandStats::z(),
};
```

Add `pub mod glob_stats;` to `src/name_resolution/mod.rs` (match the existing module visibility style there).

- [ ] **Step 4: Run it, watch it pass** — `cargo test --lib name_resolution::glob_stats` → PASS.

- [ ] **Step 5: Commit** — `feat(name-res): glob_stats per-measurement telemetry module`.

---

## Task 2: `CycleGuard` glob state + RAII glob-guard

**Files:**
- Modify: `src/name_resolution/engine.rs` (the `CycleGuard` struct + impl, ~`:75`)
- Test: inline `#[cfg(test)]` in `engine.rs` (a small unit test of the guard)

**Note (codex impl flag):** the RAII guard must NOT store `&mut CycleGuard` while callers keep using
the original `guard` (borrow conflict). Use a **`with_glob` closure** that owns the enter/leave and
hands the recursive body a reborrowed `&mut CycleGuard` — see Step 3.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod glob_guard_tests {
    use super::*;
    #[test]
    fn enter_glob_tracks_depth_and_cycle_then_leaves() {
        let stats = crate::name_resolution::glob_stats::GlobExpandStats::default();
        let mut g = CycleGuard::with_stats(Some(&stats));
        assert_eq!(g.glob_depth(), 0);
        let entered = g.enter_glob(7);
        assert!(entered);
        assert_eq!(g.glob_depth(), 1);
        assert!(!g.enter_glob(7), "re-entering the same edge is a cycle");
        g.leave_glob(7);
        assert_eq!(g.glob_depth(), 0);
        assert!(g.enter_glob(7), "leaving clears the edge");
        g.leave_glob(7);
    }
}
```

- [ ] **Step 2: Run it, watch it fail** — `cargo test --lib name_resolution::engine::glob_guard_tests` → fails (`with_stats`/`glob_depth`/`enter_glob` undefined).

- [ ] **Step 3: Extend `CycleGuard`** (spec §3.3, §3.5). Add fields + methods; add a lifetime for the
optional stats sink. Keep the existing binding-index `enter`/`leave` untouched.

```rust
use crate::name_resolution::glob_stats::GlobExpandStats;

pub(crate) const MAX_GLOB_DEPTH: usize = 2;

struct CycleGuard<'s> {
    active: std::collections::BTreeSet<usize>,        // existing: Pending-binding indices
    active_globs: std::collections::BTreeSet<usize>,  // NEW: glob-edge indices on the chain
    glob_depth: usize,                                // NEW
    stats: Option<&'s GlobExpandStats>,               // NEW: None ⇒ use glob_stats::GLOBAL
}

impl<'s> CycleGuard<'s> {
    fn with_stats(stats: Option<&'s GlobExpandStats>) -> Self {
        CycleGuard { active: Default::default(), active_globs: Default::default(),
                     glob_depth: 0, stats }
    }
    // existing enter/leave on `active` unchanged …
    fn glob_depth(&self) -> usize { self.glob_depth }
    /// false ⇒ this glob edge is already on the chain (cycle). On success, marks
    /// it active and increments depth.
    fn enter_glob(&mut self, edge_idx: usize) -> bool {
        if !self.active_globs.insert(edge_idx) { return false; }
        self.glob_depth += 1;
        true
    }
    fn leave_glob(&mut self, edge_idx: usize) {
        self.active_globs.remove(&edge_idx);
        self.glob_depth -= 1;
    }
    /// The telemetry sink (local test sink if injected, else the process-global).
    fn stats(&self) -> &GlobExpandStats {
        self.stats.unwrap_or(&crate::name_resolution::glob_stats::GLOBAL)
    }
}
```

Update the two existing guard constructions in the public entries `resolve` (`:41`) and `resolve_path`
(`:64`) from `CycleGuard::default()` to `CycleGuard::with_stats(None)` (production → global sink). The
public signatures stay unchanged.

`Default` is no longer derivable with a lifetime+reference field — replace `#[derive(Default)]` with
the explicit `with_stats` constructor (and `with_stats(None)` is the "default").

- [ ] **Step 4: Run it, watch it pass.**

- [ ] **Step 5: Commit** — `feat(name-res): CycleGuard glob-edge cycle set + depth + stats sink`.

---

## Task 3: `GlobEdgeVis` tri-state + `glob_edge_visible` policy hook + `vis_reaches`

**Files:**
- Modify: `src/name_resolution/types.rs` (`GlobEdgeVis` enum + trait method)
- Modify: `src/name_resolution/rust_policy.rs` (`vis_reaches` helper + `glob_edge_visible` impl;
  refactor `visible()` to call `vis_reaches`)
- Test: `tests/name_resolution/glob_expand_test.rs` (a `glob_edge_visible` unit + a `vis_reaches`
  parity check) — but since `vis_reaches`/`glob_edge_visible` may be private, assert through behavior
  in Task 4; here add a `#[cfg(test)]` parity test inside `rust_policy.rs`.

- [ ] **Step 1: Write the failing test** (inline in `rust_policy.rs`): assert `vis_reaches` agrees
with the existing `visible()` outcome for a `pub` and a private binding, and that a `pub(in)` with no
restrict yields `None` (the Unknown case).

```rust
#[cfg(test)]
mod vis_reaches_tests {
    // Build a tiny ScopeGraph with a module `m` (child of root) holding a `pub`
    // binding `P` and a private binding `H`; assert:
    //   vis_reaches(pub,  def=m, from=root) == Some(true)
    //   vis_reaches(priv, def=m, from=root) == Some(false)   // outside m's subtree
    //   vis_reaches(priv, def=m, from=m)    == Some(true)     // within m
    //   vis_reaches(pub(in <unresolved>), …) == None          // Unknown
    // (Construct via the populator helpers used elsewhere in this module's tests,
    //  or the lower-level graph builder; mirror the existing rust_policy tests.)
}
```

- [ ] **Step 2: Run it, watch it fail.**

- [ ] **Step 3: Implement** (spec §3.4):
  - In `types.rs`: `pub enum GlobEdgeVis { Visible, Hidden, Unknown }`; add to `ResolutionPolicy`:
    `fn glob_edge_visible(&self, _edge: &Edge, _q: &ResolveQuery, _trav: &TraversalCtx) -> GlobEdgeVis { GlobEdgeVis::Visible }` (default).
  - In `rust_policy.rs`: factor the per-`vis.kind` logic of `visible()` (`:228`–`:267`) into
    `fn vis_reaches(&self, vis: &Vis, def_scope: ScopeId, from: ScopeId) -> Option<bool>` returning
    `Some(true)`/`Some(false)` for decidable kinds and **`None`** when undecidable (notably
    `VIS_PUB_IN` with `restrict: None` — today folded to `false`). Rewrite `visible()` as
    `self.vis_reaches(&binding.vis, binding.scope, q.from).unwrap_or(false)` (behavior-preserving: a
    rib still fails closed on `None`). Implement `glob_edge_visible` mapping `edge.from` as the
    defining scope: `Some(true) ⇒ Visible`, `Some(false) ⇒ Hidden`, `None ⇒ Unknown`.

  **Behavior-preserving guard:** after refactoring `visible()`, run the FULL existing
  `--test name_resolution` suite — `visible()` must be byte-for-byte behaviorally identical (the
  `unwrap_or(false)` preserves the old `VIS_PUB_IN`-no-restrict → false).

- [ ] **Step 4: Run it, watch it pass; run `cargo test --test name_resolution` (visible() parity).**

- [ ] **Step 5: Commit** — `feat(name-res): tri-state glob_edge_visible hook + vis_reaches helper`.

---

## Task 4: `glob_lookup` expansion (the core) + `#[cfg(test)]` stats entries + the resolution tests

**Files:**
- Modify: `src/name_resolution/engine.rs` (`glob_lookup` `:252`; thread `&mut CycleGuard` from call
  sites `:132`, `:439`; add `#[cfg(test)] resolve_with_stats`/`resolve_path_with_stats`)
- Test: `tests/name_resolution/glob_expand_test.rs` (new) + `mod glob_expand_test;` in `main.rs`

This is the core. **The algorithm is spec §3.2 — implement it exactly.** Skeleton:

```rust
fn glob_lookup(graph, scope_id, q, policy, guard: &mut CycleGuard) -> GlobOutcome {
    // for each glob edge `e` from scope_id (existing vis_range byte gate kept):
    //   step 0: match policy.glob_edge_visible(e, q, trav) {
    //       Hidden  => continue,                 // skip (sound)
    //       Unknown => { guard.stats().record_vis_unknown(); return Poison; }
    //       Visible => { /* proceed */ }
    //   }
    //   Resolved-scope arm: unchanged (dead for Rust).
    //   Deferred Pending(path, anchor) arm:
    //     if guard.glob_depth() >= MAX_GLOB_DEPTH { stats.record_depth_exceeded(); return Poison; }
    //     guard.with_glob(edge_idx, |guard| {            // RAII: enter_glob → body → leave_glob
    //        if !entered (cycle) { stats.record_cycle(); return Poison; }
    //        let tgt = resolve_path_guarded(graph, path, NS_TYPE, anchor, scope_id, prefix_ns, &q.at, policy, guard);
    //        match classify(tgt) {
    //          single Scope(T) with cond tc =>
    //             match scope_member_lookup_probed(graph, T, q, policy, guard) {
    //               Resolved[mc]   => { push conjoin(e.cond, conjoin(tc, mc.cond)); stats.record_resolved(guard.glob_depth()); }
    //               ResolvedSet|Ambiguous|>1 => { stats.record_ambiguous(); return Poison; }
    //               Poisoned       => return Poison;
    //               Unresolved     => { /* contribute nothing, continue to next edge */ }
    //             }
    //          >1 scope / Ambiguous => { stats.record_multi_target(); return Poison; }
    //          Unresolved/external/non-scope/Poisoned => { stats.record_external(); return Poison; }
    //        }
    //     })
    // after loop: !saw_glob || candidates.empty => Empty; else Hit(candidates)
}
```

Implement `with_glob` on `CycleGuard` to avoid the borrow trap (codex flag):

```rust
impl<'s> CycleGuard<'s> {
    /// Enter the glob edge, run `body` with the guard reborrowed, then always leave.
    /// `body` is only called when entry succeeded (no cycle); returns `None`-equivalent
    /// handling is up to the caller via the bool. Here we expose entry success to the body.
    fn with_glob<R>(&mut self, edge_idx: usize, body: impl FnOnce(&mut Self, bool) -> R) -> R {
        let entered = self.enter_glob(edge_idx);
        let r = body(self, entered);
        if entered { self.leave_glob(edge_idx); }
        r
    }
}
```

Thread `guard` into `glob_lookup` at both call sites: `resolve_bare` (`:132`) and
`scope_member_lookup_probed` (`:439`) — both already hold `&mut CycleGuard`.

Add the local-sink entries so integration tests assert on their own `GlobExpandStats`. These must be
**always-compiled** `#[doc(hidden)] pub` (NOT `#[cfg(test)]`): `tests/name_resolution/` is a separate
integration-test crate that imports `prism::…`, and `#[cfg(test)]` library items are not exported to
it (plan-review BLOCKER 1). `#[doc(hidden)]` keeps them out of the public API docs; `glob_stats` is
`pub` so the test crate can construct `GlobExpandStats`.

```rust
#[doc(hidden)] // test-support: inject a local stats sink (not part of the public API)
pub fn resolve_with_stats(graph, q, policy, stats: &GlobExpandStats) -> Resolution {
    let mut guard = CycleGuard::with_stats(Some(stats));
    resolve_bare(graph, q, policy, &mut guard)
}
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn resolve_path_with_stats(graph, path, ns, anchor, from, anchor_ns, at, policy, stats: &GlobExpandStats) -> Resolution {
    let mut guard = CycleGuard::with_stats(Some(stats));
    resolve_path_guarded(graph, path, ns, anchor, from, anchor_ns, at, policy, &mut guard)
}
```

### The tests (`tests/name_resolution/glob_expand_test.rs`)

Mirror `rust_populate_test.rs`'s `single_file_resolve` pattern, but route through
`resolve_with_stats` with a local `GlobExpandStats` so each test asserts exact bucket counts. Add a
helper `fn resolve_with_glob_stats(src, edition, at, name, ns) -> (Resolution, GlobExpandSnapshot)`.
Each test below is written failing-first (it asserts the NEW resolved/poison behavior + the bucket;
under the old poison-always behavior it fails). Per spec §6:

- [ ] `glob_expand_single_hop_resolves` — `mod m { pub struct S; } pub use m::*;` query `S` at root
  (2018) → resolved to `m::S`; `snap.resolved_l1 == 1`.
- [ ] `glob_expand_two_hops_resolves` — root `pub use a::*`; `a` has `pub use b::*`; `b` defines `S`
  → resolved; `snap.resolved_l2 == 1`.
- [ ] `glob_expand_third_hop_blocked` — 3 nested facades; query the depth-3 name → poison;
  `snap.depth_exceeded == 1`. **Also** assert an `a::* ↔ b::*` 2-cycle → `depth_exceeded` (not
  `cycle`).
- [ ] `glob_expand_self_glob_cycle_fails_closed` — a **non-empty self-re-entering** glob
  `mod a { pub use crate::a::*; }` (NOT `pub use self::*`, which parses to an empty `RawPath` →
  `resolve_path_guarded` returns `Unresolved` at `engine.rs:327` *before* re-entry → that would be
  `external`, not `cycle` — plan-review BLOCKER 2). Querying an undefined name in `a` re-enters edge
  `E` at depth 1 → `enter_glob(E)` fails → poison; `snap.cycle >= 1`.
- [ ] `glob_expand_ambiguous_member_fails_closed` — name defined twice under *compatible* cfg in the
  target → poison; `snap.ambiguous == 1`.
- [ ] `glob_expand_resolved_set_member_fails_closed` — name defined twice under *cfg-exclusive*
  worlds (combine → ResolvedSet) → poison; `snap.ambiguous == 1`.
- [ ] `glob_expand_external_target_fails_closed` — glob path unresolvable/external → poison;
  `snap.external == 1`.
- [ ] `glob_expand_multi_target_fails_closed` — globbed module path itself ambiguous → poison;
  `snap.multi_target == 1`.
- [ ] `glob_expand_target_lacks_name_continues` — **two glob edges**, first target lacks the name,
  second provides → resolved to the second (the `Unresolved → continue` arm). (NOT a sibling rib —
  that bypasses `glob_lookup` via step 1.)
- [ ] `glob_expand_respects_member_visibility` — under `pub use m::*`, `m` has `pub S` and private
  `Hidden`: querying `S` from outside resolves (`resolved_l1 == 1`); querying `Hidden` from outside
  does NOT (unresolved/poison, never the private item).
- [ ] `glob_expand_skips_private_glob_edge` — a private (non-`pub`) `use m::*` in module `mid`, `m::S`
  public: query `S` from *outside* `mid` does NOT resolve through it (Hidden → skip); from *within*
  `mid` it does.
- [ ] `glob_expand_pub_in_unknown_fails_closed` — `pub(in some::path) use m::*` (restrict unresolved
  → Unknown): poison + `snap.vis_unknown == 1` from BOTH inside and outside vantage.
- [ ] `glob_expand_preserves_conditions` — a `#[cfg(feature="x")]`-gated `pub use m::*` with `m::S`:
  the resolved candidate carries the conjoined cond (a cfg-incompatible query does not select it).
- [ ] `glob_expand_diamond_resolves_single` — two glob paths to the SAME `S` → combine dedups → single
  `Resolved` (not `Ambiguous`).
- [ ] `glob_expand_distinct_targets_two_globs` — two glob edges yielding different `S` → `ResolvedSet`
  (cfg-exclusive) or `Ambiguous` (compatible cfg); never a silent single pick.

Implement steps per test: write the test (Step a), watch it fail (Step b), implement the matching
slice of the `glob_lookup` algorithm (Step c), watch it pass (Step d). The first test drives the core
expansion; subsequent tests drive each branch/bucket. Commit after the suite is green:
`feat(name-res): expand deferred glob re-exports (depth-2, fail-closed, instrumented)`.

- [ ] **Final step: run the full `cargo test --test name_resolution`** (the seam the §3 CI regression
  proved must not be skipped) + `cargo test --lib`. All green.

---

## Task 5: `call_stats` telemetry surface + cache version bump

**Files:**
- Modify: `src/navigation/queries.rs` (`call_stats` `:156`)
- Modify: `src/cpg_cache.rs` (`CACHE_VERSION` `:63`; version test `:568`–`:570`)
- Test: a `call_stats` test asserting the `glob_expand` key exists (extend an existing queries test or
  add one)

- [ ] **Step 1: Write the failing test** — assert `call_stats(&cg)["glob_expand"]` is an object with
  the 8 keys (`resolved_l1`, …, `vis_unknown`). Build a tiny CallGraph over a glob-facade fixture
  (reuse the `build_wiring_test` helper).

- [ ] **Step 2: Run it, watch it fail** (`glob_expand` key absent).

- [ ] **Step 3: Implement** — in `call_stats` (`queries.rs:156`): call
  `crate::name_resolution::glob_stats::GLOBAL.reset()` at entry; after the existing per-site
  re-resolution loop (`:198`–`:201`) completes, `let ge = glob_stats::GLOBAL.snapshot();` and insert
  `"glob_expand": { … }` into the returned `serde_json::Value` (8 fields). (Counts are resolution
  events during the call-stats pass — spec §3.5.)

- [ ] **Step 4: Run it, watch it pass.**

- [ ] **Step 5: Cache bump** — `src/cpg_cache.rs:63`: `CACHE_VERSION` 18 → 19 with a comment
  (`// 19: glob re-export member expansion (resolution behavior change)`); update the version
  assertion test (`:568`–`:570`). Run `cargo test --lib cpg_cache`.

- [ ] **Step 6: Commit** — `feat(nav): surface glob_expand histogram in call-stats; cache v19`.

---

## Task 6: Workspace fixture + cross-crate facade e2e

**Files:**
- Modify: `tests/name_resolution/build_wiring_test.rs` (glob-member-workspace resolution)
- Modify: `tests/integration/` (the appropriate file; cross-crate facade collision)

- [ ] **Step 1: Workspace fixture test** (`build_wiring_test.rs`) — a glob-member workspace
  (`members = ["crates/*"]`, concrete `crates/foo`, `crates/bar`) where `bar` does `use foo::SomeType`
  and `foo`'s root re-exports `SomeType` via `pub use inner::*`. Assert `SomeType` resolves from
  `bar` — requires **both** #124 (leading `foo`) and this slice (final segment through the glob).
  Watch it fail (today: unresolved), implement nothing new (Task 4 covers it), watch it pass.
  Use a **glob workspace** (the `members = ["crates/*"]` glob form) per the durable lesson, not only
  concrete members.

- [ ] **Step 2: e2e collision** (`tests/integration/`) — two crates each defining a same-named type
  behind a `pub use inner::*` facade; the *dependent* crate's call site recovers to **one** Exact, the
  *non-dependent* one stays dropped (depends on #124 dep-gating + this expansion). Assert the resolved
  edge count / single-owner. Watch fail → pass.

- [ ] **Step 3: Coverage matrix** — if a new test file changed the `tests/` layout, update the 3
  `all_test_files` copies in `tests/integration/coverage_test.rs` (per CLAUDE.md). `glob_expand_test.rs`
  is under `tests/name_resolution/` (registered via `main.rs`), so confirm whether the coverage matrix
  scans it; add if needed.

- [ ] **Step 4: Commit** — `test(name-res): glob-workspace fixture + cross-crate facade collision e2e`.

---

## Acceptance (host, after all tasks — spec §6/§7)

- [ ] `cargo fmt --check`; `cargo test --lib`; **`cargo test --test name_resolution`**;
  `cargo test --test integration`.
- [ ] `cargo build --release`, then call-stats deltas vs a `main` worktree on **ruff** and **prism**:
  `./target/release/prism nav --no-cache call-stats --repo <repo>` — report `kind_exact` (per-kind),
  `unresolved_unknown_name`, `dropped_multi_owner`, `recovery_typepath.*`, **`multi_target_exact_sites`
  (MUST be byte-identical — the canary)**, and the new `glob_expand` histogram. This is the combined-
  base measurement.
- [ ] Tier-A: `cd eval && uv run tier-a --matrix-only --allow-stale-sut` (0 regressions); ruff M2
  `uv run tier-a --corpus ruff --allow-stale-sut` (`baseline_invalid=false`, 0 regressions).
- [ ] codex xhigh final diff review → SHIP.

## Self-review checklist (run before handing to the implementer)
- Spec coverage: every spec §3/§5/§6 behavior maps to a task (visibility tri-state → T3+T4;
  depth/cycle → T2+T4; conds → T4; telemetry → T1+T4+T5; cache → T5; fixtures → T6). ✓
- No placeholders: structs/signatures/test names are concrete; the algorithm is spec §3.2 (referenced,
  not re-transcribed — the implementer reads both). ✓
- Type consistency: `GlobExpandStats`/`GlobExpandSnapshot`/`GlobEdgeVis`/`CycleGuard::with_stats`/
  `with_glob`/`MAX_GLOB_DEPTH`/`record_*` names match across tasks. ✓
