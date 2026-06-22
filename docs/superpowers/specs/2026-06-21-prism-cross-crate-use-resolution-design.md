# Cross-Crate `use` Resolution — Design

**Date:** 2026-06-21
**Status:** design-of-record (codex xhigh spec review folded — 2 BLOCKER + 2 MAJOR + 1 MINOR)
**Branch:** `cross-crate-use-resolution` (off `main` after PR #123 / edition anchoring-class merged)
**Predecessors:** #120 demote-not-drop · #121 scope-graph recovery · #122 (A) prune-through-`use` ·
#123 edition anchoring-class uniformity. This is the next slice on the
`docs/ruff-typepath-recovery-roadmap-2026-06-21.md` sequence (cross-crate **+428** → glob +304).

## 0. One-paragraph summary

prism's scope graph already builds a per-crate index `Builder::crate_roots_by_name:
BTreeMap<String, ScopeId>` (in-source crate name → its `Root` scope) and already uses it to
resolve `extern crate name;` (`walk/items.rs:160`). But the map is **ephemeral** — dropped at
`Builder::finish()` (`builder.rs:101`), never serialized into `ScopeGraph`, and **never
consulted** when a 2018 `use other_crate::Foo` (or `other_crate::foo()`) looks up its leading
segment. So a path whose leading segment is a sibling workspace crate resolves `Unresolved`
(`engine.rs:408`), and #122 (A)'s `pending_resolves_to_single_in_repo_item` declines → the
collision is kept-all. This slice (1) **persists** `crate_roots_by_name` into `ScopeGraph`, and
(2) **consults it as the strictly-last fallback** for a leading path segment via a new
`ResolutionPolicy` hook, so cross-crate `use`/call leading segments resolve to the owning
crate's `Root` scope and the existing path-walk continues into that crate. The fallback is
narrowly gated (Rust 2018+ extern-prelude roots only — `UsePath`/`Bare` anchors, never
`crate::`/`self::`/`super::`/2015; a TRUE no-rib miss only, never a claimed-but-invisible local;
an unambiguous single in-repo crate name, declining duplicates — see §2.2). Recovers the **+428**
cross-crate collision sites on ruff (and resolves ordinary cross-crate calls for navigation).
Recall safety is **conditional** on the recovered resolution being correct and uniquely resolved
(§3) — it rests on that eligibility plus #122 (A)'s single-in-repo-item gate, not on "adding a
resolution cannot drop an edge" (consumers *can* prune on a newly-resolved path; §3, MAJOR 2).

## 1. Premises (govern every requirement)

- **P1 — recall is conditional on a correct unique resolution.** The fallback fires **only** on a
  TRUE no-rib miss for the leading segment (§2.2), converting `Unresolved → Resolved`; at the
  engine call site it never changes or removes an existing resolution. But "adding a resolution"
  is **not** recall-safe by construction at the *consumer* level: a newly-`Resolved` path lets
  #122 (A)'s disproof prune a candidate pool (`resolution.rs:794`) and lets the general Rust
  scope-graph resolution early-return Exact/UnknownName ahead of older name/owner fallbacks
  (`resolution.rs:825`). Recall is therefore safe **iff** the recovered resolution is correct and
  uniquely resolved — which rests on the tightened eligibility (P2, P3, and the anchor/edition
  gate of §2.2) plus #122 (A)'s exactly-one-in-repo-item gate, **not** on the act of adding a
  resolution. (Precision can *shift* NameOnly→Exact; that is the measured surface, §7.)
- **P2 — local items win (Rust precedence).** A path leading segment that matches a local
  binding/glob in the anchored scope is returned by `scope_member_lookup` *before* the fallback is
  reached: a **visible** local rib/glob yields a non-`Unresolved` resolution, and a
  **claimed-but-invisible** local rib yields `Unresolved` *with the rib-present flag set* (§2.2 /
  BLOCKER 1) — both shadow the crate name, because the fallback fires only on a TRUE no-rib miss
  (`!rib_present`). The fallback is the **last rung**, so a same-named local module/import always
  shadows a crate name, matching Rust 2018 resolution.
- **P3 — decline on any doubt.** The fallback resolves a leading segment to a crate root only on
  an **exact, unambiguous** crate-name match (exactly one entry). No match, >1 match, or a
  non-leading position → stay `Unresolved` (keep-all). **This must be enforced, not assumed:**
  `crate_name_for_root` (`builder.rs:521-543`) derives the crate name from the workspace-member
  **directory basename** (or the `<name>/src/` parent), **not** from Cargo `[package].name`, and
  the Builder inserts **first-wins** (`.entry(name).or_insert(...)`, `builder.rs:284`) — so two
  members with the same normalized basename silently collapse to one root and the fallback would
  look "single" when it is not. The persisted map must therefore record which normalized names
  map to **>1 distinct root** and the hook must **decline** the fallback for any such name (treat
  it as ambiguous → keep-all). Package-name accuracy (a `[package].name` that differs from the
  directory) is **not** guaranteed by this source and is out of scope (§8). The collision-recovery
  precision backstop is unchanged: #122 (A) still requires the FULL path to resolve to **exactly
  one** in-repo `Target::Item` before the disproof prunes.
- **P4 — known-valid source ⇒ no new visibility checker.** An existing `use other_crate::Foo`
  compiled, so `Foo` is visible from the using crate; we need only resolve it to the *right*
  single item. Visibility across the crate boundary is not separately enforced — soundness comes
  from P3's exactly-one-item requirement and the engine's existing per-segment `visible()` hook
  inside the target crate's module walk.
- **P5 — minimal surface.** Two new serialized `ScopeGraph` fields (`crate_roots_by_name` +
  `ambiguous_crate_names`, both already-derivable at build time), one `ResolutionPolicy` hook,
  one call site in `resolve_path_guarded`, one `CACHE_VERSION` bump. The **only** `Builder`
  population change is the duplicate-name detection of §2.1 (recording collisions into
  `ambiguous_crate_names`); the single-root `extern crate` resolution is unchanged. No change to
  the disproof predicate or #122 (A)'s consumer.

## 2. Design

### 2.1 Persist `crate_roots_by_name` into `ScopeGraph`

Add a serialized field to `ScopeGraph` (`src/name_resolution/graph.rs:77-109`), as a
backward-compatible `#[serde(default)]` map alongside `file_paths`:

```rust
    /// Rust: in-source crate name → that crate's `Root` scope, for resolving a
    /// 2018 bare-crate path leading segment (`use other_crate::X`, `other_crate::f()`)
    /// and `extern crate name`. Other languages leave this empty. Keys are
    /// hyphen→underscore normalized (the Rust path identifier form). A name that
    /// maps to >1 distinct root is recorded in `ambiguous_crate_names` (below) and
    /// the §2.2 fallback DECLINES for it (P3).
    #[serde(default)]
    pub crate_roots_by_name: std::collections::BTreeMap<String, ScopeId>,

    /// Rust: normalized crate names that the Builder saw map to >1 distinct
    /// `Root` (duplicate workspace-member basenames collapsing under first-wins).
    /// The §2.2 fallback treats these as ambiguous → keep-all (P3). Other
    /// languages leave this empty.
    #[serde(default)]
    pub ambiguous_crate_names: std::collections::BTreeSet<String>,
```

Populate both at `Builder::finish()` (`src/name_resolution/rust_populator/builder.rs:101`):

```rust
    pub(crate) fn finish(mut self) -> ScopeGraph {
        self.graph.crate_roots_by_name = self.crate_roots_by_name;
        self.graph.ambiguous_crate_names = self.ambiguous_crate_names;
        self.graph
    }
```

**Key normalization (correctness).** The map is keyed by `crate_name_for_root`
(`builder.rs:521-543`, derived from the workspace-member **directory basename** / the
`<name>/src/` parent — **not** Cargo `[package].name`); a Rust path identifier uses underscores
while a package/dir name may carry hyphens. Normalize keys to the underscore form **at insertion**
(where the Builder inserts at `builder.rs:284`) so both the `extern crate` precedent and the new
fallback match the path-identifier spelling. Lookups (§2.2) normalize the query segment
identically. Insertion-side normalization keeps a single source of truth; the existing
`extern crate` lookup reads the same normalized keys, and since an `extern crate <ident>`
identifier is already underscore-form, normalization is idempotent there (it can only *add*
matches for a previously hyphenated key — a latent improvement, never a regression).

**Duplicate detection (P3 enforcement).** The current insertion is **first-wins**
(`.entry(name).or_insert(root_scope)`, `builder.rs:284`) — a *second* root with the same
normalized name is silently dropped. Because the fallback's soundness depends on a name being
unambiguous, the insertion must additionally record a collision: if `entry(name)` already holds a
**distinct** root, add the normalized `name` to `self.ambiguous_crate_names`. (Normalization can
*create* a collision two hyphen/underscore spellings did not have before, so this check runs on
the already-normalized key.) The map still keeps first-wins for the `extern crate` precedent
(unchanged behavior for the single-root common case); the new ambiguity set is consulted only by
the §2.2 fallback. This is the one population-logic change this slice makes — narrowly, to *add*
the doubt signal P3 requires; it does not alter which single root `extern crate` resolves to.

### 2.2 Leading-segment crate-root fallback (a `ResolutionPolicy` hook)

Add a `ResolutionPolicy` trait method, defaulting to `None` so only Rust opts in. The hook
**must** receive the path `Anchor` — a sibling-crate fallback is valid only for a 2018+
extern-prelude *root* (`UsePath`/`Bare`), never for `crate::`/`self::`/`super::`/`::`, and the
edition lives in the policy — so a `(graph, name)` signature is too broad (it would fire after
*every* anchor `resolve_path_guarded` walks):

```rust
    /// A path's leading segment may name another crate (Rust 2018+ extern-prelude
    /// root). Returns that crate's `Root` scope iff (a) the anchor is an
    /// extern-prelude root kind for this language/edition AND (b) `name` uniquely
    /// names an in-repo crate. `anchor` lets the policy gate on anchor kind +
    /// edition. Default: not applicable.
    fn extern_crate_root(
        &self,
        _graph: &ScopeGraph,
        _name: &str,
        _anchor: &Anchor,
    ) -> Option<ScopeId> {
        None
    }
```

`RustPolicy` implements it by **gating on edition + anchor kind first**, then consulting the
persisted map (normalized), returning `Some` only on an exact single match that is **not** in the
ambiguity set:

```rust
    fn extern_crate_root(&self, graph: &ScopeGraph, name: &str, anchor: &Anchor) -> Option<ScopeId> {
        // Eligibility (BLOCKER 2): only a 2018+ extern-prelude ROOT may name a
        // sibling crate. `crate::`/`self::`/`super::` anchor inside THIS crate;
        // `LeadingColon` (`::other_crate::X`) is excluded in v1 (see §8); 2015
        // `use sibling::X` needs an `extern crate` binding (already modeled at
        // walk/items.rs:160), so the bare fallback must not invent one.
        if !self.is_2018_plus() {
            return None;
        }
        if !matches!(anchor.kind, AnchorKind::UsePath | AnchorKind::Bare) {
            return None;
        }
        let key = normalize_crate_ident(name);
        // P3: decline a name that collapsed >1 distinct root (duplicate basenames).
        if graph.ambiguous_crate_names.contains(&key) {
            return None;
        }
        graph.crate_roots_by_name.get(&key).copied()
    }
```

(`AnchorKind` and `Anchor` are `name_resolution::types`; `resolve_path_guarded` already holds the
`&Anchor` it was called with, so threading it costs nothing. `BTreeMap::get` returns one scope per
key; the Builder's insertion is **first-wins** — a second distinct root for the same normalized
name is dropped at insertion (`builder.rs:284`) **and** recorded in `ambiguous_crate_names`
(§2.1), so the `contains` guard above turns that silent collapse into an explicit decline rather
than a confident wrong root.)

Invoke it from `resolve_path_guarded` (`src/name_resolution/engine.rs:338-366`) **only for the
leading segment (`i == 0`), only on a TRUE no-rib miss, after** the normal `scope_member_lookup`.

**The trigger is NOT `ResStatus::Unresolved` (BLOCKER 1).** `resolve_rib` deliberately returns
`unresolved()` for a **claimed-but-invisible** rib — when an explicit rib bound the name but every
candidate failed `visible()`, it returns `Unresolved` to mean "claimed; do **not** continue
outward" (`engine.rs:181-185` skip-not-contribute; `engine.rs:233-235` "Claimed but nothing
visible/resolvable → fall through (NOT outward)"). Firing the crate fallback on that status would
override a deliberately-invisible *local* name with a sibling crate — violating the invariant.
`scope_member_lookup` also returns `Unresolved` for empty globs (`engine.rs:408`). So the fallback
must fire **only** when there was a TRUE miss for the segment in the anchored scope: **no explicit
rib binding existed at all** (visible or not), no macro-wildcard poison, and globs empty.

The mechanism is to extend the single-scope lookup to report **claim metadata** ("did a rib
binding for `(name, ns)` exist in this scope, regardless of visibility?") distinct from the
`Resolution` it returns. The exact shape (e.g. a sibling `scope_member_lookup_probed` returning
`(Resolution, RibPresence)`, or a `rib_present` flag) is finalized in the plan; the **predicate**
is fixed here: `rib_present == false && status == Unresolved` (no poison, empty globs):

```rust
        // Single-scope member lookup AND a "was a rib claimed here?" probe (so a
        // claimed-but-invisible local cannot be overridden by the crate fallback).
        let (res, rib_present) = scope_member_lookup_probed(graph, scope, &seg_q, policy, guard);
        // Leading-segment crate-root fallback (strictly last; P2/P3): a 2018+
        // extern-prelude root resolves to the owning crate's Root scope ONLY on a
        // TRUE no-rib miss — no rib was claimed for the segment (so a local item,
        // even a deliberately-invisible one, always shadows — P2/BLOCKER 1) — AND
        // the policy's anchor/edition gate + the unique in-repo crate name pass
        // (P3). Poison/empty-glob `Unresolved` with a claimed rib does NOT qualify.
        let res = if i == 0
            && !rib_present
            && matches!(res.status, ResStatus::Unresolved)
        {
            match policy.extern_crate_root(graph, seg, anchor) {
                Some(root) => resolved_to_root_scope(root), // single Resolved candidate, scope-bearing
                None => res,
            }
        } else {
            res
        };
        if is_last {
            return res;
        }
        // ... unchanged: non-final must be exactly one scope-bearing target → `scope = root`
```

(`rib_present` is the rib-claim signal, computed exactly where `scope_member_lookup` builds `rib`
at `engine.rs:388-395` — `!rib.is_empty()`. A macro-wildcard poison returns `Poisoned`
(`engine.rs:401-403`), and any resolved candidate / `ResolvedSet` is also `status != Unresolved`,
so the `matches!(Unresolved)` guard already excludes those; the new `!rib_present` term is what
additionally excludes the **claimed-but-invisible rib** — the one path that surfaces as
`Unresolved` *with* a rib present — leaving only the genuine no-rib / empty-glob miss.)

The recovered candidate is a single `Resolved` candidate whose `Target` is scope-bearing (its
`scope_of_target` is the crate `Root`), so the existing non-final branch (`engine.rs:357-361`)
sets `scope = Root` and the walk continues into the crate's modules with **no** other change.
For a bare leading segment that *is* the whole path (`use other_crate;` — a crate alias import),
`is_last` returns the `Root` candidate directly; the consumer treats it as a module target.

**Why the engine, not `scope_member_lookup`.** The fallback is leading-segment-specific (`i==0`)
and must run *after* the in-scope rib/glob tiers; `resolve_path_guarded` is the only place with
both the segment index and the post-lookup status (now paired with the rib-present probe).
`scope_member_lookup` stays a pure single-scope member lookup (no positional knowledge); the
probe variant only *adds* the rib-claim boolean alongside the same `Resolution` — it does not
change the resolution it returns.

### 2.3 Cache

`CACHE_VERSION` 17→18 (`src/cpg_cache.rs:60`) — the two new serialized `ScopeGraph` fields
(`crate_roots_by_name` + `ambiguous_crate_names`) change the bincode layout. The cache uses
**bincode** and `deserialize`s the blob *before* the version check (`cpg_cache.rs:304` vs
`:310`), so an old (field-less) blob either fails to deserialize → `CacheResult::Miss`
(`cpg_cache.rs:306`) or passes the version check → `Miss` (`cpg_cache.rs:310-315`); it is **not**
silently serde-defaulted into a live build. The version bump is what guarantees the rebuild.
Update the pin test name/assertion to 18.

## 3. Soundness & recall-safety

- **Recall (P1 — conditional, not "by construction").** The only engine-level change is:
  `resolve_path_guarded` returns `Resolved` (crate root) in some cases that previously returned a
  TRUE-miss `Unresolved`. But "adding a resolution" is **not** automatically recall-safe at the
  *consumer* level: a path that newly resolves can *shrink* a kept pool. Two confirmed sites:
  #122 (A)'s disproof prune drops collision candidates once
  `pending_resolves_to_single_in_repo_item` flips `false→true` (`resolution.rs:794`), and the
  general Rust scope-graph resolution can early-return Exact/`UnknownName` for a now-resolved `::`
  path ahead of the older name/owner fallbacks (`resolution.rs:825`). So recall is preserved
  **iff** the newly-recovered resolution is *correct and uniquely resolved*. The safety argument
  therefore rests on (i) the TRUE-no-rib-miss trigger + claimed-but-invisible shadow (BLOCKER 1),
  (ii) the anchor/edition eligibility gate (BLOCKER 2), (iii) the unique-crate-name +
  duplicate-decline (P3 / MAJOR 1), and (iv) #122 (A)'s exactly-one-in-repo-item gate — **not** on
  the premise that adding a resolution cannot drop an edge. When a recovered resolution is wrong
  or ambiguous, those gates fail closed to keep-all; when it is right and unique, the pruned
  candidates were genuine FPs.
- **Precision — the collision-recovery case.** The disproof prunes a same-name owner-`::`
  collision to one Exact only when the cross-crate `use` resolves to **exactly one** in-repo
  `Target::Item{owns:Some(scope)}` (unchanged (A) requirement). A wrong single resolution would
  be a wrong prune; P3 (exact unique crate-name match) + the single-item requirement + P4
  (the `use` is real, compiled source) make a wrong resolution require a genuinely ambiguous
  in-repo program, which the single-item gate rejects → keep-all.
- **Precision — the general (navigation) case.** A cross-crate call `other_crate::foo()` that
  was `Unresolved`/NameOnly may now resolve Exact. If the crate-root + module walk lands on the
  wrong item, that is a precision shift (not a recall loss). The exactly-one-crate-match (P3),
  local-shadow precedence (P2), and the engine's existing per-segment `visible()` + non-singular
  fall-through (`engine.rs:362-364`, "anything non-singular falls through, never wrong") bound
  this. §7's Tier-A + ruff M2 + nav spot-check measure it directly (the blast-radius the design
  deliberately opted into).
- **Edition composition (#123).** The disproof still only *runs* on an
  anchoring-class-uniform workspace (`resolution.rs:1337`, unchanged). This slice changes what
  `resolve_path` *returns* inside it. The +428 is the cross-crate bucket of the post-#123 1326
  residue — the two slices compose; neither subsumes the other.

## 4. What flows from the change

- **#122 (A) collision recovery (the headline +428).** `pending_resolves_to_single_in_repo_item`
  now sees `Resolved` + one in-repo item for cross-crate `use` collisions → the disproof prunes
  them to one Exact.
- **General cross-crate resolution.** Any `resolve_path` consumer (navigation callees/callers,
  ego graph, `use`-import edges) now resolves cross-crate leading segments — NameOnly→Exact where
  the walk is unambiguous.

## 5. Buy (measured target)

Roadmap-sized: **+428** cross-crate `use` collision sites on ruff (the largest residue bucket
after #123). Expected `call-stats` delta on ruff (vs current `main` post-#123):
`recovery_typepath.singleton` ≈ **+428**, `failopen_demote` ≈ **−428**,
`multi_target_exact_sites` **unchanged** (no new collision FPs). The exact pre-change baseline is
re-measured on current `main` in the plan's acceptance (post-#123 singleton ≈ 260,
failopen_demote ≈ 1326 are the starting point). ripgrep/prism: small positive or zero (few
cross-crate workspace collisions).

## 6. Tests (TDD; RED→GREEN)

1. **Persist round-trip (unit, `graph.rs`/cache).** A built graph with two crates exposes
   `crate_roots_by_name` with both crate names → root scopes (and `ambiguous_crate_names` empty);
   a freshly-serialized graph round-trips both fields. Cross-version compat is NOT serde-default
   (the cache is bincode and deserializes before the version check — `cpg_cache.rs:304`/`:310`):
   an old field-less blob is handled by the `CACHE_VERSION` 17→18 bump (cache miss → rebuild),
   not by silently defaulting into a live build. Keep `#[serde(default)]` on both fields (matches
   the existing `edition_uniform`/`file_paths` convention so an in-memory or named-format
   round-trip stays robust); a separate named-format (e.g. JSON) test, if wanted, may assert the
   default-on-missing behavior in isolation.
2. **Resolver — cross-crate `use` resolves (unit, engine/resolver).** A two-crate fixture where
   crate `a` has `use b_crate::Foo;` resolves the leading `b_crate` to crate `b`'s root and the
   full path to `b`'s `Foo` (`Resolved`, one in-repo item). Pre-fix: `Unresolved`.
3. **Resolver — local shadow declines the fallback (unit; P2).** Crate `a` has a local
   `mod b_crate;` *and* a sibling crate named `b_crate`; `use b_crate::X` resolves to the LOCAL
   module (rib hit), never the crate root. Pins precedence.
4. **Resolver — claimed-but-invisible local blocks the fallback (unit; BLOCKER 1).** Crate `a`
   has a local rib binding for `b_crate` that is *not visible* at the use site (e.g. a
   `pub(in ...)`/private module member that `resolve_rib` claims but rejects on `visible()`),
   *and* a sibling crate `b_crate`. The leading segment must stay the LOCAL outcome
   (`Unresolved` via claimed-but-invisible — `rib_present == true`), **never** the crate root.
   This is the rib-claim probe's discriminating case (distinct from test 3's visible rib hit).
5. **Resolver — non-crate leading segment stays `Unresolved` (unit; P3).** A leading segment
   matching no crate name and no local binding stays `Unresolved` (no spurious resolution).
6. **Resolver — anchor-kind gate (unit; BLOCKER 2).** With a sibling crate `b_crate`, leading
   segments under `crate::b_crate::X`, `self::b_crate::X`, and `super::b_crate::X` must **not**
   trigger the fallback (those anchor inside the current crate — `CrateRoot`/`SelfMod`/`Super`);
   only `use b_crate::X` / bare `b_crate::f()` (`UsePath`/`Bare`) may. `::b_crate::X`
   (`LeadingColon`) is excluded in v1 (asserts no fallback — deferred, §8).
7. **Resolver — 2015 sibling without `extern crate` stays `Unresolved` (unit; BLOCKER 2).** In a
   2015-edition crate `a`, `use b_crate::X;` **without** an `extern crate b_crate;` binding stays
   `Unresolved` (the bare fallback must not invent a 2015 crate root — `is_2018_plus()` gate). A
   companion positive: with `extern crate b_crate;`, the existing map-backed binding
   (`walk/items.rs:160`) still resolves it (unchanged precedent).
8. **Resolver — duplicate normalized crate names decline (unit; P3 / MAJOR 1).** Two
   workspace members whose basenames normalize to the same crate ident (so first-wins collapse +
   `ambiguous_crate_names` records the name) → `use that_name::X` **declines** the fallback
   (stays `Unresolved`), not a confident wrong root.
9. **End-to-end collision recovery (integration, `resolution_test.rs`).** A pure-2018+ workspace
   (so #123's disproof runs) where crate `a` calls a same-name owner-`::` collision pinned by
   `use b_crate::Foo;` (the `Foo` collides bare with another crate's `Foo`); pre-fix keep-all
   (≥2 NameOnly), post-fix one Exact via the (A) disproof. Built through the real `load_repo` +
   `CallGraph::build_with_scope_graph_inputs` path.
10. **Cache pin (unit, `cpg_cache.rs`).** `CACHE_VERSION == 18`.

## 7. Acceptance (the wider blast radius, opted-in)

- Full test surface (`--lib`, `--test integration`, `--test ast`, `--test cli` via `--no-run`),
  `cargo fmt --check`.
- Tier-A matrix (`--matrix-only --allow-stale-sut`): **0 regression**.
- **ruff M2** (`--corpus ruff --allow-stale-sut`, NOT `--quick`): `baseline_invalid=false`,
  oracle/sut error 0.0, **0 regression** — this is the real recall/precision gate on the wider
  surface.
- `call-stats` on ruff: `recovery_typepath.singleton` ≈ +428, `multi_target_exact_sites`
  unchanged — report the measured pair.
- **nav spot-check (because this touches general resolution, not only the disproof):** sample a
  handful of the newly-Exact cross-crate edges in ruff via `prism nav callees`/`callers` and
  confirm they point at the correct sibling-crate definition (a precision spot-audit the
  collision counters alone don't cover).
- Independent codex gpt-5.5 xhigh diff review.

## 8. Scope boundaries

**In scope (v1):** workspace-member / `<name>/src/lib.rs`-derived crates (whatever
`crate_roots_by_name` already holds); direct crate-name leading segments in `UsePath`/`Bare`
positions (`use other_crate::X`, `other_crate::f()`); hyphen/underscore normalization with
duplicate-name decline.

**Deferred (follow-ups, not this slice):**
- **`LeadingColon` roots** (`::other_crate::X`): the `LeadingColon` anchor resolves via
  `anchor.prelude` and returns `None` when no prelude scope is recorded (`rust_policy.rs:286-291`),
  so v1 deliberately excludes it from the crate fallback (the eligibility gate admits only
  `UsePath`/`Bare`). Re-enabling it (or wiring the populator's prelude scope) is a separate
  follow-up; rare in workspace code, and the bare/`use` forms carry the +428.
- **Renamed deps** (`bar = { package = "foo" }`): `use bar::X` needs the *consuming* crate's
  `[dependencies]` rename map (in-source name → package). Rare in workspaces; the `RustCrateConfig`
  has a stub field (`rust_populator/mod.rs:82`) but it is not threaded. Separate slice if the
  residue warrants.
- **Package-name accuracy** (`[package].name` ≠ directory basename): `crate_name_for_root` keys on
  the workspace-member **directory** name, not the parsed Cargo `[package].name`. A crate whose
  package name differs from its directory will key under the wrong ident (and the fallback simply
  won't fire for the right spelling — recall-neutral, never a wrong resolution). Parsing the real
  package name is a separate refinement if the residue warrants.
- **Glob imports** (`use other_crate::*`): the +304 roadmap bucket — its own slice.

**Out of scope:** non-in-repo crates (registry/git/std) — they have no `Root` scope, so
`extern_crate_root` returns `None` and the path stays `Unresolved`/`External` exactly as today.
