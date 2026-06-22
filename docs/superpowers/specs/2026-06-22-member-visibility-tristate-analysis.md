# Member-Visibility Tri-State — Architecture Analysis (design seed) — 2026-06-22

> **codex (gpt-5.5, xhigh) read-only architecture analysis**, commissioned as the *design seed* for
> the member-visibility tri-state slice — the §9 follow-on to deferred-glob member expansion (#126,
> merged). This is **not** the design-of-record: a fresh session's brainstorm → spec → codex-review
> loop turns the recommendation below into the formal spec. Resume context + standing constraints:
> `docs/superpowers/handoffs/2026-06-22-member-visibility-tristate-handoff.md`. Arc log:
> `memory/project_prism_owner_key_collision.md`.
>
> **Spike that greenlit it:** on ruff, `glob_expand.ambiguous` = 14665 is **99.99% the claimed-but-
> visibility-filtered member case (14663)** vs 2 genuine-multi — the conservative blanket-poison from
> #126's BLOCKER fix is the dominant facade-resolution outcome. **Caveat: 14663 is the TARGET, not the
> buy** (the realized gain is the subset where *continuing* reaches a clean resolution; size it with
> the telemetry split below before claiming a number).

---

## Executive summary

The next slice should turn the current `Unresolved + rib_present => poison` blanket into a policy-proven decision: continue only when the target rib’s members are all **known hidden**, and keep poisoning when any member visibility is **unknown** or the member lookup is genuinely multi-valued. The measured ruff spike says almost all current `glob_expand.ambiguous` events, 14,663 of 14,665, are in that claimed-but-filtered member case; the architecture below recovers only the subset where falling through reaches a valid later glob or scope result, while preserving the cardinal invariant that prism may miss an edge but must not mint a wrong singleton.

**1. Problem & Opportunity**
The shipped glob expansion resolves a visible deferred glob edge by resolving its target path, then probing the target scope for the queried member at [src/name_resolution/engine.rs:431](/Users/wesleyjinks/code/slicing/src/name_resolution/engine.rs:431). If the target has no explicit rib for the name, the lookup can continue at [src/name_resolution/engine.rs:461](/Users/wesleyjinks/code/slicing/src/name_resolution/engine.rs:461). If a rib exists but `resolve_rib` returns `Unresolved`, the engine currently records `ambiguous` and poisons at [src/name_resolution/engine.rs:462](/Users/wesleyjinks/code/slicing/src/name_resolution/engine.rs:462).

That is conservative because `resolve_rib` filters invisible bindings with a boolean `policy.visible` at [src/name_resolution/engine.rs:281](/Users/wesleyjinks/code/slicing/src/name_resolution/engine.rs:281), then returns `Unresolved` if no candidates survive at [src/name_resolution/engine.rs:333](/Users/wesleyjinks/code/slicing/src/name_resolution/engine.rs:333). Rust already has the missing information: `vis_reaches` returns `Some(true)`, `Some(false)`, or `None` at [src/name_resolution/rust_policy.rs:146](/Users/wesleyjinks/code/slicing/src/name_resolution/rust_policy.rs:146), but `visible()` collapses `None` to false at [src/name_resolution/rust_policy.rs:241](/Users/wesleyjinks/code/slicing/src/name_resolution/rust_policy.rs:241).

The target population is the 14,663 claimed-but-filtered events, not the guaranteed recall gain. If a hidden `pub(crate)` name is skipped and nothing else resolves, the final outcome is still unresolved. The realized buy is the subset where continuing reaches a clean singleton, or a legitimate non-single that avoids a prior poison but still does not create an Exact edge.

Size the real buy in two layers. First, split the current blanket into `known_hidden`, `undecidable`, and `member_multi`. Second, after enabling continuation, count hidden-continued lookups by final local outcome: later hit, later empty, later poison. The global `call_stats` loop already resets and snapshots glob counters per measurement at [src/navigation/queries.rs:156](/Users/wesleyjinks/code/slicing/src/navigation/queries.rs:156) and [src/navigation/queries.rs:269](/Users/wesleyjinks/code/slicing/src/navigation/queries.rs:269), so this can be measured alongside `kind_exact` and `unresolved_unknown_name`.

**2. Cardinal Soundness Question**
The precise safe rule is:

> For a visible glob edge whose target scope is resolved, continue past a claimed-but-filtered member rib only when every explicit binding in that rib is classified by the policy as `Hidden`, and none is classified as `Unknown`. Otherwise poison.

Case walk:

- Private member: Rust `VIS_PRIV` is visible only when `q.from` is inside the defining module subtree at [src/name_resolution/rust_policy.rs:162](/Users/wesleyjinks/code/slicing/src/name_resolution/rust_policy.rs:162). From outside, `Some(false)` means the glob does not bring that member to this query vantage; continuing is sound.
- `pub(crate)` from another crate: crate roots are compared at [src/name_resolution/rust_policy.rs:153](/Users/wesleyjinks/code/slicing/src/name_resolution/rust_policy.rs:153). Cross-crate `Some(false)` is known hidden; continuing may expose a sibling public glob, which is semantically the only visible import for that consumer.
- `pub(super)` or resolved `pub(in)` from outside: `pub(super)` uses the parent module subtree at [src/name_resolution/rust_policy.rs:157](/Users/wesleyjinks/code/slicing/src/name_resolution/rust_policy.rs:157); resolved `pub(in)` maps to `Some(is_within(...))` at [src/name_resolution/rust_policy.rs:161](/Users/wesleyjinks/code/slicing/src/name_resolution/rust_policy.rs:161). `Some(false)` is a proof of absence from the glob’s visible member set.
- Undecidable `pub(in)`: `VIS_PUB_IN` with no resolved restrict returns `None` at [src/name_resolution/rust_policy.rs:161](/Users/wesleyjinks/code/slicing/src/name_resolution/rust_policy.rs:161). Today that is common because `resolve_restrict` is a Phase-1 stub returning `None` at [src/name_resolution/rust_populator/walk/mod.rs:271](/Users/wesleyjinks/code/slicing/src/name_resolution/rust_populator/walk/mod.rs:271), and type walkers often keep only the visibility kind while storing `restrict: None`, for example [src/name_resolution/rust_populator/walk/types.rs:38](/Users/wesleyjinks/code/slicing/src/name_resolution/rust_populator/walk/types.rs:38) and [src/name_resolution/rust_populator/walk/types.rs:48](/Users/wesleyjinks/code/slicing/src/name_resolution/rust_populator/walk/types.rs:48). This must poison.

Could continuing past a filtered member let a sibling glob mint a wrong singleton? Only if `Hidden` is assigned to something that might actually be visible. Under the rule above, `Hidden` means the policy has proved non-accessibility for this `q.from`. That member is not in the glob’s visible contribution set, so a later sibling singleton is not wrong. The blocker hole remains closed because `Unknown` never continues; the existing `pub(in crate::ghost)` regression at [tests/name_resolution/glob_expand_test.rs:341](/Users/wesleyjinks/code/slicing/tests/name_resolution/glob_expand_test.rs:341) should still poison.

**3. Architecture**
Recommend option **(b): a richer member probe**, backed by a new policy tri-state hook. Hook-only option (a) is not enough because `Resolution` loses the aggregate reason for `Unresolved`. Re-classifying in `glob_lookup` by inspecting Rust `Vis` directly, option (c), would violate the engine’s language-neutral rule stated at [src/name_resolution/engine.rs:1](/Users/wesleyjinks/code/slicing/src/name_resolution/engine.rs:1).

Add to [src/name_resolution/types.rs](/Users/wesleyjinks/code/slicing/src/name_resolution/types.rs:435):

```rust
pub enum MemberVis {
    Visible,
    Hidden,
    Unknown,
}

fn member_visible(
    &self,
    binding: &Binding,
    q: &ResolveQuery,
    trav: &TraversalCtx,
) -> MemberVis {
    if self.visible(binding, q, trav) {
        MemberVis::Visible
    } else {
        MemberVis::Unknown
    }
}
```

The default is intentionally conservative: existing non-Rust policies that only know boolean `visible=false` get `Unknown`, so glob expansion keeps poisoning rather than silently skipping. Rust overrides it by mapping `vis_reaches`: `Some(true) => Visible`, `Some(false) => Hidden`, `None => Unknown`, mirroring `glob_edge_visible` at [src/name_resolution/rust_policy.rs:246](/Users/wesleyjinks/code/slicing/src/name_resolution/rust_policy.rs:246).

Internally, replace the boolean return from `scope_member_lookup_probed` at [src/name_resolution/engine.rs:605](/Users/wesleyjinks/code/slicing/src/name_resolution/engine.rs:605) with a probe:

```rust
enum MemberProbe {
    NoRib,
    Rib {
        saw_hidden: bool,
        saw_unknown: bool,
    },
}
```

Keep helpers such as `rib_present()`, `all_known_hidden()`, and `has_unknown()`. `resolve_path_guarded` still uses only `!probe.rib_present()` for the extern crate-root fallback at [src/name_resolution/engine.rs:560](/Users/wesleyjinks/code/slicing/src/name_resolution/engine.rs:560), preserving the current claimed-rib shadowing behavior.

Implement a private `resolve_rib_probed(...) -> (Resolution, MemberProbe)` and keep `resolve_rib(...) -> Resolution` as a wrapper for existing callers. In the deferred glob member arm:

```rust
ResStatus::Unresolved if !probe.rib_present() => Empty,
ResStatus::Unresolved if probe.all_known_hidden() => {
    stats.record_member_hidden_continued();
    Empty
}
ResStatus::Unresolved => {
    stats.record_member_undecidable();
    Poison
}
```

Do not change the non-glob bare-name semantics. `resolve_bare` must still stop at an explicit rib found in the current scope at [src/name_resolution/engine.rs:213](/Users/wesleyjinks/code/slicing/src/name_resolution/engine.rs:213), even if all members are hidden. The new continuation applies only to glob membership: a hidden member is absent from the glob’s exported/imported set, while a direct rib claim remains authoritative under prism’s no-wrong-outer-target invariant.

Also apply `member_visible` in the already-resolved glob arm at [src/name_resolution/engine.rs:472](/Users/wesleyjinks/code/slicing/src/name_resolution/engine.rs:472): `Hidden` continues, `Unknown` poisons, `Visible` contributes. Rust currently populates `use a::*` as pending edges at [src/name_resolution/rust_populator/walk/items.rs:237](/Users/wesleyjinks/code/slicing/src/name_resolution/rust_populator/walk/items.rs:237), but the invariant should not depend on that representation.

**4. Telemetry**
Current `GlobExpandStats` has eight counters, including the overloaded `ambiguous`, at [src/name_resolution/glob_stats.rs:8](/Users/wesleyjinks/code/slicing/src/name_resolution/glob_stats.rs:8), and `call_stats` emits them at [src/navigation/queries.rs:297](/Users/wesleyjinks/code/slicing/src/navigation/queries.rs:297).

Replace the semantic use of `ambiguous` with:

- `member_multi`: member lookup returned non-single, including `ResolvedSet`, `Ambiguous`, or `Resolved` with len not equal to 1.
- `member_undecidable`: explicit rib exists, member result is `Unresolved`, and at least one binding is `MemberVis::Unknown`; poison.
- `member_hidden_continued`: explicit rib exists, member result is `Unresolved`, and all bindings are known hidden; continue.

For sizing the real buy, add either permanent or spike-only counters:

- `member_hidden_continue_hit`: a `glob_lookup` invocation had a known-hidden skip and later produced candidates.
- `member_hidden_continue_empty`: known-hidden skip, no later candidates.
- `member_hidden_continue_poison`: known-hidden skip, later fail-closed reason.

Keep `"ambiguous"` for one slice as a compatibility alias equal to `member_multi + member_undecidable`, but do not include `member_hidden_continued` in it. That keeps old dashboards conservative while making the recovery visible.

**5. Tests & Acceptance**
Add discriminating fixtures next to the existing glob tests.

- Known-hidden sibling recovery: `mod ta { struct S; } mod tb { pub struct S; } pub use ta::*; pub use tb::*; fn f(){ let _: Option<S>; }`. Expect `Resolved` to `tb::S`, `member_hidden_continued == 1`, no `member_undecidable`. This is stronger than the current “hidden does not resolve” test at [tests/name_resolution/glob_expand_test.rs:248](/Users/wesleyjinks/code/slicing/tests/name_resolution/glob_expand_test.rs:248).
- Undecidable still poisons: preserve the existing `pub(in crate::ghost)` sibling-public test at [tests/name_resolution/glob_expand_test.rs:341](/Users/wesleyjinks/code/slicing/tests/name_resolution/glob_expand_test.rs:341), but assert `member_undecidable == 1` instead of the old overloaded ambiguous bucket.
- Mixed hidden plus unknown in the same rib poisons.
- No-rib still continues, preserving [tests/name_resolution/glob_expand_test.rs:239](/Users/wesleyjinks/code/slicing/tests/name_resolution/glob_expand_test.rs:239).
- Distinct visible sibling glob targets still combine to non-single, preserving [tests/name_resolution/glob_expand_test.rs:331](/Users/wesleyjinks/code/slicing/tests/name_resolution/glob_expand_test.rs:331).

Acceptance metrics:

- `member_multi` should explain the prior “2 genuinely multiply-defined” population.
- `member_undecidable` should retain fail-closed `pub(in)` cases.
- `kind_exact` may increase only from known-hidden continuation reaching a real public target.
- `multi_target_exact_sites` must be byte-flat; the parent spec makes this the wrong-singleton canary at [docs/superpowers/specs/2026-06-22-glob-export-member-expansion-design.md:317](/Users/wesleyjinks/code/slicing/docs/superpowers/specs/2026-06-22-glob-export-member-expansion-design.md:317).
- Run the repo’s Tier-A sequence for this touched area: `cargo build --release`, then `cd eval && uv run tier-a --matrix-only --allow-stale-sut`, then `cd eval && uv run tier-a --quick --allow-stale-sut`.

**6. Risks & Edge Cases**
The main trap is classifying `Unknown` as `Hidden`. Any unresolved `pub(in)` restrict must poison until restrict resolution is populated.

Use the correct vantage. Member visibility should be computed from `binding.scope` to `q.from`, as `visible()` does today at [src/name_resolution/rust_policy.rs:241](/Users/wesleyjinks/code/slicing/src/name_resolution/rust_policy.rs:241). Edge visibility remains separate and uses `edge.from` at [src/name_resolution/rust_policy.rs:252](/Users/wesleyjinks/code/slicing/src/name_resolution/rust_policy.rs:252).

Do not loosen direct rib shadowing. The explicit-rib path in `resolve_bare` returns immediately at [src/name_resolution/engine.rs:215](/Users/wesleyjinks/code/slicing/src/name_resolution/engine.rs:215); the path fallback must still require a true no-rib miss.

Visible pending members that fail to resolve are not hidden. `resolve_rib` currently poisons unresolved pending imports at [src/name_resolution/engine.rs:323](/Users/wesleyjinks/code/slicing/src/name_resolution/engine.rs:323); keep that.

Depth and cycle bookkeeping must survive continuation. The RAII `with_glob` enters and leaves at [src/name_resolution/engine.rs:161](/Users/wesleyjinks/code/slicing/src/name_resolution/engine.rs:161), and the depth limit remains two at [src/name_resolution/engine.rs:107](/Users/wesleyjinks/code/slicing/src/name_resolution/engine.rs:107).

Do not change cross-glob combination into first-wins. Candidates still accumulate and go through `policy.combine`; Rust dedups diamonds and marks real conflicts at [src/name_resolution/rust_policy.rs:191](/Users/wesleyjinks/code/slicing/src/name_resolution/rust_policy.rs:191).

The empty-path sentinel is not a hidden member. A pending glob with an empty path records `external` and poisons at [src/name_resolution/engine.rs:372](/Users/wesleyjinks/code/slicing/src/name_resolution/engine.rs:372).

Macro wildcard ordering remains conservative. `scope_member_lookup_probed` treats an explicit rib as shadowing wildcard poison at [src/name_resolution/engine.rs:612](/Users/wesleyjinks/code/slicing/src/name_resolution/engine.rs:612); this slice should not reinterpret hidden explicit ribs as permission to inspect later wildcard/glob tiers inside the same target scope.

**7. Recommendation & Open Questions**
- Implement the richer `MemberProbe` plus `ResolutionPolicy::member_visible`; Rust maps through `vis_reaches`, other policies default to `Unknown` when boolean `visible` is false.
- Change only glob membership behavior: all-known-hidden member ribs continue; any unknown member visibility poisons.
- Update both deferred and already-resolved glob member paths so the invariant is representation-independent.
- Split telemetry before judging recall; the 14,663 events are opportunity, not promised Exact edges.
- Keep direct bare-name/path claimed-rib behavior unchanged.

Effort estimate: light-to-medium. The edge tri-state pattern already exists, but the correctness work is in preserving direct-rib semantics, reshaping tests, and making telemetry answer the “continued to what?” question.

Open questions for the formal spec: whether to introduce a generic `VisibilityDecision` instead of a second enum, whether `ambiguous` remains as a JSON compatibility alias, whether hidden-continuation outcome counters are permanent or spike-only, and whether this slice should also start resolving `pub(in)` restrict paths or explicitly defer that as a separate recall slice.
