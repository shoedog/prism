# Cross-Crate `use` Resolution — Design

**Date:** 2026-06-21
**Status:** design-of-record (pending codex xhigh spec review)
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
crate's `Root` scope and the existing path-walk continues into that crate. Recovers the **+428**
cross-crate collision sites on ruff (and resolves ordinary cross-crate calls for navigation),
recall-safe by construction (it only *adds* resolution where there was `Unresolved`).

## 1. Premises (govern every requirement)

- **P1 — recall cannot drop.** The fallback fires **only** when leading-segment resolution
  already returned `Unresolved`. It converts `Unresolved → Resolved`; it never changes or
  removes an existing resolution. No previously-kept edge can be dropped by *adding* a
  resolution. (Precision can *shift* NameOnly→Exact; that is the measured surface, §7.)
- **P2 — local items win (Rust precedence).** A path leading segment that matches a local
  binding/glob in the anchored scope is returned by `scope_member_lookup` *before* the fallback
  is reached (the rib is non-empty → not `Unresolved`). The fallback is the **last rung**, so a
  same-named local module/import always shadows a crate name, matching Rust 2018 resolution.
- **P3 — decline on any doubt.** The fallback resolves a leading segment to a crate root only on
  an **exact, unambiguous** crate-name match (exactly one entry). No match, >1 match, or a
  non-leading position → stay `Unresolved` (keep-all). The collision-recovery precision backstop
  is unchanged: #122 (A) still requires the FULL path to resolve to **exactly one** in-repo
  `Target::Item` before the disproof prunes.
- **P4 — known-valid source ⇒ no new visibility checker.** An existing `use other_crate::Foo`
  compiled, so `Foo` is visible from the using crate; we need only resolve it to the *right*
  single item. Visibility across the crate boundary is not separately enforced — soundness comes
  from P3's exactly-one-item requirement and the engine's existing per-segment `visible()` hook
  inside the target crate's module walk.
- **P5 — minimal surface.** One new serialized `ScopeGraph` field (already-built data), one
  `ResolutionPolicy` hook, one call site in `resolve_path_guarded`, one `CACHE_VERSION` bump. No
  change to `Builder`'s population logic, the disproof predicate, or #122 (A)'s consumer.

## 2. Design

### 2.1 Persist `crate_roots_by_name` into `ScopeGraph`

Add a serialized field to `ScopeGraph` (`src/name_resolution/graph.rs:77-109`), as a
backward-compatible `#[serde(default)]` map alongside `file_paths`:

```rust
    /// Rust: in-source crate name → that crate's `Root` scope, for resolving a
    /// 2018 bare-crate path leading segment (`use other_crate::X`, `other_crate::f()`)
    /// and `extern crate name`. Other languages leave this empty. Keys are
    /// hyphen→underscore normalized (the Rust path identifier form).
    #[serde(default)]
    pub crate_roots_by_name: std::collections::BTreeMap<String, ScopeId>,
```

Populate it at `Builder::finish()` (`src/name_resolution/rust_populator/builder.rs:101`):

```rust
    pub(crate) fn finish(mut self) -> ScopeGraph {
        self.graph.crate_roots_by_name = self.crate_roots_by_name;
        self.graph
    }
```

**Key normalization (correctness).** `crate_root_named` is keyed by `crate_name_for_root`
(`builder.rs:521-543`, derived from the workspace-member path / `<name>/src/lib.rs`); a Rust
path identifier uses underscores while a package/dir name may carry hyphens. Normalize keys to
the underscore form **at insertion** (where the Builder inserts at `builder.rs:283`) so both the
`extern crate` precedent and the new fallback match the path-identifier spelling. Lookups (§2.2)
normalize the query segment identically. Insertion-side normalization keeps a single source of
truth; the existing `extern crate` lookup reads the same normalized keys, and since an
`extern crate <ident>` identifier is already underscore-form, normalization is idempotent there
(it can only *add* matches for a previously hyphenated key — a latent improvement, never a
regression).

### 2.2 Leading-segment crate-root fallback (a `ResolutionPolicy` hook)

Add a `ResolutionPolicy` trait method, defaulting to `None` so only Rust opts in:

```rust
    /// A path's leading segment may name another crate (Rust 2018 bare-crate path
    /// root / `extern crate`). Returns that crate's `Root` scope iff `name`
    /// uniquely names an in-repo crate. Default: not applicable.
    fn extern_crate_root(&self, _graph: &ScopeGraph, _name: &str) -> Option<ScopeId> {
        None
    }
```

`RustPolicy` implements it by consulting the persisted map (normalized), returning `Some` only
on an exact single match:

```rust
    fn extern_crate_root(&self, graph: &ScopeGraph, name: &str) -> Option<ScopeId> {
        graph.crate_roots_by_name.get(&normalize_crate_ident(name)).copied()
    }
```

(`BTreeMap::get` returns one scope per key. Crate names are unique in a valid Cargo workspace,
so a single in-source name maps to a single root; the Builder's insertion is last-wins on the
(theoretical) duplicate, pre-existing behavior already relied on by the `extern crate` precedent.
A defensive "drop on duplicate insertion" guard is a possible follow-up but is out of scope —
this slice does not change population.)

Invoke it from `resolve_path_guarded` (`src/name_resolution/engine.rs:338-366`) **only for the
leading segment, only on `Unresolved`, after** the normal `scope_member_lookup`:

```rust
        let res = scope_member_lookup(graph, scope, &seg_q, policy, guard);
        // Leading-segment crate-root fallback (strictly last; P2/P3): a bare-crate
        // path root resolves to the owning crate's Root scope ONLY when the normal
        // member lookup found nothing (so a local item always shadows — P2) and the
        // segment uniquely names an in-repo crate (P3).
        let res = if i == 0 && matches!(res.status, ResStatus::Unresolved) {
            match policy.extern_crate_root(graph, seg) {
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

The recovered candidate is a single `Resolved` candidate whose `Target` is scope-bearing (its
`scope_of_target` is the crate `Root`), so the existing non-final branch (`engine.rs:357-361`)
sets `scope = Root` and the walk continues into the crate's modules with **no** other change.
For a bare leading segment that *is* the whole path (`use other_crate;` — a crate alias import),
`is_last` returns the `Root` candidate directly; the consumer treats it as a module target.

**Why the engine, not `scope_member_lookup`.** The fallback is leading-segment-specific (`i==0`)
and must run *after* the in-scope rib/glob tiers; `resolve_path_guarded` is the only place with
both the segment index and the post-lookup `Unresolved` status. `scope_member_lookup` stays a
pure single-scope member lookup (no positional knowledge).

### 2.3 Cache

`CACHE_VERSION` 17→18 (`src/cpg_cache.rs:60`) — the new serialized `ScopeGraph` field changes
the bincode layout. Update the pin test name/assertion to 18.

## 3. Soundness & recall-safety

- **Recall (P1).** The only behavioral change is: `resolve_path_guarded` returns `Resolved`
  (crate root) in some cases that previously returned `Unresolved`. Every consumer's
  keep-all/decline path is a superset of its resolve path, so no consumer can now drop an edge
  it previously kept solely because of this change. Concretely: #122 (A) only ever *adds* a
  prune when `pending_resolves_to_single_in_repo_item` flips `false→true`, and that flip
  requires the full path to resolve to exactly one in-repo item.
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
   `crate_roots_by_name` with both crate names → root scopes; a serialize→deserialize round-trip
   preserves it. Backward-compat: an old blob without the field deserializes to an empty map
   (`#[serde(default)]`).
2. **Resolver — cross-crate `use` resolves (unit, engine/resolver).** A two-crate fixture where
   crate `a` has `use b_crate::Foo;` resolves the leading `b_crate` to crate `b`'s root and the
   full path to `b`'s `Foo` (`Resolved`, one in-repo item). Pre-fix: `Unresolved`.
3. **Resolver — local shadow declines the fallback (unit; P2).** Crate `a` has a local
   `mod b_crate;` *and* a sibling crate named `b_crate`; `use b_crate::X` resolves to the LOCAL
   module (rib hit), never the crate root. Pins precedence.
4. **Resolver — non-crate leading segment stays `Unresolved` (unit; P3).** A leading segment
   matching no crate name and no local binding stays `Unresolved` (no spurious resolution).
5. **End-to-end collision recovery (integration, `resolution_test.rs`).** A pure-2018+ workspace
   (so #123's disproof runs) where crate `a` calls a same-name owner-`::` collision pinned by
   `use b_crate::Foo;` (the `Foo` collides bare with another crate's `Foo`); pre-fix keep-all
   (≥2 NameOnly), post-fix one Exact via the (A) disproof. Built through the real `load_repo` +
   `CallGraph::build_with_scope_graph_inputs` path.
6. **Cache pin (unit, `cpg_cache.rs`).** `CACHE_VERSION == 18`.

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
`crate_roots_by_name` already holds); direct crate-name leading segments in `use` paths and
expression paths; hyphen/underscore normalization.

**Deferred (follow-ups, not this slice):**
- **Renamed deps** (`bar = { package = "foo" }`): `use bar::X` needs the *consuming* crate's
  `[dependencies]` rename map (in-source name → package). Rare in workspaces; the `RustCrateConfig`
  has a stub field (`rust_populator/mod.rs:82`) but it is not threaded. Separate slice if the
  residue warrants.
- **Glob imports** (`use other_crate::*`): the +304 roadmap bucket — its own slice.

**Out of scope:** non-in-repo crates (registry/git/std) — they have no `Root` scope, so
`extern_crate_root` returns `None` and the path stays `Unresolved`/`External` exactly as today.
