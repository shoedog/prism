# ruff type-path owner-collision recovery — sized roadmap (2026-06-21)

Sequencing doc for recovering ruff's same-name owner-`::` collisions (the
`qualified_owner` NameOnly residue after #120 demote-not-drop / #121 scope-graph
recovery / #122 prune-through-`use`). Numbers are measured `prism nav call-stats`
deltas on `~/code/bench-repos/ruff` (pinned), via throwaway spikes; not yet shipped.

## Root cause of ruff's 0 scope-graph recovery (verified)

ruff is a **mixed-edition workspace**: some crates pin `edition = "2021"`, others
inherit `edition = { workspace = true }` → `2024`. prism models edition as one
global value + an `edition_uniform` bool, and `ScopeResolution::disproves`
(`src/resolution.rs:1337`) bails keep-all on `!graph.edition_uniform`. So on ruff
the **entire** scope-graph disproof (#121 ②B + #122 (A)) is gated off → 0 recovery.

Two compounding bugs:
1. `parse_rust_crate_config` reads `package.edition` via `.as_str()`, which is
   `None` for the `{ workspace = true }` table → `unwrap_or(2015)`
   (`src/repo_loader.rs:303-308`). No `[workspace.package]` inheritance handling.
   → workspace-inherited crates are mis-detected as 2015.
2. Even with (1) fixed, ruff is `{2021, 2024}` — not identical — so the strict
   `edition_uniform = (editions_seen.len() <= 1)` still bails. But prism's anchor
   logic only branches at the **2015 vs 2018+** boundary (`rust_policy.rs:82`,
   `is_2018_plus`); 2018/2021/2024 are anchoring-identical, so the bail is too
   strict for a pure-2018+ mix.

## Slice sequence (each independently shippable + measurable)

### 1. Edition slice — PREREQUISITE — measured **+260**, precision-safe
Parse `edition = { workspace = true }` inheritance (resolve from
`[workspace.package] edition`) **and** relax `edition_uniform` to anchoring-class
uniformity (all-2015 **or** all-2018+) instead of all-identical. Both are required
together (parse fix alone leaves `{2015,2021}` spanning the boundary; relax alone
leaves the inherited crates mis-detected as 2015). Spike forcing `edition_uniform=
true` measured: `recovery_typepath.singleton` 0→260, `failopen_demote` 1586→1326,
`kind_exact[qualified_owner]` +260, `kind_nameonly[qualified_owner]` −1092 (wrong
edges pruned), `multi_target_exact_sites` 46→46 (**zero new collision FPs**). Sound:
within the 2018+ class every crate anchors identically, so no wrong-edition
mis-resolution. Likely a `CACHE_VERSION` bump (recomputed `edition_uniform`).
**This gates everything below — without it the disproof never runs on any
mixed-edition workspace.**

### 2. Cross-crate `use` resolution — follow-on — sized **+428**
The largest residue bucket (`pending_unresolved_crosscrate_candidate` = 428 of the
1326): `use sibling_crate::Foo; Foo::m()` where `sibling_crate` is a workspace
member the scope graph doesn't link (`from_convention` leaves `workspace_members`
unnamed at resolve time; `crate_roots_by_name` is `Builder`-only, not in
`ScopeGraph`). Resolving the crate-name leading segment to its root scope (Rust
2018 extern-prelude semantics) lets the (A) Pending arm recover these. Needs the
crate-name→root map persisted into `ScopeGraph` (schema + cache change). This is
the originally-greenlit "cross-crate" idea, correctly sized (428, not 1,586).

### 3. Glob-import resolution — follow-on — sized **+304**
`empty_rib_or_glob` = 304: the leading type is glob-brought (`use crate::a::*;`)
so there is no direct binding for the leading segment. Resolving glob edges for the
leading-segment lookup would recover these.

## Not recoverable / leave alone (~594 of the 1326)
- `block_local_shadow` 316 — **correct** keep-alls (a block-local shadow of the
  type name makes the disproof non-authoritative; recall-safety, P1). Do not touch.
- `pending_poisoned` 161 — the import resolves to a poisoned/conflicting state.
  Hard, low value.
- `downstream_no_callable_edge` 82 + `downstream_empty_id_set` 4 — the owner type
  resolves, but prism has no callable edge / empty id-set for the method. A
  separate method-resolution lever, not collision-specific.
- `direct_non_item` 19 + `pending_resolved_nonitem_or_multi` 10 +
  `pending_unresolved_other` 2 — minor/edge.

## Totals (of the original 1,586 type-path collisions on ruff)
- Recoverable, sequenced: 260 (edition) + 428 (cross-crate) + 304 (glob) ≈ **992**.
- Correct keep-all / hard / out-of-scope: ≈ **594**.

## Measurement method (reproducible)
`prism nav --no-cache call-stats --repo <ruff>`; baselines vs spikes built in
throwaway worktrees. The 1326 breakdown came from a `failopen_demote_reason`
classifier mirroring `disproves`/`leading_segment_binds_directly` (separating
leading-segment failures from downstream method-resolution failures). The
classifier and the `edition_uniform=true` force were throwaway probes, not shipped.
