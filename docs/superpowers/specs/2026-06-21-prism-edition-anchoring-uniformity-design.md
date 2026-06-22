# Edition Anchoring-Class Uniformity — Design

**Date:** 2026-06-21
**Status:** design-of-record
**Branch:** `edition-anchoring-uniformity` (off `main` @ `64a0b1e`, post-#122)
**Roadmap:** [`docs/ruff-typepath-recovery-roadmap-2026-06-21.md`](../../ruff-typepath-recovery-roadmap-2026-06-21.md)

## Problem

The scope-graph disproof predicate that powers same-name owner-`::` collision
recovery (#121 ②B + #122 (A)) bails keep-all on a non-uniform-edition workspace:

```rust
// src/resolution.rs:1335-1339  (ScopeResolution::disproves)
// §2 guard: a non-uniform-edition workspace is non-authoritative for
// disproof. Keep-all.
if !graph.edition_uniform {
    return false;
}
```

`edition_uniform` is computed as *all editions identical*:

```rust
// src/repo_loader.rs:359  (parse_rust_crate_config)
cfg.edition_uniform = editions_seen.len() <= 1;
```

This gates the **entire** disproof off on any mixed-edition workspace. ruff is the
exemplar: it is a 50-crate workspace where some crates pin `edition = "2021"` and
the rest inherit `edition = { workspace = true }` (→ `2024` from
`[workspace.package] edition`). prism therefore recovers **0** collision sites on
ruff — both #121 and #122 are dead there (verified: `recovery_typepath.singleton`
is absent on ruff; `failopen_demote` = 1586).

Two compounding defects:

1. **Workspace edition inheritance is unparsed.** `parse_rust_crate_config` reads
   `package.edition` via `.as_str()`, which returns `None` for the
   `{ workspace = true }` **table**, so it falls back to `unwrap_or(2015)`
   (`src/repo_loader.rs:303-308`). There is no `[workspace.package]` handling.
   → workspace-inherited crates are mis-detected as edition **2015**.

2. **`edition_uniform` is stricter than the anchoring model needs.** prism's path
   anchoring branches **only** at the 2015↔2018+ boundary:

   ```rust
   // src/name_resolution/rust_policy.rs:82-83
   fn is_2018_plus(&self) -> bool { self.edition >= 2018 }
   ```

   The doc comment states this is "the only edition split Phase 1 needs". So
   2018/2021/2024 are **anchoring-identical**; a workspace mixing only 2018+
   editions has uniform anchoring, yet `len() <= 1` still marks it non-uniform.

Even fixing (1) alone leaves ruff at `{2021, 2024}` — not identical — so it still
bails. Fixing (2) alone leaves the inherited crates at `{2015, 2021}` — which
genuinely *spans* the anchoring boundary, so it correctly still bails. **Both fixes
are required together** to unblock ruff (and every pure-2018+ mixed workspace).

## §1 Goal & premises

**Goal.** Let the scope-graph disproof run on **pure-2018+** mixed-edition
workspaces, recovering the collision sites it already knows how to recover, without
weakening recall on genuinely anchoring-mixed (2015↔2018+) workspaces.

- **P1 — recall-safety is inherited, not weakened.** This slice does **not** touch
  the disproof. It only widens *when the unchanged, already-recall-safe disproof is
  permitted to run*, from "all editions identical" to "all resolved per-crate
  editions **and** all discovered workspace editions are on the same side of the
  2015/2018 anchoring boundary". Any workspace (or set of workspaces) that spans
  the boundary still bails (keep-all) exactly as today; the workspace-edition set
  term makes this hold even for a repo with multiple workspace roots that prism
  collects into one global manifest set (§3).
- **P2 — soundness of the widening.** Within the 2018+ anchoring class every crate
  resolves paths identically, so a single global policy edition (any 2018+ value)
  anchors every call site correctly. The wrong-edition mis-anchor the §2 guard
  exists to prevent **cannot occur** inside one anchoring class. See §3.
- **P3 — minimal surface.** No `ScopeGraph` schema change (`edition`,
  `edition_uniform` are already fields); only their *computed values* change, plus
  a `CACHE_VERSION` bump so warm caches recompute.

## §2 The change

Two edits in `src/repo_loader.rs::parse_rust_crate_config` (a workspace-edition
pre-scan that yields a `BTreeSet`, and a two-term `edition_uniform` computation),
plus a cache bump and two source-comment updates.

### §2.1 Parse `edition = { workspace = true }` inheritance

Add a pre-scan over the same `manifest_hashes` for the workspace root's
`[workspace.package] edition`, then resolve the table form against it. The
pre-scan must collect **every** discovered `[workspace.package] edition` into a
`BTreeSet<u16> workspace_editions` (not just a last-wins scalar), because prism
collects *all* `Cargo.toml` repo-wide into one flat `manifest_hashes`
(`collect_manifest_hashes_inner` recurses through every non-skipped subdir,
`src/repo_loader.rs:186-213`) and `parse_rust_crate_config` iterates that single
global set (`src/repo_loader.rs:287`). A repo can therefore contain **more than
one** workspace root (nested or sibling workspaces), and those roots may sit on
opposite anchoring sides. The full set is what the §2.2 guard needs (see the
soundness argument below); a representative scalar is still kept only to resolve
the `{ workspace = true }` value form.

```rust
// Pre-scan (before the existing per-manifest loop): collect ALL workspace
// editions, plus a representative for resolving `{ workspace = true }`.
let mut workspace_editions: std::collections::BTreeSet<u16> = std::collections::BTreeSet::new();
let mut workspace_edition: Option<u16> = None; // representative (last-wins)
for manifest_path in manifest_hashes.keys() {
    let abs = root.join(manifest_path);
    let Ok(text) = std::fs::read_to_string(&abs) else { continue; };
    let Ok(value) = text.parse::<toml::Value>() else { continue; };
    if let Some(ed) = value
        .get("workspace").and_then(|w| w.get("package"))
        .and_then(|p| p.get("edition"))
        .and_then(|e| e.as_str()).and_then(parse_edition)
    {
        workspace_editions.insert(ed);
        workspace_edition = Some(ed);
    }
}
```

In the existing per-manifest `[package]` edition block, resolve the table form:

```rust
// src/repo_loader.rs:301-311  (replacement)
if value.get("package").is_some() {
    // Cargo default: a `[package]` with no `edition` key is edition 2015.
    let pkg_ed = value.get("package").and_then(|p| p.get("edition"));
    let edition = pkg_ed
        .and_then(|e| e.as_str())
        .and_then(parse_edition)
        .or_else(|| {
            // `edition = { workspace = true }` -> the workspace root edition.
            if pkg_ed
                .and_then(|e| e.get("workspace"))
                .and_then(|w| w.as_bool())
                .unwrap_or(false)
            {
                workspace_edition
            } else {
                None
            }
        })
        .unwrap_or(2015);
    cfg.edition = edition;
    editions_seen.insert(edition);
}
```

Semantics: a string edition parses as today; `{ workspace = true }` resolves to the
workspace edition; any other shape (or `{ workspace = true }` with no
`[workspace.package] edition` found — a malformed manifest Cargo itself would
reject) falls back to 2015 (today's lenient behavior). `editions_seen` therefore
records the **resolved** editions, so the §2.2 check sees the true set.

The pre-scan is a deliberate separate pass because the workspace-root manifest may
sort after the inheriting crates in `manifest_hashes` (a `BTreeMap`), so a single
forward pass could read `{ workspace = true }` before the root edition is known.

If more than one manifest declares `[workspace.package] edition` (nested or
sibling workspaces in one repo — uncommon but real, since `manifest_hashes` is a
single repo-wide set), the representative scalar `workspace_edition` is last-wins
in sorted order. This scalar is used **only** to fill in the `{ workspace = true }`
value form, so a wrong-workspace last-wins value can mis-record an individual
crate's edition in `editions_seen`. That mis-record is harmless **because the
§2.2 guard does not rely on it**: the guard ANDs in
`anchoring_class_uniform(&workspace_editions)` over the full set of discovered
workspace editions, so as soon as two workspaces span the 2015/2018 boundary the
guard bails (keep-all) regardless of which representative was chosen and
regardless of how the `{ workspace = true }` crates were resolved. The earlier
"last in sorted order only *tightens* the check" claim was **wrong** — a
cross-boundary last-wins value can elevate a 2015 workspace's `{ workspace = true }`
crates to a 2018+ value in `editions_seen`, which is exactly the mis-anchor the
guard must defend against; the `workspace_editions` set-term is what closes that
hole (§3).

### §2.2 Relax `edition_uniform` to anchoring-class uniformity

Replace the all-identical test with a **two-term** same-side-of-2018 test over
both the resolved per-crate editions **and** the full set of discovered workspace
editions:

```rust
// src/repo_loader.rs:359  (replacement)
cfg.edition_uniform =
    anchoring_class_uniform(&editions_seen) && anchoring_class_uniform(&workspace_editions);
```

```rust
/// True iff every observed edition is on the same side of the 2015/2018 path-
/// anchoring boundary (`RustPolicy::is_2018_plus`), i.e. all >= 2018 or all < 2018.
/// An empty set is vacuously uniform (matches the prior `len() <= 1` for empty).
fn anchoring_class_uniform(editions: &std::collections::BTreeSet<u16>) -> bool {
    editions.iter().all(|&e| e >= 2018) || editions.iter().all(|&e| e < 2018)
}
```

The second term is what makes the relaxation recall-safe under prism's single
repo-wide manifest set. `editions_seen` alone is **not** sufficient: a
multi-workspace repo can mis-resolve a 2015 workspace's `{ workspace = true }`
crates to a later workspace's 2018+ edition (the representative-scalar mis-record
described in §2.1), which would falsely make `editions_seen` all-2018+. ANDing in
`anchoring_class_uniform(&workspace_editions)` defends against this directly: if
**any two** workspaces sit on opposite sides of the boundary, the set
`workspace_editions` spans it, `anchoring_class_uniform` returns `false`, and the
whole guard bails (keep-all) — independent of how `editions_seen` was resolved.
`workspace_editions` is the SET of all discovered `[workspace.package] edition`
values (a `BTreeSet`), **not** a last-wins scalar, precisely so this term cannot
be defeated by sort order. For the common single-workspace case
`workspace_editions` is a singleton (or empty for a convention/no-`[workspace]`
repo), so the second term is vacuously true and the behavior is exactly the
single-term relaxation. See §3 for the full case analysis.

The global `cfg.edition` value is unchanged in mechanism (last-wins over the
sorted `manifest_hashes` order, so deterministic). When the class is all-2018+, the
last-wins value is necessarily ≥ 2018, so `is_2018_plus()` is correct for every
site; the exact value (2021 vs 2024) is immaterial because anchoring only reads the
boundary (§3). No change to edition selection is needed.

**Source-comment update (mechanical, at implementation time).** The two
`edition_uniform` field doc comments currently define it as the manifests having
"agreed on one edition" / "a single edition". When implementing, update both to
state the new meaning — "same anchoring class (2015 vs 2018+)" — at
`src/name_resolution/rust_populator/mod.rs:89` and `src/name_resolution/graph.rs:89`.
(Comment-only; behavior is governed by the §2.2 code.)

### §2.3 Cache invalidation

`edition_uniform` is a serialized `ScopeGraph` field; a repo cached under the old
computation would serve a stale `false`. Bump `CACHE_VERSION` 16 → 17
(`src/cpg_cache.rs:60`) and update the pin test (`src/cpg_cache.rs:567`).

## §3 Soundness / recall-safety

The §2 guard (`!edition_uniform → keep-all`) exists because the disproof anchors
paths with one global edition; if crates genuinely disagreed on anchoring, a
wrong-edition anchor could mis-resolve a path and **drop a real edge** (P1). The
relaxation is sound because:

- prism's anchoring has exactly **two** classes: 2015 (crate-root-relative `use`
  roots, `::x` crate-rooted) and 2018+ (lexical / extern-prelude), per
  `rust_policy.rs:276-288`. There is no 2021- or 2024-specific anchoring.
- Within one class, *every* crate anchors identically. A single global policy
  edition drawn from that class (any 2018+ value for the 2018+ class) yields the
  same anchor decisions a per-crate-correct edition would. So no path is
  mis-anchored, and no real edge is dropped.
- A workspace that spans the boundary (`{2015, 2021}`) is **not** same-side →
  `edition_uniform = false` → bails exactly as today. The two existing guard tests
  (`mixed_edition_workspace_is_not_uniform`,
  `omitted_plus_explicit_edition_workspace_is_not_uniform`, both `{2015, 2021}`)
  are preserved unchanged.

**Multi-workspace, cross-boundary repo (the recall-safety case the two-term AND
exists for).** prism collects *all* `Cargo.toml` repo-wide into one
`manifest_hashes` (`src/repo_loader.rs:186-213`, recursive) and
`parse_rust_crate_config` iterates that single global set
(`src/repo_loader.rs:287`). A repo can therefore hold two (nested or sibling)
workspaces on opposite anchoring sides — e.g. workspace `aaa/` with
`[workspace.package] edition = "2015"` and workspace `zzz/` with
`[workspace.package] edition = "2024"`, each with members that inherit via
`edition = { workspace = true }`. With a single last-wins representative
`workspace_edition` (`zzz`'s 2024, since it sorts later), `aaa`'s
`{ workspace = true }` crates would be **mis-resolved to 2024** in
`editions_seen`, making the single-term `anchoring_class_uniform(&editions_seen)`
falsely read all-2018+ → the disproof would run across a genuine 2015/2018+
boundary → a 2015 crate's path could be anchored with 2018+ semantics →
mis-resolve → **drop a real edge** (a P1 violation). The two-term AND closes this:
`workspace_editions = {2015, 2024}` is collected as a *set* over all discovered
`[workspace.package] edition` values, so `anchoring_class_uniform(&workspace_editions)`
returns `false` and the guard bails (keep-all) — regardless of the representative
or how the inheriting crates resolved.

**Why there is no residual hole (every 2015 source is caught by one of the two
terms).** A genuine 2015 anchoring requirement in a valid-Cargo repo can arise
only three ways, and each forces `edition_uniform = false` whenever a 2018+ crate
is also present:

1. A crate with explicit `edition = "2015"` → recorded as `2015` in
   `editions_seen` → caught by the `editions_seen` term.
2. A crate that omits `edition` (no string, no table) → `unwrap_or(2015)`
   (`src/repo_loader.rs:308`) → `2015` in `editions_seen` → caught by the
   `editions_seen` term. (This is the existing
   `omitted_plus_explicit_edition_workspace_is_not_uniform` behavior, preserved.)
3. A crate with `edition = { workspace = true }` whose workspace is 2015. Valid
   Cargo **requires** that workspace to define `[workspace.package] edition`
   (Cargo rejects `{ workspace = true }` when the key is absent), so that root
   contributes `2015` to `workspace_editions` → caught by the `workspace_editions`
   term.

The only way the `{ workspace = true }` branch could elevate a real-2015 crate to
2018+ in `editions_seen` is via the cross-workspace representative scalar (case 3
above) — and that exact case is what the `workspace_editions` set-term catches.
The pathological "`{ workspace = true }` with no `[workspace.package] edition`
anywhere in its own workspace" is a manifest Cargo itself rejects (§ Risks); it
falls to `unwrap_or(2015)` (lenient), which can only *tighten* the guard, never
mis-resolve.

The recovered edges are produced by the **unchanged** #121/#122 disproof; their
recall-safety is the property already established for those slices. This slice adds
no new drop path.

## §4 What does NOT change

- The disproof predicate (`ScopeResolution::disproves`) and its id-set logic.
- The global `cfg.edition` selection mechanism (last-wins).
- Cross-boundary workspaces (2015 mixed with 2018+): still bail (keep-all).
- `ScopeGraph` schema; the navigation cache *store* path; any other resolver rung.
- Per-crate edition modelling (a heavier, separately-scoped option, deliberately
  not taken — see §8).

## §5 Buy (measured)

Spike forcing `edition_uniform = true` on ruff (`prism nav --no-cache call-stats`,
vs the gated baseline):

| metric | gated (today) | disproof allowed | delta |
| --- | --- | --- | --- |
| `recovery_typepath.singleton` | 0 | 260 | **+260 collision sites → Exact** |
| `recovery_typepath.failopen_demote` | 1586 | 1326 | −260 |
| `kind_exact[qualified_owner]` | 15799 | 16059 | +260 |
| `kind_nameonly[qualified_owner]` | 17853 | 16761 | −1092 (wrong-owner edges pruned) |
| `multi_target_exact_sites` (collision-FP risk) | 46 | 46 | **+0 — precision-safe** |

The fix unblocks the disproof on *every* pure-2018+ mixed-edition workspace, not
just ruff. The remaining 1326 demoted sites are a separately-sequenced follow-on
(§8).

## §6 Testing (TDD)

**Unit (`src/repo_loader.rs` tests):**
- `workspace_true_edition_inherits_from_workspace_package`: a crate with
  `edition = { workspace = true }` + a root `[workspace.package] edition = "2024"`
  → the crate's resolved edition is 2024 (assert via `editions_seen` / a 2024-only
  workspace records `edition == 2024`, `edition_uniform == true`).
- `pure_2018plus_mixed_workspace_is_uniform` (RED under old `len() <= 1`): editions
  `{2021, 2024}` → `edition_uniform == true`.
- `all_2018plus_three_editions_uniform`: `{2018, 2021, 2024}` → true.
- `mixed_edition_workspace_is_not_uniform` (existing, `{2015, 2021}`) → false —
  **preserved unchanged**.
- `omitted_plus_explicit_edition_workspace_is_not_uniform` (existing, `{2015,
  2021}`) → false — **preserved unchanged**.
- `single_edition_workspace_is_uniform`: `{2015}` → true; `{2024}` → true.
- `multi_workspace_spanning_boundary_is_not_uniform` (the two-term-AND guard,
  RED if only the `editions_seen` term were used): a repo with **two** workspace
  roots on opposite sides — `aaa/Cargo.toml` with `[workspace.package] edition =
  "2015"` (member `aaa/m/` uses `edition = { workspace = true }`) and
  `zzz/Cargo.toml` with `[workspace.package] edition = "2024"` (member `zzz/m/`
  uses `edition = { workspace = true }`). Under the last-wins representative
  scalar `aaa`'s member can mis-resolve to 2024, so `editions_seen` may look
  all-2018+; the test asserts `edition_uniform == false` anyway, because
  `workspace_editions == {2015, 2024}` spans the boundary. (Pins the recall-safety
  fix: a per-crate `editions_seen` that *looked* uniform must still bail when the
  workspace editions span the boundary.)

**Behavior (integration):** a 2-crate, pure-2018+ **mixed** fixture (crate `a`
edition 2021, crate `b` `edition = { workspace = true }` → 2024) with a same-name
owner collision (`a::Foo::m` and `b::Foo::m`) and a `use`-imported leading segment
that pins one owner → assert the collision **recovers to a single Exact** (this is
keep-all = 2 NameOnly under the old `len() <= 1` gating; single Exact after).

**Cache pin:** update `src/cpg_cache.rs:567` to `CACHE_VERSION == 17`.

## §7 Acceptance (per CLAUDE.md — call-resolution change)

- `cargo test --lib`, `--test integration`, `--test ast`, `--test cli` (no-run
  pattern) — 0 failed.
- `cargo fmt --check`.
- Tier-A `--matrix-only --allow-stale-sut` — 0 regression.
- Tier-A `--corpus ruff` (no `--quick`; `--quick` forces prism-only) M2 —
  `baseline_invalid == false`, **0 regression**, and the realized
  `recovery_typepath.singleton` rise (≈ +260) reported in the PR.
- Do **not** stage `eval/` or `docs/eval/` run-artifacts.

## §8 Out of scope (sequenced follow-ons — see roadmap)

- **Cross-crate `use` resolution** (+428 on ruff): resolve a crate-name leading
  segment to its workspace-member root scope. Needs the crate-name→root map
  persisted into `ScopeGraph` (schema + cache). The largest residue bucket.
- **Glob-import resolution** (+304): leading type brought by `use crate::a::*;`.
- **Per-crate editions:** a fuller model (edition per crate-root, disproof uses the
  call site's crate edition) that would *also* let the 2018+ crates of a
  2015↔2018+ workspace resolve while the 2015 crates keep-all. Strictly more work
  for the same near-term +260; deferred as YAGNI until a 2015-mixed corpus matters.

## Risks

- **Stale-cache correctness:** mitigated by the `CACHE_VERSION` bump (§2.3) + pin
  test.
- **A malformed `{ workspace = true }` with no workspace edition:** resolves to the
  global representative `workspace_edition` if one was discovered anywhere in the
  repo, and falls back to 2015 only when **no** `[workspace.package] edition` was
  found at all (lenient; Cargo would reject such a manifest). Harmless either way:
  the crate's resolved value enters `editions_seen`, and if the result spans the
  boundary — via `editions_seen` or the `workspace_editions` SET term — the guard
  bails (keep-all), never mis-resolves. See §3 (residual-hole analysis).
- **Multiple workspace roots in one repo on opposite anchoring sides:** prism
  collects all `Cargo.toml` into one global manifest set, so a last-wins
  representative could mis-resolve a 2015 workspace's inheriting crates to a later
  2018+ edition. Mitigated by the two-term `edition_uniform` (§2.2): the
  `workspace_editions` SET spans the boundary → guard bails (keep-all). Pinned by
  `multi_workspace_spanning_boundary_is_not_uniform` (§6).
- **Forced-uniform spike used a single global edition (2021 last-wins):** the
  shipped path is identical in mechanism, and §3 establishes any 2018+ value is
  correct within the class; the ruff M2 acceptance (§7) re-confirms 0 regression on
  the real corpus.
