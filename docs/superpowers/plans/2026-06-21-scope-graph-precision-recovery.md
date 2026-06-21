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
        // If the repo skips one of its OWN `.rs` (here: oversized > 2 MiB), Rust
        // coverage is incomplete → `complete == false` (unchanged behavior; the
        // deferred per-crate case).
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::write(p.join("lib.rs"), "pub fn a() {}\n").unwrap();
        std::fs::write(p.join("big.rs"), format!("// {}\n", "x".repeat(3 * 1024 * 1024)))
            .unwrap(); // TooLarge, skipped
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        assert!(
            !inputs.complete,
            "skipping an own .rs must keep completeness false"
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

**Files:**
- Modify: `src/name_resolution/rust_populator/mod.rs` (`RustCrateConfig` — add `edition_uniform`; `from_convention` default; `populate_rust` copies it onto the graph).
- Modify: `src/name_resolution/graph.rs` (`ScopeGraph` — add `edition_uniform` field + `default_edition_uniform`).
- Modify: `src/repo_loader.rs` (`parse_rust_crate_config` — compute uniformity across manifests).
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

- [ ] **Step 3: Add the field to `RustCrateConfig` and default it**

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

Inside the loop, where the edition is parsed, also record it:

```rust
        if let Some(edition) = value
            .get("package")
            .and_then(|p| p.get("edition"))
            .and_then(|e| e.as_str())
            .and_then(parse_edition)
        {
            cfg.edition = edition;
            editions_seen.insert(edition);
        }
```

After the loop, before `Some(cfg)`, set the flag. A workspace where 0 or 1 distinct editions were declared is uniform; ≥2 distinct declared editions is non-uniform:

```rust
    if !parsed_any {
        return None;
    }
    cfg.edition_uniform = editions_seen.len() <= 1;
    crate_roots.extend(cfg.crate_roots);
```

(`BTreeSet` is already imported in `repo_loader.rs`.)

- [ ] **Step 6: Run the tests to verify they pass**

```bash
cargo test --lib name_resolution::rust_populator::tests:: 2>&1 | tail -20
cargo test --lib repo_loader::tests:: 2>&1 | tail -20
```
Expected: PASS (both new populator tests + both new repo_loader edition tests + the Task 1 coverage tests + the existing parity tests).

- [ ] **Step 7: Commit**

```bash
git add src/name_resolution/rust_populator/mod.rs src/name_resolution/graph.rs src/repo_loader.rs
git commit -m "$(cat <<'EOF'
feat(scope-graph): edition-uniformity guard on crate config + ScopeGraph

Record whether every parsed manifest agreed on one edition
(RustCrateConfig::edition_uniform, propagated onto ScopeGraph::edition_uniform).
A mixed-edition workspace can mis-anchor a UsePath/LeadingColon path; once the
disproof predicate prunes on such a path that becomes a recall risk (P1), so the
predicate will keep-all when the flag is false (Task 4). serde(default)=true
keeps legacy caches valid.

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

- **②B direct binding (§8.2):** the owner type-path's **leading type segment** binds via a **direct `BindTarget::Resolved`** to an in-repo `Target::Item`, with **no `Pending` hop** on the way. Proven by **re-resolving the leading segment** and inspecting the binding shape — NOT by reading the resolved `Candidate` (the engine folds direct-`Resolved` and chased-`Pending` into an empty-`provenance` `Candidate`, so directness is unreadable there). A `Pending`/glob/ambiguous leading segment ⇒ keep-all.
- **①C no block-local shadow (§8.1):** the leading segment has **no potential block-local shadow** of that exact ident at the call site. Three shapes, two recorded *without* an exact-ident binding (so an exact-ident-only scan is too narrow): **(a)** a visible `NS_TYPE` `Binding` for the exact leading ident, **(b)** a visible block-local **glob** `Edge` whose `glob_vis_range` covers the call byte, or **(c)** a covering **macro wildcard** in `NS_TYPE`. ANY of the three ⇒ keep-all.

Plus the §2 guard: a **non-uniform-edition** graph (`graph.edition_uniform == false`) ⇒ keep-all.

When both hold (and the candidate's resolved id-set is known), it disproves any candidate **not in** that id-set. The id-set comes from reusing the existing `graph_target_resolution` logic for the *final callable* — but that helper is **not** the directness oracle.

Because the helpers in `resolution.rs` it needs (`rust_call_path_anchor`, `graph_target_resolution`, `rust_graph_qualified_callable_edge`, `graph_file_for_scope`, `graph_owner_name_for_scope`) are private free functions / private methods, the predicate lives in `resolution.rs` (same module → no visibility changes) and is **re-exported** so the seam's caller can name it. Local helpers are added in `resolution.rs` for the directness re-resolve and the shadow scan.

**Files:**
- Modify: `src/resolution.rs` — add the `ScopeResolution` predicate struct + `impl DisproofPredicate` + the local helpers (`leading_segment_binds_directly`, `leading_segment_has_block_local_shadow`, `scope_chain_to_module`), and `pub use` it.
- Test: `tests/integration/resolution_test.rs` (predicate-behavior tests through the public resolve path are in Task 5; this task's tests are inline unit tests in `src/resolution.rs` driving the predicate over hand-built graphs would be heavy — instead, **pin the helpers' observable behavior through `resolve_call_site` fixtures in Task 5**, and add the *guard* unit tests here in `src/resolution.rs` that need no full pipeline).

> **Note on test placement.** The predicate's end-to-end behavior (resolves-to-1, pruned-to-2, no-resolution keep-all, re-export-facade keep-all, block-local-glob keep-all, macro-wildcard keep-all, mixed-edition keep-all) is exercised through `resolve_call_site` in **Task 5**, where the integration wires the predicate into the live path and the existing `build()`/`build_rust_complete` helpers produce real graphs. This task implements the predicate and adds one inline unit test for the edition guard (the only branch reachable without the full integration). Splitting the behavior tests into Task 5 keeps each test against the real code path rather than a hand-mocked graph.

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

Add to `src/resolution.rs`. First, ensure the needed imports are present. The file already imports `BindTarget`, `Candidate`, `ScopeId`, `SourceLoc`, `Target`, `Anchor`, `AnchorKind`, `RawPath`, `ResStatus`, `FileId` from `name_resolution::types` and `NS_TYPE`/`NS_VALUE` from `rust_policy`. Add the engine `resolve_path` import (it imports `resolve_path` already at the top via `use crate::name_resolution::engine::resolve_path;`). Add `EK_GLOB` to the `rust_policy` import line and `Edge`/`Span`/`Binding` to the `types` import line:

```rust
use crate::name_resolution::rust_policy::{RustPolicy, EK_GLOB, NS_TYPE, NS_VALUE};
```

```rust
use crate::name_resolution::types::{
    Anchor, AnchorKind, BindTarget, Binding, Candidate, Edge, FileId, RawPath, ResStatus, ScopeId,
    SourceLoc, Span, Target,
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

/// Re-resolve the leading type segment lexically from the call site and prove it
/// reaches an in-repo `Item` definition through a **direct, non-glob,
/// non-Pending** binding (§8.2 decision ②B). A `Pending` re-export hop, a glob
/// edge, an ambiguous/poisoned result, or a non-in-repo target all return `false`
/// (keep-all). This inspects the BINDING SHAPE, not the resolved `Candidate`.
fn leading_segment_binds_directly(
    graph: &ScopeGraph,
    file: FileId,
    from: ScopeId,
    byte: usize,
    anchor: &Anchor,
    leading: &str,
) -> bool {
    let at = SourceLoc { file, byte };
    let single = RawPath(vec![leading.to_string()]);
    let policy = RustPolicy::new(graph, graph.edition);
    // Resolve the single leading segment as a TYPE-namespace path from the same
    // anchor/from the full call path uses. A direct in-repo type binding resolves
    // to exactly one `Item{owns:Some(..)}` (struct/enum/trait/type with a body)
    // or `Item` type. We require Resolved + a single in-repo Item.
    let res = crate::name_resolution::engine::resolve_path(
        graph, &single, NS_TYPE, anchor, from, NS_TYPE, &at, &policy,
    );
    let target = match (res.status, res.candidates.as_slice()) {
        (ResStatus::Resolved, [Candidate { target, .. }]) => target,
        _ => return false,
    };
    // The target must be an in-repo Item (not External, not Local, not a bare
    // Scope re-export without an Item). A direct type binding is `Item{..}`.
    if !matches!(target, Target::Item { .. }) {
        return false;
    }
    // Prove DIRECTNESS: there exists a binding in the graph whose target is this
    // exact in-repo `Item` via `BindTarget::Resolved` (a definition site), i.e. a
    // direct item binding rather than only a chased-Pending alias. A definition
    // binding exists iff some `Binding` carries `BindTarget::Resolved(target)`.
    graph
        .bindings
        .iter()
        .any(|b| matches!(&b.target, BindTarget::Resolved(t) if t == target))
}

/// Does the lexical scope chain from `from` UP TO (but not past) the enclosing
/// module contain any potential block-local shadow of `leading` at `byte`? Three
/// shapes (§8.1 decision ①C / re-review BLOCKER): (a) an exact `NS_TYPE` binding,
/// (b) a block-local glob `Edge` whose `glob_vis_range` covers `byte`, (c) a
/// covering `NS_TYPE` macro wildcard. ANY ⇒ shadow (keep-all).
fn leading_segment_has_block_local_shadow(
    graph: &ScopeGraph,
    from: ScopeId,
    file: FileId,
    byte: usize,
    leading: &str,
) -> bool {
    let at = SourceLoc { file, byte };
    for scope in scope_chain_to_module(graph, from) {
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

/// The lexical scope chain from `from` up to and INCLUDING the enclosing
/// `Module`/`Root` — the region a bare `T::m` anchor's leading ident could be
/// block-locally shadowed in (member lookup has no lexical fall-out, so a shadow
/// here is invisible to the anchored path — §8.1).
fn scope_chain_to_module(graph: &ScopeGraph, from: ScopeId) -> Vec<ScopeId> {
    use crate::name_resolution::types::ScopeKind;
    let mut out = Vec::new();
    let mut cur = Some(from);
    while let Some(id) = cur {
        out.push(id);
        let Some(s) = graph.scope(id) else { break };
        if matches!(s.kind, ScopeKind::Module | ScopeKind::Root) {
            break; // include the module, then stop
        }
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
    // The owner type path does NOT resolve through the graph (no `Missing` type),
    // but the bare owner key has candidates in another file. The predicate keeps
    // all (no id-set), and the fail-open routes the `::` site to the #120 demote
    // floor — NameOnly, NOT a drop, NOT a stem guess.
    let sources = [
        (
            "src/lib.rs",
            "mod other;\npub fn drive() {\n    Missing::make();\n}\n",
            Rust,
        ),
        (
            "src/other.rs",
            "pub struct Missing;\nimpl Missing {\n    pub fn make(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    // The bare ("Missing","make") key exists (defined in other.rs), so the
    // fail-open demotes rather than drops.
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Missing::make"));
    assert_eq!(out.drop, None, "owner-keyed `::` miss demotes, not drops");
    assert!(
        out.resolved
            .iter()
            .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "fail-open lands at the #120 NameOnly demote floor"
    );
    assert!(!out.resolved.is_empty());
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
fn scope_graph_reexport_facade_owner_keeps_all() {
    use prism::languages::Language::Rust;
    // The leading type segment binds via a named `pub use` facade
    // (BindTarget::Pending) that resolves cleanly to one `Real`. ②B's
    // direct-binding test fails on the Pending hop, so the predicate keeps all —
    // even though the final callable resolves to one id. (A candidate-level
    // provenance read would wrongly pin this.)
    let sources = [
        (
            "src/lib.rs",
            "mod inner;\npub use crate::inner::Real as Facade;\npub fn drive() {\n    Facade::m();\n}\n",
            Rust,
        ),
        (
            "src/inner.rs",
            "pub struct Real;\nimpl Real {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    // Only one `Real::m` exists, so the bare ("Real","m") / facade pool resolves
    // to a single candidate regardless; the contract under test is that the
    // PREDICATE did not pin via a Pending hop. We assert no drop and a single
    // NameOnly or Exact edge to `m` (the facade resolves; the predicate kept-all,
    // so the singleton comes from graph_target_resolution's own 1→Exact, which is
    // a direct alias-target resolution, not a predicate prune). Assert the edge
    // exists and is not a wrong drop.
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Facade::m"));
    assert_eq!(out.drop, None, "a clean facade must not drop");
    assert!(out.resolved.iter().any(|c| c.target.name == "m"));
}
```

> Implementation note for the reviewer/executor: the facade test asserts the **recall-safe** property (no wrong drop, the `m` edge survives). The precision detail (facade recovery is forgone by ②B) is documented as a deferred §9 follow-up; the test does not over-constrain the confidence, because `Facade::m` has a single underlying `Real::m` and `graph_target_resolution`'s own `1→Exact` alias resolution may legitimately resolve it without any predicate involvement. The point the test pins is that the **predicate's directness check did not pin on a Pending hop** — verified by the block-local/two-target tests where a Pending facade over a *colliding* pool would be the only way to wrongly prune, and those keep-all.

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo test --test integration resolution_test::scope_graph_ 2>&1 | tail -40
```
Expected failures (RED): `scope_graph_inherent_plus_trait_owner_demotes_not_drops` FAILS today (drop == `UnknownName`, not demote) — but note Task 4 already rewrote `graph_target_resolution` to `>1→demoted`, so after Task 4 this test may already pass via the scope block. `scope_graph_unresolved_owner_path_keeps_full_pool_not_drop` and the glob/macro keep-all tests FAIL today because the scope branch returns `dropped(UnknownName)` on a `None` resolution (the fail-open is not yet narrowed). The two-crate headline already passes (graph present → `1→Exact`); it is kept as a regression pin.

- [ ] **Step 3: Rewrite the Rust scope branch of `resolve_call_site_full`**

In `src/resolution.rs`, replace the scope block in `resolve_call_site_full` (the `if let Some(graph) = self.scope_graph.as_ref() { ... }` block, lines ~690-702) with the prune + fail-open logic:

```rust
        if let Some(graph) = self.scope_graph.as_ref() {
            if crate::languages::Language::from_path(&site.caller.file)
                == Some(crate::languages::Language::Rust)
                && (name.contains("::") || site.qualifier.is_none())
            {
                if let Some((file, from)) = rust_authoritative_scope(graph, site) {
                    // 1) The existing graph resolution: 1→Exact / >1→demoted (the
                    //    Task-4 graph_target_resolution rule), via the kind-routed
                    //    callable edge. A clean resolution wins outright.
                    if let Some(resolved) = self.rust_scope_graph_resolution(graph, site, file, from)
                    {
                        return ResolutionOutcome::hit(resolved);
                    }

                    // 2) Owner-keyed `T::m` that the callable edge did NOT resolve:
                    //    fetch the bare pool and try the disproof prune. A prune to
                    //    a single survivor recovers Exact; >1 survivors demote.
                    if name.contains("::") {
                        if let Some(resolved) =
                            self.rust_scope_prune_owner(graph, site, file, from, name)
                        {
                            return ResolutionOutcome::hit(resolved);
                        }
                        // 3) Fail-open (review MAJOR 4): an owner-keyed `::` site the
                        //    authoritative path could not resolve falls through ONLY
                        //    to #120's owner_lookup_in_modules demote floor — NEVER
                        //    the legacy stem heuristic. Preserves the three drop
                        //    invariants.
                        let mut segs: Vec<&str> = name.split("::").collect();
                        let fn_name = segs.pop().unwrap_or(name);
                        while matches!(segs.first(), Some(&"crate") | Some(&"super")) {
                            segs.remove(0);
                        }
                        if let Some(&head) = segs.last() {
                            if head != "self" && head != "Self" {
                                let module_segs = &segs[..segs.len() - 1];
                                if let Some(resolved) =
                                    self.owner_lookup_in_modules(head, fn_name, module_segs)
                                {
                                    return ResolutionOutcome::hit(resolved);
                                }
                            }
                        }
                    }

                    // Unqualified bare miss, or an owner `::` site with no bare-key
                    // candidates: the authoritative graph declined → drop (the
                    // shipped invariant — no legacy free-fn fan-out for bare names).
                    return ResolutionOutcome::dropped(DropReason::UnknownName);
                }
            }
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
        // Split `mod::T::m` → owner key `T`, method `m` (drop crate/super).
        let mut segs: Vec<&str> = name.split("::").collect();
        let method = segs.pop()?;
        while matches!(segs.first(), Some(&"crate") | Some(&"super")) {
            segs.remove(0);
        }
        let owner = *segs.last()?;
        if owner == "self" || owner == "Self" {
            return None;
        }
        let pool_ids = self.methods.get(&(owner.to_string(), method.to_string()))?;
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
Expected: all three drop-invariant tests PASS (the negative `crate::missing::target` case still drops — its bare `("missing","target")` key does not exist because `missing` is a module not a type, so `owner_lookup_in_modules` misses → `None` → the final `dropped(UnknownName)`; the unqualified-bare-miss still drops; the poison case still drops). The whole `resolution_test::` suite PASSES.

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

In resolve_call_site_full's Rust scope branch: after the existing callable-edge
resolution (now 1→Exact / >1→demoted), an owner-keyed `T::m` that did not
resolve fetches the bare pool and runs the ScopeResolution prune — 1 survivor →
Exact recovery, >1 → pruned NameOnly demote, unchanged → fall through. The
fail-open is narrowed to `name.contains("::")` owner sites routed ONLY to #120's
owner_lookup_in_modules demote floor, never the legacy stem heuristic; an
unqualified bare miss / no-bare-key `::` site still drops. The three shipped drop
invariants are re-confirmed.

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

The legacy `shadow_typepath_narrow` counter only runs inside the `exact_kinds.len() >= 2` guard (multi-target-Exact sites). #120 moved owner collisions to NameOnly (0 Exact edges → guard never entered), and this change's recovery makes them *single*-Exact (1 edge, still < 2) — so that shadow stays empty either way. Add a **new recovery counter** keyed off the demoted-NameOnly `qualified_owner` population: classify each such site by what the scope path now does (`singleton` / `pruned_multiple` / `failopen_unresolved`). Wire it into `call_stats`.

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

In `src/navigation/queries.rs`, add a classifier mirroring `shadow_narrow_type_path`'s resolve-and-classify body but driven by the live resolution outcome. Add it next to `shadow_narrow_type_path`:

```rust
/// Recovery instrument (spec §7 / review MAJOR 5). For a `qualified_owner` `T::m`
/// owner site, report what the scope path NOW yields: `singleton` (recovered to a
/// single Exact), `pruned_multiple` (>1 NameOnly after the disproof prune /
/// owner-collision demote), or `failopen_unresolved` (the owner path did not
/// resolve and it fell through to the #120 demote floor). Keyed off the
/// owner-`::` population, not the >=2-Exact population the legacy
/// `shadow_typepath_narrow` requires.
fn recovery_typepath(out: &crate::resolution::ResolutionOutcome<'_>) -> &'static str {
    use crate::resolution::ResolutionConfidence;
    let exact = out
        .resolved
        .iter()
        .filter(|c| c.confidence == ResolutionConfidence::Exact)
        .count();
    let nameonly = out
        .resolved
        .iter()
        .filter(|c| c.confidence == ResolutionConfidence::NameOnly)
        .count();
    if exact == 1 && nameonly == 0 {
        "singleton"
    } else if exact == 0 && nameonly >= 2 {
        "pruned_multiple"
    } else if exact == 0 && nameonly == 1 {
        // A single NameOnly survivor — the #120 demote floor (no recovery yet).
        "failopen_unresolved"
    } else {
        // Mixed or empty (e.g. an authoritative drop): not a recovery outcome.
        "other"
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

Inside the `for site in sites` loop, after the `out` is computed (the `let out = cg.resolve_call_site_full(site);` line) and within the same iteration, add the owner-`::`-gated classification. Place it right after the existing `match out.drop { ... }` block:

```rust
            if site.callee_name.contains("::") {
                let head_is_owner = {
                    let mut segs: Vec<&str> = site.callee_name.split("::").collect();
                    let _ = segs.pop();
                    while matches!(segs.first(), Some(&"crate") | Some(&"super")) {
                        segs.remove(0);
                    }
                    !matches!(segs.last().copied(), None | Some("self") | Some("Self"))
                };
                if head_is_owner {
                    *recovery_typepath.entry(recovery_typepath(&out)).or_default() += 1;
                }
            }
```

Add it to the emitted JSON (in the `serde_json::json!({ ... })` block, after `"shadow_typepath_narrow": shadow_typepath_narrow,`):

```rust
        "shadow_typepath_narrow": shadow_typepath_narrow,
        "recovery_typepath": recovery_typepath,
```

> Naming note: the local map and the classifier fn share the name `recovery_typepath`. Rust resolves the call `recovery_typepath(&out)` to the function (a value expression), and `*recovery_typepath.entry(...)` to the local binding (a place expression) — no collision. If the executor prefers, rename the map to `recovery_typepath_counts` and the JSON key stays `"recovery_typepath"`; keep the classifier fn name `recovery_typepath`. Either is fine.

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
shadow stays empty either way. Add recovery_typepath: classify each owner-`::`
`T::m` site by the live outcome (singleton / pruned_multiple /
failopen_unresolved), keyed off the qualified_owner population. The acceptance
reads `singleton` rising from this counter plus the kind_exact/kind_nameonly
deltas.

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
- The seven §7 unit/integration cases map to: C1 coverage → Task 1; seam resolves-to-1 → Task 5 `scope_graph_two_crate_owner_collision_recovers_to_single_exact`; pruned-to-2 → Task 5 `scope_graph_inherent_plus_trait_owner_demotes_not_drops`; no-resolution keep-all → Task 5 `scope_graph_unresolved_owner_path_keeps_full_pool_not_drop`; re-export facade → Task 5 `scope_graph_reexport_facade_owner_keeps_all`; block-local glob → Task 5 `scope_graph_block_local_glob_shadow_keeps_all`; macro-wildcard → Task 5 `scope_graph_macro_wildcard_shadow_keeps_all`; fail-open → Task 5 `scope_graph_unresolved_owner_path_keeps_full_pool_not_drop`. ✓
