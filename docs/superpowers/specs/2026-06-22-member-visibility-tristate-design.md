# Member-Visibility Tri-State — Design Spec (design-of-record) — 2026-06-22

> **Status:** design-of-record for the member-visibility tri-state slice — the §9 follow-on to
> deferred-glob member expansion (**#126, merged** to `main` `bad55a6`). Supersedes the codex
> architecture seed `docs/superpowers/specs/2026-06-22-member-visibility-tristate-analysis.md`
> (which it folds in). Pipeline: this spec → codex xhigh spec-review → plan → implement.
> Resume/constraints: `docs/superpowers/handoffs/2026-06-22-member-visibility-tristate-handoff.md`.
> Arc log: `memory/project_prism_owner_key_collision.md`.
>
> **Owner decisions folded (2026-06-22):** scope = **tri-state only, defer `pub(in)` restrict
> resolution** (with a forward-compatibility proof, §4); buy-sizing counters = **permanent** (§5);
> `ambiguous` bucket = **clean replace**, no alias (§5).
>
> **Rev 2 (2026-06-22):** codex xhigh spec-review folded — **no BLOCKER**; the cardinal soundness rule
> (§2) and architecture confirmed; 5 accuracy/completeness findings folded (see §9).

---

## 1. Problem & opportunity

#126 made `glob_lookup`'s deferred-glob member arm **poison** on a *claimed-but-visibility-filtered*
member rib. When a facade `pub use mod::*` member lookup finds a rib for the name in the target but
every candidate fails `policy.visible` (`resolve_rib` returns `Unresolved` with `rib_present == true`),
the arm at `src/name_resolution/engine.rs:462-465` does `record_ambiguous()` + `GlobOutcome::Poison`.

That is **recall-safe but conservative** — it blanket-poisons two distinct cases:

- a **known-hidden** member — a `private` / cross-crate `pub(crate)` / `pub(super)` item the glob
  soundly does **not** re-export (globs bring only items visible at the query vantage). The lookup
  could safely **continue** to a sibling glob / outer scope; and
- an **undecidable** `pub(in <path>)`-no-restrict member — `vis_reaches` returns `None` because
  `resolve_restrict` is a Phase-1 stub (`rust_populator/walk/mod.rs:271`). This **must** keep poisoning
  (cannot prove not-visible).

`visible()` (`rust_policy.rs:241`) collapses both into `false` via `vis_reaches(..).unwrap_or(false)`,
so the engine cannot tell them apart today. The information already exists: `vis_reaches`
(`rust_policy.rs:146`) returns `Some(true)` / `Some(false)` / `None`.

**The lever (measured).** A throwaway spike on ruff split the `glob_expand.ambiguous` bucket
(14,665): **14,663 (99.99%)** are the claimed-but-filtered member case; **2** are genuinely
multiply-defined members. The conservative blanket-poison is the dominant facade-resolution outcome.

**Caveat — target ≠ buy (the arc's repeated over-estimation lesson: +428→+38, +1586→+0).** 14,663 is
the **target population, not the realized recall buy.** The gain is only the subset where *continuing*
past a known-hidden member reaches a clean resolution. Many of the 14,663 are `pub(crate)`
cross-vantage names that resolve nowhere else → continue→unresolved, no edge minted. §5's permanent
continue→outcome counters size the real buy; **no recovered-edge number is claimed in this spec.**

## 2. Cardinal soundness — the rule

Under the §7 invariant *resolve-or-fall-through, NEVER a wrong target* (prism may miss an edge but must
never mint a wrong singleton):

> **For a glob's member lookup whose target scope is resolved, continue past a claimed-but-filtered
> member rib only when every explicit binding in that rib is classified `Hidden`, and none is
> `Unknown`. Otherwise poison.**

### 2.1 Case walk (Rust `vis_reaches`, `rust_policy.rs:151-164`)

- **Private** (`VIS_PRIV`): visible only when `q.from` is inside the defining module subtree
  (`:162`). From outside, `Some(false)` → `Hidden`: the glob does not bring that member to this
  vantage → continuing is sound.
- **`pub(crate)` cross-crate** (`VIS_PUB_CRATE`, `:153`): different crate roots → `Some(false)` →
  `Hidden`. The member is not in this consumer's visible import set; a sibling public glob is the only
  visible same-name → continuing is sound.
- **`pub(super)` / resolved `pub(in)` from outside** (`:157`, `:161`): `Some(false)` → `Hidden` is a
  proof of absence from the glob's visible member set → continuing is sound.
- **Undecidable `pub(in)`-no-restrict** (`:161`, `vis.restrict == None` → `None`): cannot prove
  not-visible → `Unknown` → **poison** (fail closed). Common today because `resolve_restrict` is a
  stub and walkers store `restrict: None` (`walk/types.rs:38,48`).

### 2.2 No-wrong-singleton proof

Continuing past a filtered member could let a *sibling* glob mint a wrong singleton **only if** a
member that is actually visible were misclassified `Hidden`. Under the rule, `Hidden` is assigned
**only** on `vis_reaches == Some(false)` — the policy has *proved* non-accessibility for this `q.from`.
A proved-hidden member is not in the glob's visible contribution set, so a later sibling singleton is
not wrong. `Unknown` **never** continues, so the #126 BLOCKER hole stays closed (the existing
`pub(in crate::ghost)` regression at `glob_expand_test.rs:341` still poisons).

The rule changes **only glob-membership** behavior. A hidden member means "absent from this glob's
contribution"; a **direct** rib claim (bare-name step-1 / path-segment lookup) remains authoritative
and is **not** touched (§3.5) — prism never skips an authoritative direct rib to an outer same-name.

## 3. Architecture

Recommended mechanism = codex option **(b): a richer member probe, backed by a policy tri-state hook.**
A hook alone is insufficient (`Resolution` loses the aggregate reason for `Unresolved`);
re-inspecting Rust `Vis` inside the engine would violate the engine's language-neutral rule
(`engine.rs:1`).

### 3.1 Shared `VisibilityDecision` enum + `member_visible` policy hook

Rename #126's `GlobEdgeVis` (`types.rs:436`) → a shared **`VisibilityDecision { Visible, Hidden,
Unknown }`** reused by both `glob_edge_visible` and the new `member_visible` (same 3-valued result;
the two hooks stay distinct functions with **distinct vantages** — edge uses `edge.from`, member uses
`binding.scope → q.from`, §7). Update all use sites (`types.rs` enum + `glob_edge_visible` default
return; `rust_policy.rs` `glob_edge_visible` impl; `engine.rs` glob-edge match; tests).

Add to the `ResolutionPolicy` trait (`types.rs:551`, beside `visible`/`glob_edge_visible`):

```rust
/// Classify a member binding's visibility from the query vantage as a tri-state.
/// Mirrors `glob_edge_visible` at the member level: `Hidden` = proved not-visible
/// (a glob soundly does not re-export it → safe to continue past); `Unknown` =
/// undecidable (must fail closed → poison); `Visible` = contributes.
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

Rust override (`rust_policy.rs`, beside `glob_edge_visible`) — **same vantage as `visible`**:

```rust
fn member_visible(&self, binding: &Binding, q: &ResolveQuery, _trav: &TraversalCtx) -> VisibilityDecision {
    match self.vis_reaches(&binding.vis, binding.scope, q.from) {
        Some(true) => VisibilityDecision::Visible,
        Some(false) => VisibilityDecision::Hidden,
        None => VisibilityDecision::Unknown,
    }
}
```

`visible() == true ⟺ member_visible() == Visible` (both derive from `vis_reaches`, `unwrap_or(false)`),
so the **contribute/skip decision is byte-preserved**; the hook only *adds* the Hidden/Unknown split.

### 3.2 `MemberProbe` + `resolve_rib_probed`

```rust
enum MemberProbe {
    NoRib,
    Rib { saw_hidden: bool, saw_unknown: bool, saw_visible: bool },
}
impl MemberProbe {
    fn rib_present(&self) -> bool { matches!(self, MemberProbe::Rib { .. }) }
    fn has_unknown(&self) -> bool { matches!(self, MemberProbe::Rib { saw_unknown: true, .. }) }
    // Encodes the cardinal rule DIRECTLY (MINOR 1): continue only if some binding was
    // proved Hidden, none Unknown, AND none Visible. `saw_visible` is belt-and-suspenders
    // — a Visible binding cannot leave the rib `Unresolved` today (it contributes a
    // candidate or poisons), so this is always true in the Unresolved arm; gating on it
    // future-proofs the predicate against any change to `resolve_rib`'s contribution logic.
    fn all_known_hidden(&self) -> bool {
        matches!(self, MemberProbe::Rib { saw_hidden: true, saw_unknown: false, saw_visible: false })
    }
}
```

Add a private `resolve_rib_probed(graph, rib, q, policy, guard) -> (Resolution, MemberProbe)` — the
current `resolve_rib` body (`engine.rs:265-339`) with the per-binding visibility filter at `:281`
replaced by the tri-state, accumulating flags:

```rust
match policy.member_visible(b, q, &trav) {
    VisibilityDecision::Visible => { saw_visible = true; /* fall through to the existing target match */ }
    VisibilityDecision::Hidden  => { saw_hidden  = true; continue; }
    VisibilityDecision::Unknown => { saw_unknown = true; continue; }
}
```

Callers pass a non-empty rib, so it always returns `MemberProbe::Rib { .. }` (early `return
poisoned()` paths — cycle / unresolved Pending — return `(poisoned(), Rib{ .. })`; the probe is
ignored on a `Poisoned` result). A `debug_assert!(!(saw_visible && res.status == Unresolved))`
encodes the cardinal-rule invariant directly (a visible binding cannot leave the rib unresolved).
Keep **`resolve_rib(...) -> Resolution`** as a thin wrapper (`resolve_rib_probed(...).0`) for the
existing direct-rib callers (`resolve_bare:215` and any others).

### 3.3 `scope_member_lookup_probed` returns the probe

Change `scope_member_lookup_probed` (`engine.rs:605`) return type `(Resolution, bool)` →
`(Resolution, MemberProbe)`: rib-present path returns `resolve_rib_probed(..)`; the glob/macro-tier
path returns `(res, MemberProbe::NoRib)`. **Both** callers update:

- `resolve_path_guarded` (`engine.rs:551,560`): uses `probe.rib_present()` in place of the old
  `rib_present` bool for the extern-crate-root fallback — **behavior byte-identical** (it never reads
  Hidden/Unknown; the Resolution is unchanged).
- the deferred glob member arm (`engine.rs:431`): consumes the full probe (§3.4).

### 3.4 Deferred glob member arm — the change

Replace `engine.rs:445-465`:

```rust
ResStatus::Resolved | ResStatus::ResolvedSet | ResStatus::Ambiguous => {   // was: record_ambiguous
    guard.stats().record_member_multi();
    (GlobOutcome::Poison, false)
}
ResStatus::Poisoned => (GlobOutcome::Poison, false),
ResStatus::Unresolved if !probe.rib_present() => (GlobOutcome::Empty, false), // provably absent
ResStatus::Unresolved if probe.all_known_hidden() => {                        // all Hidden, none Unknown
    guard.stats().record_member_hidden_continued();
    (GlobOutcome::Empty, true)   // continue; signal a hidden-continue for the outcome counter
}
ResStatus::Unresolved => {                                                    // some Unknown → fail closed
    guard.stats().record_member_undecidable();
    (GlobOutcome::Poison, false)
}
```

(The `Resolved` len==1 arm at `:434-444` is unchanged: contribute + `record_resolved`, returning
`(GlobOutcome::Empty, false)`.) The `with_glob` closure return type widens from `GlobOutcome` to
`(GlobOutcome, bool)` where the bool = *this edge was a hidden-continue*; `with_glob<R>` is generic so
its signature is unchanged. The other closure exits (cycle, target-resolution failures) return
`(GlobOutcome::Poison, false)`. After `with_glob`, the caller ORs the bool into the
`glob_lookup`-local `saw_hidden_continue` **before** the existing poison short-circuit (so an earlier
edge's hidden-continue is still recorded when a *later* edge poisons the invocation) (§3.7).

### 3.5 Direct bare-name / path claimed-rib — UNCHANGED (invariant)

`resolve_bare` step-1 (`engine.rs:213-216`) calls the `resolve_rib` **wrapper** → identical
`Resolution`; an all-hidden direct rib still returns `Unresolved` and resolve_bare still does **not**
ascend to an outer same-name (the §7 decoy rule). `resolve_path_guarded` reads only
`probe.rib_present()`. The continuation behavior change is **isolated to the glob member arms.** The
full existing `resolve_bare` / path / `extern_crate_root` test suite staying green is part of the
acceptance gate (the wrapper + `rib_present()` are behavior-preserving).

### 3.6 Already-resolved glob member arm — soundness completion

The non-deferred arm (`engine.rs:472-501`, glob `e.to` already a resolved scope) currently does a
**blanket** `if !policy.visible(b, q, &trav) { continue; }` (`:482`) — it already continues past
*any* invisible member, including an **undecidable** one. That is the member-level mirror of the hole
#126 fixed: continuing past an `Unknown` member could let a sibling mint a wrong singleton. Apply the
same hook so the invariant is **representation-independent**:

```rust
match policy.member_visible(b, q, &trav) {
    VisibilityDecision::Visible => { /* fall through to the existing push */ }
    VisibilityDecision::Hidden  => continue,   // pre-existing skip; NOT a new-recall hidden-continue
    VisibilityDecision::Unknown => { guard.stats().record_member_undecidable(); return GlobOutcome::Poison; }
}
```

(`Hidden`→`continue` here deliberately does **not** set `saw_hidden_continue` — this arm already
skipped invisible members, so it is not new recall and must not inflate the deferred-arm buy counters.)
This is **soundness-monotonic** — it only *adds* poison (`Unknown`→poison, was continue); it can
never mint a new edge, so the **wrong-singleton canary cannot regress**. It **may reduce `kind_exact`
recall**, though — an `Unknown` member that would later prove `Hidden` now poisons instead of letting
a sibling resolve — which acceptance measures and surfaces (§6.2). On Rust, glob re-exports populate
as **pending** edges (`walk/items.rs:237`) → the deferred arm dominates and this arm is expected
~inert; acceptance measures it (§6). `member_undecidable` aggregates the
undecidable-poison across both arms. (Hidden→continue here is pre-existing behavior — not new recall —
so it is **not** counted as `member_hidden_continued` and does **not** feed the continue→outcome
counters, which size the deferred-arm lever only.)

### 3.7 Continue→outcome counters — single classification point

The buy is "did continuing past a known-hidden member let this `glob_lookup` invocation resolve?"
Scope the outcome to **one `glob_lookup` invocation** (codex §4): a `glob_lookup`-local
`saw_hidden_continue: bool`, ORed from the deferred-arm signal (§3.4). Classify **once** at the
invocation's single exit by splitting the body into `glob_lookup_inner` (the current body, taking
`&mut saw_hidden_continue`) wrapped by `glob_lookup`:

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

Nesting is handled naturally — each `glob_lookup` invocation owns its own local (a depth-2 inner
expansion is a separate invocation classified separately).

**These are same-invocation OPPORTUNITY telemetry, NOT a bound on the final buy (MAJOR 2).** `_hit`
means the invocation produced candidates after a hidden-continue — but `policy.combine`
(`engine.rs:232`) may fold them to `ResolvedSet`/`Ambiguous` (no Exact edge), and a nested hit can be
swallowed by an outer poison, so `_hit` can **overcount** Exact recovery; conversely a cross-scope
fall-through (resolve at an *outer* scope) registers `_empty` here then resolves outside, so it can
**undercount**. The **final buy is read from the `kind_exact` / `unresolved_unknown_name` deltas**
(§6.2), never these counters. The counters answer "how often did continuing past a hidden member yield
candidates in the same invocation" — the conservatism/opportunity signal, not the edge count; the
counter doc-comments must say this explicitly.

## 4. Forward compatibility — deferring `pub(in)` restrict resolution does NOT corner us

The deferred work (a future recall slice) is populating `resolve_restrict`
(`rust_populator/walk/mod.rs:271`, today a stub → `None`) so `Vis::PubIn { restrict }` carries a
resolved `ScopeId`. **Proof the tri-state design is its forward-compatible consumption seam — no
refactor of this slice:**

1. `vis_reaches(VIS_PUB_IN, ..)` reads `vis.restrict` and returns `Some(is_within(restrict, from))`
   once that field is populated (`rust_policy.rs:161`). **The future work is broader than one site
   (MAJOR 1):** the populator currently *drops* the parsed `vin` restrict path — `walk_use` does
   `vis(vis_kind, None)` (`walk/items.rs:185,192`), structs (`walk/types.rs:37,48`), and the
   enum/trait/assoc/value/type-alias walkers do the same — so the deferred slice must thread `vin`
   through **every** `Vis` construction that discards it, *and* resolve it (the `resolve_restrict`
   stub at `walk/mod.rs:271`).
2. Crucially, that work lives **entirely in the populator / `Vis`-construction layer.** When it lands,
   `member_visible` maps those bindings `Unknown → Hidden / Visible` **automatically** — the hook body,
   signature, and `match` are unchanged.
3. The `MemberProbe`, the engine arms (§3.4/§3.6), and the telemetry buckets are **unchanged**; only
   the *distribution* shifts (`member_undecidable` shrinks; `member_hidden_continued` / resolves grow).
4. The hook already receives everything `vis_reaches` needs — `binding.vis` (incl. a future
   `restrict: Some(scope)`), `binding.scope`, `q.from`. The `restrict: Option<ScopeId>` field
   **already exists** in the `Vis` data model (`types.rs`); no data-model change.

So restrict resolution is a **populator-layer** change (broader than a single site, but confined to
`Vis` construction + `resolve_restrict`), orthogonal to this slice; the tri-state engine seam (hook,
probe, arms, telemetry) is **exactly its unchanged extension point** — codex confirmed "the tri-state
engine seam can remain unchanged." The seam is correctly placed. **Non-goal for this slice:** resolving
`pub(in)` restrict paths.

## 5. Telemetry — clean replace + permanent counters

`glob_stats.rs` `GlobExpandStats` (+ `GlobExpandSnapshot`, `record_*`, `reset`, `snapshot`, the
`GLOBAL` literal, and the round-trip test): **remove `ambiguous`** (no alias — clean replace), add
**six** permanent counters:

- `member_multi` — a single glob target's member lookup returned non-single (`Resolved`-multi /
  `ResolvedSet` / `Ambiguous`); poison. (The spike's "2 genuinely multiply-defined.")
- `member_undecidable` — rib claimed, member result `Unresolved`, ≥1 binding `Unknown` (or an
  already-resolved-arm `Unknown` member); poison. (Retains the fail-closed `pub(in)` cases.)
- `member_hidden_continued` — rib claimed, member result `Unresolved`, all bindings known `Hidden`;
  continue. (The deferred-arm recovery event.)
- `member_hidden_continue_hit` / `_empty` / `_poison` — the `glob_lookup` invocation that had a
  hidden-continue ended Hit / Empty / Poison (§3.7 — **same-invocation opportunity telemetry, not a buy
  bound**; the final buy is the `kind_exact` delta).

`queries.rs:297-306` (`glob_expand` JSON): drop `"ambiguous"`, add the six keys. The
reset-at-entry / snapshot-after-loop wiring (`queries.rs:157,269`) is unchanged.

`record_ambiguous` has two call sites today (`engine.rs:446` multi, `:463` filtered) — both are
replaced per §3.4. The clean replace must also update **every `ambiguous` assertion site (MINOR 2)**:
the `glob_stats.rs` round-trip test, the `glob_expand` JSON key (`queries.rs:304`), the histogram-shape
test at `tests/integration/resolution_test.rs:1774-1783`, and the `snap.ambiguous` asserts at
`tests/name_resolution/glob_expand_test.rs:194,203,359` — each remapped to the split buckets. No
remaining `ambiguous` references anywhere.

## 6. Tests & acceptance

### 6.1 Discriminating fixtures (`tests/name_resolution/glob_expand_test.rs`)

- **Known-hidden sibling recovery (the headline buy):**
  `mod ta { struct S; }\nmod tb { pub struct S; }\npub use ta::*;\npub use tb::*;\nfn f(){ let _: Option<S>; }` →
  resolves to **`tb::S`**; `member_hidden_continued == 1`, `member_undecidable == 0`,
  `member_hidden_continue_hit == 1`, `resolved_l1 == 1`. (Stronger than the current `:248` test, which
  has no public sibling.)
- **Undecidable still poisons:** update the existing `:341` test — status stays `Poisoned`; replace
  `assert!(snap.ambiguous >= 1)` with `assert_eq!(snap.member_undecidable, 1)`.
- **Mixed Hidden+Unknown in one rib poisons:** a cfg-alternative rib with a private `S` and a
  `pub(in crate::ghost) S` (both cfg-compatible under default `CfgCtx`) → `Poisoned`,
  `member_undecidable == 1`, `member_hidden_continued == 0` (proves *any* `Unknown` poisons even with a
  `Hidden` sibling). Plan verifies the cfg-compat assumption with a RED step.
- **No-rib still continues:** preserve `:239` (`glob_expand_target_lacks_name_continues`).
- **Hidden member, no public sibling:** update `:248` (`glob_expand_respects_member_visibility`) — the
  `Hidden` lookup now flows status `Poisoned → Unresolved` (both = no edge); pin
  `member_hidden_continued == 1`, `member_hidden_continue_empty == 1`, `resolved_l1 == 0`.
- **Distinct visible siblings still combine non-single:** preserve `:331`
  (`glob_expand_distinct_targets_two_globs`, `Ambiguous`).
- **Already-resolved arm:** a unit/integration fixture exercising a non-deferred glob whose target
  scope has an `Unknown` (`pub(in)`-no-restrict) member → `Poison` (soundness completion, §3.6).
- **Edge tri-state preserved:** `:288` (`glob_expand_pub_in_unknown_fails_closed`) stays green
  (`vis_unknown == 1`) — the slice does not touch edge handling beyond the enum rename.
- **Remaining `ambiguous` asserts remapped (MINOR 2):** the plan updates each existing
  `snap.ambiguous` / `"ambiguous"` assertion (`glob_expand_test.rs:194,203,359`; the
  `resolution_test.rs:1774-1783` histogram shape) to the correct split bucket (`member_multi` for a
  genuine multi-defined member; `member_undecidable` for a filtered-rib poison) as a per-test TDD step.

### 6.2 Acceptance metrics

- **Canary `multi_target_exact_sites` byte-flat** on **ruff + ripgrep** (`prism corpus` absent from
  bench-repos) — the wrong-singleton gate (`call-stats`).
- `kind_exact` may rise only from known-hidden continuation reaching a real public target
  (the buy; report the delta, do not pre-claim it).
- The bucket split sizes the conservatism: `member_undecidable` (fail-closed `pub(in)`),
  `member_hidden_continued` (target population continued), `member_hidden_continue_{hit,empty,poison}`
  (the realized within-scope buy), `member_multi` (the ~2 genuine-multi).
- Already-resolved-arm change expected ~inert on Rust (deferred edges dominate); if
  `member_undecidable` shows a material already-resolved component or `kind_exact` *drops*, surface it.
- Tier-A `cd eval && uv run tier-a --matrix-only --allow-stale-sut` **0-regr**; then `--quick`
  M2 dogfood (P/fp unchanged); ruff M2 `--corpus ruff` (`baseline_invalid == false`, `shortfall == 0`).
- `CACHE_VERSION` **19 → 20** (resolution behavior changes) + the version test at `cpg_cache.rs:571`.

## 7. Risks & edge cases

- **Classifying `Unknown` as `Hidden`** is the cardinal trap — any unresolved `pub(in)` restrict must
  stay `Unknown`→poison until restrict population lands (§4). `member_visible` maps `None → Unknown`;
  the arm continues only on `all_known_hidden()` (`saw_unknown == false`).
- **Vantage:** member visibility is `binding.scope → q.from` (as `visible()` does, `:241`); edge
  visibility stays `edge.from → q.from` (`:252`). Do not cross them.
- **Direct rib shadowing not loosened** (§3.5): `resolve_path_guarded` still requires a *true* no-rib
  miss (`!probe.rib_present()`) for the crate-root fallback; `resolve_bare` step-1 still returns at the
  claimed rib.
- **Visible-but-unresolvable Pending members are not hidden:** `resolve_rib` still poisons an
  unresolved/cyclic *visible* Pending import (`:323-328`) — `member_visible` only reclassifies the
  visibility *filter* at `:281`, not the Pending-chase outcome.
- **Depth/cycle bookkeeping survives continuation:** the RAII `with_glob` (`:161`) and `MAX_GLOB_DEPTH
  = 2` (`:107`) are unchanged; continuing past a hidden member does not re-enter or skip a leave.
- **Cross-glob `combine` unchanged:** candidates still accumulate through `policy.combine` (diamonds
  dedup, real conflicts → `Ambiguous`, `rust_policy.rs:191`) — not first-wins.
- **Empty-path sentinel is not a hidden member:** a pending glob with empty path still records
  `external` + poison (`:372-375`).
- **Macro-wildcard ordering unchanged:** an explicit rib still shadows a covering macro wildcard
  (`scope_member_lookup_probed`, `:612-634`); this slice does not let a hidden explicit rib re-open
  later wildcard/glob tiers in the same target scope.
- **Enum rename churn:** `GlobEdgeVis → VisibilityDecision` is mechanical across ~4 src sites + tests;
  no behavior change.

## 8. Scope / non-goals / deferred

**In scope:** the `member_visible` hook + shared `VisibilityDecision`; `MemberProbe` +
`resolve_rib_probed`; deferred + already-resolved glob member arms; the 6-counter telemetry split
(clean replace); CACHE 19→20; tests.

**Non-goals / deferred:** resolving `pub(in)` restrict paths (§4 — its own recall slice); changing
direct bare-name/path claimed-rib semantics; touching `resolve_rib`'s Pending-chase; any non-glob
behavior. Non-Rust policies keep the conservative `Unknown` default (no per-language member-visibility
work in this slice).

## 9. Open questions — RESOLVED by codex spec-review (rev 2, 2026-06-22)

The codex xhigh spec-review (`/tmp/member-vis-tristate-spec-review-out.md`) found **no BLOCKER** and
confirmed the cardinal soundness rule (§2) and architecture. Verdict REVISE → 5 findings folded into
this rev (2 MAJOR claim-accuracy, 3 MINOR completeness/defensive — none changed the design). The four
questions are resolved:

1. **Shared enum:** use `VisibilityDecision` (codex concurs — same semantics, less drift). ✓ §3.1.
2. **Already-resolved arm:** include now — it is the same Unknown-skip hole in another representation
   (codex concurs). ✓ §3.6.
3. **Continue→outcome scoping:** keep the per-invocation counters as **opportunity telemetry only**,
   not a buy bound (codex MAJOR 2); the buy is the `kind_exact` delta. ✓ §3.7/§5.
4. **`member_multi` granularity:** keep **one** `member_multi` bucket. Splitting `ResolvedSet` from
   `Ambiguous` only matters if member-multi drives the next recall slice — it does not (the next lever
   is `member_undecidable` / `pub(in)` restrict), and one bucket is sound (codex). ✓ §5.

**Folded findings:** MAJOR 1 (§4 future-work scope — thread `vin` through all `Vis` constructions, not
one site; engine seam unchanged); MAJOR 2 (§3.7/§5 counters = opportunity telemetry, buy = `kind_exact`);
MINOR 1 (§3.2 `saw_visible` encodes the cardinal rule directly); MINOR 2 (§5/§6 enumerate all
`ambiguous` assertion sites); MINOR 3 (§3.6 separate canary-safety from recall cost).
