# Cross-Crate `use` Resolution — Design

**Date:** 2026-06-21
**Status:** design-of-record — codex xhigh spec re-review folded — per-crate dependency-gated (owner-decided); 2nd-round MAJOR+BLOCKER addressed.
**Branch:** `cross-crate-use-resolution` (off `main` after PR #123 / edition anchoring-class merged)
**Predecessors:** #120 demote-not-drop · #121 scope-graph recovery · #122 (A) prune-through-`use` ·
#123 edition anchoring-class uniformity. This is the next slice on the
`docs/ruff-typepath-recovery-roadmap-2026-06-21.md` sequence (cross-crate **+428** → glob +304).

## 0. One-paragraph summary

prism's scope graph already builds a per-crate index `Builder::crate_roots_by_name:
BTreeMap<String, ScopeId>` (in-source crate name → its `Root` scope) and already uses it to
resolve `extern crate name;` (`walk/items.rs:160`). But that map is **ephemeral** (dropped at
`Builder::finish()`, `builder.rs:101`) and is **never consulted** when a 2018 `use other_crate::Foo`
(or `other_crate::foo()`) looks up its leading segment. So a path whose leading segment is a sibling
workspace crate resolves `Unresolved` (`engine.rs:408`), and #122 (A)'s
`pending_resolves_to_single_in_repo_item` declines → the collision is kept-all. A naïve fix
("persist `crate_roots_by_name` as a global name→root map and consult it") is **unsound**: Rust's
extern prelude is **per-consuming-crate** — `use b::Foo` is valid only if the consuming crate
actually depends on the in-repo crate `b`; a repo-global map would wrongly resolve `use b::Foo` to
an in-repo member `b` even when the consumer depends on an *external* `b` (or no `b`). So this slice
instead (1) **captures, per workspace member, that member's in-repo dependencies** (`[dependencies]`
PATH deps + WORKSPACE deps that resolve to an in-repo crate), (2) **persists a per-consuming-crate
dependency map** `crate_deps_by_root: BTreeMap<ScopeId, BTreeMap<String, ScopeId>>` (consuming
library `Root` → in-source dep name → depended-on in-repo library `Root`) into `ScopeGraph`, and
(3) **consults it as the strictly-last fallback** for a leading path segment via a new
`ResolutionPolicy` hook that now takes `from`, so a cross-crate `use`/call leading segment resolves
to the owning crate's `Root` scope **iff** the consuming crate declares an in-repo dependency under
that exact in-source name. Dependency targets are resolved **path-based** — a dep's resolved target
member directory → that member's library `Root` via a build-time `lib_root_by_member_dir` index
(library roots only) — so the existing `crate_roots_by_name` and the `extern crate` path are **left
unchanged** (not re-keyed, not persisted). There is no global `ambiguous_crate_names` set: each dep
resolves to one specific target root at build time. Because targets are library roots resolved by
directory (not by name), a normal lib+bin package does not self-collide and no `[package].name`
parsing is needed. The fallback
is narrowly gated (Rust 2018+ extern-prelude roots only — `UsePath`/`Bare` anchors, never
`crate::`/`self::`/`super::`/2015; a TRUE no-rib miss only, never a claimed-but-invisible local;
and only through the consuming crate's in-repo dep map — see §2.2). Recovers the **+428**
cross-crate collision sites on ruff (and resolves ordinary cross-crate calls for navigation).
Recall safety is **conditional** on the recovered resolution being correct and uniquely resolved
(§3) — it rests on that eligibility plus the dep-gating plus #122 (A)'s single-in-repo-item gate,
not on "adding a resolution cannot drop an edge" (consumers *can* prune on a newly-resolved path;
§3, MAJOR 2).

## 1. Premises (govern every requirement)

- **P1 — recall is conditional on a correct unique resolution.** The fallback fires **only** on a
  TRUE no-rib miss for the leading segment (§2.2), converting `Unresolved → Resolved`; at the
  engine call site it never changes or removes an existing resolution. But "adding a resolution"
  is **not** recall-safe by construction at the *consumer* level: a newly-`Resolved` path lets
  #122 (A)'s disproof prune a candidate pool (`resolution.rs:794`) and lets the general Rust
  scope-graph resolution early-return Exact/UnknownName ahead of older name/owner fallbacks
  (`resolution.rs:825`). Recall is therefore safe **iff** the recovered resolution is correct and
  uniquely resolved — which rests on the tightened eligibility (P2, the **per-consuming-crate
  dependency gate** P3, and the anchor/edition gate of §2.2) plus #122 (A)'s exactly-one-in-repo-item
  gate, **not** on the act of adding a resolution. The per-crate dep map (§2.1) is what makes the
  resolution **correct and unique**: it resolves a leading segment to an in-repo crate `Root` only
  when the consuming crate declares an in-repo dependency under that exact in-source name, and each
  such dependency was resolved (by path/workspace) to **one specific** target library root at build
  time — so there is no global name collision to adjudicate at query time. (Precision can *shift*
  NameOnly→Exact; that is the measured surface, §7.)
- **P2 — local items win (Rust precedence).** A path leading segment that matches a local
  binding/glob in the anchored scope is returned by `scope_member_lookup` *before* the fallback is
  reached: a **visible** local rib/glob yields a non-`Unresolved` resolution, and a
  **claimed-but-invisible** local rib yields `Unresolved` *with the rib-present flag set* (§2.2 /
  BLOCKER 1) — both shadow the crate name, because the fallback fires only on a TRUE no-rib miss
  (`!rib_present`). The fallback is the **last rung**, so a same-named local module/import always
  shadows a crate name, matching Rust 2018 resolution.
- **P3 — resolve only through the consuming crate's in-repo dependency map.** The fallback
  resolves a leading segment to a crate `Root` **iff** the consuming crate (the crate containing
  the use site) declares an in-repo dependency under that exact in-source name — i.e. the segment
  is present in `crate_deps_by_root[consuming_root]`. No such dependency, or a non-leading
  position → stay `Unresolved` (keep-all). This is sound by construction and replaces the prior
  fold's global-ambiguity machinery: each in-repo dependency was resolved at build time (by
  `[dependencies]` PATH / WORKSPACE path) to **one specific** target library `Root`, so the map
  value is a single root — there is no global name collision to detect or decline. Two distinct
  consequences fall out directly:
  - A workspace member named `b` is resolvable from crate `a` **only if** `a` actually depends on
    the in-repo `b` (extern prelude is per-crate). If `a` instead depends on an *external* `b`
    (version-only/git/registry), or does not depend on `b` at all, the segment is absent from
    `a`'s dep map → DECLINE → `Unresolved`/`External` (BLOCKER, this round).
  - Because dependency targets are resolved **path-based to library roots only** (a member's
    `target_dir` → its `src/lib.rs` Root via `lib_root_by_member_dir`, §2.1), a normal lib+bin
    package does **not** self-collide: its `src/main.rs`/bins/tests are never dependency targets,
    only its library root is (MAJOR, this round). Resolution is by directory, not by name, so no
    `[package].name` parsing or basename adjudication is needed at all.

  The collision-recovery precision backstop is unchanged: #122 (A) still requires the FULL path to
  resolve to **exactly one** in-repo `Target::Item` before the disproof prunes.
- **P4 — known-valid source ⇒ no new visibility checker.** An existing `use other_crate::Foo`
  compiled, so `Foo` is visible from the using crate; we need only resolve it to the *right*
  single item. Visibility across the crate boundary is not separately enforced — soundness comes
  from P3's per-consuming-crate dep gate (the leading segment binds to one specific in-repo crate
  root) plus the engine's existing per-segment `visible()` hook inside the target crate's module
  walk.
- **P5 — minimal surface.** **One** new serialized `ScopeGraph` field — the per-consuming-crate
  dependency map `crate_deps_by_root` (built at finish() from build-time data); one
  `ResolutionPolicy` hook (now taking `from`); one call site in `resolve_path_guarded` (threading
  the `from: ScopeId` it already holds + a `crate_root_of` ascent helper); one `CACHE_VERSION`
  17→18 bump. The `Builder`/`repo_loader` changes are: **per-member in-repo dependency capture**
  (repo_loader, §2.1), a build-time `lib_root_by_member_dir` index (member dir → its library `Root`,
  library roots only), and building `crate_deps_by_root` from those at `finish()`.
  `crate_roots_by_name` and the `extern crate` path are **left unchanged** (not re-keyed, not
  persisted). The prior fold's persisted `crate_roots_by_name` + `ambiguous_crate_names` fields are
  **dropped**. No change to the disproof predicate or #122 (A)'s consumer.

## 2. Design

### 2.1 Persist a per-consuming-crate dependency map into `ScopeGraph`

Add **one** serialized field to `ScopeGraph` (`src/name_resolution/graph.rs:77-109`), as a
backward-compatible `#[serde(default)]` map alongside `file_paths`:

```rust
    /// Rust: per consuming-crate library `Root` → (in-source dependency name →
    /// the depended-on in-repo library `Root`). Built from each member's
    /// `[dependencies]` PATH and WORKSPACE deps that resolve to an in-repo crate
    /// (external/registry/git deps excluded). The §2.2 fallback resolves a 2018+
    /// bare-crate leading segment ONLY through this per-crate map, so a crate can
    /// name another in-repo crate iff it actually depends on it (Rust's extern
    /// prelude is per-crate). Keys (dep names) are hyphen→underscore normalized.
    /// Other languages leave this empty.
    #[serde(default)]
    pub crate_deps_by_root:
        std::collections::BTreeMap<ScopeId, std::collections::BTreeMap<String, ScopeId>>,
```

`Builder::crate_roots_by_name` (`builder.rs:48`) is **left entirely unchanged** — it remains a
build-time, Builder-internal, directory-basename-keyed field backing the existing `extern crate`
resolution (`b.crate_root_named`, `builder.rs:79` → `walk/items.rs:160`). This slice does **not**
touch `extern crate` behavior, the prior fold's "persist it as a global name→root map" plan is
**dropped**, and there is no `ambiguous_crate_names` set. Instead, dependency-target resolution is
**path-based**: the per-crate map's values are found by mapping a dependency's resolved *target
member directory* to that member's library `Root` via a new build-time index
`lib_root_by_member_dir: BTreeMap<String /*member dir*/, ScopeId /*lib Root*/>` (step 3 below). No
name-keying, no `[package].name` parsing, and no extern-crate blast radius: this keeps P5 minimal
and resolves each dep to **one specific** target root at build time (no global-ambiguity decline to
model).

Persist the new field at `Builder::finish()` (`builder.rs:101`):

```rust
    pub(crate) fn finish(mut self) -> ScopeGraph {
        self.graph.crate_deps_by_root = self.crate_deps_by_root;
        self.graph
    }
```

**Current source limitation this slice removes.** `RustCrateConfig` (`mod.rs:71-96`) is
**repo-global / flat** — it carries `edition`, `crate_roots`, `workspace_members`, `lib_path`,
`bin_paths`, and a single repo-wide `dep_renames` map (`collect_dep_renames`, `repo_loader.rs:437`,
which captures only `package=`-renamed deps across `[dependencies]`/`[dev-]`/`[build-]`). It has
**no per-member dependency list**. So this slice ADDS per-member dependency capture. (The decided
design states the design level; the plan finalizes exact TOML parsing.)

1. **Per member M** (a workspace member dir, or the single-crate root when there is no workspace):
   identify M's **library root file** (`lib_path`, or `<M>/src/lib.rs` by convention) and record
   `lib_root_by_member_dir[M_dir] = <that library Root scope>`. Members with **no library root are
   not dependency targets** — skip them; a bin-only crate is not `use`-nameable as a library. (No
   `[package].name` is needed — both the consuming side and the target side are keyed by member
   directory, resolved to library roots by path.)
2. **Parse M's `[dependencies]`** (v1: `[dependencies]` only; dev-/build-dependencies deferred —
   §8). For each entry `(in_source_name, spec)`:
   - `spec.path = "..."` → target member dir = `normalize(join(M_dir, path))`; in-repo. Record
     `(M, in_source_name → target_dir)`.
   - `spec.workspace == true` → resolve via the **workspace-root** manifest's
     `[workspace.dependencies][in_source_name]`; if that entry has a `path`, the target member dir
     is that path; in-repo. Record. (ruff uses this form heavily — it is **required** for the +428.)
   - otherwise (version-only / git / registry, no in-repo path) → **EXTERNAL**; do not record. The
     §2.2 hook then declines for that name → stays `Unresolved`/`External` (correct).
   - `spec.package = "..."` (rename) is handled **naturally**: the KEY is `in_source_name` (what
     `use` writes); the TARGET is resolved by path/workspace, not by package name. So a renamed
     in-repo dep resolves correctly — this **subsumes** the previously-deferred renamed-deps case.
3. **Builder.** Using the `lib_root_by_member_dir` index from step 1 (library roots only — so a
   member's `src/main.rs`/bins/tests are never dependency targets, the lib+bin-no-self-collide fix),
   for each consuming member M with library root `Rc = lib_root_by_member_dir[M_dir]`, and each
   recorded `(in_source_name → target_dir)`, set
   `crate_deps_by_root[Rc][normalize(in_source_name)] = lib_root_by_member_dir[target_dir]` (skip a
   target with no library root). Persist `crate_deps_by_root` at `finish()`. `crate_roots_by_name`
   and `extern crate` are not involved.

**Key normalization.** Dependency names (the keys of each per-crate map) are normalized
hyphen→underscore to the Rust path-identifier form (a Cargo dependency name may carry hyphens
while `use` writes underscores). The §2.2 hook normalizes the query segment identically via the
same `normalize_crate_ident` helper (introduced by this slice — see §2.2). Only the
`crate_deps_by_root` keys and the hook's query segment are normalized; `crate_roots_by_name` and
the `extern crate` path are untouched.

### 2.2 Leading-segment crate-root fallback (a `ResolutionPolicy` hook)

Add a `ResolutionPolicy` trait method, defaulting to `None` so only Rust opts in. The hook
**must** receive both the path `Anchor` and `from` (the query origin scope): the anchor gates on
kind + edition (a sibling-crate fallback is valid only for a 2018+ extern-prelude *root* —
`UsePath`/`Bare`, never `crate::`/`self::`/`super::`/`::`), and `from` identifies the **consuming
crate** so the per-crate dep gate (P3) can be applied — Rust's extern prelude is per-crate. A
`(graph, name)` signature is too broad on both axes (it would fire after *every* anchor and would
ignore which crate is consuming):

```rust
    /// A path's leading segment may name a depended-on in-repo crate (Rust 2018+
    /// extern-prelude root). Returns that crate's library `Root` iff the anchor is
    /// an extern-prelude root kind AND the CONSUMING crate (containing `from`)
    /// declares an in-repo dependency under `name`. `from` is needed because the
    /// extern prelude is per-crate. Default: not applicable.
    fn extern_crate_root(
        &self,
        _graph: &ScopeGraph,
        _name: &str,
        _anchor: &Anchor,
        _from: ScopeId,
    ) -> Option<ScopeId> {
        None
    }
```

`RustPolicy` implements it by **gating on edition + anchor kind first**, then climbing to the
consuming crate's `Root` and consulting only that crate's in-repo dependency map (normalized):

```rust
    fn extern_crate_root(
        &self,
        graph: &ScopeGraph,
        name: &str,
        anchor: &Anchor,
        from: ScopeId,
    ) -> Option<ScopeId> {
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
        // P3 (per-crate dep gate): resolve `name` ONLY through the consuming
        // crate's in-repo dependency map. The extern prelude is per-crate, so a
        // crate can name another in-repo crate iff it actually depends on it.
        let consuming_root = crate_root_of(graph, from)?; // climb `parent` links to the enclosing Root
        graph
            .crate_deps_by_root
            .get(&consuming_root)?
            .get(&normalize_crate_ident(name))
            .copied()
    }
```

`crate_root_of(graph, from)` walks `graph.scope(id).parent` (equivalently `graph.parent_of(id)`,
`graph.rs:148`) up to the `ScopeKind::Root` ancestor (`types.rs:170`) and returns it — a small
ascent helper this slice adds; the plan finalizes it. `normalize_crate_ident` is the
hyphen→underscore identifier normalizer shared with §2.1's key build (also added by this slice;
there is no existing normalization helper in `name_resolution`/`repo_loader`).

(`AnchorKind` and `Anchor` are `name_resolution::types`; `resolve_path_guarded` already holds the
`&Anchor` **and** the `from: ScopeId` it was called with — `engine.rs:320-321` — so threading both
into the hook costs nothing. Each per-crate dep map value is a single `ScopeId` resolved at build
time, so the lookup is unambiguous by construction; an unknown name, or a name the consuming crate
does not depend on in-repo, returns `None` → decline.)

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
        // the policy's anchor/edition gate + the consuming crate's per-crate
        // in-repo dependency gate pass (P3). Poison/empty-glob `Unresolved` with a
        // claimed rib does NOT qualify. `from` (the query origin) is threaded so the
        // policy can identify the consuming crate.
        let res = if i == 0
            && !rib_present
            && matches!(res.status, ResStatus::Unresolved)
        {
            match policy.extern_crate_root(graph, seg, anchor, from) {
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

`CACHE_VERSION` 17→18 (`src/cpg_cache.rs:60`) — the new serialized `ScopeGraph` field
(`crate_deps_by_root`) changes the bincode layout. The cache uses **bincode** and `deserialize`s
the blob *before* the version check (`cpg_cache.rs:304` vs `:310`), so an old (field-less) blob
either fails to deserialize → `CacheResult::Miss` (`cpg_cache.rs:306`) or passes the version check
→ `Miss` (`cpg_cache.rs:310-315`); it is **not** silently serde-defaulted into a live build. The
version bump is what guarantees the rebuild. Update the pin test name/assertion to 18.

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
  (ii) the anchor/edition eligibility gate (BLOCKER 2), (iii) the **per-consuming-crate in-repo
  dependency gate** (P3) — the segment resolves only through `crate_deps_by_root[consuming_root]`,
  built-time-resolved to one specific target library root — and (iv) #122 (A)'s
  exactly-one-in-repo-item gate — **not** on the premise that adding a resolution cannot drop an
  edge. When a recovered resolution is wrong or the consuming crate does not declare the
  dependency, those gates fail closed to keep-all; when it is right, the pruned candidates were
  genuine FPs. The dep-gate makes the fallback sound for **both** surfaces below: it binds to an
  in-repo crate only when the consuming crate declares an in-repo dependency under that exact
  in-source name, so an external-name collision (consumer depends on an external `b`, an unrelated
  in-repo member is also named `b`) correctly **declines** (external `b` is not in the consumer's
  in-repo dep map).
- **Precision — the collision-recovery case.** The disproof prunes a same-name owner-`::`
  collision to one Exact only when the cross-crate `use` resolves to **exactly one** in-repo
  `Target::Item{owns:Some(scope)}` (unchanged (A) requirement). A wrong single resolution would
  be a wrong prune; the per-crate dep gate (P3 — the leading segment binds to the one in-repo crate
  the consumer actually depends on under that name) + the single-item requirement + P4 (the `use`
  is real, compiled source) make a wrong resolution require a genuinely ambiguous in-repo program,
  which the single-item gate rejects → keep-all.
- **Precision — the general (navigation) case.** A cross-crate call `other_crate::foo()` that
  was `Unresolved`/NameOnly may now resolve Exact. If the crate-root + module walk lands on the
  wrong item, that is a precision shift (not a recall loss). The per-crate dep gate (P3 — resolves
  to the consumer's declared in-repo dependency, one specific root), local-shadow precedence (P2),
  and the engine's existing per-segment `visible()` + non-singular fall-through
  (`engine.rs:362-364`, "anything non-singular falls through, never wrong") bound this. §7's
  Tier-A + ruff M2 + nav spot-check measure it directly (the blast-radius the design deliberately
  opted into).
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

1. **Persist round-trip (unit, `graph.rs`/cache).** A built two-crate workspace where crate `a`
   depends (path dep) on crate `b` exposes `crate_deps_by_root` with `a`'s library root mapping
   the in-source dep name → `b`'s library root; a freshly-serialized graph round-trips the field.
   Cross-version compat is NOT serde-default (the cache is bincode and deserializes before the
   version check — `cpg_cache.rs:304`/`:310`): an old field-less blob is handled by the
   `CACHE_VERSION` 17→18 bump (cache miss → rebuild), not by silently defaulting into a live build.
   Keep `#[serde(default)]` on the field (matches the existing `edition_uniform`/`file_paths`
   convention so an in-memory or named-format round-trip stays robust); a separate named-format
   (e.g. JSON) test, if wanted, may assert the default-on-missing behavior in isolation.
2. **Resolver — cross-crate `use` resolves when the consumer declares the dep (unit,
   engine/resolver; path-dep form).** A two-crate fixture where crate `a`'s manifest has
   `b_crate = { path = "../b" }` and `a` has `use b_crate::Foo;` resolves the leading `b_crate` to
   crate `b`'s library root and the full path to `b`'s `Foo` (`Resolved`, one in-repo item).
   Pre-fix: `Unresolved`.
3. **Resolver — workspace-dependency form resolves (unit).** Crate `a` declares
   `b_crate = { workspace = true }` and the workspace-root manifest has
   `[workspace.dependencies] b_crate = { path = "../b" }`; `use b_crate::Foo;` resolves to `b`'s
   library root (the ruff-heavy form — required for the +428).
4. **Resolver — renamed in-repo dep resolves (unit).** Crate `a` declares
   `bar = { package = "foo", path = "../foo" }`; `use bar::X` resolves to `foo`'s library root
   (the KEY is the in-source name `bar`; the TARGET resolves by path). Subsumes the
   previously-deferred renamed-deps case.
5. **Resolver — local shadow declines the fallback (unit; P2).** Crate `a` has a local
   `mod b_crate;` *and* declares a dep on a sibling crate named `b_crate`; `use b_crate::X`
   resolves to the LOCAL module (rib hit), never the crate root. Pins precedence.
6. **Resolver — claimed-but-invisible local blocks the fallback (unit; BLOCKER 1).** Crate `a`
   has a local rib binding for `b_crate` that is *not visible* at the use site (e.g. a
   `pub(in ...)`/private module member that `resolve_rib` claims but rejects on `visible()`),
   *and* declares a dep on a sibling crate `b_crate`. The leading segment must stay the LOCAL
   outcome (`Unresolved` via claimed-but-invisible — `rib_present == true`), **never** the crate
   root. This is the rib-claim probe's discriminating case (distinct from test 5's visible rib hit).
7. **Resolver — external-vs-in-repo same-name DECLINES (unit; the key soundness test).** Crate `a`
   depends on an **external** `b` (version-only, e.g. `b = "1.0"`) while an unrelated in-repo
   workspace member is also named `b`; `use b::X` stays `Unresolved`/`External` (the external `b`
   is not in `a`'s in-repo dep map → decline). This is the per-crate dep gate's discriminating
   case (BLOCKER, this round).
8. **Resolver — non-dependency same-name member DECLINES (unit; P3).** An in-repo crate `b` exists
   but crate `a` does NOT declare any dependency on it; `use b::X` from `a` **declines** (stays
   `Unresolved`) — the extern prelude is per-crate.
9. **Resolver — anchor-kind gate (unit; BLOCKER 2).** With crate `a` depending on a sibling crate
   `b_crate`, leading segments under `crate::b_crate::X`, `self::b_crate::X`, and
   `super::b_crate::X` must **not** trigger the fallback (those anchor inside the current crate —
   `CrateRoot`/`SelfMod`/`Super`); only `use b_crate::X` / bare `b_crate::f()` (`UsePath`/`Bare`)
   may. `::b_crate::X` (`LeadingColon`) is excluded in v1 (asserts no fallback — deferred, §8).
10. **Resolver — 2015 sibling without `extern crate` stays `Unresolved` (unit; BLOCKER 2).** In a
    2015-edition crate `a` (declaring a dep on `b_crate`), `use b_crate::X;` **without** an
    `extern crate b_crate;` binding stays `Unresolved` (the bare fallback must not invent a 2015
    crate root — `is_2018_plus()` gate). A companion positive: with `extern crate b_crate;`, the
    existing map-backed binding (`walk/items.rs:160`) still resolves it (unchanged precedent).
11. **Resolver — lib+bin package does not self-poison (unit; MAJOR, this round).** A single member
    `b` with `src/lib.rs` **and** `src/main.rs` (plus optionally a `[[bin]]`/test root), depended on
    by a consuming crate `a` via a path/workspace dep. `use b::X` from `a` resolves to `b`'s
    **library** root (`lib_root_by_member_dir` indexes only the library root), never the bin root.
12. **End-to-end collision recovery (integration, `resolution_test.rs`).** A pure-2018+ workspace
    (so #123's disproof runs) where crate `a` **declares a path/workspace dep on** crate `b` and
    calls a same-name owner-`::` collision pinned by `use b_crate::Foo;` (the `Foo` collides bare
    with another crate's `Foo`); pre-fix keep-all (≥2 NameOnly), post-fix one Exact via the (A)
    disproof. Built through the real `load_repo` + `CallGraph::build_with_scope_graph_inputs` path.
13. **Cache pin (unit, `cpg_cache.rs`).** `CACHE_VERSION == 18`.

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

**In scope (v1):** per-consuming-crate in-repo dependency capture from `[dependencies]` — PATH
deps (`b = { path = "../b" }`) and WORKSPACE deps (`b = { workspace = true }` + the workspace-root
`[workspace.dependencies] b = { path }`); renamed in-repo deps (`bar = { package = "foo", path }`),
handled naturally because the dep map keys on the in-source name and resolves the target by path;
path-based target resolution to library roots only (`lib_root_by_member_dir`); direct crate-name
leading segments in `UsePath`/`Bare` positions (`use other_crate::X`, `other_crate::f()`);
hyphen/underscore normalization of dep names.

**Deferred (follow-ups, not this slice):**
- **dev-/build-dependencies** (`[dev-dependencies]`/`[build-dependencies]`): test and build-script
  consuming code can name in-repo crates declared only under those tables. v1 parses
  `[dependencies]` only; capturing the dev-/build- tables (and gating which consuming roots see
  them) is a separate slice if the residue warrants.
- **Non-library consuming roots** (bin/test/bench/example roots): v1 builds `crate_deps_by_root`
  for library consuming roots; a `use other_crate::X` in a member's `main.rs`/integration test
  resolves only once its consuming root is keyed. Separate refinement.
- **`LeadingColon` roots** (`::other_crate::X`): the `LeadingColon` anchor resolves via
  `anchor.prelude` and returns `None` when no prelude scope is recorded (`rust_policy.rs:286-291`),
  so v1 deliberately excludes it from the crate fallback (the eligibility gate admits only
  `UsePath`/`Bare`). Re-enabling it (or wiring the populator's prelude scope) is a separate
  follow-up; rare in workspace code, and the bare/`use` forms carry the +428.
- **Glob imports** (`use other_crate::*`): the +304 roadmap bucket — its own slice.

**Out of scope:** external/registry/git deps and std/core/alloc — they resolve to no in-repo
library `Root`, so they are never recorded in `crate_deps_by_root`, the hook returns `None`, and
the path stays `Unresolved`/`External` exactly as today (this is the correct decline, and the
soundness backbone of the per-crate dep gate).
