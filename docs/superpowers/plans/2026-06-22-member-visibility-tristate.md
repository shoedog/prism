# Member-Visibility Tri-State Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a glob facade (`pub use mod::*`) member lookup *continue* past a claimed-but-filtered member rib when every binding is provably `Hidden`, while still poisoning on any `Unknown` (undecidable `pub(in)`), recovering recall over #126's conservative blanket-poison without ever minting a wrong singleton.

**Architecture:** A policy tri-state hook `member_visible -> VisibilityDecision{Visible,Hidden,Unknown}` (Rust maps `vis_reaches`; non-Rust defaults to `Unknown`), surfaced through a `MemberProbe{NoRib, Rib{saw_hidden,saw_unknown,saw_visible}}` from `resolve_rib_probed`, consumed in both glob member arms (`engine.rs`). Inert tasks first (hook/probe/signature byte-preserving), then the single behavior task, then telemetry clean-replace + cache bump.

**Tech Stack:** Rust, the prism `name_resolution` engine + `rust_policy`, `glob_stats` telemetry, `cargo test`.

**Design-of-record:** `docs/superpowers/specs/2026-06-22-member-visibility-tristate-design.md` (rev 2, codex-reviewed). Read §2 (soundness), §3 (architecture), §6 (tests).

---

## Standing constraints (every task)

- **TDD:** write the failing test, watch it fail for the right reason, minimal code to green, verify, commit. No production code without a failing test first.
- **Build green per task:** each task compiles and all named suites pass before commit. Tasks 1–5 + (6 plumbing) are **behavior-preserving** — the *only* behavior change is Task 6's deferred-arm recovery and Task 7's already-resolved Unknown→poison.
- **Test commands (macOS):** `cargo test --lib`, `cargo test --test name_resolution`, `cargo test --test integration`, `cargo test --test ast`. **Run `--test ast` before declaring green** (CPG consumer). Bare `cargo test --test cli` stalls — skip it. `cargo test` takes ONE name filter before `--`.
- **Shared working tree:** `git add <explicit files>`, **never `-a`**. Verify `git diff --cached --name-only` before each commit.
- **Commit trailer:** end every commit message with
  `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.
- **Never** stage `eval/` or `docs/eval/` artifacts.

## File Structure

- `src/name_resolution/types.rs` — rename `GlobEdgeVis`→`VisibilityDecision`; add the `member_visible` trait hook (default).
- `src/name_resolution/rust_policy.rs` — rename refs; add the Rust `member_visible` override.
- `src/name_resolution/engine.rs` — `VisibilityDecision` refs; `MemberProbe` + `resolve_rib_probed` + `resolve_rib` wrapper; `scope_member_lookup_probed` returns the probe; `glob_lookup_inner`/`glob_lookup` wrapper; the two glob member arms.
- `src/name_resolution/glob_stats.rs` — telemetry: add 6 counters (Task 5), remove `ambiguous` (Task 8).
- `src/navigation/queries.rs` — `glob_expand` JSON keys (Task 8).
- `src/cpg_cache.rs` — `CACHE_VERSION` 19→20 + version test (Task 9).
- `tests/name_resolution/glob_expand_test.rs` — behavior fixtures + assert remaps.
- `tests/integration/resolution_test.rs` — histogram-shape assert remap (Task 8).

---

### Task 1: Rename `GlobEdgeVis` → `VisibilityDecision`

**Files:** Modify `src/name_resolution/types.rs:436` (def) + `:582-589` (default), `src/name_resolution/rust_policy.rs:246-257`, `src/name_resolution/engine.rs:383-389`, and any test refs.

Pure mechanical rename, no behavior change — the gate is the existing suite staying green.

- [ ] **Step 1: Find all occurrences**

Run: `rg -n 'GlobEdgeVis' src/ tests/`
Expected: the def, the trait default (`-> GlobEdgeVis`), the Rust impl (`GlobEdgeVis::Visible/Hidden/Unknown`), the engine match arms, plus any test references.

- [ ] **Step 2: Rename every occurrence** `GlobEdgeVis` → `VisibilityDecision` across `src/` and `tests/` (identifier is unambiguous — type name + 3 variants). The enum at `types.rs:436` becomes:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityDecision {
    Visible,
    Hidden,
    Unknown,
}
```

- [ ] **Step 3: Build + existing tests green**

Run: `cargo build && cargo test --lib && cargo test --test name_resolution && cargo test --test integration && cargo test --test ast`
Expected: PASS, zero behavior change (the rename is total).

- [ ] **Step 4: Commit**

```bash
git add src/name_resolution/types.rs src/name_resolution/rust_policy.rs src/name_resolution/engine.rs tests/
git commit -m "refactor(name-res): rename GlobEdgeVis -> VisibilityDecision (shared member/edge tri-state)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 2: `member_visible` policy hook (read-inert)

**Files:** Modify `src/name_resolution/types.rs` (trait, after `glob_edge_visible` default ~`:589`), `src/name_resolution/rust_policy.rs` (after the Rust `glob_edge_visible` ~`:257`). Test: `src/name_resolution/rust_policy.rs` test module (mirror `vis_reaches_matches_visible_and_preserves_unknown_pub_in` at `:579`).

The hook exists but is not consumed yet — fully inert.

- [ ] **Step 1: Write the failing test** (mirror the existing `vis_reaches` test's binding construction). Assert the three mappings:

```rust
#[test]
fn member_visible_maps_vis_reaches_tristate() {
    // Reuse the same scope-graph + policy fixture as
    // `vis_reaches_matches_visible_and_preserves_unknown_pub_in`.
    // A `pub` binding viewed from anywhere -> Visible.
    assert_eq!(policy.member_visible(&pub_binding, &q_outside, &trav), VisibilityDecision::Visible);
    // A private binding viewed from OUTSIDE its module -> Hidden (vis_reaches == Some(false)).
    assert_eq!(policy.member_visible(&priv_binding, &q_outside, &trav), VisibilityDecision::Hidden);
    // A `pub(in <unresolved>)` binding -> Unknown (vis_reaches == None).
    assert_eq!(policy.member_visible(&pub_in_unresolved, &q_outside, &trav), VisibilityDecision::Unknown);
}
```

- [ ] **Step 2: Run — verify it fails** (method not defined)

Run: `cargo test --lib member_visible_maps_vis_reaches_tristate`
Expected: FAIL — `no method named member_visible`.

- [ ] **Step 3: Add the trait default** (`types.rs`, conservative — non-Rust hidden → Unknown):

```rust
/// Classify a member binding's visibility from the query vantage as a tri-state.
/// Mirrors `glob_edge_visible` at the member level: `Hidden` = proved not-visible
/// (a glob soundly does not re-export it -> safe to continue past); `Unknown` =
/// undecidable (must fail closed -> poison); `Visible` = contributes.
///
/// Default is intentionally conservative: a policy that only knows boolean
/// `visible == false` returns `Unknown`, so glob expansion keeps poisoning rather
/// than silently skipping a member it cannot prove hidden.
fn member_visible(&self, binding: &Binding, q: &ResolveQuery, trav: &TraversalCtx) -> VisibilityDecision {
    if self.visible(binding, q, trav) {
        VisibilityDecision::Visible
    } else {
        VisibilityDecision::Unknown
    }
}
```

- [ ] **Step 4: Add the Rust override** (`rust_policy.rs`, same vantage as `visible`):

```rust
fn member_visible(&self, binding: &Binding, q: &ResolveQuery, _trav: &TraversalCtx) -> VisibilityDecision {
    match self.vis_reaches(&binding.vis, binding.scope, q.from) {
        Some(true) => VisibilityDecision::Visible,
        Some(false) => VisibilityDecision::Hidden,
        None => VisibilityDecision::Unknown,
    }
}
```

- [ ] **Step 5: Run — green; full suites green** (inert)

Run: `cargo test --lib member_visible_maps_vis_reaches_tristate && cargo test --lib && cargo test --test name_resolution`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/name_resolution/types.rs src/name_resolution/rust_policy.rs
git commit -m "feat(name-res): member_visible tri-state policy hook (read-inert)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 3: `MemberProbe` + `resolve_rib_probed` (read-inert)

**Files:** Modify `src/name_resolution/engine.rs` (`MemberProbe` near `resolve_rib`; `resolve_rib_probed` from the current `resolve_rib` body `:265-339`; `resolve_rib` becomes a wrapper). Test: `engine.rs` test module + a `name_resolution` integration assertion via the probe (kept internal — exercised through Task 4's consumer; here a unit test on the helpers + a probe smoke test).

Behavior-preserving: `resolve_rib` (wrapper) returns the identical `Resolution`; the probe is new data, not yet consumed.

- [ ] **Step 1: Write failing unit tests for the helpers**

```rust
#[test]
fn member_probe_all_known_hidden_predicate() {
    assert!(MemberProbe::Rib { saw_hidden: true, saw_unknown: false, saw_visible: false }.all_known_hidden());
    assert!(!MemberProbe::Rib { saw_hidden: true, saw_unknown: true, saw_visible: false }.all_known_hidden());
    assert!(!MemberProbe::Rib { saw_hidden: false, saw_unknown: false, saw_visible: true }.all_known_hidden());
    assert!(!MemberProbe::NoRib.rib_present());
    assert!(MemberProbe::Rib { saw_hidden: true, saw_unknown: false, saw_visible: false }.rib_present());
}
```

- [ ] **Step 2: Run — verify it fails** (type not defined)

Run: `cargo test --lib member_probe_all_known_hidden_predicate`
Expected: FAIL — `cannot find type MemberProbe`.

- [ ] **Step 3: Add `MemberProbe` + helpers**

```rust
enum MemberProbe {
    NoRib,
    Rib { saw_hidden: bool, saw_unknown: bool, saw_visible: bool },
}
impl MemberProbe {
    fn rib_present(&self) -> bool { matches!(self, MemberProbe::Rib { .. }) }
    fn has_unknown(&self) -> bool { matches!(self, MemberProbe::Rib { saw_unknown: true, .. }) }
    /// Encodes the cardinal rule directly: continue only if some binding was proved
    /// Hidden, none Unknown, AND none Visible. `saw_visible` is belt-and-suspenders —
    /// a Visible binding cannot leave the rib Unresolved today; gating on it future-proofs
    /// the predicate against any change to `resolve_rib`'s contribution logic.
    fn all_known_hidden(&self) -> bool {
        matches!(self, MemberProbe::Rib { saw_hidden: true, saw_unknown: false, saw_visible: false })
    }
}
```

- [ ] **Step 4: Convert `resolve_rib` body to `resolve_rib_probed`** — replace the `if !policy.visible(b, q, &trav) { continue; }` at `:281` with the tri-state, accumulating flags; return `(Resolution, MemberProbe::Rib { .. })`:

```rust
fn resolve_rib_probed(
    graph: &ScopeGraph, rib: &[usize], q: &ResolveQuery,
    policy: &dyn ResolutionPolicy, guard: &mut CycleGuard<'_>,
) -> (Resolution, MemberProbe) {
    let mut candidates: Vec<Candidate> = Vec::new();
    let (mut saw_hidden, mut saw_unknown, mut saw_visible) = (false, false, false);
    for &bidx in rib {
        let b = &graph.bindings[bidx];
        let trav = TraversalCtx { lookup_scope: Some(b.scope), via_glob: false, edge_kind: None };
        match policy.member_visible(b, q, &trav) {
            VisibilityDecision::Visible => { saw_visible = true; }
            VisibilityDecision::Hidden  => { saw_hidden  = true; continue; }
            VisibilityDecision::Unknown => { saw_unknown = true; continue; }
        }
        match &b.target {
            BindTarget::Resolved(t) => { /* unchanged: push candidate */ }
            BindTarget::Pending(path, anchor) => {
                // unchanged Pending-chase; early `return poisoned()` paths become
                // `return (poisoned(), MemberProbe::Rib { saw_hidden, saw_unknown, saw_visible })`.
            }
        }
    }
    let res = if candidates.is_empty() { unresolved() } else { policy.combine(candidates) };
    debug_assert!(!(saw_visible && matches!(res.status, ResStatus::Unresolved)),
        "a visible member binding cannot leave the rib Unresolved (cardinal-rule invariant)");
    (res, MemberProbe::Rib { saw_hidden, saw_unknown, saw_visible })
}

/// Thin wrapper for the direct-rib callers (`resolve_bare`): identical Resolution.
fn resolve_rib(
    graph: &ScopeGraph, rib: &[usize], q: &ResolveQuery,
    policy: &dyn ResolutionPolicy, guard: &mut CycleGuard<'_>,
) -> Resolution {
    resolve_rib_probed(graph, rib, q, policy, guard).0
}
```

(Keep the existing `BindTarget::Resolved`/`Pending` bodies verbatim — only the visibility filter and the return change. The two early `return poisoned()` sites inside the Pending arm return the tuple.)

- [ ] **Step 5: Run — green; full suites green** (wrapper preserves behavior)

Run: `cargo test --lib && cargo test --test name_resolution && cargo test --test integration`
Expected: PASS — `resolve_bare`/path behavior unchanged.

- [ ] **Step 6: Commit**

```bash
git add src/name_resolution/engine.rs
git commit -m "feat(name-res): MemberProbe + resolve_rib_probed (read-inert; resolve_rib wrapper)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 4: `scope_member_lookup_probed` returns `MemberProbe` (byte-identical)

**Files:** Modify `src/name_resolution/engine.rs` (`scope_member_lookup_probed` `:605-642` return type; `resolve_path_guarded` `:551,560` consumer; the deferred glob member arm `:431` call site — temporarily bind the probe but keep using `probe.rib_present()` to preserve the existing `member_rib_present` behavior).

Signature change only — every consumer's behavior is byte-identical (only `rib_present()` is read).

- [ ] **Step 1: Change the return type** to `(Resolution, MemberProbe)`:

```rust
fn scope_member_lookup_probed(...) -> (Resolution, MemberProbe) {
    let rib: Vec<usize> = /* unchanged */;
    if !rib.is_empty() {
        return resolve_rib_probed(graph, &rib, q, policy, guard); // returns (Resolution, Rib{..})
    }
    if macro_wildcard_poisons(graph, scope, q.ns, &q.at) {
        return (poisoned(), MemberProbe::NoRib);
    }
    let res = match glob_lookup(graph, scope, q, policy, guard) {
        GlobOutcome::Poison => poisoned(),
        GlobOutcome::Hit(cands) => policy.combine(cands),
        GlobOutcome::Empty => unresolved(),
    };
    (res, MemberProbe::NoRib)
}
```

- [ ] **Step 2: Update `resolve_path_guarded`** (`:551,560`): bind `(res, probe)` and use `probe.rib_present()` where the old `rib_present` bool was (`:560` `&& !probe.rib_present()`). No other change.

- [ ] **Step 3: Update the deferred-arm call site** (`:431`): `let (member_res, probe) = scope_member_lookup_probed(...)` and keep the existing match using `!probe.rib_present()` in place of `!member_rib_present` (the all_known_hidden split lands in Task 6 — here the arm is unchanged behavior).

- [ ] **Step 4: Run — green; path + extern-crate suites green** (byte-identical)

Run: `cargo test --lib && cargo test --test name_resolution && cargo test --test integration && cargo test --test ast`
Expected: PASS — crate-root fallback + claimed-rib shadowing unchanged.

- [ ] **Step 5: Commit**

```bash
git add src/name_resolution/engine.rs
git commit -m "refactor(name-res): scope_member_lookup_probed returns MemberProbe (byte-identical)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 5: Telemetry — add the 6 counters (additive; keep `ambiguous`)

**Files:** Modify `src/name_resolution/glob_stats.rs` (`GlobExpandStats`, `GlobExpandSnapshot`, `record_*`, `reset`, `snapshot`, `GLOBAL`, the round-trip test).

Additive only — `ambiguous` stays until Task 8 so every intermediate task builds.

- [ ] **Step 1: Extend the round-trip test** (`record_reset_snapshot_roundtrip`) to record + assert the 6 new counters (`member_multi`, `member_undecidable`, `member_hidden_continued`, `member_hidden_continue_hit`, `_empty`, `_poison`).

- [ ] **Step 2: Run — verify it fails** (methods/fields missing)

Run: `cargo test --lib record_reset_snapshot_roundtrip`
Expected: FAIL — `no method named record_member_multi`.

- [ ] **Step 3: Add the 6 `AtomicUsize` fields** to `GlobExpandStats` + `GlobExpandSnapshot`, the `record_member_multi` / `record_member_undecidable` / `record_member_hidden_continued` / `record_member_hidden_continue_hit` / `_empty` / `_poison` methods (each `fetch_add(1, Relaxed)`), wire them into `reset` (the array), `snapshot`, and the `GLOBAL` const literal (`z()` initializers). The counter doc-comment for the three `member_hidden_continue_*` states: **"same-invocation opportunity telemetry, NOT a buy bound (combine may fold to ResolvedSet/Ambiguous; nested hits swallowed by outer poison); the final buy is the kind_exact delta."**

- [ ] **Step 4: Run — green**

Run: `cargo test --lib record_reset_snapshot_roundtrip && cargo test --lib`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/name_resolution/glob_stats.rs
git commit -m "feat(name-res): add member-visibility tri-state glob_stats counters (additive)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 6: Deferred glob member arm — the behavior change (core)

**Files:** Modify `src/name_resolution/engine.rs` (`glob_lookup` → `glob_lookup_inner` + wrapper; widen the `with_glob` closure to `(GlobOutcome, bool)`; the member-arm 3-way split). Test: `tests/name_resolution/glob_expand_test.rs`.

This is the soundness-sensitive core. Continuing past an all-`Hidden` member rib lets a sibling glob resolve; any `Unknown` still poisons.

- [ ] **Step 1: Write the headline RED test** (known-hidden sibling recovery):

```rust
#[test]
fn glob_expand_known_hidden_member_continues_to_public_sibling() {
    let src = "mod ta { struct S; }\nmod tb { pub struct S; }\npub use ta::*;\npub use tb::*;\nfn f(){ let _: Option<S>; }\n";
    let (res, snap, _, _) = single_file_resolve(src, "S>;", "S");
    assert_resolved_item(&res); // resolves to tb::S
    assert_eq!(snap.member_hidden_continued, 1);
    assert_eq!(snap.member_undecidable, 0);
    assert_eq!(snap.member_hidden_continue_hit, 1);
    assert_eq!(snap.resolved_l1, 1);
}
```

- [ ] **Step 2: Run — verify it fails** (today `ta::S` private → poison, not continue)

Run: `cargo test --test name_resolution glob_expand_known_hidden_member_continues_to_public_sibling`
Expected: FAIL — `res.status` is `Poisoned`, not `Resolved`.

- [ ] **Step 3: Split `glob_lookup` into wrapper + inner** (§3.7). The wrapper owns `saw_hidden_continue` and classifies once:

```rust
fn glob_lookup(graph, scope_id, q, policy, guard) -> GlobOutcome {
    let mut saw_hidden_continue = false;
    let outcome = glob_lookup_inner(graph, scope_id, q, policy, guard, &mut saw_hidden_continue);
    if saw_hidden_continue {
        match &outcome {
            GlobOutcome::Hit(_) => guard.stats().record_member_hidden_continue_hit(),
            GlobOutcome::Empty  => guard.stats().record_member_hidden_continue_empty(),
            GlobOutcome::Poison => guard.stats().record_member_hidden_continue_poison(),
        }
    }
    outcome
}
```

`glob_lookup_inner` is the current `glob_lookup` body + a `saw_hidden_continue: &mut bool` param.

- [ ] **Step 4: Widen the `with_glob` closure to `(GlobOutcome, bool)`** and implement the member-arm split. Every closure exit returns a tuple; the cycle/target-resolution-failure exits return `(GlobOutcome::Poison, false)`. The member match (replacing `:433-466`):

```rust
let (member_res, probe) = scope_member_lookup_probed(graph, target_scope, q, policy, guard);
match member_res.status {
    ResStatus::Resolved if member_res.candidates.len() == 1 => {
        let mut member_candidates = member_res.candidates;
        let mc = member_candidates.pop().expect("checked len == 1");
        candidates.push(Candidate {
            target: mc.target,
            cond: conjoin(&cond_of(&e.cond), &conjoin(&target_cond, &mc.cond)),
            provenance: Default::default(),
        });
        guard.stats().record_resolved(guard.glob_depth());
        (GlobOutcome::Empty, false)
    }
    ResStatus::Resolved | ResStatus::ResolvedSet | ResStatus::Ambiguous => {
        guard.stats().record_member_multi();
        (GlobOutcome::Poison, false)
    }
    ResStatus::Poisoned => (GlobOutcome::Poison, false),
    ResStatus::Unresolved if !probe.rib_present() => (GlobOutcome::Empty, false), // provably absent
    ResStatus::Unresolved if probe.all_known_hidden() => {                        // all Hidden, none Unknown
        guard.stats().record_member_hidden_continued();
        (GlobOutcome::Empty, true)
    }
    ResStatus::Unresolved => {                                                    // some Unknown -> fail closed
        guard.stats().record_member_undecidable();
        (GlobOutcome::Poison, false)
    }
}
```

Then at the call site (replacing `:467-470`):

```rust
let (edge_outcome, hc) = guard.with_glob(edge_idx, |guard, entered| { ... });
if hc { saw_hidden_continue = true; }                 // OR before the poison short-circuit
if matches!(edge_outcome, GlobOutcome::Poison) { return GlobOutcome::Poison; }
```

- [ ] **Step 5: Run the headline test — green**

Run: `cargo test --test name_resolution glob_expand_known_hidden_member_continues_to_public_sibling`
Expected: PASS — resolves to `tb::S`, `member_hidden_continued == 1`, `member_hidden_continue_hit == 1`.

- [ ] **Step 6: Add the undecidable + mixed RED→GREEN tests**

```rust
#[test]
fn glob_expand_undecidable_member_still_poisons() {
    // ta::S is pub(in <unresolved>) -> Unknown; must poison, not continue to tb::S.
    let src = "mod ta { pub(in crate::ghost) struct S; }\nmod tb { pub struct S; }\npub use ta::*;\npub use tb::*;\nfn f(){ let _: Option<S>; }\n";
    let (res, snap, _, _) = single_file_resolve(src, "S>;", "S");
    assert_eq!(res.status, ResStatus::Poisoned);
    assert_eq!(snap.member_undecidable, 1);
    assert_eq!(snap.member_hidden_continued, 0);
}

#[test]
fn glob_expand_mixed_hidden_and_unknown_rib_poisons() {
    // ta has a private S AND a pub(in <unresolved>) S (cfg alternatives, both cfg-compatible).
    // Any Unknown in the rib -> poison even with a Hidden sibling.
    let src = "mod ta { #[cfg(feature=\"x\")] struct S; #[cfg(feature=\"y\")] pub(in crate::ghost) struct S; }\nmod tb { pub struct S; }\npub use ta::*;\npub use tb::*;\nfn f(){ let _: Option<S>; }\n";
    let (res, snap, _, _) = single_file_resolve(src, "S>;", "S");
    assert_eq!(res.status, ResStatus::Poisoned);
    assert_eq!(snap.member_undecidable, 1);
    assert_eq!(snap.member_hidden_continued, 0);
}
```

**Note (cfg assumption):** if the mixed fixture's two cfg-gated `S` bindings are not both in the rib under default `CfgCtx`, adjust to a representation that yields one rib with a `Hidden` + an `Unknown` binding (e.g. distinct items), confirming with a RED run first.

- [ ] **Step 7: Remap the behavior-flipped pre-existing asserts** (in THIS task — the deferred-arm change is what flips them; the `ambiguous` *field* still exists, it is just no longer written by this arm):

  - `glob_expand_filtered_member_rib_does_not_fall_through` (`:341/:359`): status stays `Poisoned`; replace `assert!(snap.ambiguous >= 1)` with `assert_eq!(snap.member_undecidable, 1)`.
  - `glob_expand_respects_member_visibility` (`:248`): the `Hidden`-no-sibling lookup now yields `Unresolved` (was `Poisoned`); pin `member_hidden_continued == 1` + `member_hidden_continue_empty == 1` + `resolved_l1 == 0` (both = no edge minted).
  - Any other `snap.ambiguous` assert the suite surfaces (e.g. `:194`/`:203`) → the bucket the arm now routes to (`member_multi` for a single-target multi result; `member_undecidable` for a filtered-rib poison).

Run: `cargo test --test name_resolution glob_expand && cargo test --test integration && cargo test --test ast`
Expected: PASS — every flipped assert remapped here; only the `ambiguous` *field/key* removal is deferred to Task 8.

- [ ] **Step 8: Commit**

```bash
git add src/name_resolution/engine.rs tests/name_resolution/glob_expand_test.rs
git commit -m "feat(name-res): deferred glob arm continues past all-hidden member ribs

Member-visibility tri-state: continue when every filtered binding is proved
Hidden (none Unknown), else poison. glob_lookup split into inner+wrapper for
per-invocation continue-outcome counters. The recall recovery + the
undecidable/mixed fail-closed cases.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 7: Already-resolved glob member arm — `Unknown`→poison (soundness completion)

**Files:** Modify `src/name_resolution/engine.rs` (the `BindTarget::Resolved(Target::Scope)` arm `:472-501`, the `if !policy.visible(...)` at `:482`). Test: `tests/name_resolution/glob_expand_test.rs`.

Representation-independence: this arm currently blanket-skips invisible members (incl. `Unknown`). Tighten so `Unknown` poisons. Soundness-monotonic (only adds poison).

- [ ] **Step 1: Write the RED test** — a non-deferred glob whose resolved target scope has an `Unknown` member must poison (construct a fixture that reaches the `BindTarget::Resolved(Target::Scope)` arm; if Rust always defers module globs, use an intra-scope already-resolved glob target per the existing `glob_expand_diamond`/`distinct_targets` setups).

```rust
#[test]
fn glob_expand_already_resolved_arm_unknown_member_poisons() {
    // A resolved-scope glob target with a pub(in <unresolved>) member -> Unknown -> poison.
    // (If unreachable for Rust module globs, this documents the representation-independent
    // invariant via the closest constructible fixture; verify reachability with a RED run.)
    let src = /* fixture reaching the already-resolved arm with an Unknown member */;
    let (res, _snap, _, _) = single_file_resolve(src, "S>;", "S");
    assert_eq!(res.status, ResStatus::Poisoned);
}
```

- [ ] **Step 2: Run — verify it fails** (today the arm continues past the Unknown member)

Run: `cargo test --test name_resolution glob_expand_already_resolved_arm_unknown_member_poisons`
Expected: FAIL — resolves/continues instead of poisoning. (If the arm is genuinely unreachable for Rust, document that the change is inert and the test asserts the closest reachable behavior; do not force an artificial path.)

- [ ] **Step 3: Apply the tri-state** at `:482`:

```rust
match policy.member_visible(b, q, &trav) {
    VisibilityDecision::Visible => { /* fall through to the existing push */ }
    VisibilityDecision::Hidden  => continue, // pre-existing skip; NOT a new-recall continue
    VisibilityDecision::Unknown => { guard.stats().record_member_undecidable(); return GlobOutcome::Poison; }
}
```

(Do **not** set `saw_hidden_continue` here — this arm's Hidden-skip is pre-existing, not new recall.)

- [ ] **Step 4: Run — green; full suites green**

Run: `cargo test --test name_resolution glob_expand && cargo test --test integration && cargo test --test ast`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/name_resolution/engine.rs tests/name_resolution/glob_expand_test.rs
git commit -m "feat(name-res): already-resolved glob arm poisons on Unknown member (soundness completion)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 8: Telemetry clean replace — remove `ambiguous`, remap all asserts

**Files:** Modify `src/name_resolution/glob_stats.rs` (remove `ambiguous` field/record/snapshot/reset), `src/navigation/queries.rs:304` (JSON), `tests/integration/resolution_test.rs:1774-1783` (histogram shape), `tests/name_resolution/glob_expand_test.rs:194,203,248,341,359` (asserts).

- [ ] **Step 1: Update the field-shape tests** (the behavior asserts were already remapped in T6/T7 — this is only the tests that enumerate the counter *keys*): the `glob_stats.rs` round-trip test (drop the `ambiguous` record + assert) and the histogram-shape test at `tests/integration/resolution_test.rs:1774-1783` (drop `"ambiguous"`, add the 6 keys). Confirm via `rg` that **no behavior assert still reads `snap.ambiguous`** before removing the field.

- [ ] **Step 2: Remove `ambiguous`** from `GlobExpandStats`, `GlobExpandSnapshot`, `record_ambiguous`, `reset`, `snapshot`, `GLOBAL`, and the round-trip test; drop `"ambiguous"` from the `queries.rs` JSON and add the 6 keys.

Run: `rg -n 'ambiguous' src/name_resolution/glob_stats.rs src/navigation/queries.rs tests/`
Expected: zero remaining `glob_stats`/`glob_expand` `ambiguous` references (the `ResStatus::Ambiguous` enum + `policy.combine` ambiguity are unrelated and stay).

- [ ] **Step 3: Run — green**

Run: `cargo test --lib && cargo test --test name_resolution && cargo test --test integration && cargo test --test ast`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/name_resolution/glob_stats.rs src/navigation/queries.rs tests/integration/resolution_test.rs tests/name_resolution/glob_expand_test.rs
git commit -m "feat(nav): clean-replace glob_expand ambiguous with member tri-state buckets

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 9: `CACHE_VERSION` 19 → 20

**Files:** Modify `src/cpg_cache.rs:64` (const + comment), `:571` (version test).

- [ ] **Step 1: Update the version test** `cache_version_is_19_for_glob_re_export_member_expansion` → assert `20`, rename to `..._is_20_for_member_visibility_tristate`.

Run: `cargo test --lib cache_version`
Expected: FAIL — asserts 19.

- [ ] **Step 2: Bump the const** to `20` with comment `// 20: member-visibility tri-state (glob member-rib continuation — resolution behavior change).`

- [ ] **Step 3: Run — green**

Run: `cargo test --lib cache_version && cargo test --lib`
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src/cpg_cache.rs
git commit -m "chore(cache): CACHE_VERSION 19->20 (member-visibility tri-state behavior change)

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>"
```

---

### Task 10: Acceptance (no new code — the gate)

**Files:** none (verification only; never stage `eval/`/`docs/eval/` artifacts).

- [ ] **Step 1: Full suite + format**

Run: `cargo fmt && cargo build --release && cargo test --lib && cargo test --test name_resolution && cargo test --test integration && cargo test --test ast`
Expected: PASS, clean fmt.

- [ ] **Step 2: Canary + bucket split on ruff + ripgrep** (NO `timeout` — ruff call-stats > 5 min)

Run: `./target/release/prism nav --no-cache call-stats --repo /Users/wesleyjinks/code/bench-repos/ruff` and `.../ripgrep`
Expected: **`multi_target_exact_sites` byte-flat** vs `main` (the wrong-singleton gate); `glob_expand` shows the split (`member_undecidable`, `member_hidden_continued`, `member_hidden_continue_{hit,empty,poison}`, `member_multi`, no `ambiguous`); report the `kind_exact` delta as the realized buy.

- [ ] **Step 3: Tier-A matrix + ruff M2**

Run: `cd eval && uv run tier-a --matrix-only --allow-stale-sut` then `cd eval && uv run tier-a --corpus ruff --allow-stale-sut`
Expected: matrix **0-regr**; ruff M2 (`docs/eval/tier-a/<date>-ruff.{json,md}`) `baseline_invalid == false`, `shortfall == 0`.

- [ ] **Step 4: Record the call-stats deltas** (canary flat, kind_exact buy, bucket split, already-resolved-arm inertness) in the PR description — do not re-baseline.

---

## Self-Review

- **Spec coverage:** §3.1 hook → T2; §3.2 probe → T3; §3.3 signature → T4; §3.4 deferred arm → T6; §3.5 direct-rib invariant → T3/T4 (wrapper + rib_present, existing suites green); §3.6 already-resolved arm → T7; §3.7 counters → T5+T6; §4 forward-compat → no code (documented); §5 telemetry → T5+T8; §6 tests → T6/T7/T8; cache → T9; acceptance → T10. ✓
- **Type consistency:** `VisibilityDecision` (not `GlobEdgeVis`/`MemberVis`) throughout; `MemberProbe::Rib{saw_hidden,saw_unknown,saw_visible}`; `all_known_hidden()` matches the cardinal rule; `with_glob` closure returns `(GlobOutcome, bool)` consistently across T6.
- **Inert/behavior split:** T1–T5 + T6-plumbing behavior-preserving; T6 deferred-arm + T7 already-resolved-arm are the only behavior changes — each gated by a RED→GREEN fixture and the canary.
- **Build-green:** `ambiguous` retained additively (T5) until T8 so every task compiles; `record_ambiguous` callers migrated in T6 before removal in T8.
