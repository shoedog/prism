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
  permitted to run*, from "all editions identical" to "all editions on the same
  side of the 2015/2018 anchoring boundary". A workspace that spans the boundary
  still bails (keep-all) exactly as today.
- **P2 — soundness of the widening.** Within the 2018+ anchoring class every crate
  resolves paths identically, so a single global policy edition (any 2018+ value)
  anchors every call site correctly. The wrong-edition mis-anchor the §2 guard
  exists to prevent **cannot occur** inside one anchoring class. See §3.
- **P3 — minimal surface.** No `ScopeGraph` schema change (`edition`,
  `edition_uniform` are already fields); only their *computed values* change, plus
  a `CACHE_VERSION` bump so warm caches recompute.

## §2 The change

Two edits in `src/repo_loader.rs::parse_rust_crate_config`, plus a cache bump.

### §2.1 Parse `edition = { workspace = true }` inheritance

Add a pre-scan over the same `manifest_hashes` for the workspace root's
`[workspace.package] edition`, then resolve the table form against it.

```rust
// Pre-scan (before the existing per-manifest loop): find the workspace edition.
let mut workspace_edition: Option<u16> = None;
for manifest_path in manifest_hashes.keys() {
    let abs = root.join(manifest_path);
    let Ok(text) = std::fs::read_to_string(&abs) else { continue; };
    let Ok(value) = text.parse::<toml::Value>() else { continue; };
    if let Some(ed) = value
        .get("workspace").and_then(|w| w.get("package"))
        .and_then(|p| p.get("edition"))
        .and_then(|e| e.as_str()).and_then(parse_edition)
    {
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
forward pass could read `{ workspace = true }` before the root edition is known. If
more than one manifest declared `[workspace.package] edition` (nested workspaces —
rare), the pre-scan takes the last in sorted order; deterministic, and a
cross-boundary disagreement would only *tighten* the §2.2 check (bail), never
mis-resolve.

### §2.2 Relax `edition_uniform` to anchoring-class uniformity

Replace the all-identical test with a same-side-of-2018 test:

```rust
// src/repo_loader.rs:359  (replacement)
cfg.edition_uniform = anchoring_class_uniform(&editions_seen);
```

```rust
/// True iff every observed edition is on the same side of the 2015/2018 path-
/// anchoring boundary (`RustPolicy::is_2018_plus`), i.e. all >= 2018 or all < 2018.
/// An empty set is vacuously uniform (matches the prior `len() <= 1` for empty).
fn anchoring_class_uniform(editions: &std::collections::BTreeSet<u16>) -> bool {
    editions.iter().all(|&e| e >= 2018) || editions.iter().all(|&e| e < 2018)
}
```

The global `cfg.edition` value is unchanged in mechanism (last-wins over the
sorted `manifest_hashes` order, so deterministic). When the class is all-2018+, the
last-wins value is necessarily ≥ 2018, so `is_2018_plus()` is correct for every
site; the exact value (2021 vs 2024) is immaterial because anchoring only reads the
boundary (§3). No change to edition selection is needed.

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
- **A malformed `{ workspace = true }` with no workspace edition:** falls back to
  2015 (lenient; Cargo would reject the manifest). Harmless — at worst that crate
  is treated as 2015, and if it then spans the boundary the workspace simply bails
  (keep-all), never mis-resolves.
- **Forced-uniform spike used a single global edition (2021 last-wins):** the
  shipped path is identical in mechanism, and §3 establishes any 2018+ value is
  correct within the class; the ruff M2 acceptance (§7) re-confirms 0 regression on
  the real corpus.
