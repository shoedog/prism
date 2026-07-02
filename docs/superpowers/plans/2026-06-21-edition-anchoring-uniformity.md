# Edition Anchoring-Class Uniformity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the scope-graph disproof (the #121/#122 same-name owner-`::` collision recovery) run on pure-2018+ mixed-edition workspaces by parsing `edition = { workspace = true }` inheritance and relaxing `edition_uniform` from all-identical to anchoring-class (all-2015 or all-2018+), recovering +260 collision sites on ruff with zero new FPs.

**Architecture:** Two edits in `src/repo_loader.rs::parse_rust_crate_config` — a pre-scan collecting all `[workspace.package] edition` values into a `BTreeSet`, and a two-term `edition_uniform` computation (`anchoring_class_uniform(editions_seen) && anchoring_class_uniform(workspace_editions)`). The disproof predicate is unchanged; this only widens *when* it is permitted to run. Plus a `CACHE_VERSION` bump so warm caches recompute, and two stale field-comment updates.

**Tech Stack:** Rust; the `name_resolution` scope-graph engine; `cargo test`; the Tier-A accuracy harness (`eval/`, `uv run tier-a`).

**Spec:** [`docs/superpowers/specs/2026-06-21-prism-edition-anchoring-uniformity-design.md`](../specs/2026-06-21-prism-edition-anchoring-uniformity-design.md) (design-of-record, codex xhigh SHIP).
**Roadmap:** [`docs/archive/plans/prism/ruff-typepath-recovery-roadmap-2026-06-21.md`](../../archive/plans/prism/ruff-typepath-recovery-roadmap-2026-06-21.md).

---

## Premises (from spec §1/§3) — govern every task

- **P1 — recall-safety is inherited, not weakened.** The disproof is untouched. The relaxation only widens when it runs (all-identical → same-anchoring-side). Any workspace (or repo with multiple workspace roots) that spans the 2015/2018 boundary still bails keep-all. The `workspace_editions` SET term makes this hold even though prism collects all `Cargo.toml` repo-wide into one manifest set.
- **P2 — soundness.** prism's path anchoring branches only at 2015 vs 2018+ (`src/name_resolution/rust_policy.rs:82`, `is_2018_plus`). Within the 2018+ class every crate anchors identically, so the single global policy edition (any 2018+ value, last-wins) is correct for every site.
- **P3 — minimal surface.** No `ScopeGraph` schema change; only computed values change, plus the `CACHE_VERSION` bump.

## Executor / commit protocol (READ FIRST)

If executed by codex under `workspace-write` (the established pattern): codex edits + runs `cargo`, the **host** commits (codex cannot write `.git`). Each task shows the exact commit + the precise file set to stage. **Never** stage `eval/` or `docs/eval/` artifacts. Commit trailer:

```
Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
```

## macOS test-runner note (READ FIRST)

`--lib`, `--test integration`, `--test ast` run normally. A bare `cargo test --test cli` may stall at `_dyld_start`; for CLI use `--no-run` then run the freshest binary:

```bash
cargo test --test cli --no-run 2>&1 | tail -3
CLI_BIN=$(ls -t target/debug/deps/cli-* | grep -vE '\.(d|dSYM)$' | head -1)
"$CLI_BIN" 2>&1 | tail -5
```

---

## Task 1: Edition inheritance + anchoring-class uniformity (RED→GREEN)

The whole production change, driven by repo_loader unit tests **and** an end-to-end behavior test. All tests are written RED first; the one `parse_rust_crate_config` change makes them GREEN.

**Files:**
- Modify: `src/repo_loader.rs` (the pre-scan + edition block at `:301-311`, the `edition_uniform` line at `:359`, and a new `anchoring_class_uniform` helper).
- Test: `src/repo_loader.rs` (`#[cfg(test)] mod tests`, alongside `mixed_edition_workspace_is_not_uniform` at `:556`).
- Test: `tests/integration/resolution_test.rs` (one end-to-end behavior test).

### Step 1: Write the failing tests (RED)

**1a — three repo_loader unit tests.** Add inside `src/repo_loader.rs`'s `mod tests`, after `single_edition_workspace_is_uniform` (`:640`):

```rust
#[test]
fn workspace_true_inheritance_resolves_to_workspace_edition() {
    // `a` inherits `edition = { workspace = true }`; the workspace root sets 2024.
    // Pre-fix this mis-parses to 2015 (table -> `.as_str()` None -> unwrap_or(2015));
    // post-fix it resolves to 2024, so a single-edition workspace is uniform.
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    std::fs::create_dir_all(p.join("a/src")).unwrap();
    std::fs::write(
        p.join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\"]\n[workspace.package]\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(
        p.join("a/Cargo.toml"),
        "[package]\nname = \"a\"\nedition = { workspace = true }\n",
    )
    .unwrap();
    std::fs::write(p.join("a/src/lib.rs"), "pub fn a() {}\n").unwrap();
    let repo = load_repo(p).unwrap();
    let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
    assert_eq!(
        inputs.cfg.edition, 2024,
        "`workspace = true` must inherit the workspace edition 2024, not fall back to 2015"
    );
    assert!(
        inputs.cfg.edition_uniform,
        "one resolved edition is uniform"
    );
}

#[test]
fn pure_2018plus_mixed_workspace_is_uniform() {
    // Two crates on different but same-anchoring-class editions (2021 + 2024).
    // Pre-fix: `editions_seen.len() == 2` -> not uniform. Post-fix: both >= 2018 ->
    // anchoring-class uniform -> the disproof is permitted to run.
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    std::fs::create_dir_all(p.join("a/src")).unwrap();
    std::fs::create_dir_all(p.join("b/src")).unwrap();
    std::fs::write(p.join("Cargo.toml"), "[workspace]\nmembers = [\"a\", \"b\"]\n").unwrap();
    std::fs::write(
        p.join("a/Cargo.toml"),
        "[package]\nname = \"a\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(
        p.join("b/Cargo.toml"),
        "[package]\nname = \"b\"\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(p.join("a/src/lib.rs"), "pub fn a() {}\n").unwrap();
    std::fs::write(p.join("b/src/lib.rs"), "pub fn b() {}\n").unwrap();
    let repo = load_repo(p).unwrap();
    let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
    assert!(
        inputs.cfg.edition_uniform,
        "a pure-2018+ workspace ({{2021, 2024}}) is anchoring-class uniform"
    );
}

#[test]
fn multi_workspace_spanning_boundary_is_not_uniform() {
    // prism collects ALL Cargo.toml repo-wide into one manifest set. Two workspace
    // roots on opposite anchoring sides (ws1: 2015, ws2: 2024) must force
    // edition_uniform == false via the `workspace_editions` SET term -- even though a
    // last-wins representative could mis-resolve ws1's inheriting crate to 2024 and
    // make `editions_seen` look all-2018+. Recall-safety (P1).
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    std::fs::create_dir_all(p.join("ws1/a/src")).unwrap();
    std::fs::create_dir_all(p.join("ws2/b/src")).unwrap();
    std::fs::write(
        p.join("ws1/Cargo.toml"),
        "[workspace]\nmembers = [\"a\"]\n[workspace.package]\nedition = \"2015\"\n",
    )
    .unwrap();
    std::fs::write(
        p.join("ws1/a/Cargo.toml"),
        "[package]\nname = \"a\"\nedition = { workspace = true }\n",
    )
    .unwrap();
    std::fs::write(p.join("ws1/a/src/lib.rs"), "pub fn a() {}\n").unwrap();
    std::fs::write(
        p.join("ws2/Cargo.toml"),
        "[workspace]\nmembers = [\"b\"]\n[workspace.package]\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(
        p.join("ws2/b/Cargo.toml"),
        "[package]\nname = \"b\"\nedition = { workspace = true }\n",
    )
    .unwrap();
    std::fs::write(p.join("ws2/b/src/lib.rs"), "pub fn b() {}\n").unwrap();
    let repo = load_repo(p).unwrap();
    let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
    assert!(
        !inputs.cfg.edition_uniform,
        "workspace editions spanning the 2015/2018 boundary must keep edition_uniform == false"
    );
}
```

**1b — one end-to-end behavior test.** Add at the end of `tests/integration/resolution_test.rs`:

```rust
#[test]
fn mixed_edition_workspace_recovers_intra_crate_collision() {
    use prism::repo_loader::load_repo;
    // A pure-2018+ MIXED-edition workspace driven end-to-end through the real loader:
    // crate `a` (2021) holds an intra-crate same-name owner collision (m1::Foo,
    // m2::Foo) and a `use`-imported call site that pins m1::Foo; crate `b` inherits
    // `{ workspace = true }` -> 2024, making the workspace mixed. Pre-fix:
    // edition_uniform == false (a:2021 + b mis-parsed 2015) -> disproof bails ->
    // keep-all (2 NameOnly). Post-fix: {2021,2024} anchoring-uniform -> disproof runs
    // -> single Exact (m1::Foo).
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path();
    std::fs::create_dir_all(p.join("a/src")).unwrap();
    std::fs::create_dir_all(p.join("b/src")).unwrap();
    std::fs::write(
        p.join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\"]\n[workspace.package]\nedition = \"2024\"\n",
    )
    .unwrap();
    std::fs::write(
        p.join("a/Cargo.toml"),
        "[package]\nname = \"a\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(
        p.join("b/Cargo.toml"),
        "[package]\nname = \"b\"\nedition = { workspace = true }\n",
    )
    .unwrap();
    std::fs::write(
        p.join("a/src/lib.rs"),
        "mod m1;\nmod m2;\nuse crate::m1::Foo;\npub fn drive() {\n    Foo::m();\n}\n",
    )
    .unwrap();
    std::fs::write(
        p.join("a/src/m1.rs"),
        "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
    )
    .unwrap();
    std::fs::write(
        p.join("a/src/m2.rs"),
        "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
    )
    .unwrap();
    std::fs::write(p.join("b/src/lib.rs"), "pub fn b() {}\n").unwrap();
    let repo = load_repo(p).unwrap();
    let inputs = repo
        .scope_graph_inputs
        .as_ref()
        .expect("scope graph inputs");
    let cg = CallGraph::build_with_scope_graph_inputs(&repo.files, Some(inputs));
    assert_eq!(
        cg.methods
            .get(&("Foo".to_string(), "m".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key must collide across m1::Foo and m2::Foo"
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Foo::m"));
    assert_eq!(out.drop, None, "the recovered owner path must not drop");
    assert_eq!(
        out.resolved.len(),
        1,
        "a pure-2018+ mixed-edition workspace now recovers the collision to one Exact"
    );
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
}
```

### Step 2: Run the tests to verify they fail (RED)

```bash
# cargo accepts only ONE test-name filter before `--`; use the broad module filter
# (runs the 3 new tests + the existing guards) and read which fail.
cargo test --lib repo_loader::tests:: 2>&1 | tail -30
cargo test --test integration resolution_test::mixed_edition_workspace_recovers_intra_crate_collision -- --exact 2>&1 | tail -20
```

Expected (RED): among the `repo_loader::tests::` results, `workspace_true_inheritance_resolves_to_workspace_edition` fails (`edition` is 2015, not 2024); `pure_2018plus_mixed_workspace_is_uniform` fails (not uniform); `multi_workspace_spanning_boundary_is_not_uniform` fails (mis-parsed both to 2015 → `editions_seen == {2015}` → wrongly uniform, so `!edition_uniform` is false); the two existing `*_is_not_uniform` guards still pass. The integration test fails at `out.resolved.len()` (`left: 2, right: 1` — keep-all).

### Step 3: Implement the change (minimal)

In `src/repo_loader.rs`, **insert the pre-scan** immediately before the existing per-manifest loop (before `for manifest_path in manifest_hashes.keys() {` at `:287`, after the `let mut editions_seen ...` declaration at `:285`):

```rust
    // Collect EVERY discovered `[workspace.package] edition` (not just a last-wins
    // scalar): prism collects all `Cargo.toml` repo-wide into one `manifest_hashes`,
    // so a repo may hold multiple workspace roots on opposite anchoring sides. The
    // full set drives the recall-safe second uniformity term (§2.2); a representative
    // scalar resolves the `{ workspace = true }` value form.
    let mut workspace_editions: BTreeSet<u16> = BTreeSet::new();
    let mut workspace_edition: Option<u16> = None;
    for manifest_path in manifest_hashes.keys() {
        let abs = root.join(manifest_path);
        let Ok(text) = std::fs::read_to_string(&abs) else {
            continue;
        };
        let Ok(value) = text.parse::<toml::Value>() else {
            continue;
        };
        if let Some(ed) = value
            .get("workspace")
            .and_then(|w| w.get("package"))
            .and_then(|p| p.get("edition"))
            .and_then(|e| e.as_str())
            .and_then(parse_edition)
        {
            workspace_editions.insert(ed);
            workspace_edition = Some(ed);
        }
    }
```

**Replace the `[package]` edition block** (`:301-311`):

```rust
        if value.get("package").is_some() {
            // Cargo default: a `[package]` with no `edition` key is edition 2015.
            let edition = value
                .get("package")
                .and_then(|p| p.get("edition"))
                .and_then(|e| e.as_str())
                .and_then(parse_edition)
                .unwrap_or(2015);
            cfg.edition = edition;
            editions_seen.insert(edition);
        }
```

with:

```rust
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

**Replace the `edition_uniform` line** (`:359`):

```rust
    cfg.edition_uniform = editions_seen.len() <= 1;
```

with:

```rust
    cfg.edition_uniform =
        anchoring_class_uniform(&editions_seen) && anchoring_class_uniform(&workspace_editions);
```

**Add the helper** immediately after `parse_edition` (`:367-375`):

```rust
/// True iff every observed edition is on the same side of the 2015/2018 path-
/// anchoring boundary (`RustPolicy::is_2018_plus`), i.e. all >= 2018 or all < 2018.
/// An empty set is vacuously uniform (matches the prior `len() <= 1` for empty). This
/// is the recall-safety floor for the disproof: 2018/2021/2024 anchor identically, so
/// a same-side workspace is authoritative, but a 2015/2018+ mix is not (keep-all).
fn anchoring_class_uniform(editions: &BTreeSet<u16>) -> bool {
    editions.iter().all(|&e| e >= 2018) || editions.iter().all(|&e| e < 2018)
}
```

(`BTreeSet` is already imported in `src/repo_loader.rs`; `parse_edition` and `BTreeSet` are in scope.)

### Step 4: Run the tests to verify they pass (GREEN), and the preserved guards

```bash
cargo test --lib repo_loader::tests:: 2>&1 | tail -15
cargo test --test integration resolution_test::mixed_edition_workspace_recovers_intra_crate_collision -- --exact 2>&1 | tail -10
```

Expected: all repo_loader `tests::` pass — the three new ones GREEN **and** `mixed_edition_workspace_is_not_uniform` + `omitted_plus_explicit_edition_workspace_is_not_uniform` (both `{2015,2021}`, span the boundary) still `false`; the integration test passes (`1 passed`).

### Step 5: Format + commit

```bash
cargo fmt && cargo fmt --check
```

**Host commits** (stage exactly these two files):

```bash
git add src/repo_loader.rs tests/integration/resolution_test.rs
```

Commit message:

```
feat(repo_loader): edition anchoring-class uniformity for mixed-edition workspaces

Parse `edition = { workspace = true }` inheritance (pre-scan for
`[workspace.package] edition`) and relax `edition_uniform` from all-identical
to anchoring-class: `anchoring_class_uniform(editions_seen) &&
anchoring_class_uniform(workspace_editions)` (all-2015 or all-2018+). prism
anchors only at the 2015/2018+ boundary, so the unchanged #121/#122 disproof
now runs on pure-2018+ mixed workspaces (e.g. ruff, +260). The workspace-edition
SET term keeps it recall-safe under prism's one repo-wide manifest set: multiple
workspace roots spanning the boundary still bail keep-all. Existing {2015,2021}
guards preserved; end-to-end recovery pinned by an integration fixture.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
```

---

## Task 2: CACHE_VERSION bump + stale field-comment updates

The recomputed `edition_uniform` is a serialized `ScopeGraph` field, so warm caches must recompute. Plus two doc comments now mis-describe `edition_uniform`.

**Files:**
- Modify: `src/cpg_cache.rs:60` (`CACHE_VERSION`) and `:567` (pin test).
- Modify: `src/name_resolution/rust_populator/mod.rs:89` and `src/name_resolution/graph.rs:89` (comments).

### Step 1: Flip the pin test to 17 (RED)

In `src/cpg_cache.rs`, update the whole pin test (`:564-568`) — its name, comment, and
assertion all reference "16":

```rust
    #[test]
    fn cache_version_is_16_for_scope_graph_precision_recovery() {
        // v16: ScopeGraph.edition_uniform + scope-graph recovery behavior.
        assert_eq!(super::CACHE_VERSION, 16);
    }
```

to:

```rust
    #[test]
    fn cache_version_is_17_for_anchoring_class_uniformity() {
        // v17: edition_uniform recomputed as anchoring-class (2015 vs 2018+).
        assert_eq!(super::CACHE_VERSION, 17);
    }
```

### Step 2: Run to verify it fails (RED)

```bash
cargo test --lib cpg_cache 2>&1 | tail -10
```

Expected: the pin test fails (`left: 16, right: 17`).

### Step 3: Bump the constant + fix the comments (GREEN)

In `src/cpg_cache.rs:60`:

```rust
const CACHE_VERSION: u32 = 16; // bincode ignores serde(default) for new trailing fields.
```

to:

```rust
const CACHE_VERSION: u32 = 17; // 17: edition_uniform recomputed as anchoring-class.
```

In `src/name_resolution/rust_populator/mod.rs:89` and `src/name_resolution/graph.rs:89`, reword the `edition_uniform` doc comment from the "agreed on one/single edition" phrasing to the new meaning. At `src/name_resolution/graph.rs:89` the line reads `/// Whether every parsed manifest agreed on one edition (spec §2 BLOCKER-2).` — change to:

```rust
    /// Whether every parsed manifest is on the same path-anchoring class
    /// (all 2015 or all 2018+); see repo_loader `anchoring_class_uniform`.
```

At `src/name_resolution/rust_populator/mod.rs:89`, apply the analogous reword (match that file's existing comment wording for the `edition_uniform` field; keep it one–two lines, "same anchoring class (2015 vs 2018+)").

### Step 4: Run to verify it passes (GREEN)

```bash
cargo test --lib cpg_cache 2>&1 | tail -10
```

Expected: `cpg_cache` tests pass (pin now matches 17).

### Step 5: Format + commit

```bash
cargo fmt && cargo fmt --check
```

**Host commits** (stage exactly these files):

```bash
git add src/cpg_cache.rs src/name_resolution/rust_populator/mod.rs src/name_resolution/graph.rs
```

Commit message:

```
chore(cache): CACHE_VERSION 16->17 for recomputed edition_uniform + comment fixes

The anchoring-class edition_uniform changes the serialized ScopeGraph field
value for mixed-edition workspaces; bump CACHE_VERSION so warm caches recompute.
Reword the two edition_uniform doc comments from "agreed on one edition" to
"same anchoring class (2015 vs 2018+)".

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
```

---

## Task 3: Acceptance (non-code) — spec §7

Build, run the full test surface, format-check, and run the Tier-A recall gate + the ruff M2 recovery acceptance. **No commit** (verification only); record realized deltas in the PR.

**Files:** none.

### Step 1: Full test surface (macOS-correct)

```bash
cargo build --release 2>&1 | tail -3
cargo test --lib 2>&1 | tail -5
cargo test --test integration 2>&1 | tail -5
cargo test --test ast 2>&1 | tail -5
cargo test --test cli --no-run 2>&1 | tail -3
CLI_BIN=$(ls -t target/debug/deps/cli-* | grep -vE '\.(d|dSYM)$' | head -1)
"$CLI_BIN" 2>&1 | tail -5
cargo fmt --check
```

Expected: each suite `test result: ok.` with `0 failed`; `fmt --check` clean.

### Step 2: Tier-A matrix gate (REQUIRED — before committing per CLAUDE.md)

```bash
cd eval && uv run tier-a --matrix-only --allow-stale-sut 2>&1 | tail -30
```

Expected: 0 regression (no `ok`→`gap` flips vs the committed baseline).

### Step 3: ruff M2 recovery acceptance (the real end-to-end — spec §5/§7)

`--quick` forces prism-only (cli.py:732), so use `--corpus ruff` WITHOUT `--quick`:

```bash
cd eval && uv run tier-a --corpus ruff --allow-stale-sut 2>&1 | tail -10
# read the run report (paths are relative to eval/ after the cd above):
RUFF_RUN=runs/$(ls -t runs | grep ruff | head -1)
grep -nE '"baseline_invalid"|"oracle_error_rate"|"sut_error_rate"' "$RUFF_RUN"
grep -c '"outcome": "regression"' "$RUFF_RUN"
```

Expected: `baseline_invalid == false`, oracle/sut error 0.0, **0 regression**.

### Step 4: Recovery delta — `call-stats` on ruff (report the +260)

```bash
./target/release/prism nav --no-cache call-stats --repo ~/code/bench-repos/ruff > /tmp/cs-ruff-after.json 2>/dev/null
grep -A7 '"recovery_typepath"' /tmp/cs-ruff-after.json | grep -E 'singleton|failopen_demote'
grep -A11 '"kind_exact"' /tmp/cs-ruff-after.json | grep qualified_owner
```

Expected (vs the pre-change main baseline): `recovery_typepath.singleton` ≈ 260 (was 0), `failopen_demote` ≈ 1326 (was 1586), `kind_exact[qualified_owner]` up ≈ +260, `multi_target_exact_sites` unchanged at 46 (no new collision FPs). Report the measured pair in the PR.

> **Do not** stage `eval/` or `docs/eval/` artifacts.

### Step 5: Final review

Request an independent codex (gpt-5.5, xhigh) diff review of the branch. Fold any findings as host-committed fix-ups with the trailer. Headline to call out: the disproof is unchanged (recall-safety inherited); `edition_uniform` is the two-term AND with the recall-safe `workspace_editions` SET term; cross-boundary mixes still bail; +260 on ruff with 0 new collision FPs.

---

## Self-review checklist (spec coverage → task)

- **§2.1 workspace=true inheritance + pre-scan SET** → Task 1 Step 3 (pre-scan collecting `workspace_editions: BTreeSet` + representative; table resolution in the edition block).
- **§2.2 two-term anchoring-class uniform + helper** → Task 1 Step 3 (`anchoring_class_uniform(editions_seen) && anchoring_class_uniform(workspace_editions)` + the helper).
- **§2.3 CACHE_VERSION 16→17** → Task 2.
- **§2.2 source-comment NIT** → Task 2 Step 3.
- **§3 soundness / recall-safety** → Task 1 unit tests (`pure_2018plus_*` enables; `multi_workspace_spanning_*` + preserved `{2015,2021}` guards bail) + the behavior test.
- **§6 tests** → Task 1 (the three unit tests + the integration behavior test; existing guards preserved) + Task 2 (cache pin).
- **§5 buy + §7 acceptance** → Task 3 (Tier-A matrix + ruff M2 0-regression + the +260 call-stats delta + codex review).

**Placeholder scan:** no `TODO`/`...`; every test + edit is complete code; every command runnable.

**Type/signature consistency (verified against source):**
- `load_repo(&Path) -> Result<LoadedRepo>` (pub, `repo_loader.rs:61`); `LoadedRepo.scope_graph_inputs: Option<ScopeGraphBuildInputs>` (pub, `:39`); `LoadedRepo.files` accessible (used at `repo_loader.rs:655`).
- `CallGraph::build_with_scope_graph_inputs(&BTreeMap<String, ParsedFile>, Option<&ScopeGraphBuildInputs>)` (pub, `call_graph.rs:419`).
- `site_in(&CallGraph, &str, &str) -> CallSite` (`resolution_test.rs:370`); `ResolutionConfidence`, `CallGraph`, `CallSite` already imported there (`:1-4`).
- `cg.methods: BTreeMap<(String,String), Vec<FunctionId>>`; `cg.resolve_call_site_full(&CallSite) -> ResolutionOutcome { resolved, drop }` (as used by the (A) fixtures).
- `parse_edition(&str) -> Option<u16>` and `BTreeSet` already in `repo_loader.rs` scope; `inputs.cfg.edition: u16`, `inputs.cfg.edition_uniform: bool`.
- `CACHE_VERSION: u32` (`cpg_cache.rs:60`); pin test at `:567`.
