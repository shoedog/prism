# Scope-Graph Precision Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Rust scope graph on repos that today fail the whole-repo completeness gate purely on non-Rust files, and route owner-keyed `T::m()` resolution through a sound candidate-elimination (disproof) seam so same-name owner collisions recover toward a single `Exact` (or a pruned NameOnly), with zero recall regression.

**Architecture:** Three components, in build order. (1) `has_complete_rust_coverage` makes the scope-graph `complete` flag depend on **Rust-file** coverage only (a non-UTF-8 `.py` no longer blocks ruff's graph), plus an edition-uniformity guard recorded on the crate config / `ScopeGraph`. (2) A new `src/resolution_disproof.rs` module with a `DisproofPredicate` trait, a read-only `DisproofCx`, an intersection `prune` (prune-to-empty keeps the original pool), and one shipped predicate `ScopeResolution` that disproves a candidate only when the leading owner type-path binds **directly** (`BindTarget::Resolved` to an in-repo `Target::Item`, no `Pending` hop) **and** the call site has **no block-local shadow** of the leading ident. (3) Integration in `src/resolution.rs`: `graph_target_resolution` returns `1→Exact / >1→demoted`; the Rust scope path fetches the bare pool, runs `prune`, and decides `1→Exact / >1→pruned-demote / unchanged→fall through`; the fail-open is narrowed to `::`-owner sites routed to #120's demote floor (never the legacy stem heuristic), preserving the three shipped drop invariants.

**Tech Stack:** Rust, tree-sitter, `petgraph`; the existing `name_resolution` scope-graph engine (`engine::resolve_path`, `RustPolicy`, `NS_TYPE`); `cargo test`; the Tier-A accuracy harness (`eval/`, `uv run tier-a`).

---

## Premises that govern every task (from spec §1)

- **P1 — recall-safety.** Never drop a real edge. A candidate is eliminated only when *proven* not the target; uncertainty keeps it (NameOnly). A wrong drop is worse than a wrong demote.
- **P2 — recovery is upstream.** #120's demote stays the terminal floor. Disproof runs strictly *before* the demote and never un-demotes in place.

## macOS test-runner note (READ FIRST)

`--lib` and `--test integration` run normally on this machine. A bare `cargo test --test cli` may stall at `_dyld_start`. For **CLI** tests only, compile without running, then run the freshest non-debug-artifact binary:

```bash
cargo test --test cli --no-run 2>&1 | tail -3
CLI_BIN=$(ls -t target/debug/deps/cli-* | grep -vE '\.(d|dSYM)$' | head -1)
"$CLI_BIN" call_stats_test:: --nocapture
```

Lib/integration tests use the ordinary `cargo test --lib <filter>` / `cargo test --test integration <filter>` invocations shown in each task.

## Accuracy-harness reminder

This change touches call resolution and CPG construction (`src/call_graph.rs`, `src/resolution.rs`, `src/repo_loader.rs`). The §6 cache bump invalidates warm caches once. Task 8 runs the Tier-A gate; do **not** stage `eval/` or `docs/eval/` artifacts in any commit.

---

## Task 1: Rust-scoped completeness gate (Component 1, spec §2)

Replace the all-language `has_complete_file_coverage` with a Rust-only `has_complete_rust_coverage` so the scope graph builds when a repo only skips non-Rust files (ruff: 10+ non-UTF-8 `.py`, 0 non-UTF-8 `.rs`). The Rust scope graph is Rust-only (`populate_rust` walks only `.rs`; `populate_method_identity_indices` skips non-Rust), so Rust-file coverage is the correct authoritativeness condition for in-repo Rust resolution.

**Files:**
- Modify: `src/repo_loader.rs:170` (the `complete` assignment in `scope_graph_build_inputs`) and `src/repo_loader.rs:215-221` (`has_complete_file_coverage` → `has_complete_rust_coverage`).
- Test: `src/repo_loader.rs` (`#[cfg(test)] mod tests` at the bottom — add two unit tests there).

- [ ] **Step 1: Write the failing tests**

Add these two tests inside the existing `#[cfg(test)] mod tests { ... }` block in `src/repo_loader.rs` (after `parallel_loader_is_element_for_element_identical_to_serial_reference`):

```rust
    #[test]
    fn rust_coverage_ignores_non_utf8_python() {
        // A repo whose ONLY skipped file is a non-UTF-8 `.py` (ruff's lint
        // fixtures) must still build a complete scope graph: Rust coverage is
        // total, so `complete == true` and the graph is populated.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::write(p.join("lib.rs"), "pub fn a() {}\n").unwrap();
        std::fs::write(p.join("bad.py"), [0xFFu8, 0xFE, 0x00]).unwrap(); // NotUtf8, skipped
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        assert!(
            inputs.complete,
            "a non-UTF-8 .py must not block Rust-scoped completeness"
        );
    }

    #[test]
    fn rust_coverage_false_when_a_rust_file_is_skipped() {
        // If the repo skips one of its OWN `.rs`, Rust coverage is incomplete →
        // `complete == false` (unchanged behavior; the deferred per-crate case).
        // We trigger the skip with a NON-UTF-8 `.rs` — the `String::from_utf8`
        // arm in `walk` (SkipReason::NotUtf8, repo_loader.rs:511) drops `a.rs`
        // before it becomes a parse candidate, so it is absent from `files`. The
        // bytes `[0xFF, 0xFE, 0xFC]` are invalid UTF-8 (no valid sequence begins
        // with 0xFF/0xFE). `a.rs` is still supported-by-path (Language::from_path
        // maps `.rs` → Rust), so `collect_supported_source_paths` counts it in the
        // EXPECTED Rust set while it is missing from the ACTUAL set ⇒ expected !=
        // actual ⇒ complete == false. This avoids an impractical > 2 MiB
        // oversized-file fixture (MAX_FILE_BYTES, repo_loader.rs:12).
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::write(p.join("lib.rs"), "pub fn a() {}\n").unwrap();
        std::fs::write(p.join("a.rs"), [0xFFu8, 0xFE, 0xFC]).unwrap(); // NotUtf8, skipped
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        assert!(
            !inputs.complete,
            "skipping an own .rs (non-UTF-8) must keep completeness false"
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo test --lib repo_loader::tests::rust_coverage 2>&1 | tail -20
```
Expected: `rust_coverage_ignores_non_utf8_python` FAILS (asserts `inputs.complete`, but today `has_complete_file_coverage` sees `bad.py` skipped → `false`). `rust_coverage_false_when_a_rust_file_is_skipped` PASSES already (it pins unchanged behavior). The task is GREEN only once the first test passes.

- [ ] **Step 3: Add `has_complete_rust_coverage` and wire it in**

In `src/repo_loader.rs`, replace the body of `has_complete_file_coverage` (lines 215-221) with a Rust-scoped helper. Change the function name and restrict both the expected and actual sets to Rust files:

```rust
fn has_complete_rust_coverage(root: &Path, files: &BTreeMap<String, ParsedFile>) -> bool {
    let Some(expected) = collect_supported_source_paths(root) else {
        return false;
    };
    let is_rust = |p: &String| Language::from_path(p) == Some(Language::Rust);
    let expected_rust: BTreeSet<String> = expected.into_iter().filter(is_rust).collect();
    let actual_rust: BTreeSet<String> = files.keys().filter(|k| is_rust(k)).cloned().collect();
    actual_rust == expected_rust
}
```

Then update the single call site in `scope_graph_build_inputs` (line 170):

```rust
    let complete = has_complete_rust_coverage(root, files);
```

`has_complete_file_coverage` had exactly one caller (the scope-graph path), so it is replaced outright — no other consumer depends on all-language coverage.

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cargo test --lib repo_loader:: 2>&1 | tail -20
```
Expected: PASS (both new tests + the existing parity test).

- [ ] **Step 5: Commit**

```bash
git add src/repo_loader.rs
git commit -m "$(cat <<'EOF'
feat(scope-graph): Rust-scoped completeness gate (Component 1)

Replace has_complete_file_coverage with has_complete_rust_coverage so the
scope-graph `complete` flag depends on Rust-file coverage only. A repo that
skips non-Rust files (ruff: non-UTF-8 .py lint fixtures) now builds the scope
graph; skipping an own .rs still yields complete=false (deferred per-crate
case). The Rust scope graph is Rust-only, so Rust-file completeness is the
correct authoritativeness condition.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Edition-uniformity guard (Component 1, spec §2 BLOCKER-2 fold)

`ScopeGraph` carries a single repo-wide `edition`; `parse_rust_crate_config` populates it last-write-wins while iterating manifests. A mixed-edition workspace can mis-anchor a `UsePath`/`LeadingColon` path, which becomes a **recall risk** once the `ScopeResolution` predicate prunes on such a path. P1 demands we treat a non-uniform-edition workspace as **non-authoritative for disproof** (keep-all). Record whether all parsed manifests agreed on one edition on `RustCrateConfig`, carry it onto `ScopeGraph`, and let the predicate (Task 4) early-return keep-all when false.

**Edition-omitted = 2015 (faithfulness).** A `[package]` manifest that omits `edition` defaults to **edition 2015** per Cargo's spec — and the code already encodes this: `parse_edition` accepts only `2015/2018/2021/2024`, and `RustCrateConfig`'s `edition` field doc plus `from_convention` both pin "Default 2015 (Cargo's default when omitted)" (`src/name_resolution/rust_populator/mod.rs:67,85,114`). The uniformity computation must honour that default: an omitted-edition crate counts as a *seen edition of 2015*, **not** "no edition". Otherwise a workspace mixing an omitted (2015) crate with an explicit `edition = "2021"` crate would record only `{2021}`, be wrongly judged **uniform**, and admit an unsound prune on a genuinely mixed-edition repo (a P1 recall regression). With the default honoured, that workspace resolves to `{2015, 2021}` ⇒ **non-uniform ⇒ keep-all** (recall-safe). Step 5 below counts every parsed `[package]` (explicit-or-default-2015); a pure `[workspace]`-root manifest declares no crate edition and contributes nothing.

**Files:**
- Modify: `src/name_resolution/rust_populator/mod.rs` (`RustCrateConfig` — add `edition_uniform`; **drop `Default` from the derive + add a manual `impl Default` with `edition_uniform: true`** so the `..RustCrateConfig::default()` struct-update helpers keep the predicate enabled, BLOCKER-3; `from_convention` default; `populate_rust` copies it onto the graph).
- Modify: `src/name_resolution/graph.rs` (`ScopeGraph` — add `edition_uniform` field + `default_edition_uniform`).
- Modify: `src/repo_loader.rs` (`parse_rust_crate_config` — compute uniformity across manifests).
- Modify: `tests/name_resolution/rust_populate_test.rs` (four full `RustCrateConfig { ... }` literals — add `edition_uniform: true` so they compile, BLOCKER-3).
- Test: `src/repo_loader.rs` `#[cfg(test)] mod tests` (mixed-edition fixture) and `src/name_resolution/rust_populator/mod.rs` `#[cfg(test)] mod tests` (default + propagation).

- [ ] **Step 1: Write the failing tests**

In `src/name_resolution/rust_populator/mod.rs` `#[cfg(test)] mod tests`, add:

```rust
    #[test]
    fn from_convention_edition_is_uniform_by_default() {
        let mut files = BTreeMap::new();
        files.insert(
            "main.rs".to_string(),
            ParsedFile::parse("main.rs", "fn main() {}\n", Language::Rust).unwrap(),
        );
        let cfg = RustCrateConfig::from_convention(&files);
        assert!(cfg.edition_uniform, "convention fallback is single-edition → uniform");
    }

    #[test]
    fn populate_rust_propagates_edition_uniform_flag() {
        let mut files = BTreeMap::new();
        files.insert(
            "src/lib.rs".to_string(),
            ParsedFile::parse("src/lib.rs", "pub fn a() {}\n", Language::Rust).unwrap(),
        );
        let mut cfg = RustCrateConfig::from_convention(&files);
        cfg.crate_roots = vec!["src/lib.rs".to_string()];
        cfg.edition_uniform = false;
        let graph = populate_rust(&files, &cfg, None);
        assert!(!graph.edition_uniform, "the flag must ride onto the ScopeGraph");
    }
```

In `src/repo_loader.rs` `#[cfg(test)] mod tests`, add a mixed-edition workspace fixture:

```rust
    #[test]
    fn mixed_edition_workspace_is_not_uniform() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("a/src")).unwrap();
        std::fs::create_dir_all(p.join("b/src")).unwrap();
        std::fs::write(
            p.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\n",
        )
        .unwrap();
        std::fs::write(
            p.join("a/Cargo.toml"),
            "[package]\nname = \"a\"\nedition = \"2015\"\n",
        )
        .unwrap();
        std::fs::write(
            p.join("b/Cargo.toml"),
            "[package]\nname = \"b\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(p.join("a/src/lib.rs"), "pub fn a() {}\n").unwrap();
        std::fs::write(p.join("b/src/lib.rs"), "pub fn b() {}\n").unwrap();
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        assert!(
            !inputs.cfg.edition_uniform,
            "a 2015 + 2021 workspace must record edition_uniform == false"
        );
    }

    #[test]
    fn omitted_plus_explicit_edition_workspace_is_not_uniform() {
        // Faithfulness to Cargo's default: crate `a` OMITS `edition` (⇒ 2015) and
        // crate `b` sets `edition = "2021"`. The resolved edition set is
        // {2015, 2021} ⇒ genuinely mixed ⇒ edition_uniform == false ⇒ the
        // ScopeResolution predicate keeps-all (recall-safe, P1). If omitted were
        // treated as "no edition" the set would be {2021} and this would WRONGLY
        // read uniform, enabling an unsound prune on a mixed-edition repo.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("a/src")).unwrap();
        std::fs::create_dir_all(p.join("b/src")).unwrap();
        std::fs::write(
            p.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\n",
        )
        .unwrap();
        std::fs::write(
            p.join("a/Cargo.toml"),
            "[package]\nname = \"a\"\n", // edition omitted ⇒ Cargo default 2015
        )
        .unwrap();
        std::fs::write(
            p.join("b/Cargo.toml"),
            "[package]\nname = \"b\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(p.join("a/src/lib.rs"), "pub fn a() {}\n").unwrap();
        std::fs::write(p.join("b/src/lib.rs"), "pub fn b() {}\n").unwrap();
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        assert!(
            !inputs.cfg.edition_uniform,
            "omitted (2015) + explicit 2021 must record edition_uniform == false"
        );
    }

    #[test]
    fn single_edition_workspace_is_uniform() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("src")).unwrap();
        std::fs::write(
            p.join("Cargo.toml"),
            "[package]\nname = \"a\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(p.join("src/lib.rs"), "pub fn a() {}\n").unwrap();
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        assert!(inputs.cfg.edition_uniform, "one manifest is trivially uniform");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo test --lib name_resolution::rust_populator::tests::from_convention_edition_is_uniform_by_default 2>&1 | tail -20
```
Expected: FAIL to **compile** — `RustCrateConfig` has no `edition_uniform` field, `ScopeGraph` has no `edition_uniform` field. (Compile failure is the RED signal here.)

- [ ] **Step 3: Add the field to `RustCrateConfig`, drop derived `Default`, and add a manual `Default`**

In `src/name_resolution/rust_populator/mod.rs`, add the field to the struct (after `bin_paths`):

```rust
    /// `[[bin]] path = "..."` overrides (non-convention binary roots).
    pub bin_paths: Vec<String>,
    /// Whether every parsed manifest agreed on a single edition (spec §2
    /// BLOCKER-2). `false` for a mixed-edition workspace; the `ScopeResolution`
    /// disproof predicate keeps-all (disproves nothing) when this is false,
    /// because a wrong-edition anchor could mis-resolve a path and drop a real
    /// edge (P1). Convention fallback (single edition) is `true`.
    #[serde(default = "default_edition_uniform")]
    pub edition_uniform: bool,
```

**Drop `Default` from the derive and add a manual `impl Default` (plan re-confirm
BLOCKER-3).** `RustCrateConfig` currently `#[derive(... Default ...)]`
(`src/name_resolution/rust_populator/mod.rs:63`), and several helpers build it via
struct-update from the default — `..RustCrateConfig::default()` — to set only
`crate_roots` (verified: `src/receiver_index.rs:463,557`, `src/call_graph.rs:2136,2156,2516`,
`src/resolution_identity.rs:202`, `src/name_resolution/binding_lookup.rs:136`,
`src/resolution_receiver/tests.rs:24`, `tests/integration/resolution_test.rs:67`).
A **derived** `Default` would set the new `bool` to `false`, silently **disabling**
the disproof predicate in every one of those fixtures (including the `build()` helper
that drives all the Task-5 integration tests) — the headline recovery would never
fire. `#[serde(default = ...)]` only governs *deserialization*, not
`Default::default()`, so the field default and the serde default must be set
independently. Remove `Default` from the derive:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RustCrateConfig {
```

and add a manual impl immediately after the `struct RustCrateConfig { ... }`
definition (above `impl RustCrateConfig`). It reproduces the prior derived defaults
(all-empty collections / `None`) but pins `edition: 2015` (Cargo's omitted-edition
default — `is_2018_plus` is `edition >= 2018`, so `2015` is anchor-identical to the
old derived `0` for every existing fixture, just faithful) and `edition_uniform:
true` so the struct-update helpers keep the predicate enabled:

```rust
impl Default for RustCrateConfig {
    fn default() -> Self {
        RustCrateConfig {
            edition: 2015,
            crate_roots: Vec::new(),
            workspace_members: Vec::new(),
            dep_renames: BTreeMap::new(),
            lib_path: None,
            bin_paths: Vec::new(),
            edition_uniform: true,
        }
    }
}
```

Add the default function near the top of the module (below the `use` block, above `RustCrateConfig`):

```rust
/// `serde(default)` for `RustCrateConfig::edition_uniform` and
/// `ScopeGraph::edition_uniform`: a legacy cache without the field is treated as
/// uniform (the prior single-edition assumption).
pub fn default_edition_uniform() -> bool {
    true
}
```

Set it in `from_convention` (the returned `RustCrateConfig { ... }` literal):

```rust
        RustCrateConfig {
            edition: 2015,
            crate_roots,
            workspace_members: Vec::new(),
            dep_renames: BTreeMap::new(),
            lib_path: None,
            bin_paths: Vec::new(),
            edition_uniform: true,
        }
```

In `populate_rust`, copy the flag onto the graph (right after `graph.edition = config.edition;`):

```rust
    let mut graph = b.finish();
    graph.edition = config.edition;
    graph.edition_uniform = config.edition_uniform;
    graph
```

- [ ] **Step 4: Add the field to `ScopeGraph`**

In `src/name_resolution/graph.rs`, add a `default_edition_uniform` and the field. Add near the existing `default_edition`:

```rust
fn default_edition_uniform() -> bool {
    true
}
```

Add the field to the `ScopeGraph` struct (after `edition`):

```rust
    /// Rust crate edition used by edition-dependent path anchors.
    ///
    /// Phase 2: per-crate editions for mixed-edition workspaces.
    #[serde(default = "default_edition")]
    pub edition: u16,
    /// Whether every parsed manifest agreed on one edition (spec §2 BLOCKER-2).
    /// Consumed by the `ScopeResolution` disproof predicate: a non-uniform
    /// workspace is non-authoritative for disproof (keep-all), because a
    /// wrong-edition anchor could mis-resolve and drop a real edge (P1).
    #[serde(default = "default_edition_uniform")]
    pub edition_uniform: bool,
```

Set it in `ScopeGraph::new()` (so an empty graph is uniform by default):

```rust
    pub fn new() -> Self {
        ScopeGraph {
            complete: true,
            edition: default_edition(),
            edition_uniform: default_edition_uniform(),
            ..Self::default()
        }
    }
```

Note: `#[derive(Default)]` on `ScopeGraph` makes `bool` default to `false`; the `..Self::default()` path is only used by `new()` which overrides it. The `populate_rust` path always sets the flag explicitly, and `serde(default)` covers cache reads. This is consistent.

- [ ] **Step 5: Compute uniformity in `parse_rust_crate_config`**

In `src/repo_loader.rs`, the manifest loop in `parse_rust_crate_config` currently does last-write-wins on `cfg.edition`. Track the set of editions seen and set `edition_uniform`. Add a local before the loop (after `let mut parsed_any = false;`):

```rust
    let mut parsed_any = false;
    let mut editions_seen: BTreeSet<u16> = BTreeSet::new();
```

Inside the loop, where the edition is parsed, also record it — **faithful to Cargo's
default**: a manifest that declares a `[package]` (i.e. is a crate, not a pure
`[workspace]` root) but *omits* `edition` defaults to **2015** (`parse_edition`
accepts only 2015/2018/2021/2024, and `RustCrateConfig`'s field doc /
`from_convention` already pin "Default 2015 (Cargo's default when omitted)";
verified `src/name_resolution/rust_populator/mod.rs:67,85,114`). An omitted edition
must therefore count as a *seen edition of 2015*, not "no edition" — otherwise a
workspace mixing an omitted (2015) crate with an explicit `edition = "2021"` crate
would record only `{2021}` and be wrongly judged uniform, enabling an unsound prune
on a genuinely mixed-edition repo (P1 recall risk). Pure `[workspace]`-root manifests
(no `[package]`) declare no crate edition and contribute nothing:

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

After the loop, before `Some(cfg)`, set the flag. A workspace where the parsed
`[package]` manifests resolve to 0 or 1 distinct (explicit-or-default-2015) editions
is uniform; ≥2 distinct resolved editions — including an omitted (2015) crate beside
an explicit non-2015 crate — is non-uniform:

```rust
    if !parsed_any {
        return None;
    }
    cfg.edition_uniform = editions_seen.len() <= 1;
    crate_roots.extend(cfg.crate_roots);
```

(`BTreeSet` is already imported in `repo_loader.rs`.)

- [ ] **Step 6: Fix the full `RustCrateConfig { ... }` test literals (compile, BLOCKER-3)**

The struct-update helpers (`..RustCrateConfig::default()`) need no change — the new
manual `Default` (Step 3) supplies `edition_uniform: true`. But four tests in
`tests/name_resolution/rust_populate_test.rs` build `RustCrateConfig` with a **full
field-by-field literal** (no `..default()`), so they will **fail to compile** until
`edition_uniform` is added. Add `edition_uniform: true` (these are single- or
explicit-edition fixtures — uniform) to each (verified line anchors:
`rust_populate_test.rs:755,791,827,1205`). Example for the `:755` literal:

```rust
    let cfg = RustCrateConfig {
        edition: 2015,
        crate_roots: vec!["a/src/lib.rs".to_string(), "dep/src/lib.rs".to_string()],
        workspace_members: vec!["a".to_string(), "dep".to_string()],
        dep_renames: BTreeMap::new(),
        lib_path: None,
        bin_paths: vec![],
        edition_uniform: true,
    };
```

The `:791`/`:827`/`:1205` literals get the same `edition_uniform: true` line. (The
`convention()` helper at `rust_populate_test.rs:158` delegates to `from_convention`,
which Step 3 already sets, so it needs no change.) Grep to confirm none were missed:

```bash
grep -rn "RustCrateConfig {" src/ tests/ | grep -v "edition_uniform\|impl Default"
# any hit that is a full literal (not `..default()`-completed) still needs the field
```

- [ ] **Step 7: Run the tests to verify they pass**

```bash
cargo test --lib name_resolution::rust_populator::tests:: 2>&1 | tail -20
cargo test --lib repo_loader::tests:: 2>&1 | tail -20
cargo test --test name_resolution rust_populate_test:: 2>&1 | tail -20
```
Expected: PASS (both new populator tests + the three new repo_loader edition tests — `mixed_edition_workspace_is_not_uniform`, `omitted_plus_explicit_edition_workspace_is_not_uniform`, `single_edition_workspace_is_uniform` — plus the Task 1 coverage tests, the four updated `rust_populate_test` literals, and the existing parity tests).

- [ ] **Step 8: Commit**

```bash
git add src/name_resolution/rust_populator/mod.rs src/name_resolution/graph.rs src/repo_loader.rs tests/name_resolution/rust_populate_test.rs
git commit -m "$(cat <<'EOF'
feat(scope-graph): edition-uniformity guard on crate config + ScopeGraph

Record whether every parsed manifest agreed on one edition
(RustCrateConfig::edition_uniform, propagated onto ScopeGraph::edition_uniform).
A mixed-edition workspace can mis-anchor a UsePath/LeadingColon path; once the
disproof predicate prunes on such a path that becomes a recall risk (P1), so the
predicate will keep-all when the flag is false (Task 4). serde(default)=true
keeps legacy caches valid.

Drop the derived Default on RustCrateConfig for a manual impl that pins
edition_uniform=true (and edition=2015, Cargo's omitted default, anchor-identical
to the old derived 0): the `..RustCrateConfig::default()` test helpers must keep
the predicate ENABLED — a derived bool default of false would silently disable it
(serde(default) governs only deserialization, not Default::default()). Four full
RustCrateConfig literals in rust_populate_test.rs gain edition_uniform: true.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: The disproof seam (Component 2, spec §3)

A new module with the reusable primitive: a `DisproofPredicate` trait (sound iff it returns `true` only when it can *prove* the candidate is not the target), a read-only `DisproofCx`, and `prune` (intersection; prune-to-empty returns the ORIGINAL pool — no-confidence, never a drop, P1). This task ships the **mechanics only**, validated with a trivial in-test predicate. The real `ScopeResolution` predicate is Task 4.

**Files:**
- Create: `src/resolution_disproof.rs`.
- Modify: `src/lib.rs` (register `pub mod resolution_disproof;`).
- Test: inline `#[cfg(test)] mod tests` in `src/resolution_disproof.rs`.

- [ ] **Step 1: Create the module with the seam + failing tests**

Create `src/resolution_disproof.rs`:

```rust
//! The disproof seam (spec §3): a sound candidate-elimination primitive shared by
//! owner-keyed call resolution.
//!
//! A [`DisproofPredicate`] **proves a candidate is not the callee at a site**; it
//! must be sound — return "not disproved" whenever uncertain. [`prune`] composes
//! predicates by **intersection**: a candidate survives unless *some* predicate
//! disproves it. Recall-safe by construction (P1): adding predicates can only
//! shrink the surviving set, never wrongly drop the true target, and a prune that
//! would empty a non-empty pool returns the ORIGINAL pool (no-confidence, never a
//! drop).
//!
//! This slice ships exactly one predicate — [`ScopeResolution`] (in `resolution.rs`,
//! wired in Task 5). The seam is the extensibility deliverable: future
//! precision-recovery (reachability, arity, receiver-type, trait-bound) becomes new
//! predicates composed into the same intersection.

use crate::call_graph::{CallSite, FunctionId};
use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::types::{FileId, ScopeId};

/// Read-only context a [`DisproofPredicate`] may consult. Borrows the scope graph
/// and the call site's already-resolved enclosing scope (the `(file, from)` the
/// caller computed via `rust_authoritative_scope`), so a predicate need not
/// recompute authority.
#[derive(Clone, Copy)]
pub struct DisproofCx<'a> {
    /// The whole-repo scope graph (authoritative — the caller gated on `complete`).
    pub graph: &'a ScopeGraph,
    /// The call site's containing file in the graph.
    pub file: FileId,
    /// The call site's enclosing lexical scope (from `enclosing_scope`).
    pub from: ScopeId,
}

/// A sound disproof predicate.
///
/// SOUND iff `disproves` returns `true` ONLY when it can prove `cand` is not the
/// target at `site`. Implementations must return `false` (not disproved) on any
/// uncertainty.
pub trait DisproofPredicate {
    fn disproves(&self, cand: &FunctionId, site: &CallSite, cx: &DisproofCx<'_>) -> bool;
}

/// Prune `pool` to the candidates no predicate disproves.
///
/// Intersection semantics: a candidate is kept unless *some* predicate disproves
/// it. If pruning would empty a non-empty `pool`, the ORIGINAL `pool` is returned
/// — a disproof that eliminates everything is treated as no-confidence (P1), never
/// a drop. An empty input `pool` returns empty.
pub fn prune<'a>(
    pool: Vec<&'a FunctionId>,
    site: &CallSite,
    cx: &DisproofCx<'_>,
    preds: &[&dyn DisproofPredicate],
) -> Vec<&'a FunctionId> {
    if pool.is_empty() {
        return pool;
    }
    let kept: Vec<&'a FunctionId> = pool
        .iter()
        .copied()
        .filter(|cand| !preds.iter().any(|p| p.disproves(cand, site, cx)))
        .collect();
    if kept.is_empty() {
        pool
    } else {
        kept
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::call_graph::FunctionId;
    use crate::name_resolution::graph::ScopeGraph;
    use crate::name_resolution::types::{FileId, ScopeId};

    fn fid(name: &str) -> FunctionId {
        FunctionId {
            file: "a.rs".to_string(),
            name: name.to_string(),
            start_line: 1,
            end_line: 1,
        }
    }

    fn dummy_site() -> CallSite {
        CallSite {
            caller: fid("caller"),
            callee_name: "Foo::m".to_string(),
            line: 1,
            kind: Default::default(),
            start_byte: 0,
            end_byte: 0,
            qualifier: None,
            receiver_type: None,
            receiver_recovery: None,
            arg_count: None,
            arg_spread: false,
            receiver_outcome: None,
        }
    }

    fn dummy_cx(graph: &ScopeGraph) -> DisproofCx<'_> {
        DisproofCx {
            graph,
            file: FileId(0),
            from: ScopeId(0),
        }
    }

    /// A trivial predicate that disproves exactly the candidates whose name is in
    /// its deny-list. Used to validate the seam mechanics (not shipped).
    struct DenyNames(Vec<String>);
    impl DisproofPredicate for DenyNames {
        fn disproves(&self, cand: &FunctionId, _site: &CallSite, _cx: &DisproofCx<'_>) -> bool {
            self.0.contains(&cand.name)
        }
    }

    #[test]
    fn prune_keeps_undisproved_candidates() {
        let graph = ScopeGraph::new();
        let a = fid("keep_a");
        let b = fid("deny_b");
        let pool = vec![&a, &b];
        let pred = DenyNames(vec!["deny_b".to_string()]);
        let kept = prune(pool, &dummy_site(), &dummy_cx(&graph), &[&pred]);
        assert_eq!(kept, vec![&a], "only the disproved candidate is removed");
    }

    #[test]
    fn prune_to_empty_returns_original_pool() {
        // P1: a prune that eliminates EVERYTHING is no-confidence → keep the
        // original pool, never a drop.
        let graph = ScopeGraph::new();
        let a = fid("deny_a");
        let b = fid("deny_b");
        let pred = DenyNames(vec!["deny_a".to_string(), "deny_b".to_string()]);
        let kept = prune(vec![&a, &b], &dummy_site(), &dummy_cx(&graph), &[&pred]);
        assert_eq!(kept, vec![&a, &b], "prune-to-empty returns the original pool");
    }

    #[test]
    fn prune_with_no_predicates_is_identity() {
        let graph = ScopeGraph::new();
        let a = fid("a");
        let b = fid("b");
        let kept = prune(vec![&a, &b], &dummy_site(), &dummy_cx(&graph), &[]);
        assert_eq!(kept, vec![&a, &b]);
    }

    #[test]
    fn prune_empty_input_is_empty() {
        let graph = ScopeGraph::new();
        let pred = DenyNames(vec![]);
        let kept: Vec<&FunctionId> = prune(Vec::new(), &dummy_site(), &dummy_cx(&graph), &[&pred]);
        assert!(kept.is_empty());
    }

    #[test]
    fn prune_intersection_across_two_predicates() {
        // A candidate survives only if NO predicate disproves it.
        let graph = ScopeGraph::new();
        let a = fid("a");
        let b = fid("b");
        let c = fid("c");
        let p1 = DenyNames(vec!["b".to_string()]);
        let p2 = DenyNames(vec!["c".to_string()]);
        let kept = prune(vec![&a, &b, &c], &dummy_site(), &dummy_cx(&graph), &[&p1, &p2]);
        assert_eq!(kept, vec![&a]);
    }
}
```

- [ ] **Step 2: Register the module**

In `src/lib.rs`, add the module declaration in alphabetical position (immediately after `pub mod resolution;`):

```rust
pub mod resolution;
pub mod resolution_disproof;
pub mod resolution_identity;
```

- [ ] **Step 3: Run the tests to verify they fail, then pass**

```bash
cargo test --lib resolution_disproof:: 2>&1 | tail -20
```
Expected: PASS. (Because the implementation and tests are written together here, confirm the suite compiles and all five tests pass. If you want to see RED first, temporarily change `prune`'s empty-fallback to `kept` and re-run — `prune_to_empty_returns_original_pool` then FAILS — then restore.)

- [ ] **Step 4: Commit**

```bash
git add src/resolution_disproof.rs src/lib.rs
git commit -m "$(cat <<'EOF'
feat(scope-graph): disproof seam (DisproofPredicate, DisproofCx, prune)

New src/resolution_disproof.rs: a sound candidate-elimination primitive.
Predicates compose by intersection; a candidate survives unless some predicate
proves it not the target. prune-to-empty returns the ORIGINAL pool (P1: a prune
that eliminates everything is no-confidence, never a drop). Mechanics validated
with a trivial deny-list predicate; the shipped ScopeResolution predicate lands
next.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: The `ScopeResolution` predicate (Component 2, spec §3 + §8.1 ①C + §8.2 ②B)

The one shipped predicate. It disproves a candidate **only when BOTH**:

- **②B direct binding (§8.2):** the owner type-path's **leading type segment** binds via a **direct `BindTarget::Resolved`** to an in-repo `Target::Item`, with **no `Pending` hop**. Proven by inspecting the **call site's own visible rib binding** for that segment (the binding the engine would select at the anchored scope, BEFORE it folds any `Pending` chase) and requiring it to be a single visible non-glob `Resolved(Item)` — NOT by reading the resolved `Candidate` (the engine folds direct-`Resolved` and chased-`Pending` into an empty-`provenance` `Candidate`, so directness is unreadable there), and NOT by a global `graph.bindings` search for any `Resolved(target)` (the target type's def site always carries such a binding, so a global search mistakes a `Pending` re-export/alias hit at the call site for a direct hit — the codex BLOCKER → unsound prune → recall loss). A `Pending` (named `use`/re-export) rib binding, a glob-only hit, or an ambiguous/multi rib ⇒ keep-all.
- **①C no block-local shadow (§8.1):** the leading segment has **no potential block-local shadow** of that exact ident at the call site. Three shapes, two recorded *without* an exact-ident binding (so an exact-ident-only scan is too narrow): **(a)** a visible `NS_TYPE` `Binding` for the exact leading ident, **(b)** a visible block-local **glob** `Edge` whose `glob_vis_range` covers the call byte, or **(c)** a covering **macro wildcard** in `NS_TYPE`. ANY of the three ⇒ keep-all.

Plus the §2 guard: a **non-uniform-edition** graph (`graph.edition_uniform == false`) ⇒ keep-all.

When both hold (and the candidate's resolved id-set is known), it disproves any candidate **not in** that id-set. The id-set comes from reusing the existing `graph_target_resolution` logic for the *final callable* — but that helper is **not** the directness oracle.

Because the helpers in `resolution.rs` it needs (`rust_call_path_anchor`, `graph_target_resolution`, `rust_graph_qualified_callable_edge`, `graph_file_for_scope`, `graph_owner_name_for_scope`) are private free functions / private methods, the predicate lives in `resolution.rs` (same module → no visibility changes) and is **re-exported** so the seam's caller can name it. Local helpers are added in `resolution.rs` for the directness re-resolve and the shadow scan.

**Files:**
- Modify: `src/resolution.rs` — add the `ScopeResolution` predicate struct + `impl DisproofPredicate` + the local helpers (`leading_segment_binds_directly`, `leading_segment_has_block_local_shadow`, `scope_chain_below_module` — excludes the module/root scope so the resolved type's own def-binding is not mis-read as a shadow, plan re-confirm BLOCKER-1), and `pub use` it.
- Test: `tests/integration/resolution_test.rs` (predicate-behavior tests through the public resolve path are in Task 5; this task's tests are inline unit tests in `src/resolution.rs` driving the predicate over hand-built graphs would be heavy — instead, **pin the helpers' observable behavior through `resolve_call_site` fixtures in Task 5**, and add the *guard* unit tests here in `src/resolution.rs` that need no full pipeline).

> **Note on test placement.** The predicate's end-to-end behavior (resolves-to-1, pruned-to-2, no-resolution keep-all, Pending-import-alias-over-colliding-pool keep-all [②B directness], block-local-glob keep-all, macro-wildcard keep-all, mixed-edition keep-all) is exercised through `resolve_call_site` in **Task 5**, where the integration wires the predicate into the live path and the existing `build()`/`build_rust_complete` helpers produce real graphs. This task implements the predicate and adds one inline unit test for the edition guard (the only branch reachable without the full integration). Splitting the behavior tests into Task 5 keeps each test against the real code path rather than a hand-mocked graph.

- [ ] **Step 1: Write the failing inline guard test**

In `src/resolution.rs`, add a new test module at the end of the file (after `embedding_kind_tests`):

```rust
#[cfg(test)]
mod scope_resolution_predicate_tests {
    use super::*;
    use crate::call_graph::{CallSite, FunctionId};
    use crate::name_resolution::graph::ScopeGraph;
    use crate::name_resolution::types::{FileId, ScopeId};
    use crate::resolution_disproof::{DisproofCx, DisproofPredicate};

    fn fid(name: &str) -> FunctionId {
        FunctionId {
            file: "a.rs".to_string(),
            name: name.to_string(),
            start_line: 1,
            end_line: 1,
        }
    }

    fn site(callee: &str) -> CallSite {
        CallSite {
            caller: fid("caller"),
            callee_name: callee.to_string(),
            line: 1,
            kind: Default::default(),
            start_byte: 0,
            end_byte: 0,
            qualifier: None,
            receiver_type: None,
            receiver_recovery: None,
            arg_count: None,
            arg_spread: false,
            receiver_outcome: None,
        }
    }

    #[test]
    fn non_uniform_edition_disproves_nothing() {
        // §2 guard: a mixed-edition graph is non-authoritative for disproof.
        let cg = CallGraph::build(&std::collections::BTreeMap::new());
        let mut graph = ScopeGraph::new();
        graph.edition_uniform = false;
        let cx = DisproofCx {
            graph: &graph,
            file: FileId(0),
            from: ScopeId(0),
        };
        let pred = ScopeResolution::new(&cg);
        let cand = fid("with_file");
        assert!(
            !pred.disproves(&cand, &site("CliTest::with_file"), &cx),
            "non-uniform edition must disprove nothing (keep-all)"
        );
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test --lib scope_resolution_predicate_tests:: 2>&1 | tail -20
```
Expected: FAIL to compile — `ScopeResolution` does not exist yet.

- [ ] **Step 3: Implement the `ScopeResolution` predicate + helpers**

Add to `src/resolution.rs`. First, ensure the needed imports are present. The file already imports `BindTarget`, `Candidate`, `ScopeId`, `SourceLoc`, `Target`, `Anchor`, `AnchorKind`, `RawPath`, `ResStatus`, `FileId` from `name_resolution::types` and `NS_TYPE`/`NS_VALUE` from `rust_policy` (verified: `src/resolution.rs:9` and `:11-14`). Add `EK_GLOB` to the `rust_policy` import line and `Binding`/`Edge`/`Span`/`ResolveQuery`/`ResolutionPolicy`/`TraversalCtx` to the `types` import line (`ResolveQuery`/`TraversalCtx` are needed by the corrected ②B call-site-rib directness helper to mirror the engine's `visible()` hook; **`ResolutionPolicy` is the trait whose `anchor`/`visible` methods the directness helper calls on `RustPolicy` — without it in scope those method calls do not resolve, plan re-confirm BLOCKER-2**; `Binding`/`Span` by the ①C shadow scan; `Edge` by its glob-edge check). `resolve_path` is already imported at the top (`src/resolution.rs:7`):

```rust
use crate::name_resolution::rust_policy::{RustPolicy, EK_GLOB, NS_TYPE, NS_VALUE};
```

```rust
use crate::name_resolution::types::{
    Anchor, AnchorKind, BindTarget, Binding, Candidate, Edge, FileId, RawPath, ResolutionPolicy,
    ResolveQuery, ResStatus, ScopeId, SourceLoc, Span, Target, TraversalCtx,
};
```

Now add the predicate and helpers. Place this block immediately **after** the `impl CallGraph { ... }` block that ends near line 1180 (i.e. as free items at module scope, alongside `rust_call_path_anchor` and friends), and add `MacroWildcard` access via `graph.macro_wildcards`:

```rust
/// The one shipped disproof predicate (spec §3). Disproves a candidate only when
/// the owner type-path's leading segment binds **directly** to an in-repo item
/// (②B) AND has **no block-local shadow** at the call site (①C) AND the graph is
/// edition-uniform (§2 guard). On all uncertainty it disproves nothing (P1).
pub struct ScopeResolution<'a> {
    cg: &'a CallGraph,
}

impl<'a> ScopeResolution<'a> {
    pub fn new(cg: &'a CallGraph) -> Self {
        ScopeResolution { cg }
    }
}

impl crate::resolution_disproof::DisproofPredicate for ScopeResolution<'_> {
    fn disproves(
        &self,
        cand: &FunctionId,
        site: &CallSite,
        cx: &crate::resolution_disproof::DisproofCx<'_>,
    ) -> bool {
        let graph = cx.graph;
        // §2 guard: a non-uniform-edition workspace is non-authoritative for
        // disproof. Keep-all.
        if !graph.edition_uniform {
            return false;
        }
        // Only `T::m` / `mod::T::m` owner paths are in scope for this predicate.
        let Some((anchor, path)) = rust_call_path_anchor(site.callee_name.as_str()) else {
            return false;
        };
        // The leading type segment is the path's first segment; the trailing
        // segment is the method. A path with <2 segments has no owner type.
        if path.0.len() < 2 {
            return false;
        }
        let leading = &path.0[0];

        // ①C: any potential block-local shadow of the leading ident → keep-all.
        if leading_segment_has_block_local_shadow(graph, cx.from, cx.file, site.start_byte, leading)
        {
            return false;
        }

        // ②B: the leading segment must bind DIRECTLY to an in-repo Item (no
        // Pending hop). Proven from the binding shape, not the resolved Candidate.
        if !leading_segment_binds_directly(graph, cx.file, cx.from, site.start_byte, &anchor, leading)
        {
            return false;
        }

        // Both contracts hold → the path is deterministic. Resolve the FINAL
        // callable target and its id-set (reusing graph_target_resolution's
        // body); disprove `cand` iff it is NOT in that id-set.
        let Some(target) =
            rust_graph_qualified_callable_edge(graph, site, cx.file, cx.from)
        else {
            // The final callable did not resolve to one target → no id-set to
            // prove membership against → keep-all.
            return false;
        };
        let ids = self.cg.graph_target_ids(graph, &target);
        if ids.is_empty() {
            return false;
        }
        // Disprove only candidates outside the resolved id-set.
        !ids.contains(&cand)
    }
}

/// Prove the leading type segment binds **directly** at the CALL SITE — i.e. the
/// binding the call site actually sees for `leading` is itself a non-glob,
/// non-`Pending` `BindTarget::Resolved(Target::Item)` (§8.2 decision ②B).
///
/// **Why a global target search is WRONG (codex BLOCKER):** the engine folds a
/// chased re-export (`pub use Real as Facade`) — a `BindTarget::Pending` binding
/// at the call site (rust_populator/walk/items.rs:216) — into an empty-provenance
/// `Candidate` identical to a direct hit (engine.rs:214-220), so the resolved
/// `Candidate` cannot tell direct from aliased. And the target type's OWN
/// definition always carries a `BindTarget::Resolved(Item)` binding at its def
/// site (rust_populator/walk/types.rs:52), so *any* `graph.bindings` search for
/// `Resolved(target)` matches that def even when the call site reached it via a
/// `Pending` facade. That mistakes an alias hit for a direct call-site hit →
/// unsound prune → recall loss. Directness MUST be proven from the call site's
/// own visible binding.
///
/// We mirror the engine's member-lookup rib selection (engine.rs
/// `scope_member_lookup` / `resolve_rib`): anchor `leading` from the call site,
/// take the explicit `NS_TYPE` bindings claimed at the anchored scope's rib
/// (cfg-compatible + visible), and require the binding the rib selects to be a
/// direct `Resolved(Target::Item)` — **without chasing any `Pending`**. A
/// `Pending` rib binding (a named `use`/re-export — facade OR plain import), a
/// rib that resolves only through a glob, an ambiguous/multi rib, or a non-`Item`
/// target all return `false` (keep-all). This deliberately forgoes the recovery
/// on *all* import aliases, not just facades — that is the spec's "direct-binding
/// -only" precision floor (a plain `use` is itself a hop), and it is recall-safe.
///
/// `leading` is the path's first NON-anchor segment (`rust_call_path_anchor`
/// folds `crate`/`self`/`super` into `anchor`). For a bare-owner `T::m` that is
/// the owner type `T` itself (the realized-recovery case, e.g. the two-crate
/// `CliTest::with_file` headline). For a module-qualified `mod::T::m` it is the
/// MODULE segment, whose binding is `Resolved(Target::Scope)` (a `mod foo;` is a
/// Scope, not an Item) → this returns `false` → keep-all. That matches the prior
/// helper (which also required `Target::Item`, so it too kept-all on module-
/// prefixed paths): module-qualified owner collisions stay NameOnly (recall-safe,
/// precision-forgone). The realized recovery surface is bare-owner `T::m`, the
/// dominant shape; extending to `mod::T::m` is the §9 deferred follow-up.
fn leading_segment_binds_directly(
    graph: &ScopeGraph,
    file: FileId,
    from: ScopeId,
    byte: usize,
    anchor: &Anchor,
    leading: &str,
) -> bool {
    let at = SourceLoc { file, byte };
    let policy = RustPolicy::new(graph, graph.edition);
    // Anchor the leading segment exactly as the full call path does. `None` ⇒
    // conservative keep-all (no provable anchor).
    let Some((start, _)) = policy.anchor(anchor, from) else {
        return false;
    };
    // The explicit rib for (`leading`, NS_TYPE) the call site sees in the anchored
    // scope, cfg-compatible. (A member-lookup rib is module-wide for a path
    // segment — engine.rs `scope_member_lookup` does NOT byte-gate it — so we do
    // not filter on vis_extents here; we still require visibility below.)
    let rib: Vec<&Binding> = graph
        .bindings
        .iter()
        .filter(|b| b.scope == start && b.name == leading && b.ns == NS_TYPE)
        .collect();
    // No explicit binding at the anchored rib ⇒ the call site reaches `leading`
    // only via a glob/lexical-outer/prelude path (or not at all): NOT a direct
    // in-scope item ⇒ keep-all. (We deliberately do not consult `glob_lookup`: a
    // glob hit is by definition not a direct binding.)
    if rib.is_empty() {
        return false;
    }
    // Exactly one visible rib binding, and it must be a DIRECT in-repo item — a
    // `Resolved(Item)`, never a `Pending` (re-export/import) hop. >1 visible ⇒
    // ambiguous ⇒ keep-all. The visibility check mirrors the engine's `visible()`
    // hook via a from-vantage TYPE-ns query.
    let q = ResolveQuery {
        name: leading.to_string(),
        ns: NS_TYPE,
        from,
        at: at.clone(),
        cfg: Default::default(),
        ctx: Default::default(),
    };
    let visible: Vec<&Binding> = rib
        .into_iter()
        .filter(|b| {
            let trav = TraversalCtx {
                lookup_scope: Some(b.scope),
                via_glob: false,
                edge_kind: None,
            };
            policy.visible(b, &q, &trav)
        })
        .collect();
    match visible.as_slice() {
        [b] => matches!(&b.target, BindTarget::Resolved(Target::Item { .. })),
        _ => false,
    }
}

/// Does the lexical scope chain from `from` UP TO **but excluding** the enclosing
/// module/root contain any potential block-local shadow of `leading` at `byte`?
/// Three shapes (§8.1 decision ①C / re-review BLOCKER): (a) an exact `NS_TYPE`
/// binding, (b) a block-local glob `Edge` whose `glob_vis_range` covers `byte`,
/// (c) a covering `NS_TYPE` macro wildcard. ANY ⇒ shadow (keep-all).
///
/// **Why the scan excludes the module/root scope (plan re-confirm BLOCKER-1).**
/// A bare `T::m` anchors its leading ident at the **enclosing module** (`RustPolicy`
/// `anchor` for `AnchorKind::Bare`, rust_policy.rs:296-302), and the resolved type
/// `T`'s OWN definition is a module/root `NS_TYPE` `Resolved(Item)` binding
/// (`walk_struct`, walk/types.rs:52). If the scan INCLUDED the module/root scope,
/// shape (a) would match that very def-binding and classify `T`'s own definition as
/// a "shadow" — keep-all would fire on EVERY direct in-module type, so the headline
/// `CliTest::with_file` singleton recovery could never run. ①C only guards
/// **block-local** shadows the module-anchored ②B resolution cannot see; those live
/// strictly below the module (a block-local `use`/`struct`/macro lands in a
/// `Block`/`Callable`/`Type` scope: locals.rs Block/Callable scopes, items.rs
/// item-position bindings). A *module-level* `use Foo` is ②B's domain — it is a
/// `Pending` module-rib binding, so ②B already keeps-all. So excluding the
/// module/root loses no real shadow (recall-safe) and only removes the
/// self-shadow false positive. (Shape (b) is independently safe: a module/root glob
/// records `glob_vis_range == None` — items.rs:205 — so it could never match the
/// `is_some_and` byte-cover check regardless.)
fn leading_segment_has_block_local_shadow(
    graph: &ScopeGraph,
    from: ScopeId,
    file: FileId,
    byte: usize,
    leading: &str,
) -> bool {
    let at = SourceLoc { file, byte };
    for scope in scope_chain_below_module(graph, from) {
        // (a) an exact-ident NS_TYPE binding in this block/callable/type scope
        //     whose visibility extent covers the call byte.
        let exact_binding = graph.bindings.iter().any(|b| {
            b.scope == scope
                && b.name == leading
                && b.ns == NS_TYPE
                && binding_vis_covers(b, &at)
        });
        if exact_binding {
            return true;
        }
        // (b) a block-local glob edge whose glob_vis_range covers the call byte.
        let glob_shadow = graph.edges.iter().any(|e| {
            e.from == scope
                && e.kind == EK_GLOB
                && e.vis_range.as_ref().is_some_and(|s| span_covers(s, &at))
        });
        if glob_shadow {
            return true;
        }
        // (c) a covering NS_TYPE macro wildcard in this scope.
        let macro_shadow = graph
            .macro_wildcards
            .iter()
            .any(|m| m.scope == scope && m.ns == NS_TYPE && span_covers(&m.range, &at));
        if macro_shadow {
            return true;
        }
    }
    false
}

/// The lexical scope chain from `from` up to but **EXCLUDING** the enclosing
/// `Module`/`Root` — the block/callable/type region strictly between the call site
/// and its enclosing module, where a bare `T::m` anchor's leading ident could be
/// block-locally shadowed (member lookup has no lexical fall-out, so a shadow here
/// is invisible to the module-anchored path — §8.1). The module/root scope itself
/// is **NOT** scanned: the resolved type `T`'s own def-binding lives there, and
/// including it would self-shadow every direct in-module type and defeat the
/// recovery (plan re-confirm BLOCKER-1); module-level imports are ②B's domain.
fn scope_chain_below_module(graph: &ScopeGraph, from: ScopeId) -> Vec<ScopeId> {
    use crate::name_resolution::types::ScopeKind;
    let mut out = Vec::new();
    let mut cur = Some(from);
    while let Some(id) = cur {
        let Some(s) = graph.scope(id) else { break };
        if matches!(s.kind, ScopeKind::Module | ScopeKind::Root) {
            break; // stop AT the module/root WITHOUT scanning it
        }
        out.push(id);
        cur = s.parent;
    }
    out
}

/// `Binding::vis_extents` cover `at` (empty extents ⇒ scope-wide ⇒ covered),
/// mirroring the engine's `vis_extent_covers`.
fn binding_vis_covers(b: &Binding, at: &SourceLoc) -> bool {
    if b.vis_extents.is_empty() {
        return true;
    }
    b.vis_extents.iter().any(|s| span_covers(s, at))
}

/// Half-open `[lo, hi)` same-file span cover (mirrors `engine::span_covers`).
fn span_covers(s: &Span, at: &SourceLoc) -> bool {
    s.lo.file == at.file && at.byte >= s.lo.byte && at.byte < s.hi.byte
}
```

The predicate calls `self.cg.graph_target_ids(graph, &target)` — a method that returns the resolved id-set. Today `graph_target_resolution` computes that set but returns `None` unless `len()==1`. Refactor it (next step) to expose the set, so both the predicate and Task 5's integration share one id-computation.

- [ ] **Step 4: Extract `graph_target_ids` from `graph_target_resolution`**

In `src/resolution.rs`, inside `impl CallGraph`, add a method that returns the raw id-set (the existing loop body of `graph_target_resolution`, minus the `len()` decision). Place it directly above `graph_target_resolution`:

```rust
    /// The in-repo `FunctionId`s a resolved callable `Target` maps to, applying
    /// the same per-binding file + owner narrowing `graph_target_resolution`
    /// uses. Shared by the `ScopeResolution` predicate and `graph_target_resolution`.
    fn graph_target_ids<'b>(&'b self, graph: &ScopeGraph, target: &Target) -> Vec<&'b FunctionId> {
        let mut ids: Vec<&FunctionId> = Vec::new();
        if !matches!(target, Target::Item { callable: true, .. }) {
            return ids;
        }
        for binding in graph.bindings.iter() {
            if !matches!(&binding.target, BindTarget::Resolved(t) if t == target) {
                continue;
            }
            let Some(file) = graph_file_for_scope(graph, binding.scope) else {
                continue;
            };
            let owner = graph_owner_name_for_scope(graph, binding.scope);
            if let Some(functions) = self.functions.get(&binding.name) {
                for fid in functions
                    .iter()
                    .filter(|fid| graph.file_paths.get(&fid.file).copied() == Some(file))
                {
                    match owner.as_deref() {
                        Some(owner)
                            if self.method_owners.get(fid).map(String::as_str) != Some(owner) =>
                        {
                            continue;
                        }
                        None if self.method_owners.contains_key(fid) => {
                            continue;
                        }
                        _ => {}
                    }
                    if !ids.contains(&fid) {
                        ids.push(fid);
                    }
                }
            }
        }
        ids
    }
```

Now rewrite `graph_target_resolution` to call it and apply the new `1→Exact / >1→demoted` rule (this is also a Task 5 requirement; doing it here keeps the predicate and the integration consistent in one edit):

```rust
    fn graph_target_resolution(
        &self,
        graph: &ScopeGraph,
        site: &CallSite,
        target: &Target,
    ) -> Option<Vec<ResolvedCallee<'_>>> {
        let ids = self.graph_target_ids(graph, target);
        if ids.is_empty() {
            return None;
        }
        let qualified = site.callee_name.contains("::");
        match ids.len() {
            1 => {
                let kind = if qualified {
                    ResolutionKind::QualifiedOwner
                } else if ids[0].file == site.caller.file {
                    ResolutionKind::LocalDef
                } else {
                    ResolutionKind::FreeSingle
                };
                Some(exact(ids, kind))
            }
            _ => {
                // >1: a `::`-qualified owner that owns inherent + trait (or cfg
                // variants) demotes to NameOnly (recall-safe — keep every edge).
                // The unqualified LocalDef/FreeSingle arms stay singleton-only:
                // a >1 unqualified free-fn set routes through the existing free-fn
                // rungs, so decline here (return None) to fall through.
                if qualified {
                    Some(demoted(ids, ResolutionKind::QualifiedOwner))
                } else {
                    None
                }
            }
        }
    }
```

- [ ] **Step 5: Re-export the predicate**

At the top of `src/resolution.rs`, near the other `pub use` re-exports (after the `pub use crate::resolution_receiver::...` line), add:

```rust
pub use crate::resolution_disproof::{prune, DisproofCx, DisproofPredicate};
```

The `ScopeResolution` struct is already `pub` and lives in this module, so `prism::resolution::ScopeResolution` is the public path; no extra re-export is needed for it.

- [ ] **Step 6: Run the guard test to verify it passes + full lib suite**

```bash
cargo test --lib scope_resolution_predicate_tests:: 2>&1 | tail -20
cargo test --lib 2>&1 | tail -25
```
Expected: `non_uniform_edition_disproves_nothing` PASSES. The full `--lib` suite PASSES (the `graph_target_resolution` rewrite is behavior-equivalent for the `len()==1` case the existing lib tests exercise; the `>1` case was previously `None`/drop and is not asserted by any `--lib` test — Task 5 pins it through integration).

- [ ] **Step 7: Run formatting + commit**

```bash
cargo fmt
cargo test --lib resolution 2>&1 | tail -10
git add src/resolution.rs
git commit -m "$(cat <<'EOF'
feat(scope-graph): ScopeResolution disproof predicate (decisions ①C + ②B)

The one shipped predicate. Disproves a candidate only when the owner type-path's
leading segment binds DIRECTLY to an in-repo Item (②B: re-resolve the segment,
require a non-glob BindTarget::Resolved with no Pending hop — proven from the
binding shape, not the resolved Candidate) AND has NO block-local shadow at the
call site (①C: exact NS_TYPE binding, block-local glob edge, or covering
NS_TYPE macro wildcard) AND the graph is edition-uniform (§2). On any
uncertainty it disproves nothing (P1).

Extract graph_target_ids (shared id-set) and switch graph_target_resolution to
1→Exact / >1→demoted (qualified owner) so the >1 owner-collision case demotes
instead of dropping; unqualified >1 still falls through to the free-fn rungs.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Integration — prune → Exact / pruned-demote, fail-open (Component 3, spec §4)

Wire the predicate into the live Rust scope path in `resolve_call_site_full`. When the graph is present and authoritative: fetch the bare-name pool for `(owner, method)` from `self.methods`, run `prune` with `ScopeResolution`, and decide `1→Exact / >1→pruned-demote / unchanged→fall through`. Narrow the fail-open at `resolution.rs:687` to `name.contains("::")` owner sites routed ONLY to `owner_lookup_in_modules`'s #120 demote (NOT the legacy stem block), preserving the three shipped drop invariants.

The probes that anchor this task's RED/GREEN (run during planning, reproducible):
- A 2-crate `CliTest::with_file()` with the graph present already resolves `1→Exact` via `graph_target_resolution`; **without** the graph it is `2×NameOnly` (ruff today). Component 1 builds the graph on ruff → that flip is the headline recovery.
- An inherent+trait `Widget::make()` with the graph present resolves through the scope block but `graph_target_resolution` returned `None` (ids==2) → **today it DROPS (`UnknownName`)**. The §4 change makes it **`2×NameOnly` demote** (the predicate keeps both — both are owned by the resolved `Widget` scope). This is a recall fix (drop→demote).

**Files:**
- Modify: `src/resolution.rs` — the Rust scope branch of `resolve_call_site_full` (lines ~690-702) and the fail-open path.
- Test: `tests/integration/resolution_test.rs` — the predicate behavior tests + the headline recovery + fail-open + re-confirm the three drop invariants.

- [ ] **Step 1: Write the failing integration tests**

Add to `tests/integration/resolution_test.rs` (near the other scope-graph tests, e.g. after `rust_scope_graph_authority_gate_and_poison_skip_legacy`):

```rust
#[test]
fn scope_graph_two_crate_owner_collision_recovers_to_single_exact() {
    use prism::languages::Language::Rust;
    // The ruff CliTest::with_file class in miniature: two crates each define
    // `CliTest::with_file`. With the scope graph present, a call in crate `a`
    // resolves to crate `a`'s definition alone — single Exact (the headline
    // recovery). The bare `("CliTest","with_file")` key holds BOTH defs.
    //
    // This is THE test BLOCKER-1 (the module-scope exclusion) unblocks: there is
    // NO block-local shadow here, so ①C must NOT fire — the `CliTest` def lives at
    // module/root scope (②B's anchor + directness proof), and `scope_chain_below_
    // module` excludes that scope, so the predicate prunes the cross-crate `b`
    // candidate and the pool resolves to a single Exact. Under the old module-
    // INCLUSIVE scan, `CliTest`'s own def-binding would self-shadow → keep-all →
    // this would stay 2×NameOnly and FAIL. The block-local-glob test below pins the
    // complementary direction (a real block shadow still keeps-all).
    let sources = [
        (
            "a/src/lib.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn with_file(&self) {}\n}\npub fn drive() {\n    CliTest::with_file();\n}\n",
            Rust,
        ),
        (
            "b/src/lib.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn with_file(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    assert!(cg.scope_graph.is_some(), "convention build has a scope graph");
    assert_eq!(
        cg.methods
            .get(&("CliTest".to_string(), "with_file".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key collides across both crates"
    );
    let r = cg.resolve_call_site(&site_in(&cg, "drive", "CliTest::with_file"));
    assert_eq!(r.len(), 1, "recovers to a single candidate");
    assert_eq!(r[0].target.file, "a/src/lib.rs");
    assert_eq!(r[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(r[0].kind, ResolutionKind::QualifiedOwner);
}

#[test]
fn scope_graph_inherent_plus_trait_owner_demotes_not_drops() {
    use prism::languages::Language::Rust;
    // The resolved type `Widget` owns BOTH an inherent `make` and a trait `make`.
    // The leading segment binds directly + unshadowed, so the predicate runs, but
    // it cannot prune below the inherent/trait pair (both owned by Widget). Today
    // this DROPS (graph_target_resolution returned None on ids==2); the §4 change
    // demotes it to 2 NameOnly edges (recall fix: drop -> demote).
    let sources = [(
        "src/lib.rs",
        "pub struct Widget;\npub trait Build { fn make(&self); }\nimpl Widget { pub fn make(&self) {} }\nimpl Build for Widget { fn make(&self) {} }\npub fn drive() {\n    Widget::make();\n}\n",
        Rust,
    )];
    let (cg, _) = build(&sources);
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Widget::make"));
    assert_eq!(out.drop, None, "must not drop — recall fix");
    assert_eq!(out.resolved.len(), 2, "inherent + trait both kept");
    assert!(
        out.resolved
            .iter()
            .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "the unprunable owner pair demotes to NameOnly"
    );
    assert!(out
        .resolved
        .iter()
        .all(|c| c.kind == ResolutionKind::QualifiedOwner));
}

#[test]
fn scope_graph_unresolved_owner_path_keeps_full_pool_not_drop() {
    use prism::languages::Language::Rust;
    // The owner type path does NOT resolve through the graph (`Missing` is not in
    // scope at the call site — it is neither defined nor imported in lib.rs), but
    // the bare owner key COLLIDES across two files. ②B's call-site directness
    // fails (no `Missing` rib binding at the call site) → keep-all, and the
    // fail-open routes the `::` site to the #120 demote floor — NameOnly×2, NOT a
    // drop, NOT a stem guess.
    //
    // The pool MUST collide (two `Missing::make` defs): `owner_lookup_in_modules`
    // demotes a >1 same-owner pool to NameOnly but returns *Exact* for a singleton
    // (resolution.rs:677-679) — a single-def fixture would (correctly) be Exact and
    // FALSIFY the NameOnly assertion (codex MAJOR @ this fixture). The collision is
    // the demote floor the test means to pin.
    let sources = [
        (
            "src/lib.rs",
            "mod other;\nmod more;\npub fn drive() {\n    Missing::make();\n}\n",
            Rust,
        ),
        (
            "src/other.rs",
            "pub struct Missing;\nimpl Missing {\n    pub fn make(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/more.rs",
            "pub struct Missing;\nimpl Missing {\n    pub fn make(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    // Two same-owner `Missing::make` defs → the fail-open demotes (NameOnly),
    // never Exact, never a drop, never the stem heuristic.
    assert_eq!(
        cg.methods
            .get(&("Missing".to_string(), "make".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key must collide so the floor is a NameOnly demote, not Exact",
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Missing::make"));
    assert_eq!(out.drop, None, "owner-keyed `::` miss demotes, not drops");
    assert_eq!(out.resolved.len(), 2, "both colliding defs are kept (recall)");
    assert!(
        out.resolved
            .iter()
            .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "fail-open lands at the #120 NameOnly demote floor"
    );
    assert!(
        out.resolved
            .iter()
            .all(|c| c.kind == ResolutionKind::QualifiedOwner),
        "a same-owner collision demotes as QualifiedOwner (not TraitCha / not stem)"
    );
}

#[test]
fn scope_graph_block_local_glob_shadow_keeps_all() {
    use prism::languages::Language::Rust;
    // Module-level `use a::Foo;` + a function with a block-local `use b::*;` and a
    // `Foo::m()` call. Both `a::Foo` and `b::Foo` exist with `m`. The
    // module-anchored resolution sees only `a::Foo`, but the block-local glob is a
    // potential shadow of `Foo` → the predicate keeps all (does NOT prune to
    // a::Foo::m). An exact-ident-only ①C would wrongly drop b::Foo::m (the BLOCKER).
    let sources = [
        (
            "src/lib.rs",
            "mod a;\nmod b;\nuse crate::a::Foo;\npub fn drive() {\n    use crate::b::*;\n    Foo::m();\n}\n",
            Rust,
        ),
        ("src/a.rs", "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n", Rust),
        ("src/b.rs", "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n", Rust),
    ];
    let (cg, _) = build(&sources);
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Foo::m"));
    // Keep-all: the bare ("Foo","m") key holds both; the block-local glob blocks
    // the prune, so this falls through to the demote floor (NameOnly, not a single
    // Exact, not a drop).
    assert_eq!(out.drop, None);
    assert!(
        out.resolved.len() >= 2
            && out
                .resolved
                .iter()
                .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "block-local glob shadow keeps the full pool at NameOnly"
    );
}

#[test]
fn scope_graph_macro_wildcard_shadow_keeps_all() {
    use prism::languages::Language::Rust;
    // An item-position name-introducing macro invocation poisons NS_TYPE over the
    // trailing block scope; a bare `Foo::m()` after it could be shadowed by an
    // unknowable macro-introduced `Foo` → keep-all.
    let sources = [
        (
            "src/lib.rs",
            "mod a;\nmod b;\nuse crate::a::Foo;\nmacro_rules! gen { () => {}; }\npub fn drive() {\n    gen!();\n    Foo::m();\n}\n",
            Rust,
        ),
        ("src/a.rs", "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n", Rust),
        ("src/b.rs", "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n", Rust),
    ];
    let (cg, _) = build(&sources);
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Foo::m"));
    assert_eq!(out.drop, None);
    assert!(
        out.resolved
            .iter()
            .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "a covering macro wildcard keeps the full pool at NameOnly"
    );
}

#[test]
fn scope_graph_pending_import_alias_over_colliding_pool_keeps_all() {
    use prism::languages::Language::Rust;
    // ②B directness, against a COLLIDING pool — the test that distinguishes the
    // correct call-site-rib directness oracle from the unsound global target
    // search (codex BLOCKER). The leading segment `Foo` binds at the call site
    // via a NAMED IMPORT `use crate::a::Foo;` — a `BindTarget::Pending` binding —
    // and there is NO block-local shadow/glob (so ①C does not fire; ②B is the
    // ONLY thing standing between the prune and a wrong drop). The bare
    // ("Foo","m") pool COLLIDES: both `a::Foo::m` and `b::Foo::m` are in it.
    //
    // Correct ②B (inspect the call site's own rib binding): `Foo`'s rib binding
    // is `Pending` → not a direct `Resolved(Item)` → keep-all → BOTH survive at
    // NameOnly (the demote floor). The BUGGY global search would chase the
    // `Pending` import to `a::Foo`, then find `a::Foo`'s DEFINITION binding
    // (`Resolved(Item)`) anywhere in the graph and call it "direct", resolve the
    // final callable to `a::Foo::m` alone, and DISPROVE `b::Foo::m` — dropping a
    // real edge. So this test FAILS (recall loss) iff the global-search oracle is
    // used, and PASSES only with the call-site-rib oracle.
    let sources = [
        (
            "src/lib.rs",
            "mod a;\nmod b;\nuse crate::a::Foo;\npub fn drive() {\n    Foo::m();\n}\n",
            Rust,
        ),
        ("src/a.rs", "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n", Rust),
        ("src/b.rs", "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n", Rust),
    ];
    let (cg, _) = build(&sources);
    // Pin the colliding pool so a future extraction change can't silently make
    // this test vacuous (a singleton pool can't be wrongly pruned).
    assert_eq!(
        cg.methods
            .get(&("Foo".to_string(), "m".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key must collide across a::Foo and b::Foo",
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Foo::m"));
    assert_eq!(out.drop, None, "a Pending import alias must not drop");
    assert!(
        out.resolved.len() == 2
            && out
                .resolved
                .iter()
                .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "②B keeps the full colliding pool at NameOnly (a global-search oracle \
         would wrongly prune to a::Foo::m and drop b::Foo::m)",
    );
}
```

> Implementation note (reviewer/executor): this replaces the earlier
> singleton-`Facade::m` test, which could NOT distinguish the two oracles — with
> one underlying `Real::m`, `graph_target_resolution`'s own `1→Exact` resolves the
> edge regardless of the predicate, so a global-search directness bug would go
> undetected. The collision above is the discriminator: the `m` edge to `b::Foo`
> survives ONLY when directness is proven from the call site's `Pending` rib
> binding (kept-all), not from a global `Resolved(target)` search (wrong prune).
> The forgone *precision* on import aliases (we keep-all rather than recover the
> single Exact) is the spec's deferred §9 follow-up; the recall-safety this test
> pins is the contract. The `pub use … as Facade` re-export form is precision-
> forgone for the same reason (its leading ident is also a `Pending` binding) and
> needs no separate test — the owner key for an aliased re-export is the real
> impl owner, not the alias, so it does not even reach the bare-pool prune.

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo test --test integration resolution_test::scope_graph_ 2>&1 | tail -40
```
Expected failures (RED): `scope_graph_inherent_plus_trait_owner_demotes_not_drops` FAILS today (drop == `UnknownName`, not demote) — but note Task 4 already rewrote `graph_target_resolution` to `>1→demoted`, so after Task 4 this test may already pass via the scope block. `scope_graph_unresolved_owner_path_keeps_full_pool_not_drop` FAILS today (scope branch returns `dropped(UnknownName)` on a `None` resolution — the fail-open is not yet narrowed). The two ①C keep-all tests (`scope_graph_block_local_glob_shadow_keeps_all`, `scope_graph_macro_wildcard_shadow_keeps_all`) and the ②B directness test (`scope_graph_pending_import_alias_over_colliding_pool_keeps_all`) FAIL today for a *recall* reason that the new ordering fixes: the un-narrowed scope branch (`rust_scope_graph_resolution` first) resolves the module-anchored `Foo::m` to a single `a::Foo::m` Exact and DROPS the real shadowed/colliding `b::Foo::m` edge. (This is also why Step-3's ordering correction matters: those three tests are the regression guard for the bypass.) The two-crate headline (`scope_graph_two_crate_owner_collision_recovers_to_single_exact`) already passes (graph present → `1→Exact`) and is kept as a regression pin.

- [ ] **Step 3: Rewrite the Rust scope branch of `resolve_call_site_full`**

In `src/resolution.rs`, replace the scope block in `resolve_call_site_full` (the `if let Some(graph) = self.scope_graph.as_ref() { ... }` block, lines ~690-702) with the prune + fail-open logic.

**Ordering correction (codex BLOCKER @ Task-5 Step-3 + MAJOR @ the keep-all fixtures).** The original draft ran `rust_scope_graph_resolution` FIRST for *every* site. But for an owner-keyed `T::m` whose leading ident the **module-anchored** callable edge resolves to a single target (a block-local glob/macro/`Pending`-import colliding pool — exactly the ①C/②B keep-all cases), that path mints a single Exact and the disproof prune (which carries the ①C shadow guard and ②B directness) **never runs** → it would drop the real shadowed/aliased edge. So an owner-method `::` site MUST be driven THROUGH the prune (the spec's owner-path authority — disproof gates the id-set resolution); `rust_scope_graph_resolution` is reserved for the shapes the prune does not own: **free-function `::` paths** (`crate::m::free_fn`, `self::f`, `super::f` — no bare `(owner, method)` pool) and **unqualified** calls (the shipped `graph_callable_edge` path). We distinguish owner-method from free-fn by whether a bare `(owner, method)` pool exists in `self.methods`.

```rust
        if let Some(graph) = self.scope_graph.as_ref() {
            if crate::languages::Language::from_path(&site.caller.file)
                == Some(crate::languages::Language::Rust)
                && (name.contains("::") || site.qualifier.is_none())
            {
                if let Some((file, from)) = rust_authoritative_scope(graph, site) {
                    // Split an owner-keyed `::` name into (owner, method) and ask
                    // whether a bare `(owner, method)` pool exists — that is what
                    // separates an owner-METHOD `::` site (which the disproof prune
                    // owns) from a free-FUNCTION `::` path / `self::`/`Self::`/`::x`
                    // (which it does not). `None` ⇒ not an owner-method site.
                    let owner_method = if name.contains("::") {
                        owner_method_key(name)
                    } else {
                        None
                    };
                    let has_bare_pool = owner_method
                        .as_ref()
                        .is_some_and(|(o, m)| self.methods.contains_key(&(o.clone(), m.clone())));

                    if has_bare_pool {
                        // Owner-method `T::m`: the prune is the authority. It gates
                        // the id-set resolution behind ①C (no block-local shadow)
                        // and ②B (leading segment binds DIRECTLY), so a single Exact
                        // is minted ONLY when both contracts hold. 1 survivor →
                        // Exact (recovery); >1 → pruned NameOnly demote.
                        if let Some(resolved) =
                            self.rust_scope_prune_owner(graph, site, file, from, name)
                        {
                            return ResolutionOutcome::hit(resolved);
                        }
                        // Fail-open: the predicate proved nothing (uncertain — a
                        // shadow/alias/ambiguous leading segment, or an unresolved
                        // owner). Route ONLY to #120's owner_lookup_in_modules demote
                        // floor and STOP — NEVER the legacy stem heuristic (that
                        // would re-introduce the same-stem guess the three shipped
                        // drop invariants forbid). A singleton bare pool here is a
                        // legitimate Exact; a collision demotes to NameOnly.
                        let (owner, method) = owner_method.as_ref().expect("has_bare_pool");
                        let segs: Vec<&str> = name.split("::").collect();
                        // module_segs = the path between crate/super-stripped head
                        // and the method, for the existing module-narrowing.
                        let mut prefix: Vec<&str> = segs[..segs.len() - 1].to_vec();
                        while matches!(prefix.first(), Some(&"crate") | Some(&"super")) {
                            prefix.remove(0);
                        }
                        let module_segs = &prefix[..prefix.len().saturating_sub(1)];
                        if let Some(resolved) =
                            self.owner_lookup_in_modules(owner, method, module_segs)
                        {
                            return ResolutionOutcome::hit(resolved);
                        }
                        // A bare pool existed but module-narrowing eliminated it (a
                        // wrong module hint must not drop a real edge — but here the
                        // pool is empty post-narrow; owner_lookup_in_modules already
                        // ignores an empty narrow, so this arm is unreachable for a
                        // present key — fall to the drop below defensively).
                        return ResolutionOutcome::dropped(DropReason::UnknownName);
                    }

                    // Free-function `::` path or unqualified call: the shipped
                    // graph resolution (1→Exact / >1→demoted via the Task-4
                    // graph_target_resolution rule). NOTE: this path does NOT mint
                    // an owner-method Exact (no bare pool), so the ①C/②B bypass
                    // above cannot recur here.
                    if let Some(resolved) = self.rust_scope_graph_resolution(graph, site, file, from)
                    {
                        return ResolutionOutcome::hit(resolved);
                    }

                    // Authoritative graph declined → drop (the shipped invariant: no
                    // legacy free-fn fan-out, no stem heuristic, for an authoritative
                    // miss). A free-fn `::` miss (`crate::missing::target`) and an
                    // unqualified bare miss both land here.
                    return ResolutionOutcome::dropped(DropReason::UnknownName);
                }
            }
        }
```

This introduces one small free helper, `owner_method_key`, placed next to `rust_call_path_anchor` (it mirrors the owner/method split `rust_scope_prune_owner` does, so the splice and the prune agree on the key):

```rust
/// Split an owner-keyed `mod::T::m` call name into the bare `(owner, method)`
/// key — owner = the segment immediately before the method, after stripping
/// leading `crate::`/`super::`. Returns `None` for `self`/`Self` heads (handled
/// by R7/SelfReceiver elsewhere) or a single-segment name (no owner).
fn owner_method_key(name: &str) -> Option<(String, String)> {
    let mut segs: Vec<&str> = name.split("::").collect();
    let method = segs.pop()?;
    while matches!(segs.first(), Some(&"crate") | Some(&"super")) {
        segs.remove(0);
    }
    let owner = *segs.last()?;
    if owner == "self" || owner == "Self" {
        return None;
    }
    Some((owner.to_string(), method.to_string()))
}
```

- [ ] **Step 4: Add the `rust_scope_prune_owner` method**

In `src/resolution.rs`, inside `impl CallGraph`, add the prune helper (place it next to `rust_scope_graph_resolution`):

```rust
    /// Owner-keyed disproof prune (spec §4). Fetch the bare `(owner, method)` pool
    /// from `self.methods`, run the `ScopeResolution` predicate, and decide:
    /// 1 survivor → Exact (recovery); >1 → demoted (pruned NameOnly); unchanged
    /// from the bare pool → `None` (fall through to the #120 demote floor). Only
    /// reduces below the bare pool when the predicate actually disproved someone.
    fn rust_scope_prune_owner(
        &self,
        graph: &ScopeGraph,
        site: &CallSite,
        file: FileId,
        from: ScopeId,
        name: &str,
    ) -> Option<Vec<ResolvedCallee<'_>>> {
        // Owner key `(T, m)` from `mod::T::m` (shared with the splice via
        // owner_method_key, so both agree on the bare key).
        let (owner, method) = owner_method_key(name)?;
        let pool_ids = self.methods.get(&(owner, method))?;
        let pool: Vec<&FunctionId> = pool_ids.iter().collect();
        let pred = ScopeResolution::new(self);
        let cx = crate::resolution_disproof::DisproofCx { graph, file, from };
        let pruned = crate::resolution_disproof::prune(
            pool.clone(),
            site,
            &cx,
            &[&pred as &dyn crate::resolution_disproof::DisproofPredicate],
        );
        // Unchanged from the bare pool → the predicate proved nothing → fall
        // through to the #120 demote floor (do NOT mint here).
        if pruned.len() == pool.len() {
            return None;
        }
        Some(match pruned.len() {
            1 => exact(pruned, ResolutionKind::QualifiedOwner),
            _ => demoted(pruned, ResolutionKind::QualifiedOwner),
        })
    }
```

> Why `pruned.len() == pool.len()` is the "unchanged" test: `prune` either returns a strict subset (the predicate disproved at least one), or the original pool (no disproof, or prune-to-empty). Comparing lengths is sufficient because `prune` never reorders-without-removing — a same-length result is the original pool object. A single-element bare pool that the predicate "keeps" also returns unchanged here (len 1 == len 1 → `None`), so it correctly falls through to `owner_lookup_in_modules`, which mints the singleton Exact. This avoids double-handling the trivial singleton.

- [ ] **Step 5: Run the integration tests to verify they pass**

```bash
cargo test --test integration resolution_test::scope_graph_ 2>&1 | tail -40
```
Expected: all six new `scope_graph_*` tests PASS.

- [ ] **Step 6: Re-confirm the three shipped drop invariants + the broader resolution suite**

```bash
cargo test --test integration resolution_test::rust_scope_graph_unqualified_declines_do_not_legacy_guess 2>&1 | tail -10
cargo test --test integration resolution_test::rust_scope_graph_qualified_paths_resolve_or_disable_legacy_stem 2>&1 | tail -10
cargo test --test integration resolution_test::rust_scope_graph_authority_gate_and_poison_skip_legacy 2>&1 | tail -10
cargo test --test integration resolution_test:: 2>&1 | tail -15
```
Expected: all three drop-invariant tests PASS. The negative `crate::missing::target` case still drops via the FREE-FN path: its bare `("missing","target")` key does not exist (`missing` is a module, `target` a free fn, not a method) → `has_bare_pool == false` → it routes to `rust_scope_graph_resolution`, where the path fails to resolve (no `mod missing;` in lib.rs) → `None` → the final `dropped(UnknownName)`. It never reaches `owner_lookup_in_modules` and never reaches the legacy stem block — so the same-stem guess the invariant forbids cannot fire. The unqualified-bare-miss (`process`) still drops (no bare pool, graph declines). The poison case still drops. The whole `resolution_test::` suite PASSES.

- [ ] **Step 7: Run the full test suite + fmt**

```bash
cargo fmt
cargo test 2>&1 | tail -25
```
Expected: PASS (no regressions across unit + integration).

- [ ] **Step 8: Commit**

```bash
git add src/resolution.rs tests/integration/resolution_test.rs
git commit -m "$(cat <<'EOF'
feat(scope-graph): integrate disproof prune into the Rust scope path (Component 3)

In resolve_call_site_full's Rust scope branch, an owner-method `T::m` site (one
with a bare `(owner, method)` pool) is driven THROUGH the ScopeResolution prune
— which gates the id-set resolution behind ①C (no block-local shadow) and ②B
(direct call-site binding) — so a single Exact is minted only when both contracts
hold: 1 survivor → Exact recovery, >1 → pruned NameOnly demote. When the prune
proves nothing (shadow/alias/ambiguous/unresolved owner) the site fail-opens ONLY
to #120's owner_lookup_in_modules demote floor (NameOnly on a collision, Exact on
a singleton), never the legacy stem heuristic. Free-function `::` paths and
unqualified calls (no bare pool) take the shipped graph resolution (1→Exact /
>1→demoted) and drop on a miss. This ordering (prune before any owner-method
Exact) is required so a module-anchored callable edge cannot mint a single Exact
that bypasses the ①C/②B guards and drops a real shadowed/colliding edge. The
three shipped drop invariants are re-confirmed.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: CACHE_VERSION bump (spec §6)

`ScopeGraph` is serialized in the CPG cache, and the new `edition_uniform` field + the resolution-behavior change alter both cached bytes and behavior for repos that previously had no graph. Bump `CACHE_VERSION` 15 → 16, document it, and update the pin test.

**Files:**
- Modify: `src/cpg_cache.rs` (the `CACHE_VERSION` const + history comment, line ~57; the pin test at line ~564).

- [ ] **Step 1: Update the failing pin test**

In `src/cpg_cache.rs`, the test at line ~564 asserts `super::CACHE_VERSION == 15`. Change it to 16:

```rust
        assert_eq!(super::CACHE_VERSION, 16);
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test --lib cpg_cache 2>&1 | tail -15
```
Expected: the version pin test FAILS (`CACHE_VERSION` is still 15).

- [ ] **Step 3: Bump the constant + history comment**

In `src/cpg_cache.rs`, add the v16 history line and bump the constant:

```rust
/// - v15: Phase-2a PR-3 CallGraph.extension_methods external receiver index.
/// - v16: Scope-graph precision recovery: ScopeGraph.edition_uniform field +
///   Rust-scoped completeness builds the graph on more repos (changed bytes) +
///   owner-keyed disproof prune (changed resolution behavior).
const CACHE_VERSION: u32 = 16; // bincode ignores serde(default) for new trailing fields.
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cargo test --lib cpg_cache 2>&1 | tail -15
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/cpg_cache.rs
git commit -m "$(cat <<'EOF'
chore(scope-graph): bump CACHE_VERSION 15 → 16

ScopeGraph gains edition_uniform (changed serialized bytes), Rust-scoped
completeness builds the graph on repos that previously had None, and owner-keyed
resolution behavior changes — all require a one-time cache invalidation.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 7: Recovery counter (spec §7 / review MAJOR-5)

The legacy `shadow_typepath_narrow` counter only runs inside the `exact_kinds.len() >= 2` guard (multi-target-Exact sites). #120 moved owner collisions to NameOnly (0 Exact edges → guard never entered), and this change's recovery makes them *single*-Exact (1 edge, still < 2) — so that shadow stays empty either way. Add a **new recovery counter** keyed off the owner-method `::` population: classify each such site by the disproof prune's ACTUAL decision, re-derived measurement-only (`singleton` recovered-Exact / `pruned_multiple` real-prune-still-demoted / `failopen_singleton` clean-singleton-floor / `failopen_demote` un-recovered-collision-floor / `not_owner_method` skipped). Classifying from final edge counts is insufficient — a fail-open demote and a prune demote both read as `>1 NameOnly` (codex MAJOR), so the classifier re-runs the prune and reads the predicate's decision. Wire it into `call_stats`.

**Files:**
- Modify: `src/navigation/queries.rs` — add `recovery_typepath` classification + a counter in `call_stats`.
- Test: `tests/cli/call_stats_test.rs` — a recovered fixture moves the counter.

- [ ] **Step 1: Write the failing CLI test**

Add to `tests/cli/call_stats_test.rs`:

```rust
#[test]
fn call_stats_recovery_counter_moves_on_recovered_owner_site() {
    // A repo whose scope graph DOES build (all Rust → complete) and where a
    // `T::m` owner site recovers to a single Exact. The new recovery counter
    // records that site under `singleton`. The same fixture without a graph
    // would sit at NameOnly (the #120 floor); with the graph the recovery
    // instrument shows the win.
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("a/src")).unwrap();
    std::fs::create_dir_all(dir.path().join("b/src")).unwrap();
    std::fs::write(
        dir.path().join("a/src/lib.rs"),
        "pub struct CliTest;\nimpl CliTest {\n    pub fn with_file(&self) {}\n}\npub fn drive() {\n    CliTest::with_file();\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.path().join("b/src/lib.rs"),
        "pub struct CliTest;\nimpl CliTest {\n    pub fn with_file(&self) {}\n}\n",
    )
    .unwrap();
    let out = Command::cargo_bin("prism")
        .unwrap()
        .args(["nav", "--no-cache", "call-stats", "--repo"])
        .arg(dir.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    // The recovered owner site lands a single Exact qualified_owner edge AND is
    // counted under the new recovery instrument's `singleton` bucket.
    assert_eq!(v["kind_exact"]["qualified_owner"], 1);
    assert_eq!(
        v["recovery_typepath"]["singleton"], 1,
        "the recovered owner site is recorded as a singleton recovery"
    );
}
```

- [ ] **Step 2: Run the test to verify it fails**

Compile the CLI test binary without running (macOS note), then run the filter:

```bash
cargo test --test cli --no-run 2>&1 | tail -3
CLI_BIN=$(ls -t target/debug/deps/cli-* | grep -vE '\.(d|dSYM)$' | head -1)
"$CLI_BIN" call_stats_test::call_stats_recovery_counter_moves_on_recovered_owner_site --nocapture 2>&1 | tail -15
```
Expected: FAIL — `v["recovery_typepath"]` is null (the counter does not exist yet). (`kind_exact["qualified_owner"] == 1` already holds, since Task 5 recovers the site.)

- [ ] **Step 3: Add the recovery classifier**

In `src/navigation/queries.rs`, add a classifier mirroring `shadow_narrow_type_path`'s **measurement-only re-derivation** pattern (it re-runs the resolution logic independently — it does NOT read final edge counts). Classifying from `out.resolved`'s Exact/NameOnly counts alone is WRONG (codex MAJOR): a `>1 NameOnly` result is produced BOTH by the disproof prune (predicate disproved someone but >1 survived) AND by the #120 fail-open demote (no prune at all — the floor demotes a colliding pool) — the two are indistinguishable from counts, so a count-based classifier mislabels every fail-open demote as `pruned_multiple` and erases the recovery signal. Instead, re-run the actual prune and classify from the predicate's DECISION (did it disprove anyone; how many survived vs the bare pool). Add it next to `shadow_narrow_type_path`:

```rust
/// Recovery instrument (spec §7 / review MAJOR 5). For an owner-method `T::m`
/// site, report what the disproof prune ACTUALLY decided (re-derived measurement-
/// only, exactly as `shadow_narrow_type_path` re-derives the narrowing — never
/// read from final edge counts, which cannot tell a prune-demote from a fail-open
/// demote):
///   `singleton`        — the prune disproved down to a single survivor (the
///                        recovered Exact: ①C+②B held and the id-set pinned one).
///   `pruned_multiple`  — the prune disproved ≥1 but >1 survived (a real prune
///                        that still demotes to NameOnly).
///   `failopen_singleton` — the predicate proved nothing; the bare pool is a
///                        singleton (the #120 floor mints Exact — not a recovery).
///   `failopen_demote`  — the predicate proved nothing; the bare pool collides
///                        (the #120 floor demotes to NameOnly — the un-recovered
///                        residue this slice aims to shrink).
///   `not_owner_method` — no bare `(owner, method)` pool / unresolvable scope.
/// Keyed off the owner-`::` population, not the >=2-Exact population the legacy
/// `shadow_typepath_narrow` requires.
fn classify_recovery_typepath(cg: &CallGraph, site: &CallSite) -> &'static str {
    use crate::name_resolution::rust_populator::enclosing_scope;
    use crate::resolution::{prune, DisproofCx, DisproofPredicate, ScopeResolution};
    // Owner-method key `(T, m)` from `mod::T::m` (mirror the resolver's split;
    // `crate`/`super` stripped, `self`/`Self` heads excluded).
    let mut segs: Vec<&str> = site.callee_name.split("::").collect();
    let Some(method) = segs.pop() else {
        return "not_owner_method";
    };
    while matches!(segs.first(), Some(&"crate") | Some(&"super")) {
        segs.remove(0);
    }
    let Some(&owner) = segs.last() else {
        return "not_owner_method";
    };
    if owner == "self" || owner == "Self" {
        return "not_owner_method";
    }
    let Some(pool_ids) = cg.methods.get(&(owner.to_string(), method.to_string())) else {
        return "not_owner_method";
    };
    // Re-derive the authoritative (file, enclosing-scope) the prune ran from. If
    // the graph is absent/incomplete or the byte has no scope, this site never
    // reached the prune → not a recovery outcome.
    let Some(graph) = cg.scope_graph.as_ref() else {
        return "not_owner_method";
    };
    if !graph.complete {
        return "not_owner_method";
    }
    let Some(file) = graph.file_paths.get(&site.caller.file).copied() else {
        return "not_owner_method";
    };
    let Some(from) = enclosing_scope(graph, file, site.start_byte) else {
        return "not_owner_method";
    };
    let pool: Vec<&FunctionId> = pool_ids.iter().collect();
    let pred = ScopeResolution::new(cg);
    let cx = DisproofCx { graph, file, from };
    let kept = prune(pool.clone(), site, &cx, &[&pred as &dyn DisproofPredicate]);
    if kept.len() < pool.len() {
        // The predicate disproved at least one candidate (a real prune).
        if kept.len() == 1 {
            "singleton"
        } else {
            "pruned_multiple"
        }
    } else if pool.len() == 1 {
        "failopen_singleton"
    } else {
        "failopen_demote"
    }
}
```

- [ ] **Step 4: Wire the counter into `call_stats`**

In `src/navigation/queries.rs`, in `call_stats`, add the counter map near the other maps (after `let mut shadow_typepath_narrow ...`):

```rust
    let mut shadow_typepath_narrow: BTreeMap<&'static str, usize> = BTreeMap::new();
    // Forward recovery instrument (spec §7): classify each owner-`::` `T::m` site
    // by what the scope path now yields, keyed off the qualified_owner population
    // (the demoted-NameOnly + recovered-Exact sites #120/this slice produce),
    // independent of the legacy >=2-Exact shadow guard.
    let mut recovery_typepath: BTreeMap<&'static str, usize> = BTreeMap::new();
```

Inside the `for site in sites` loop, after the `out` is computed (the `let out = cg.resolve_call_site_full(site);` line) and within the same iteration, add the owner-`::`-gated classification. The classifier is named `classify_recovery_typepath` (distinct from the `recovery_typepath` map — no value-namespace shadow, codex MAJOR), and it only counts a site that resolves to a real owner-method outcome (it returns `not_owner_method` for free-fn `::` / `self::` / unresolvable, which we skip). Place it right after the existing `match out.drop { ... }` block:

```rust
            if site.callee_name.contains("::") {
                let bucket = classify_recovery_typepath(cg, site);
                if bucket != "not_owner_method" {
                    *recovery_typepath.entry(bucket).or_default() += 1;
                }
            }
```

Add it to the emitted JSON (in the `serde_json::json!({ ... })` block, after `"shadow_typepath_narrow": shadow_typepath_narrow,`):

```rust
        "shadow_typepath_narrow": shadow_typepath_narrow,
        "recovery_typepath": recovery_typepath,
```

> The classifier fn (`classify_recovery_typepath`) and the counter map (`recovery_typepath`) now have distinct names, so the call site is unambiguous. (The earlier draft named both `recovery_typepath`; that does NOT compile — a `let recovery_typepath = ...` binding shadows a same-name free fn in the value namespace, so `recovery_typepath(&out)` would try to call the `BTreeMap`. The rename, not "place vs value expression", is what fixes it.)

- [ ] **Step 5: Run the test to verify it passes**

```bash
cargo test --test cli --no-run 2>&1 | tail -3
CLI_BIN=$(ls -t target/debug/deps/cli-* | grep -vE '\.(d|dSYM)$' | head -1)
"$CLI_BIN" call_stats_test::call_stats_recovery_counter_moves_on_recovered_owner_site --nocapture 2>&1 | tail -15
```
Expected: PASS.

- [ ] **Step 6: Re-run the existing call_stats tests (no regression)**

```bash
"$CLI_BIN" call_stats_test:: --nocapture 2>&1 | tail -20
```
Expected: all existing `call_stats_test::` tests PASS (the new counter is additive; the demoted-collision tests still see `multi_target_exact_sites == 0` and `kind_nameonly[qualified_owner] == 2`).

- [ ] **Step 7: fmt + commit**

```bash
cargo fmt
git add src/navigation/queries.rs tests/cli/call_stats_test.rs
git commit -m "$(cat <<'EOF'
feat(scope-graph): recovery counter over the owner-:: population (review MAJOR 5)

The legacy shadow_typepath_narrow only fires on >=2-Exact sites; #120 demotes
collisions to NameOnly and this slice recovers them to single-Exact, so that
shadow stays empty either way. Add recovery_typepath: classify each owner-method
`T::m` site by the disproof prune's ACTUAL decision (re-derived measurement-only,
like shadow_narrow_type_path — NOT from final edge counts, which cannot tell a
prune demote from a fail-open demote): singleton / pruned_multiple /
failopen_singleton / failopen_demote / not_owner_method. The classifier fn
(classify_recovery_typepath) is named distinctly from the counter map so it does
not shadow it in the value namespace. The acceptance reads `singleton` rising
from this counter plus the kind_exact/kind_nameonly deltas.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 8: Tier-A acceptance gate (spec §7, non-code)

Run the accuracy gate and the recovery-signal measurement. This task produces **evidence to paste into the PR**, not code. STOP and tighten the predicate if any recall regression appears.

**Files:** none committed. Do NOT stage `eval/` or `docs/eval/` artifacts.

- [ ] **Step 1: Release build**

```bash
cargo build --release 2>&1 | tail -5
```
Expected: builds clean.

- [ ] **Step 2: Tier-A matrix (fast, no LSP)**

```bash
cd eval && uv run tier-a --matrix-only --allow-stale-sut 2>&1 | tail -40
```
Expected: matrix **0 regressions** vs the committed baseline in `docs/eval/tier-a/`. Any new `gap`/flip → record it in the PR description (do not re-baseline).

- [ ] **Step 3: Clear the nav cache (the §6 bump invalidates it; clear to be safe), then Tier-A quick**

```bash
rm -rf "$(python3 -c 'import os,sys;
try:
    import platformdirs as d; print(d.user_cache_dir("prism"))
except Exception:
    print(os.path.expanduser("~/Library/Caches/prism"))')/nav" 2>/dev/null || true
cd eval && uv run tier-a --quick --allow-stale-sut 2>&1 | tail -50
```
Expected (recall-safety gate): `--quick` M2 **fn = 0** (no dropped edges) on every anchor; fp not increased. If recall drops on any anchor → **STOP** (a prune dropped a real edge; tighten `ScopeResolution`'s soundness).

- [ ] **Step 4: Recovery signal — call-stats before/after on ruff/prism/ripgrep**

Capture the branch's call-stats for the three Rust anchors and diff against the #120 baseline (the parent commit of this branch's first commit). For each anchor, report the `kind_nameonly[qualified_owner]` drop, the `kind_exact[qualified_owner]` rise, and the new `recovery_typepath` histogram:

```bash
# Resolve the anchor repo paths from the eval config (adjust to your local checkout).
for repo in /path/to/ruff /path/to/prism /path/to/ripgrep; do
  echo "=== $repo (branch) ==="
  ./target/release/prism nav --no-cache call-stats --repo "$repo" \
    | python3 -c 'import sys,json; v=json.load(sys.stdin); print("kind_exact.qualified_owner:", v["kind_exact"].get("qualified_owner")); print("kind_nameonly.qualified_owner:", v["kind_nameonly"].get("qualified_owner")); print("recovery_typepath:", v.get("recovery_typepath"))'
done
```

Then `git stash` / check out the #120 baseline commit, rebuild, and re-run the same loop to get the "before" numbers. Report the per-anchor deltas in the PR:
- ruff `kind_nameonly[qualified_owner]` should **drop materially** with a matching **rise** in `kind_exact[qualified_owner]` (the headline recovery — ruff gains the scope graph via Component 1).
- `recovery_typepath.singleton` should be **non-zero** on ruff (recovered owner sites).

- [ ] **Step 5: Paste the evidence + request review**

Paste into the PR description: the `--matrix-only` summary (0 regr), the `--quick` M2 fn/fp per anchor (fn=0), and the per-anchor call-stats before/after deltas + `recovery_typepath` histograms. Then request the independent codex (gpt-5.5, xhigh) **diff review** per the established loop. Address findings before merge.

- [ ] **Step 6: No commit (evidence-only task)**

This task commits nothing. Confirm `git status` shows no staged `eval/` or `docs/eval/` artifacts before opening the PR:

```bash
git status --short | grep -E 'eval/' && echo "WARNING: unstage eval artifacts" || echo "clean of eval artifacts"
```

---

## Final self-review checklist (run before declaring the plan complete)

- Component 1 (§2): Task 1 (`has_complete_rust_coverage`) + Task 2 (edition-uniformity guard). ✓
- Component 2 (§3): Task 3 (seam) + Task 4 (`ScopeResolution` predicate, ①C + ②B + §2 guard). ✓
- Component 3 (§4): Task 5 (prune → Exact / pruned-demote, narrowed fail-open, three drop invariants). ✓
- Cache (§6): Task 6 (CACHE_VERSION 15 → 16 + pin test). ✓
- Recovery signal (§7 / MAJOR-5): Task 7 (`recovery_typepath` counter). ✓
- Acceptance (§7): Task 8 (Tier-A matrix + quick, call-stats deltas, codex diff review). ✓
- The seven §7 unit/integration cases map to: C1 coverage → Task 1; seam resolves-to-1 → Task 5 `scope_graph_two_crate_owner_collision_recovers_to_single_exact`; pruned-to-2 → Task 5 `scope_graph_inherent_plus_trait_owner_demotes_not_drops`; no-resolution keep-all → Task 5 `scope_graph_unresolved_owner_path_keeps_full_pool_not_drop`; ②B directness (Pending import alias over a colliding pool) → Task 5 `scope_graph_pending_import_alias_over_colliding_pool_keeps_all`; block-local glob → Task 5 `scope_graph_block_local_glob_shadow_keeps_all`; macro-wildcard → Task 5 `scope_graph_macro_wildcard_shadow_keeps_all`; fail-open → Task 5 `scope_graph_unresolved_owner_path_keeps_full_pool_not_drop`. ✓
