# Cross-Crate `use` Resolution Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve a 2018+ `use other_crate::Foo` (and `other_crate::f()`) whose leading segment names a sibling **in-repo** crate, by capturing each workspace member's in-repo dependencies and persisting a per-consuming-crate dependency map `crate_deps_by_root` into `ScopeGraph`, then consulting it as the strictly-last leading-segment fallback through a new `ResolutionPolicy` hook. This makes `#122 (A)`'s collision-recovery disproof see a single in-repo item for cross-crate `use` collisions — recovering the **+428** cross-crate collision sites on ruff with zero new collision FPs.

**Architecture:** (1) One new serialized `ScopeGraph` field `crate_deps_by_root: BTreeMap<ScopeId, BTreeMap<String, ScopeId>>` (consuming library `Root` → in-source dep name → depended-on in-repo library `Root`). (2) Per-member in-repo dependency capture in `repo_loader::parse_rust_crate_config` into a new `RustCrateConfig::member_in_repo_deps` map (PATH + WORKSPACE deps that resolve to an in-repo member; external/registry/git excluded). (3) A build-time `Builder::lib_root_by_member_dir` index (library roots only) + `Builder::crate_deps_by_root`, built at `finish()` from the captured per-member deps. (4) A `ResolutionPolicy::extern_crate_root(graph, name, anchor, from)` hook (default `None`, Rust opts in), gated on 2018+ edition + `UsePath`/`Bare` anchor + the consuming crate's dep map; invoked from `resolve_path_guarded` for the leading segment on a TRUE no-rib miss only (a new `scope_member_lookup_probed` reports rib-presence). (5) `CACHE_VERSION` 17→18. `crate_roots_by_name` and the `extern crate` path are left entirely unchanged.

**Tech Stack:** Rust; the `name_resolution` scope-graph engine + Rust policy; `cargo test`; the Tier-A accuracy harness (`eval/`, `uv run tier-a`).

**Spec:** [`docs/superpowers/specs/2026-06-21-prism-cross-crate-use-resolution-design.md`](../specs/2026-06-21-prism-cross-crate-use-resolution-design.md) (design-of-record, codex xhigh SHIP).
**Roadmap:** [`docs/ruff-typepath-recovery-roadmap-2026-06-21.md`](../../ruff-typepath-recovery-roadmap-2026-06-21.md) (cross-crate **+428** → glob +304).
**Predecessors (merged):** #120 demote-not-drop · #121 scope-graph recovery · #122 (A) prune-through-`use` · #123 edition anchoring-class uniformity.

---

## Premises (from spec §1/§3) — govern every task

- **P1 — recall is conditional on a correct unique resolution.** The only engine change converts a TRUE-no-rib-miss `Unresolved` leading segment into `Resolved` (the owning crate `Root`). "Adding a resolution" is **not** automatically recall-safe at the *consumer* level: #122 (A)'s disproof prunes a kept pool once `pending_resolves_to_single_in_repo_item` flips `false→true` (`resolution.rs:1466`), and the general Rust scope-graph resolution can early-return ahead of older fallbacks (`resolution.rs:769-826`). Safety rests on (i) the TRUE-no-rib-miss trigger + claimed-but-invisible shadow, (ii) the anchor/edition gate, (iii) the **per-consuming-crate in-repo dependency gate** (`crate_deps_by_root[consuming_root]`, build-time-resolved to one specific target library root), and (iv) #122 (A)'s exactly-one-in-repo-item gate — **not** on "adding a resolution cannot drop an edge".
- **P2 — local items win (Rust precedence).** A leading segment that matches a local rib/glob is returned by `scope_member_lookup` before the fallback. A **visible** local rib yields a non-`Unresolved` resolution; a **claimed-but-invisible** local rib yields `Unresolved` *with the rib-present flag set*. The fallback fires only on `!rib_present && Unresolved`, so a same-named local module/import always shadows a crate name.
- **P3 — resolve only through the consuming crate's in-repo dependency map.** A leading segment resolves to a crate `Root` iff the consuming crate declares an in-repo dependency under that exact in-source name (present in `crate_deps_by_root[consuming_root]`). Each dep was resolved at build time (by `[dependencies]` PATH / WORKSPACE path) to one specific target library `Root`, so the value is a single root — no global name collision to adjudicate. An external same-name dep (consumer depends on an *external* `b`) is absent from the in-repo map → DECLINE. Library-root-only targets mean a lib+bin package does not self-collide.
- **P4 — known-valid source ⇒ no new visibility checker.** An existing `use other_crate::Foo` compiled, so `Foo` is visible; we resolve it to the right single item. Cross-boundary visibility is not separately enforced — soundness is P3's per-crate dep gate + the engine's existing per-segment `visible()` inside the target crate's module walk.
- **P5 — minimal surface.** One new serialized `ScopeGraph` field (`crate_deps_by_root`); one `ResolutionPolicy` hook (taking `from`); one call site in `resolve_path_guarded`; one `crate_root_of` ascent helper; one `normalize_crate_ident` helper; one `CACHE_VERSION` 17→18 bump; the repo_loader/Builder per-member capture. `crate_roots_by_name` and `extern crate` are untouched.

## Executor / commit protocol (READ FIRST)

If executed by codex under `workspace-write` (the established pattern): codex edits + runs `cargo`, the **host** commits (codex cannot write `.git`). Each task shows the exact commit message + the precise file set to stage. **Never** stage `eval/` or `docs/eval/` artifacts. Commit trailer:

```
Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
```

## macOS test-runner note (READ FIRST)

`--lib`, `--test integration`, `--test ast`, `--test name_resolution` run normally. A bare `cargo test --test cli` may stall at `_dyld_start`; for CLI use `--no-run` then run the freshest binary:

```bash
cargo test --test cli --no-run 2>&1 | tail -3
CLI_BIN=$(ls -t target/debug/deps/cli-* | grep -vE '\.(d|dSYM)$' | head -1)
"$CLI_BIN" 2>&1 | tail -5
```

`cargo test` accepts only ONE test-name filter before `--`; to run several new tests in a file, use a broad module filter (e.g. `--lib repo_loader::tests::`) and read which pass/fail.

---

## Task 1: `crate_deps_by_root` field on `ScopeGraph` + cache bump (foundation, inert)

Add the new serialized field and bump the cache. The field is empty until Task 3 populates it; a round-trip test pins serde-default behavior and the new field's presence.

**Files:**
- Modify: `src/name_resolution/graph.rs` (the `ScopeGraph` struct at `:77-109`).
- Modify: `src/cpg_cache.rs` (`CACHE_VERSION` at `:60`; pin test at `:564-568`).
- Test: `src/name_resolution/graph.rs` (a new `#[cfg(test)] mod tests`; the file currently has none).

### Step 1: Write the failing tests (RED)

**1a — cache pin flip.** In `src/cpg_cache.rs`, replace the whole pin test (`:564-568`):

```rust
    #[test]
    fn cache_version_is_17_for_anchoring_class_uniformity() {
        // v17: edition_uniform recomputed as anchoring-class (2015 vs 2018+).
        assert_eq!(super::CACHE_VERSION, 17);
    }
```

with:

```rust
    #[test]
    fn cache_version_is_18_for_cross_crate_dep_map() {
        // v18: ScopeGraph.crate_deps_by_root (per-consuming-crate in-repo dep map).
        assert_eq!(super::CACHE_VERSION, 18);
    }
```

**1b — graph field round-trip.** `src/name_resolution/graph.rs` has no `mod tests`. Add one at the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::name_resolution::types::ScopeId;

    #[test]
    fn crate_deps_by_root_round_trips_through_serde() {
        // A graph carrying a per-consuming-crate dep map must serialize and
        // deserialize the new field intact (in-memory bincode round-trip).
        let mut g = ScopeGraph::new();
        let mut deps = std::collections::BTreeMap::new();
        deps.insert("b_crate".to_string(), ScopeId(7));
        g.crate_deps_by_root.insert(ScopeId(0), deps);
        let bytes = bincode::serialize(&g).expect("serialize");
        let back: ScopeGraph = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(
            back.crate_deps_by_root.get(&ScopeId(0)).and_then(|m| m.get("b_crate")),
            Some(&ScopeId(7)),
            "the per-crate dep map must survive a serde round-trip"
        );
    }

    #[test]
    fn crate_deps_by_root_defaults_empty_on_missing_field() {
        // A new graph has an empty map (serde(default) keeps an old field-less
        // named-format blob robust; cross-VERSION compat is the CACHE_VERSION bump,
        // since the cache is bincode and deserializes before the version check).
        let g = ScopeGraph::new();
        assert!(g.crate_deps_by_root.is_empty());
    }
}
```

### Step 2: Run the tests to verify they fail (RED)

```bash
cargo test --lib cpg_cache::tests::cache_version_is_18_for_cross_crate_dep_map -- --exact 2>&1 | tail -15
cargo test --lib name_resolution::graph::tests:: 2>&1 | tail -20
```

Expected (RED): the cache pin fails (`left: 17, right: 18`). The two graph tests fail to **compile** (`no field crate_deps_by_root on ScopeGraph`) — that compile error is the RED signal for 1b.

### Step 3: Implement the field + bump (GREEN)

In `src/name_resolution/graph.rs`, add the field to `ScopeGraph` immediately after `file_paths` (the `#[serde(default)]` map convention; matches `file_paths` at `:96-101`). Insert before the `pub scopes:` line (`:102`):

```rust
    /// Rust: per consuming-crate library `Root` → (in-source dependency name →
    /// the depended-on in-repo library `Root`). Built at `Builder::finish()` from
    /// each member's `[dependencies]` PATH and WORKSPACE deps that resolve to an
    /// in-repo crate (external/registry/git deps excluded). The leading-segment
    /// crate-root fallback resolves a 2018+ bare-crate leading segment ONLY through
    /// this per-crate map, so a crate can name another in-repo crate iff it actually
    /// depends on it (Rust's extern prelude is per-crate). Keys (dep names) are
    /// hyphen→underscore normalized. Other languages leave this empty.
    #[serde(default)]
    pub crate_deps_by_root:
        std::collections::BTreeMap<ScopeId, std::collections::BTreeMap<String, ScopeId>>,
```

In `src/cpg_cache.rs:60`, replace:

```rust
const CACHE_VERSION: u32 = 17; // 17: edition_uniform recomputed as anchoring-class.
```

with:

```rust
const CACHE_VERSION: u32 = 18; // 18: ScopeGraph.crate_deps_by_root (per-crate dep map).
```

Also extend the version history comment block — after the `v16:` entry (ending `:59`), insert a `v17`/`v18` note (the prior bump left the history at v16). Insert after `src/cpg_cache.rs:59` (the line ending `owner-keyed disproof prune (changed resolution behavior).`):

```rust
/// - v17: edition_uniform recomputed as anchoring-class (2015 vs 2018+).
/// - v18: ScopeGraph.crate_deps_by_root — per-consuming-crate in-repo dep map
///   for cross-crate `use` leading-segment resolution (changed bincode layout).
```

### Step 4: Run the tests to verify they pass (GREEN)

```bash
cargo test --lib cpg_cache::tests::cache_version_is_18_for_cross_crate_dep_map -- --exact 2>&1 | tail -10
cargo test --lib name_resolution::graph::tests:: 2>&1 | tail -10
```

Expected: all three pass (cache pin == 18; both round-trip/default tests GREEN).

### Step 5: Format + commit

```bash
cargo fmt && cargo fmt --check
```

**Host commits** (stage exactly these two files):

```bash
git add src/name_resolution/graph.rs src/cpg_cache.rs
```

Commit message:

```
feat(name-res): add ScopeGraph.crate_deps_by_root field + CACHE_VERSION 17->18

Add the serialized per-consuming-crate in-repo dependency map
`crate_deps_by_root: BTreeMap<ScopeId, BTreeMap<String, ScopeId>>` (consuming
library Root -> in-source dep name -> depended-on in-repo library Root), behind
`#[serde(default)]` (matches the file_paths convention). Empty/inert until the
populator builds it; a bincode round-trip + default-empty test pin it. Bump
CACHE_VERSION so warm caches recompute the new bincode layout (the cache
deserializes before the version check, so the bump is what guarantees rebuild).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
```

---

## Task 2: Per-member in-repo dependency capture (`repo_loader` + `RustCrateConfig`)

Add `RustCrateConfig::member_in_repo_deps` and populate it in `parse_rust_crate_config`: for each member manifest's `[dependencies]`, record PATH deps and WORKSPACE deps that resolve to an in-repo member; exclude external/version-only/git/registry. Two correctness requirements baked in from the spec-review:
- **Lexical path normalization + escape guard (BLOCKER 1 + re-review BLOCKER A):** dep targets are normalized with a pure-string repo-relative normalizer (`normalize_repo_rel`) so a `path = "../b"` from a non-root member `a/` yields target `b`, the SAME spelling the Builder keys `lib_root_by_member_dir` by (the manifest dir from `join_manifest_rel`, no trailing slash). `join_manifest_rel` alone preserves `..` (`a/../b`) and would miss. The normalizer returns `Option<String>` and yields **`None` when a `..` would pop past the repo root** (an out-of-repo path dep like `path = "../../external/b"`): such a dep target is NOT in-repo, so BOTH the path-dep and workspace-dep call sites must **skip** it (record nothing) rather than silently clamp it to an in-repo-looking dir (spec §2.1 "PATH deps that resolve to an **in-repo** member"). From a non-root member `a/`, `../b → b` is still valid in-repo; only an *underflow* past the root escapes.
- **Per-workspace-root `[workspace.dependencies]` maps (BLOCKER 2):** a repo can hold MULTIPLE workspace roots (`multi_workspace_spanning_boundary_is_not_uniform`, `repo_loader.rs:755`). The `[workspace.dependencies]` pre-scan is keyed **per workspace-root dir**, each member is associated with its **owning** workspace root (nearest ancestor dir that declared `[workspace]`), and a member's `dep = { workspace = true }` resolves through ITS owning workspace's map — never a same-named dep in a different workspace.

**Files:**
- Modify: `src/name_resolution/rust_populator/mod.rs` (add the field to `RustCrateConfig` `:71-96`; the `Default` impl `:100-109`; `from_convention` `:141-149`).
- Modify: `src/repo_loader.rs` (capture in `parse_rust_crate_config` `:271-404`; a per-workspace-root `[workspace.dependencies]` pre-scan + a workspace-root set + per-manifest dep parse; a `normalize_repo_rel` helper; a `parse_member_in_repo_deps` helper; an `owning_workspace_root` helper).
- Modify (compile-fix, BLOCKER 3): `tests/name_resolution/rust_populate_test.rs` — the FOUR explicit full-struct `RustCrateConfig { .. }` literals (no `..default()`) at `:755`, `:792`, `:829`, `:1208` each gain the new `member_in_repo_deps: BTreeMap::new(),` field. (All `src/` literals and `tests/integration/resolution_test.rs:65` already use `..RustCrateConfig::default()` and need no change — verified by `grep -rn "RustCrateConfig {" src/ tests/`.)
- Test: `src/repo_loader.rs` (`#[cfg(test)] mod tests`, after `multi_workspace_spanning_boundary_is_not_uniform` `:794`).

### Step 1: Write the failing tests (RED)

**2a — the field on `RustCrateConfig` (shape).** Add inside `src/name_resolution/rust_populator/mod.rs`'s `mod tests` (after `from_convention_does_not_root_multifile_even_with_top_level_main` ending `:381`):

```rust
    #[test]
    fn member_in_repo_deps_defaults_empty() {
        let cfg = RustCrateConfig::default();
        assert!(
            cfg.member_in_repo_deps.is_empty(),
            "the per-member dep map defaults empty"
        );
        let mut files = BTreeMap::new();
        files.insert(
            "src/lib.rs".to_string(),
            ParsedFile::parse("src/lib.rs", "pub fn a() {}\n", Language::Rust).unwrap(),
        );
        let conv = RustCrateConfig::from_convention(&files);
        assert!(
            conv.member_in_repo_deps.is_empty(),
            "convention fallback (no manifest) records no member deps"
        );
    }
```

**2b — three repo_loader capture tests.** Add inside `src/repo_loader.rs`'s `mod tests`, after `multi_workspace_spanning_boundary_is_not_uniform` (`:794`):

```rust
    #[test]
    fn path_dep_records_in_repo_member_dependency() {
        // `a` declares `b_crate = { path = "../b" }`; `b` is an in-repo member.
        // member_in_repo_deps must map a's member dir -> (b_crate -> b's dir).
        // This is ALSO the BLOCKER-1 normalization case: `join("a", "../b")` is
        // `a/../b` lexically, but the recorded target must be the normalized `b`
        // (the same spelling `lib_root_by_member_dir` keys by) -- `normalize_repo_rel`
        // pops the `..`. A non-normalizing `join_manifest_rel` would record `a/../b`
        // and the Builder lookup in Task 3 would miss.
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
            "[package]\nname = \"a\"\nedition = \"2021\"\n[dependencies]\nb_crate = { path = \"../b\" }\n",
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
        assert_eq!(
            inputs
                .cfg
                .member_in_repo_deps
                .get("a")
                .and_then(|m| m.get("b_crate"))
                .map(String::as_str),
            Some("b"),
            "a path dep on ../b must record (a -> b_crate -> b)"
        );
    }

    #[test]
    fn workspace_dep_records_in_repo_member_dependency() {
        // `a` declares `b_crate = { workspace = true }`; the workspace root has
        // `[workspace.dependencies] b_crate = { path = "b" }`. The ruff-heavy form.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("a/src")).unwrap();
        std::fs::create_dir_all(p.join("b/src")).unwrap();
        std::fs::write(
            p.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"b\"]\n[workspace.dependencies]\nb_crate = { path = \"b\" }\n",
        )
        .unwrap();
        std::fs::write(
            p.join("a/Cargo.toml"),
            "[package]\nname = \"a\"\nedition = \"2021\"\n[dependencies]\nb_crate = { workspace = true }\n",
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
        assert_eq!(
            inputs
                .cfg
                .member_in_repo_deps
                .get("a")
                .and_then(|m| m.get("b_crate"))
                .map(String::as_str),
            Some("b"),
            "a workspace dep resolving to ../b via [workspace.dependencies] must record (a -> b_crate -> b)"
        );
    }

    #[test]
    fn external_version_dep_is_not_recorded() {
        // `a` depends on an EXTERNAL `serde = "1.0"` (version-only, no path). It
        // must NOT enter member_in_repo_deps (no in-repo target).
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("a/src")).unwrap();
        std::fs::write(
            p.join("Cargo.toml"),
            "[workspace]\nmembers = [\"a\"]\n",
        )
        .unwrap();
        std::fs::write(
            p.join("a/Cargo.toml"),
            "[package]\nname = \"a\"\nedition = \"2021\"\n[dependencies]\nserde = \"1.0\"\n",
        )
        .unwrap();
        std::fs::write(p.join("a/src/lib.rs"), "pub fn a() {}\n").unwrap();
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        assert!(
            inputs
                .cfg
                .member_in_repo_deps
                .get("a")
                .map(|m| !m.contains_key("serde"))
                .unwrap_or(true),
            "a version-only external dep must not be recorded as an in-repo dep"
        );
    }

    #[test]
    fn normalize_repo_rel_pops_parent_and_drops_dot() {
        // BLOCKER 1: the lexical repo-relative normalizer is pure string (no fs).
        // It pops `..`, drops `.`, and yields no trailing slash so the result matches
        // the manifest-dir form `lib_root_by_member_dir` keys by. In-repo cases return
        // `Some(dir)`.
        assert_eq!(super::normalize_repo_rel("a/../b").as_deref(), Some("b"));
        assert_eq!(
            super::normalize_repo_rel("crates/a/../b").as_deref(),
            Some("crates/b")
        );
        assert_eq!(super::normalize_repo_rel("./b").as_deref(), Some("b"));
        assert_eq!(super::normalize_repo_rel("a/./b/").as_deref(), Some("a/b"));
        assert_eq!(super::normalize_repo_rel("b").as_deref(), Some("b"));
        assert_eq!(super::normalize_repo_rel("").as_deref(), Some(""));
    }

    #[test]
    fn normalize_repo_rel_returns_none_when_escaping_repo_root() {
        // Re-review BLOCKER A: a `..` that pops past the repo root is an out-of-repo
        // path dep, NOT an in-repo member. The normalizer must return `None` (escaped)
        // so the caller skips it — never silently clamp `../b` -> `b` or
        // `a/../../b` -> `b`.
        assert_eq!(super::normalize_repo_rel("../b"), None);
        assert_eq!(super::normalize_repo_rel("a/../../b"), None);
        assert_eq!(super::normalize_repo_rel("../../external/b"), None);
        // A `..` that nets back inside is fine (pops `a`, lands on `b` in-repo).
        assert_eq!(super::normalize_repo_rel("a/../b").as_deref(), Some("b"));
    }

    #[test]
    fn path_dep_escaping_repo_root_is_not_recorded() {
        // Re-review BLOCKER A (capture call site): a single crate AT the repo root
        // (manifest_dir == "") declares an OUT-OF-REPO path dep `ext = { path = "../b" }`.
        // `join_manifest_rel("", "../b")` = `../b`, which `normalize_repo_rel` rejects
        // (a `..` underflows the repo root → None), so `ext` must NOT be recorded — the
        // old clamp-to-`b` behavior would have falsely recorded an in-repo target.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        std::fs::create_dir_all(p.join("src")).unwrap();
        std::fs::write(
            p.join("Cargo.toml"),
            "[package]\nname = \"root\"\nedition = \"2021\"\n[dependencies]\next = { path = \"../b\" }\n",
        )
        .unwrap();
        std::fs::write(p.join("src/lib.rs"), "pub fn a() {}\n").unwrap();
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        // The escaping `../b` dep from the root member must NOT be recorded as
        // in-repo (it would have escaped the repo root).
        assert!(
            inputs
                .cfg
                .member_in_repo_deps
                .get("")
                .map(|m| !m.contains_key("ext"))
                .unwrap_or(true),
            "an out-of-repo `../b` path dep from the root member must not be recorded"
        );
    }

    #[test]
    fn multi_workspace_same_dep_name_resolves_per_owning_workspace() {
        // BLOCKER 2: two SEPARATE workspaces (ws1, ws2), each with a member that
        // declares `dep = { workspace = true }` under the SAME in-source name `shared`,
        // but each workspace's `[workspace.dependencies] shared` points at a DISTINCT
        // in-repo crate (ws1 -> ws1/libx, ws2 -> ws2/liby). Each consuming member must
        // resolve `shared` through ITS OWN owning workspace's map, never the other's.
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        for sub in ["ws1/a/src", "ws1/libx/src", "ws2/b/src", "ws2/liby/src"] {
            std::fs::create_dir_all(p.join(sub)).unwrap();
        }
        // ws1: member `a` deps `shared = workspace`; ws root maps shared -> libx.
        std::fs::write(
            p.join("ws1/Cargo.toml"),
            "[workspace]\nmembers = [\"a\", \"libx\"]\n[workspace.dependencies]\nshared = { path = \"libx\" }\n",
        )
        .unwrap();
        std::fs::write(
            p.join("ws1/a/Cargo.toml"),
            "[package]\nname = \"a\"\nedition = \"2021\"\n[dependencies]\nshared = { workspace = true }\n",
        )
        .unwrap();
        std::fs::write(p.join("ws1/libx/Cargo.toml"), "[package]\nname = \"libx\"\nedition = \"2021\"\n").unwrap();
        // ws2: member `b` deps `shared = workspace`; ws root maps shared -> liby.
        std::fs::write(
            p.join("ws2/Cargo.toml"),
            "[workspace]\nmembers = [\"b\", \"liby\"]\n[workspace.dependencies]\nshared = { path = \"liby\" }\n",
        )
        .unwrap();
        std::fs::write(
            p.join("ws2/b/Cargo.toml"),
            "[package]\nname = \"b\"\nedition = \"2021\"\n[dependencies]\nshared = { workspace = true }\n",
        )
        .unwrap();
        std::fs::write(p.join("ws2/liby/Cargo.toml"), "[package]\nname = \"liby\"\nedition = \"2021\"\n").unwrap();
        for f in ["ws1/a/src/lib.rs", "ws1/libx/src/lib.rs", "ws2/b/src/lib.rs", "ws2/liby/src/lib.rs"] {
            std::fs::write(p.join(f), "pub fn f() {}\n").unwrap();
        }
        let repo = load_repo(p).unwrap();
        let inputs = repo.scope_graph_inputs.expect("scope graph inputs");
        // ws1/a's `shared` must point at ws1/libx (its OWN workspace), not ws2/liby.
        assert_eq!(
            inputs
                .cfg
                .member_in_repo_deps
                .get("ws1/a")
                .and_then(|m| m.get("shared"))
                .map(String::as_str),
            Some("ws1/libx"),
            "ws1/a resolves `shared` through ws1's [workspace.dependencies]"
        );
        // ws2/b's `shared` must point at ws2/liby (its OWN workspace), not ws1/libx.
        assert_eq!(
            inputs
                .cfg
                .member_in_repo_deps
                .get("ws2/b")
                .and_then(|m| m.get("shared"))
                .map(String::as_str),
            Some("ws2/liby"),
            "ws2/b resolves `shared` through ws2's [workspace.dependencies]"
        );
    }
```

### Step 2: Run the tests to verify they fail (RED)

```bash
cargo test --lib rust_populator::tests::member_in_repo_deps_defaults_empty -- --exact 2>&1 | tail -15
cargo test --lib repo_loader::tests:: 2>&1 | tail -25
```

Expected (RED): everything fails to **compile** first (`no field member_in_repo_deps on RustCrateConfig`, plus `cannot find function normalize_repo_rel`). That compile error is the RED signal. After the field lands (Step 3a) but before the capture lands (Step 3b), the capture tests (`path_dep_records_*`, `workspace_dep_records_*`, `multi_workspace_same_dep_name_resolves_per_owning_workspace`) fail on the `assert_eq!`, `normalize_repo_rel_pops_parent_and_drops_dot` + `normalize_repo_rel_returns_none_when_escaping_repo_root` fail until the `Option`-returning normalizer lands (re-review BLOCKER A — they also fail to **compile** until then, since they reference `super::normalize_repo_rel`), and `external_version_dep_is_not_recorded` + `path_dep_escaping_repo_root_is_not_recorded` pass throughout (guards: both assert a NON-recording, which holds while the capture is empty and must STILL hold after Step 3b — `path_dep_escaping_repo_root_is_not_recorded` is the discriminating escape-guard, green only because the escaping dep is correctly skipped, not merely because capture is absent; to confirm it discriminates, a reviewer may temporarily point the dep at an in-repo `../b`-style target and see it recorded). Note: the `--lib` build is unaffected by the four `tests/name_resolution/rust_populate_test.rs` full-struct literals, but the **`--test name_resolution`** build will not compile until those four literals gain `member_in_repo_deps` (Step 3d, BLOCKER 3) — run `cargo test --lib repo_loader::tests::` for this task's RED/GREEN loop, and the `--test name_resolution` literal fix is verified in Task 4b / Task 6.

### Step 3: Implement (minimal)

**3a — add the field to `RustCrateConfig`.** In `src/name_resolution/rust_populator/mod.rs`, add to the struct (`:71-96`) after the `edition_uniform` field (after `:95`):

```rust
    /// Rust: per workspace-member directory → (in-source dependency name → the
    /// target in-repo member directory) for that member's `[dependencies]` PATH
    /// and WORKSPACE deps that resolve to an in-repo crate. External/version-only/
    /// git/registry deps are NOT recorded. The Builder turns this into
    /// `ScopeGraph::crate_deps_by_root` at `finish()` (library roots only). The KEY
    /// is the in-source name (what `use` writes); the VALUE is the target member dir
    /// (resolved by path), so a renamed in-repo dep is handled naturally. Convention
    /// fallback (no manifest) is empty.
    #[serde(default)]
    pub member_in_repo_deps: BTreeMap<String, BTreeMap<String, String>>,
```

In the `Default` impl (`:100-109`), add the field before the closing brace (after `edition_uniform: true,`):

```rust
            member_in_repo_deps: BTreeMap::new(),
```

In `from_convention` (the literal at `:141-149`), add the same line after `edition_uniform: true,`:

```rust
            member_in_repo_deps: BTreeMap::new(),
```

**3b — capture in `repo_loader`.** In `src/repo_loader.rs`, two changes inside `parse_rust_crate_config`.

First, **pre-scan `[workspace.dependencies]` PER WORKSPACE ROOT** (BLOCKER 2) so workspace-dep targets are resolvable through the *owning* workspace. A repo may hold multiple workspace roots (`repo_loader.rs:755`), so the map is keyed by the workspace-root dir, and a set of workspace-root dirs lets each member find its owning (nearest-ancestor) workspace root. Targets are normalized with `normalize_repo_rel` (BLOCKER 1) so they share the manifest-dir spelling. Insert immediately after the existing `workspace_edition` pre-scan loop closes (after `:312`, the `}` ending the first `for manifest_path in manifest_hashes.keys()` loop), before the second per-manifest loop at `:314`:

```rust
    // Per-workspace-root `[workspace.dependencies]` pre-scan (BLOCKER 2): a repo can
    // hold MULTIPLE workspace roots, so a global name->target map would cross-resolve
    // a same-named workspace dep into the wrong workspace. Key the map by the
    // workspace-root dir (the dir that declared `[workspace]`), and record the set of
    // workspace-root dirs so each member resolves through its OWNING (nearest-ancestor)
    // workspace. Only entries carrying a `path` (the in-repo workspace-dep targets) are
    // recorded; targets are normalized (`..`/`.` collapsed) to the manifest-dir form.
    let mut workspace_dep_paths: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut workspace_root_dirs: BTreeSet<String> = BTreeSet::new();
    for manifest_path in manifest_hashes.keys() {
        let abs = root.join(manifest_path);
        let Ok(text) = std::fs::read_to_string(&abs) else {
            continue;
        };
        let Ok(value) = text.parse::<toml::Value>() else {
            continue;
        };
        let manifest_dir = manifest_path
            .strip_suffix("Cargo.toml")
            .unwrap_or("")
            .trim_end_matches('/');
        // A manifest that declares `[workspace]` is a workspace root (members and/or
        // `[workspace.dependencies]` hang off it).
        let ws = value.get("workspace");
        if ws.is_some() {
            workspace_root_dirs.insert(manifest_dir.to_string());
        }
        if let Some(ws_deps) = ws
            .and_then(|w| w.get("dependencies"))
            .and_then(|d| d.as_table())
        {
            let entry = workspace_dep_paths
                .entry(manifest_dir.to_string())
                .or_default();
            for (name, spec) in ws_deps {
                if let Some(path) = spec.get("path").and_then(|p| p.as_str()) {
                    // Skip an out-of-repo workspace-dep target (a `..` that escapes the
                    // repo root): `normalize_repo_rel` returns `None` (re-review BLOCKER A).
                    if let Some(target) =
                        normalize_repo_rel(&join_manifest_rel(manifest_dir, path))
                    {
                        entry.insert(name.clone(), target);
                    }
                }
            }
        }
    }
    let mut member_in_repo_deps: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
```

Second, **capture each member's deps** inside the existing per-manifest loop, resolving any `workspace = true` dep through the member's **owning** workspace map. Insert immediately after the existing `collect_dep_renames(&value, &mut cfg.dep_renames);` line (`:391`), before the loop's closing `}` (`:392`):

```rust
        // Resolve this member's `workspace = true` deps through its OWNING workspace
        // root's `[workspace.dependencies]` (nearest ancestor dir that declared
        // `[workspace]`); an empty map when the member is not under any workspace root.
        let owning_ws = owning_workspace_root(manifest_dir, &workspace_root_dirs);
        let empty_ws_deps = BTreeMap::new();
        let ws_deps_for_member = owning_ws
            .and_then(|ws| workspace_dep_paths.get(ws))
            .unwrap_or(&empty_ws_deps);
        let member_deps = parse_member_in_repo_deps(&value, manifest_dir, ws_deps_for_member);
        if !member_deps.is_empty() {
            member_in_repo_deps.insert(manifest_dir.to_string(), member_deps);
        }
```

Then, **store it onto `cfg`** in the finalization block. Insert after the existing `cfg.bin_paths = bin_paths.into_iter().collect();` line (`:402`), before `Some(cfg)` (`:403`):

```rust
    cfg.member_in_repo_deps = member_in_repo_deps;
```

**Add the helpers** immediately after `anchoring_class_uniform` (`:421-423`) and before `join_manifest_rel` (`:425`):

```rust
/// Lexically normalize a repo-relative path to the manifest-dir spelling (BLOCKER 1),
/// returning `None` when a `..` escapes the repo root (re-review BLOCKER A).
/// PURE STRING (no filesystem): split on `/`, drop `.` and empty segments, pop the
/// previous segment on `..`. A `..` with nothing left to pop means the path points
/// OUT of the repo (e.g. `../b`, `a/../../b`) → `None`, so the caller skips it (an
/// out-of-repo path dep is not an in-repo member). Produces no trailing slash and no
/// `.`/`..` components, so a `path = "../b"` dep from a non-root member `a/` (`join`
/// → `a/../b`) collapses to `Some("b")`, the SAME key the Builder uses for
/// `lib_root_by_member_dir`. `join_manifest_rel` alone keeps the `..` and would miss
/// the index. (`Some("")` is the repo root itself — a valid in-repo single-crate dir.)
fn normalize_repo_rel(path: &str) -> Option<String> {
    let mut out: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                // Pop the parent; if there is nothing to pop, the `..` escapes the
                // repo root → out-of-repo → decline (do not clamp to an in-repo dir).
                if out.pop().is_none() {
                    return None;
                }
            }
            s => out.push(s),
        }
    }
    Some(out.join("/"))
}

/// The OWNING workspace root for `manifest_dir` (BLOCKER 2): the LONGEST workspace-root
/// dir that is an ancestor of (or equal to) `manifest_dir`. `""` (the repo root) is an
/// ancestor of everything, so a single top-level workspace owns all members. Returns
/// `None` when the member is under no workspace root (a non-workspace single crate).
fn owning_workspace_root<'a>(
    manifest_dir: &str,
    workspace_root_dirs: &'a BTreeSet<String>,
) -> Option<&'a str> {
    workspace_root_dirs
        .iter()
        .filter(|ws| is_dir_ancestor(ws, manifest_dir))
        .map(String::as_str)
        // Longest matching ancestor = the nearest (innermost) owning workspace.
        .max_by_key(|ws| ws.len())
}

/// True iff `ancestor` is a directory-prefix of (or equal to) `dir`, both repo-relative
/// with no trailing slash. `""` (repo root) is an ancestor of everything. Avoids the
/// `"a"` ⊂ `"ab"` false match by requiring a `/` boundary.
fn is_dir_ancestor(ancestor: &str, dir: &str) -> bool {
    if ancestor.is_empty() || ancestor == dir {
        return true;
    }
    dir.strip_prefix(ancestor)
        .map(|rest| rest.starts_with('/'))
        .unwrap_or(false)
}

/// Capture one member manifest's in-repo `[dependencies]` (PATH + WORKSPACE forms)
/// as `in_source_name → target member dir`. External/version-only/git/registry deps
/// are skipped (they resolve to no in-repo path). `manifest_dir` is the member's
/// directory (no trailing slash, "" for the repo root); `workspace_dep_paths` maps a
/// `[workspace.dependencies]` name to its resolved (normalized) target dir, scoped to
/// THIS member's owning workspace (for `workspace = true`).
fn parse_member_in_repo_deps(
    value: &toml::Value,
    manifest_dir: &str,
    workspace_dep_paths: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Some(deps) = value.get("dependencies").and_then(|d| d.as_table()) else {
        return out;
    };
    for (in_source_name, spec) in deps {
        // A bare `dep = "1.0"` string spec is version-only / external → skip.
        let Some(table) = spec.as_table() else {
            continue;
        };
        if let Some(path) = table.get("path").and_then(|p| p.as_str()) {
            // PATH dep: target dir is the member dir joined with the relative path,
            // normalized (`..`/`.` collapsed) to the manifest-dir spelling (BLOCKER 1).
            // `normalize_repo_rel` returns `None` if a `..` escapes the repo root — an
            // out-of-repo dep, so skip it (do not record) (re-review BLOCKER A).
            if let Some(target) = normalize_repo_rel(&join_manifest_rel(manifest_dir, path)) {
                out.insert(in_source_name.clone(), target);
            }
        } else if table
            .get("workspace")
            .and_then(|w| w.as_bool())
            .unwrap_or(false)
        {
            // WORKSPACE dep: resolve through the owning workspace's
            // `[workspace.dependencies][name].path` (already normalized at pre-scan).
            if let Some(target_dir) = workspace_dep_paths.get(in_source_name) {
                out.insert(in_source_name.clone(), target_dir.clone());
            }
            // No `[workspace.dependencies]` path entry → external → skip.
        }
        // else (version/git/registry table, no path, not workspace) → external → skip.
    }
    out
}
```

(`BTreeMap`, `BTreeSet`, `join_manifest_rel`, `toml::Value`, `root`, `manifest_hashes` are all already in scope in `repo_loader.rs` — `BTreeSet` is imported at `repo_loader.rs:9`.)

**3d — fix the four explicit `RustCrateConfig` full-struct literals (BLOCKER 3).** Adding the non-`Option` `member_in_repo_deps` field breaks any literal that lists every field WITHOUT `..RustCrateConfig::default()`. `grep -rn "RustCrateConfig {" src/ tests/` confirms only FOUR such literals exist, all in `tests/name_resolution/rust_populate_test.rs` (`:755`, `:792`, `:829`, `:1208`); every `src/` literal and `tests/integration/resolution_test.rs:65` already use `..RustCrateConfig::default()` and are unaffected (the field has `#[serde(default)]` and is in `Default`). In each of the four literals, add the field after the existing `edition_uniform: true,` line:

```rust
        member_in_repo_deps: BTreeMap::new(),
```

`BTreeMap` is already imported in `rust_populate_test.rs` (`use std::collections::{BTreeMap, BTreeSet};` at `:14`). The four literals look like (each ends with `edition_uniform: true,` then `};`) — e.g. `:792`:

```rust
    let cfg = RustCrateConfig {
        edition: 2018,
        crate_roots: vec![
            "crate_a/src/lib.rs".to_string(),
            "crate_b/src/lib.rs".to_string(),
        ],
        workspace_members: vec!["crate_a".to_string(), "crate_b".to_string()],
        dep_renames: BTreeMap::new(),
        lib_path: None,
        bin_paths: vec![],
        edition_uniform: true,
        member_in_repo_deps: BTreeMap::new(),
    };
```

Apply the identical one-line addition at `:755`, `:829`, and `:1208`. (Leaving any one unedited fails the `--test name_resolution` build with `missing field member_in_repo_deps`.)

### Step 4: Run the tests to verify they pass (GREEN)

```bash
cargo test --lib rust_populator::tests::member_in_repo_deps_defaults_empty -- --exact 2>&1 | tail -10
cargo test --lib repo_loader::tests:: 2>&1 | tail -15
cargo test --test name_resolution --no-run 2>&1 | tail -5   # confirms the 4 literal fixes compile
```

Expected: the field test GREEN; `normalize_repo_rel_pops_parent_and_drops_dot`, `normalize_repo_rel_returns_none_when_escaping_repo_root` (re-review BLOCKER A), all path/workspace capture tests, and `multi_workspace_same_dep_name_resolves_per_owning_workspace` GREEN; `external_version_dep_is_not_recorded` and `path_dep_escaping_repo_root_is_not_recorded` still GREEN; and the existing edition/coverage `repo_loader::tests::` (e.g. `pure_2018plus_mixed_workspace_is_uniform`, `multi_workspace_spanning_boundary_is_not_uniform`, the loader-parity test) still pass. The `--test name_resolution --no-run` build compiles (the four literal fixes landed).

### Step 5: Format + commit

```bash
cargo fmt && cargo fmt --check
```

**Host commits** (stage exactly these three files):

```bash
git add src/name_resolution/rust_populator/mod.rs src/repo_loader.rs tests/name_resolution/rust_populate_test.rs
```

Commit message:

```
feat(repo_loader): capture per-member in-repo dependencies

Add `RustCrateConfig::member_in_repo_deps` (member dir -> in-source dep name ->
target member dir) and populate it in parse_rust_crate_config: PATH deps
(`b = { path = "../b" }`) join+NORMALIZE the member dir (lexical normalize_repo_rel
collapses `..`/`.` to the manifest-dir spelling the Builder keys by); WORKSPACE
deps (`b = { workspace = true }`) resolve through a PER-WORKSPACE-ROOT
`[workspace.dependencies][b].path` pre-scan via the member's owning (nearest-
ancestor) workspace root, so a same-named dep in another workspace never
cross-resolves. normalize_repo_rel returns Option and yields None when a `..`
escapes the repo root, so an out-of-repo path dep (`path = "../../external/b"`)
is skipped, never clamped to an in-repo-looking dir. Version-only/git/registry
deps carry no in-repo path and are not recorded. The KEY is the in-source name
and the target resolves by path, so renamed in-repo deps are subsumed. Adds
member_in_repo_deps to the four explicit RustCrateConfig literals in
rust_populate_test.rs. Inert until the Builder consumes it.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
```

---

## Task 3: Builder `lib_root_by_member_dir` + build `crate_deps_by_root`

Record each LIBRARY root's member dir → its `Root` scope at root creation, then at `finish()` build `crate_deps_by_root` from `config.member_in_repo_deps` + that index. Add the `normalize_crate_ident` helper (hyphen→underscore).

**Keying contract (re-review BLOCKER B):** `lib_root_by_member_dir` MUST be keyed by the **member directory** (the manifest dir — the exact spelling `member_in_repo_deps` uses, since both its KEYS and its `normalize_repo_rel`'d VALUES are manifest dirs). The member dir for a library root is derived from `config.workspace_members` (the longest member prefix of `root_path`), NOT from the library FILE's parent — because for an explicit `[lib] path = "src/lib.rs"` the root file is `<member>/src/lib.rs` and its parent is `<member>/src` (and a nested `[lib] path = "src/inner/lib.rs"` parent is `<member>/src/inner`), neither of which is the member dir `<member>` that dep-target resolution produces. `config.workspace_members` carries those member dirs verbatim (`repo_loader.rs:357` inserts `join_manifest_rel(manifest_dir, member)`; `crate_name_for_root` `builder.rs:521-543` already prefix-matches `root_path` against them). The single-crate-at-root case (empty `workspace_members`) keys by `""` (the root manifest dir), accepted by the exact no-member gate (`root_path == "src/lib.rs"` or a root `config.lib_path`). No new data is threaded through the Builder — `config.workspace_members` + `config.lib_path` are the real, already-available signals.

**Files:**
- Modify: `src/name_resolution/rust_policy.rs` (add the `pub(crate) fn normalize_crate_ident`).
- Modify: `src/name_resolution/rust_populator/builder.rs` (two new `Builder` fields; record at `create_root` `:269-287`; build at `finish()` `:101-103`; a `lib_root_member_dir` free helper that derives the member dir from `config.workspace_members` — keyed by member dir, not the library file's parent — re-review BLOCKER B).
- Test: `src/name_resolution/rust_populator/builder.rs` (a new `#[cfg(test)] mod tests`; the file currently has none).

### Step 1: Write the failing tests (RED)

Add a `mod tests` at the end of `src/name_resolution/rust_populator/builder.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::super::{populate_rust, RustCrateConfig};
    use crate::ast::ParsedFile;
    use crate::languages::Language;
    use crate::name_resolution::types::{ScopeId, ScopeKind};
    use std::collections::BTreeMap;

    fn rs(src: &str) -> ParsedFile {
        ParsedFile::parse("x.rs", src, Language::Rust).unwrap()
    }

    fn root_for(graph: &crate::name_resolution::graph::ScopeGraph, file_path_idx: u32) -> ScopeId {
        // The Root scope whose extent is in the file at FileId(idx).
        graph
            .scopes
            .iter()
            .find(|(_, s)| {
                matches!(s.kind, ScopeKind::Root)
                    && s.extents.iter().any(|e| e.file.0 == file_path_idx)
            })
            .map(|(id, _)| *id)
            .expect("a Root scope for the file")
    }

    #[test]
    fn crate_deps_by_root_maps_consumer_to_target_lib_root() {
        // Two-crate workspace: a depends on b (recorded by member_in_repo_deps).
        // crate_deps_by_root[a_lib_root]["b_crate"] must be b's lib Root.
        let mut files = BTreeMap::new();
        files.insert("a/src/lib.rs".to_string(), rs("pub fn a() {}\n"));
        files.insert("b/src/lib.rs".to_string(), rs("pub fn b() {}\n"));
        let mut member_deps = BTreeMap::new();
        let mut a_deps = BTreeMap::new();
        a_deps.insert("b_crate".to_string(), "b".to_string());
        member_deps.insert("a".to_string(), a_deps);
        let cfg = RustCrateConfig {
            crate_roots: vec!["a/src/lib.rs".to_string(), "b/src/lib.rs".to_string()],
            workspace_members: vec!["a".to_string(), "b".to_string()],
            member_in_repo_deps: member_deps,
            ..RustCrateConfig::default()
        };
        let graph = populate_rust(&files, &cfg, None);
        // FileIds follow sorted key order: a/src/lib.rs = 0, b/src/lib.rs = 1.
        let a_root = root_for(&graph, 0);
        let b_root = root_for(&graph, 1);
        assert_eq!(
            graph
                .crate_deps_by_root
                .get(&a_root)
                .and_then(|m| m.get("b_crate")),
            Some(&b_root),
            "a's dep map must point b_crate at b's lib Root"
        );
    }

    #[test]
    fn lib_plus_bin_member_does_not_self_collide() {
        // A single member `b` with BOTH src/lib.rs and src/main.rs, depended on by
        // `a`. The recorded dep target must be b's LIBRARY root, never the bin root.
        let mut files = BTreeMap::new();
        files.insert("a/src/lib.rs".to_string(), rs("pub fn a() {}\n"));
        files.insert("b/src/lib.rs".to_string(), rs("pub fn b() {}\n"));
        files.insert("b/src/main.rs".to_string(), rs("fn main() {}\n"));
        let mut member_deps = BTreeMap::new();
        let mut a_deps = BTreeMap::new();
        a_deps.insert("b".to_string(), "b".to_string());
        member_deps.insert("a".to_string(), a_deps);
        let cfg = RustCrateConfig {
            crate_roots: vec![
                "a/src/lib.rs".to_string(),
                "b/src/lib.rs".to_string(),
                "b/src/main.rs".to_string(),
            ],
            workspace_members: vec!["a".to_string(), "b".to_string()],
            member_in_repo_deps: member_deps,
            ..RustCrateConfig::default()
        };
        let graph = populate_rust(&files, &cfg, None);
        // a/src/lib.rs=0, b/src/lib.rs=1, b/src/main.rs=2 (sorted key order).
        let a_root = root_for(&graph, 0);
        let b_lib_root = root_for(&graph, 1);
        assert_eq!(
            graph.crate_deps_by_root.get(&a_root).and_then(|m| m.get("b")),
            Some(&b_lib_root),
            "the dep target must be b's library root (FileId 1), never the bin root (FileId 2)"
        );
    }

    #[test]
    fn explicit_lib_path_target_keys_by_member_dir_not_file_parent() {
        // Re-review BLOCKER B: target `b` has an EXPLICIT `[lib] path = "src/lib.rs"`
        // (cfg.lib_path = "b/src/lib.rs"). The dep-target index MUST key b's lib Root
        // by the MEMBER DIR `b` (from workspace_members), NOT the library file's parent
        // `b/src`. The old `parent_dir`-based helper keyed `b/src` -> crate_deps miss.
        let mut files = BTreeMap::new();
        files.insert("a/src/lib.rs".to_string(), rs("pub fn a() {}\n"));
        files.insert("b/src/lib.rs".to_string(), rs("pub fn b() {}\n"));
        let mut member_deps = BTreeMap::new();
        let mut a_deps = BTreeMap::new();
        a_deps.insert("b".to_string(), "b".to_string()); // target dir = the member dir `b`
        member_deps.insert("a".to_string(), a_deps);
        let cfg = RustCrateConfig {
            // `a` is conventional; `b`'s library root is the explicit [lib].path.
            crate_roots: vec!["a/src/lib.rs".to_string()],
            lib_path: Some("b/src/lib.rs".to_string()),
            workspace_members: vec!["a".to_string(), "b".to_string()],
            member_in_repo_deps: member_deps,
            ..RustCrateConfig::default()
        };
        let graph = populate_rust(&files, &cfg, None);
        // Sorted key order: a/src/lib.rs=0, b/src/lib.rs=1.
        let a_root = root_for(&graph, 0);
        let b_lib_root = root_for(&graph, 1);
        assert_eq!(
            graph.crate_deps_by_root.get(&a_root).and_then(|m| m.get("b")),
            Some(&b_lib_root),
            "an explicit [lib].path target must key by member dir `b`, not `b/src`"
        );
    }

    #[test]
    fn nested_custom_lib_path_target_keys_by_member_dir() {
        // Re-review BLOCKER B: target `b`'s library root is a NESTED custom path
        // `[lib] path = "src/inner/lib.rs"` (cfg.lib_path = "b/src/inner/lib.rs").
        // The member dir is still `b` (from workspace_members) — never the file parent
        // `b/src/inner`. This is the case a file-parent derivation alone gets wrong,
        // so the workspace_members-prefix derivation is required.
        let mut files = BTreeMap::new();
        files.insert("a/src/lib.rs".to_string(), rs("pub fn a() {}\n"));
        files.insert("b/src/inner/lib.rs".to_string(), rs("pub fn b() {}\n"));
        let mut member_deps = BTreeMap::new();
        let mut a_deps = BTreeMap::new();
        a_deps.insert("b".to_string(), "b".to_string());
        member_deps.insert("a".to_string(), a_deps);
        let cfg = RustCrateConfig {
            crate_roots: vec!["a/src/lib.rs".to_string()],
            lib_path: Some("b/src/inner/lib.rs".to_string()),
            workspace_members: vec!["a".to_string(), "b".to_string()],
            member_in_repo_deps: member_deps,
            ..RustCrateConfig::default()
        };
        let graph = populate_rust(&files, &cfg, None);
        // Sorted key order: a/src/lib.rs=0, b/src/inner/lib.rs=1.
        let a_root = root_for(&graph, 0);
        let b_lib_root = root_for(&graph, 1);
        assert_eq!(
            graph.crate_deps_by_root.get(&a_root).and_then(|m| m.get("b")),
            Some(&b_lib_root),
            "a nested custom [lib].path target must still key by member dir `b`"
        );
    }

    #[test]
    fn custom_bin_path_ending_src_lib_rs_does_not_shadow_library_root() {
        // Re-review round-3 MAJOR (round-4 rework): member `b` has the conventional
        // library root `b/src/lib.rs` AND a bin/tool root whose path ALSO ends in
        // `src/lib.rs`. A bare `ends_with("src/lib.rs")` gate would attribute the bin
        // path to member `b` too; the EXACT `b/src/lib.rs` gate keeps it out, so the
        // dep target is always b's LIBRARY root.
        //
        // This test discriminates TWO ways:
        //   1. DIRECT, order-independent helper assertions (the bulletproof
        //      discriminator): the old bare-suffix gate returns `Some("b")` for a
        //      `b/.../src/lib.rs` bin path, the new exact gate returns `None`. These
        //      do NOT depend on FileId/`or_insert` order, so they catch the buggy
        //      helper regardless of insertion ordering.
        //   2. An end-to-end `crate_deps_by_root` check using a decoy bin path that
        //      sorts BEFORE the real lib (`b/bin/src/lib.rs` < `b/src/lib.rs`), so
        //      under the OLD helper the bin root would win the FIRST-wins `or_insert`
        //      in sorted-FileId order and the e2e assertion would fail. (The round-3
        //      decoy `b/tools/src/lib.rs` sorts AFTER `b/src/lib.rs`, so the real lib
        //      won `or_insert` first even under the buggy helper — that decoy did not
        //      discriminate; `b/bin` fixes it.)
        let mut files = BTreeMap::new();
        files.insert("a/src/lib.rs".to_string(), rs("pub fn a() {}\n"));
        files.insert("b/bin/src/lib.rs".to_string(), rs("pub fn tool() {}\n"));
        files.insert("b/src/lib.rs".to_string(), rs("pub fn b() {}\n"));
        let mut member_deps = BTreeMap::new();
        let mut a_deps = BTreeMap::new();
        a_deps.insert("b".to_string(), "b".to_string());
        member_deps.insert("a".to_string(), a_deps);
        let cfg = RustCrateConfig {
            // All three are roots; b's conventional lib + a tool bin under b/bin.
            crate_roots: vec![
                "a/src/lib.rs".to_string(),
                "b/bin/src/lib.rs".to_string(),
                "b/src/lib.rs".to_string(),
            ],
            workspace_members: vec!["a".to_string(), "b".to_string()],
            member_in_repo_deps: member_deps,
            ..RustCrateConfig::default()
        };

        // (1) Direct helper assertions — order-independent, the bulletproof
        // discriminator. `lib_root_member_dir` is a private FREE fn in builder.rs and
        // `mod tests` is in builder.rs, so `super::` reaches it; `cfg` is in scope here.
        assert_eq!(
            super::lib_root_member_dir("b/bin/src/lib.rs", &cfg),
            None,
            "a `b/bin/src/lib.rs` bin/tool path is NOT member b's library root \
             (old bare-suffix gate wrongly returned Some(\"b\"); exact gate returns None)"
        );
        assert_eq!(
            super::lib_root_member_dir("b/src/lib.rs", &cfg),
            Some("b".to_string()),
            "the conventional `b/src/lib.rs` IS member b's library root"
        );

        // (2) End-to-end, made discriminating by the sort-before-bin decoy.
        let graph = populate_rust(&files, &cfg, None);
        // Sorted key order: a/src/lib.rs=0, b/bin/src/lib.rs=1, b/src/lib.rs=2.
        let a_root = root_for(&graph, 0);
        let b_bin_root = root_for(&graph, 1);
        let b_lib_root = root_for(&graph, 2);
        let target = graph.crate_deps_by_root.get(&a_root).and_then(|m| m.get("b"));
        assert_eq!(
            target,
            Some(&b_lib_root),
            "the dep target must be b's library root (FileId 2), never the bin/tool root"
        );
        assert_ne!(
            target,
            Some(&b_bin_root),
            "a `bin/src/lib.rs` bin root (FileId 1, sorts BEFORE the real lib) must \
             not win or_insert as b's library root"
        );
    }

    #[test]
    fn normalize_crate_ident_hyphen_to_underscore() {
        use crate::name_resolution::rust_policy::normalize_crate_ident;
        assert_eq!(normalize_crate_ident("my-crate"), "my_crate");
        assert_eq!(normalize_crate_ident("plain"), "plain");
    }
}
```

### Step 2: Run the tests to verify they fail (RED)

```bash
cargo test --lib name_resolution::rust_populator::builder::tests:: 2>&1 | tail -25
```

Expected (RED): `normalize_crate_ident_hyphen_to_underscore` fails to **compile** (`unresolved import ... normalize_crate_ident`); and `custom_bin_path_ending_src_lib_rs_does_not_shadow_library_root` fails to **compile** until 3e lands (it calls `super::lib_root_member_dir`, which does not yet exist). The `crate_deps_by_root_*`/`lib_plus_bin_*` tests and the two BLOCKER B tests (`explicit_lib_path_target_keys_by_member_dir_not_file_parent`, `nested_custom_lib_path_target_keys_by_member_dir`) fail on the `assert_eq!` (the map is empty — `finish()` does not yet populate it). After 3c/3d/3e land, these are the discriminating cases: with the OLD `parent_dir`-based helper the BLOCKER B tests fail (explicit/nested lib root keys by `b/src`/`b/src/inner`); with a bare `ends_with("src/lib.rs")` gate the round-3 MAJOR test fails on BOTH its discriminators — the DIRECT, order-independent assertion `lib_root_member_dir("b/bin/src/lib.rs", &cfg) == None` (the old gate returns `Some("b")`), AND the end-to-end `crate_deps_by_root` check whose decoy bin path `b/bin/src/lib.rs` sorts BEFORE the real `b/src/lib.rs`, so under the old gate the bin root wins the FIRST-wins `or_insert` and the dep target is wrong; the reworked member-dir-keyed, EXACT-`m/src/lib.rs`-gated helper is what turns them all GREEN.

### Step 3: Implement (minimal)

**3a — the normalizer.** In `src/name_resolution/rust_policy.rs`, add a free function after the `pairwise_all_exclusive` helper (after `:320`, at the end of the file):

```rust
/// Normalize a crate/dependency identifier to the Rust path-identifier form: a
/// Cargo dependency name may carry hyphens (`my-crate`) while a `use` path writes
/// underscores (`my_crate`). Used to key `ScopeGraph::crate_deps_by_root` and to
/// normalize the leading-segment query in `extern_crate_root` identically.
pub(crate) fn normalize_crate_ident(name: &str) -> String {
    name.replace('-', "_")
}
```

**3b — Builder fields.** In `src/name_resolution/rust_populator/builder.rs`, add two fields to the `Builder` struct after `crate_roots_by_name` (`:48`):

```rust
    /// Library-root-only index: member directory → its `Root` scope. A bin/test
    /// root is NEVER recorded here (only a library root is a dependency target), so
    /// a lib+bin member does not self-collide. Built as each library root is minted.
    lib_root_by_member_dir: BTreeMap<String, ScopeId>,
    /// Per consuming-crate library `Root` → (normalized in-source dep name →
    /// depended-on in-repo library `Root`). Built at `finish()` from
    /// `config.member_in_repo_deps` + `lib_root_by_member_dir`; moved onto the graph.
    crate_deps_by_root: BTreeMap<ScopeId, BTreeMap<String, ScopeId>>,
```

Initialize them in `Builder::new` (after `crate_roots_by_name: BTreeMap::new(),` at `:73`):

```rust
            lib_root_by_member_dir: BTreeMap::new(),
            crate_deps_by_root: BTreeMap::new(),
```

Add the `normalize_crate_ident` import to the existing `use` block (after the `use super::{...}` at `:15`):

```rust
use crate::name_resolution::rust_policy::normalize_crate_ident;
```

**3c — record library roots at `create_root`.** In `create_root` (`:269-287`), record the member dir → Root for **library roots only**. Insert immediately after the existing `if let Some(name) = crate_name_for_root(...) { ... }` block (after `:285`), before `Some(root_scope)` (`:286`):

```rust
        // Library-root-only dependency-target index (P3 / MAJOR): record the member
        // dir → this Root iff `root_path` is a library root (`[lib].path` override or
        // the conventional `.../src/lib.rs`). A bin/test/bench/example root is never a
        // `use`-nameable dependency target, so it is excluded (lib+bin no-self-collide).
        if let Some(member_dir) = lib_root_member_dir(root_path, self.config) {
            self.lib_root_by_member_dir
                .entry(member_dir)
                .or_insert(root_scope);
        }
```

**3d — build `crate_deps_by_root` at `finish()`.** Replace the existing `finish` (`:101-103`):

```rust
    pub(crate) fn finish(self) -> ScopeGraph {
        self.graph
    }
```

with:

```rust
    pub(crate) fn finish(mut self) -> ScopeGraph {
        // Build the per-consuming-crate dep map from captured per-member deps +
        // the library-root index. For each consuming member M with a LIBRARY root
        // Rc, and each recorded `(in_source_name -> target_dir)`, map the normalized
        // in-source name to the target member's LIBRARY root (skip a target with no
        // library root — a bin-only member is not `use`-nameable). Keys are
        // normalized hyphen→underscore (the §2.2 hook normalizes the query the same).
        for (member_dir, deps) in &self.config.member_in_repo_deps {
            let Some(&consuming_root) = self.lib_root_by_member_dir.get(member_dir) else {
                continue; // consuming member has no library root → not keyed (v1)
            };
            for (in_source_name, target_dir) in deps {
                if let Some(&target_root) = self.lib_root_by_member_dir.get(target_dir) {
                    self.crate_deps_by_root
                        .entry(consuming_root)
                        .or_default()
                        .insert(normalize_crate_ident(in_source_name), target_root);
                }
            }
        }
        self.graph.crate_deps_by_root = self.crate_deps_by_root;
        self.graph
    }
```

**3e — the `lib_root_member_dir` helper.** Add a free helper after `crate_name_for_root` (`:521-543`, at the end of the file):

```rust
/// The workspace-member DIRECTORY for a root path **iff it is a library root**
/// (re-review BLOCKER B). A library root is EXACTLY `<member>/src/lib.rs` (the
/// convention) OR the `[lib].path` override (`config.lib_path == Some(root_path)`)
/// for that member; a bin/test/bench/example root returns `None` (never a dependency
/// target — the lib+bin no-self-collide guarantee).
///
/// The member dir is the MANIFEST DIR — the exact spelling `member_in_repo_deps`
/// keys by (Task 2 normalizes both its KEYS (the manifest dir) and the dep-target
/// VALUES (`normalize_repo_rel`) to that form), so the `finish()` lookups
/// `lib_root_by_member_dir.get(member_dir)`/`.get(target_dir)` hit. It is derived
/// from `config.workspace_members` (the LONGEST member prefix of `root_path`),
/// **not** the library file's parent: an explicit `[lib] path = "src/lib.rs"` has
/// root file `<member>/src/lib.rs` whose parent is `<member>/src`, and a nested
/// `[lib] path = "src/inner/lib.rs"` parent is `<member>/src/inner` — neither is the
/// member dir `<member>` that dep-target resolution produces. `workspace_members`
/// carries those member dirs verbatim (`repo_loader.rs:357` =
/// `join_manifest_rel(manifest_dir, member)`, no trailing slash); this mirrors
/// `crate_name_for_root`'s prefix match but returns the FULL member dir, not the
/// basename.
///
/// The library-root gate is EXACT (re-review round-3 MAJOR): for a matched member
/// `m`, accept ONLY `m/src/lib.rs` or the explicit `config.lib_path` override — a
/// bare `ends_with("src/lib.rs")` is too broad and would mis-record a bin/tool path
/// like `m/tools/src/lib.rs` as `m`'s library root, shadowing the real lib root.
/// When no member prefix matches (the single crate at the repo root, member dir `""`,
/// absent from `workspace_members`), accept only the bare `src/lib.rs` or a root
/// `[lib].path`. (Repo-wide, `config.lib_path` holds only the LAST explicit
/// `[lib].path` — a pre-existing `RustCrateConfig` flatness; convention library roots
/// are unaffected because the exact `m/src/lib.rs` gate matches them without it.)
fn lib_root_member_dir(root_path: &str, config: &RustCrateConfig) -> Option<String> {
    // Member dir = the LONGEST workspace-member prefix of root_path (the manifest dir).
    let mut member: Option<&str> = None;
    for m in &config.workspace_members {
        let m = m.trim_end_matches('/');
        let prefix = format!("{m}/");
        if root_path.starts_with(&prefix) && member.map(|b| m.len() > b.len()).unwrap_or(true) {
            member = Some(m);
        }
    }
    match member {
        // A matched member's library root is EXACTLY `<m>/src/lib.rs` or its explicit
        // [lib].path; anything else under `<m>/` (e.g. `<m>/tools/src/lib.rs`) is a
        // bin/test/example root → not a dependency target.
        Some(m) => {
            if config.lib_path.as_deref() == Some(root_path)
                || root_path == format!("{m}/src/lib.rs")
            {
                Some(m.to_string())
            } else {
                None
            }
        }
        // No workspace member matched: the single crate at the repo root (member `""`).
        None => {
            if root_path == "src/lib.rs" || config.lib_path.as_deref() == Some(root_path) {
                Some(String::new())
            } else {
                None
            }
        }
    }
}
```

### Step 4: Run the tests to verify they pass (GREEN)

```bash
cargo test --lib name_resolution::rust_populator::builder::tests:: 2>&1 | tail -15
cargo test --lib name_resolution::rust_populator::tests:: 2>&1 | tail -10
```

Expected: all SIX new builder tests GREEN (`crate_deps_by_root_maps_consumer_to_target_lib_root`, `lib_plus_bin_member_does_not_self_collide`, the three re-review library-root tests `explicit_lib_path_target_keys_by_member_dir_not_file_parent` + `nested_custom_lib_path_target_keys_by_member_dir` + `custom_bin_path_ending_src_lib_rs_does_not_shadow_library_root` (round-3 MAJOR), and `normalize_crate_ident_hyphen_to_underscore`); the existing `rust_populator::tests::` (edition propagation, convention roots) still pass.

### Step 5: Format + commit

```bash
cargo fmt && cargo fmt --check
```

**Host commits** (stage exactly these two files):

```bash
git add src/name_resolution/rust_policy.rs src/name_resolution/rust_populator/builder.rs
```

Commit message:

```
feat(name-res): build crate_deps_by_root from per-member deps (library roots)

Record each LIBRARY root's member dir -> Root scope at create_root
(`lib_root_member_dir` gates on `[lib].path` / `.../src/lib.rs`, so a bin/test
root is never a dependency target -> lib+bin no-self-collide), then at finish()
map each consuming member's recorded `(in_source_name -> target_dir)` to the
target's library Root via `crate_deps_by_root[consuming_root][normalize(name)]`.
The index is keyed by the MEMBER DIR derived from config.workspace_members (the
manifest-dir spelling member_in_repo_deps uses), NOT the library file's parent,
so an explicit/nested `[lib].path` (root file `<member>/src/lib.rs` or
`<member>/src/inner/lib.rs`) still keys by `<member>`. Add `normalize_crate_ident`
(hyphen->underscore) shared with the resolver hook. `crate_roots_by_name` and the
`extern crate` path are untouched.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
```

---

## Task 4a: The `ResolutionPolicy::extern_crate_root` hook + `crate_root_of` (policy-level)

Add the trait hook (default `None`) and the `RustPolicy` implementation gated on edition + anchor kind + the consuming crate's dep map. Add the `crate_root_of` ascent helper. Unit-tested by calling the policy directly.

**Files:**
- Modify: `src/name_resolution/types.rs` (add `extern_crate_root` to the `ResolutionPolicy` trait `:542-596`).
- Modify: `src/name_resolution/rust_policy.rs` (impl `extern_crate_root` on `RustPolicy`; add the free `crate_root_of`).
- Test: `src/name_resolution/rust_policy.rs` (a new `#[cfg(test)] mod tests`; the file currently has none).

### Step 1: Write the failing tests (RED)

Add a `mod tests` at the end of `src/name_resolution/rust_policy.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::name_resolution::graph::ScopeGraph;
    use crate::name_resolution::types::{
        Anchor, ResolutionPolicy, Scope, ScopeExtent, ScopeId, ScopeKind, SourceLoc, Span,
    };

    fn root_scope(id: u32) -> Scope {
        Scope {
            id: ScopeId(id),
            kind: ScopeKind::Root,
            parent: None,
            extents: vec![ScopeExtent {
                file: crate::name_resolution::types::FileId(id),
                range: Span {
                    lo: SourceLoc {
                        file: crate::name_resolution::types::FileId(id),
                        byte: 0,
                    },
                    hi: SourceLoc {
                        file: crate::name_resolution::types::FileId(id),
                        byte: 10,
                    },
                },
                cond: None,
                occ: None,
            }],
            owner_item: None,
            cond: None,
        }
    }

    fn module_under(id: u32, parent: u32) -> Scope {
        let mut s = root_scope(id);
        s.kind = ScopeKind::Module;
        s.parent = Some(ScopeId(parent));
        s
    }

    /// A graph: Root(0) [crate a] with module(2); Root(1) [crate b]. a depends on b.
    fn two_crate_graph() -> ScopeGraph {
        let mut g = ScopeGraph::new();
        g.edition = 2021;
        g.add_scope(root_scope(0));
        g.add_scope(root_scope(1));
        g.add_scope(module_under(2, 0));
        let mut a_deps = std::collections::BTreeMap::new();
        a_deps.insert("b_crate".to_string(), ScopeId(1));
        g.crate_deps_by_root.insert(ScopeId(0), a_deps);
        g
    }

    #[test]
    fn extern_crate_root_resolves_declared_dep_from_use_path() {
        let g = two_crate_graph();
        let policy = RustPolicy::new(&g, 2021);
        // From module(2) inside crate a, a UsePath leading `b_crate` -> b's Root(1).
        let got = policy.extern_crate_root(&g, "b_crate", &Anchor::use_path_2015(), ScopeId(2));
        assert_eq!(got, Some(ScopeId(1)));
    }

    #[test]
    fn extern_crate_root_declines_2015() {
        let g = two_crate_graph();
        let policy = RustPolicy::new(&g, 2015); // 2015: the bare fallback must not fire.
        assert_eq!(
            policy.extern_crate_root(&g, "b_crate", &Anchor::use_path_2015(), ScopeId(2)),
            None
        );
    }

    #[test]
    fn extern_crate_root_declines_crate_self_super_anchors() {
        let g = two_crate_graph();
        let policy = RustPolicy::new(&g, 2021);
        for anchor in [Anchor::crate_root(), Anchor::self_mod(), Anchor::super_n(1)] {
            assert_eq!(
                policy.extern_crate_root(&g, "b_crate", &anchor, ScopeId(2)),
                None,
                "crate::/self::/super:: anchor inside THIS crate, not a sibling"
            );
        }
    }

    #[test]
    fn extern_crate_root_declines_leading_colon() {
        let g = two_crate_graph();
        let policy = RustPolicy::new(&g, 2021);
        assert_eq!(
            policy.extern_crate_root(&g, "b_crate", &Anchor::leading_colon_2018(99), ScopeId(2)),
            None,
            "LeadingColon is excluded in v1 (spec §8)"
        );
    }

    #[test]
    fn extern_crate_root_declines_undeclared_name() {
        let g = two_crate_graph();
        let policy = RustPolicy::new(&g, 2021);
        // `other` is not in a's dep map → decline (per-crate dep gate, P3).
        assert_eq!(
            policy.extern_crate_root(&g, "other", &Anchor::use_path_2015(), ScopeId(2)),
            None
        );
    }

    #[test]
    fn extern_crate_root_is_per_consuming_crate() {
        let g = two_crate_graph();
        let policy = RustPolicy::new(&g, 2021);
        // From crate b's Root(1) there is no dep map entry → `b_crate` declines.
        assert_eq!(
            policy.extern_crate_root(&g, "b_crate", &Anchor::use_path_2015(), ScopeId(1)),
            None,
            "the extern prelude is per-crate; b does not declare b_crate"
        );
    }

    #[test]
    fn extern_crate_root_normalizes_hyphen() {
        let mut g = two_crate_graph();
        // a depends on a hyphenated in-repo crate keyed underscore in the map.
        g.crate_deps_by_root
            .get_mut(&ScopeId(0))
            .unwrap()
            .insert("my_dep".to_string(), ScopeId(1));
        let policy = RustPolicy::new(&g, 2021);
        // A `use my-dep::X` writes `my_dep`; either spelling normalizes to the key.
        assert_eq!(
            policy.extern_crate_root(&g, "my-dep", &Anchor::use_path_2015(), ScopeId(2)),
            Some(ScopeId(1))
        );
    }

    #[test]
    fn crate_root_of_climbs_to_root() {
        let g = two_crate_graph();
        assert_eq!(crate_root_of(&g, ScopeId(2)), Some(ScopeId(0)));
        assert_eq!(crate_root_of(&g, ScopeId(0)), Some(ScopeId(0)));
    }
}
```

### Step 2: Run the tests to verify they fail (RED)

```bash
cargo test --lib name_resolution::rust_policy::tests:: 2>&1 | tail -25
```

Expected (RED): every test fails to **compile** — `no method named extern_crate_root` and `cannot find function crate_root_of`. That compile error is the RED signal.

### Step 3: Implement (minimal)

**3a — the trait method.** In `src/name_resolution/types.rs`, add to the `ResolutionPolicy` trait. Insert after the `inject` default method (after `:595`, before the trait's closing `}` at `:596`):

```rust
    /// A path's leading segment may name a depended-on in-repo crate (Rust 2018+
    /// extern-prelude root). Returns that crate's library `Root` iff the anchor is
    /// an extern-prelude root kind AND the CONSUMING crate (containing `from`)
    /// declares an in-repo dependency under `name`. `from` is needed because the
    /// extern prelude is per-crate. Default: not applicable (returns `None`).
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

(`Anchor`, `ScopeId`, `ScopeGraph` — `Anchor` and `ScopeId` are defined in this file; `ScopeGraph` must be importable. The trait already references types from `graph` via callers; add `use crate::name_resolution::graph::ScopeGraph;` to the top of `types.rs` if not present. Verify: `types.rs` currently has NO `use crate::name_resolution::graph` line — add it. Place it after `use serde::{Deserialize, Serialize};` at `:20`:)

```rust
use crate::name_resolution::graph::ScopeGraph;
```

> **Cycle note (verify, do not assume):** `graph.rs` already `use`s `crate::name_resolution::types::{...}` (`graph.rs:22-23`), so `types.rs` importing `graph::ScopeGraph` forms a module-path cycle that Rust permits (modules, not crates). If the compiler objects, reference it inline as `crate::name_resolution::graph::ScopeGraph` in the signature and drop the `use`. Confirm at Step 4.

**3b — the `RustPolicy` impl + `crate_root_of`.** In `src/name_resolution/rust_policy.rs`, add the impl method inside `impl ResolutionPolicy for RustPolicy<'_>` (after the `anchor` method closes at `:304`, before the impl's closing `}` at `:305`):

```rust
    fn extern_crate_root(
        &self,
        graph: &ScopeGraph,
        name: &str,
        anchor: &Anchor,
        from: ScopeId,
    ) -> Option<ScopeId> {
        // Eligibility: only a 2018+ extern-prelude ROOT may name a sibling crate.
        // `crate::`/`self::`/`super::` anchor inside THIS crate; `LeadingColon`
        // (`::other::X`) is excluded in v1 (spec §8); a 2015 `use sibling::X` needs
        // an `extern crate` binding (modeled at walk/items.rs:160), so the bare
        // fallback must not invent one.
        if !self.is_2018_plus() {
            return None;
        }
        if !matches!(anchor.kind, AnchorKind::UsePath | AnchorKind::Bare) {
            return None;
        }
        // P3 (per-crate dep gate): resolve `name` ONLY through the consuming crate's
        // in-repo dependency map. A crate can name another in-repo crate iff it
        // actually depends on it; each map value is one specific target root.
        let consuming_root = crate_root_of(graph, from)?;
        graph
            .crate_deps_by_root
            .get(&consuming_root)?
            .get(&normalize_crate_ident(name))
            .copied()
    }
```

Add the free `crate_root_of` helper after `normalize_crate_ident` (the function added in Task 3a, at the end of the file):

```rust
/// Climb `graph.scope(id).parent` from `from` to its enclosing `Root` scope and
/// return it. A free helper (the trait hook receives `graph` as a parameter, not
/// `RustPolicy`'s borrowed graph). Returns `None` only for a malformed graph with
/// no Root ancestor.
pub(crate) fn crate_root_of(graph: &ScopeGraph, from: ScopeId) -> Option<ScopeId> {
    let mut cur = Some(from);
    while let Some(id) = cur {
        let s = graph.scope(id)?;
        if matches!(s.kind, ScopeKind::Root) {
            return Some(id);
        }
        cur = s.parent;
    }
    None
}
```

(`AnchorKind`, `ScopeKind`, `Anchor`, `ScopeId`, `ScopeGraph`, `ResolutionPolicy` are all already imported at `rust_policy.rs:31-35`.)

### Step 4: Run the tests to verify they pass (GREEN)

```bash
cargo test --lib name_resolution::rust_policy::tests:: 2>&1 | tail -15
```

Expected: all nine policy/helper tests GREEN. (If the `types.rs` `use` triggered a complaint, the inline-path fallback in 3a's note resolves it; re-run.)

### Step 5: Format + commit

```bash
cargo fmt && cargo fmt --check
```

**Host commits** (stage exactly these two files):

```bash
git add src/name_resolution/types.rs src/name_resolution/rust_policy.rs
```

Commit message:

```
feat(name-res): ResolutionPolicy::extern_crate_root hook + crate_root_of

Add the `extern_crate_root(graph, name, anchor, from)` trait method (default
None) and implement it on RustPolicy: gate on is_2018_plus() + UsePath/Bare
anchor, climb `from` to its enclosing Root via the new free `crate_root_of`, and
resolve `name` ONLY through `crate_deps_by_root[consuming_root]` (normalized
hyphen->underscore). LeadingColon, crate::/self::/super::, 2015, and undeclared
names all decline. Unit-tested by calling the policy directly. Inert until the
engine wires the call site (Task 4b).

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
```

---

## Task 4b: Engine probe + leading-segment fallback wiring (behavior change)

Introduce `scope_member_lookup_probed` returning `(Resolution, rib_present)`, and fire `extern_crate_root` from `resolve_path_guarded` for the leading segment on a TRUE no-rib miss only. This is the first behavior change; resolver unit tests drive it RED→GREEN and the end-to-end recovery follows in Task 5.

**Files:**
- Modify: `src/name_resolution/engine.rs` (`scope_member_lookup` `:375-410` → add a probed variant; thread the fallback into `resolve_path_guarded` `:338-366`).
- Test: `tests/name_resolution/rust_populate_test.rs` (resolver-level behavior tests through `populate_rust` + `resolve_path`). Add a `mod <stem>;` only if a new file is created — here we add to the existing file, no registration change.

> **TDD sequencing note.** Task 4b's resolver tests (Step 1) are written and observed RED **before** the engine change in Step 3. Task 5's end-to-end integration test is the capstone and is written/committed in Task 5; it stays RED until 4b's engine change lands (it is the same behavior surfaced through `load_repo` + `CallGraph`). 4b's resolver tests are the faster RED→GREEN loop for the engine wiring.

### Step 1: Write the failing tests (RED)

The file ALREADY imports everything these tests need at module level (`tests/name_resolution/rust_populate_test.rs:16-25`: `resolve_path`, `RustPolicy`, `NS_TYPE`, `enclosing_scope`, `file_id`, `populate_rust`, `RustCrateConfig`, `Anchor`, `RawPath`, `ResStatus`, `SourceLoc`, `Target`) and provides the `files(&[(path, src)])` map-builder and `byte_of(src, needle)` helpers (`:38`/`:44`). **Do NOT add per-test `use` blocks** (they would shadow the module imports). Add these resolver tests at the **end** of the file, reusing those helpers. They build a graph with an explicit per-member dep map and resolve a leading cross-crate segment via `resolve_path`:

```rust
#[test]
fn cross_crate_use_resolves_when_dep_declared() {
    // Crate a (2021) `use b_crate::Foo`; crate b defines `Foo`. The leading
    // `b_crate` resolves via the dep map to b's Root, then Foo to b's Foo item.
    let a_src = "use b_crate::Foo;\npub fn drive() { Foo::m(); }\n";
    let fs = files(&[
        ("a/src/lib.rs", a_src),
        ("b/src/lib.rs", "pub struct Foo;\nimpl Foo { pub fn m(&self) {} }\n"),
    ]);
    let mut a_deps = BTreeMap::new();
    a_deps.insert("b_crate".to_string(), "b".to_string());
    let mut member_deps = BTreeMap::new();
    member_deps.insert("a".to_string(), a_deps);
    let cfg = RustCrateConfig {
        edition: 2021,
        crate_roots: vec!["a/src/lib.rs".to_string(), "b/src/lib.rs".to_string()],
        workspace_members: vec!["a".to_string(), "b".to_string()],
        member_in_repo_deps: member_deps,
        ..RustCrateConfig::default()
    };
    let graph = populate_rust(&fs, &cfg, None);
    let policy = RustPolicy::new(&graph, 2021);
    // Resolve the `use b_crate::Foo` leading-segment path from a's lib scope.
    let a_file = file_id(&fs, "a/src/lib.rs").unwrap();
    let at_byte = byte_of(a_src, "b_crate");
    let from = enclosing_scope(&graph, a_file, at_byte).expect("a scope at the use site");
    let at = SourceLoc {
        file: a_file,
        byte: at_byte,
    };
    let res = resolve_path(
        &graph,
        &RawPath(vec!["b_crate".to_string(), "Foo".to_string()]),
        NS_TYPE,
        &Anchor::use_path_2015(),
        from,
        NS_TYPE,
        &at,
        &policy,
    );
    assert_eq!(res.status, ResStatus::Resolved, "cross-crate use resolves");
    assert_eq!(res.candidates.len(), 1);
    assert!(
        matches!(res.candidates[0].target, Target::Item { .. }),
        "the full path lands on b's Foo item"
    );
}

#[test]
fn cross_crate_use_declines_when_no_dep() {
    // Crate a does NOT declare a dep on b; the leading `b` segment stays Unresolved.
    let a_src = "pub fn drive() { let _ = 0; }\n";
    let fs = files(&[
        ("a/src/lib.rs", a_src),
        ("b/src/lib.rs", "pub struct Foo;\nimpl Foo { pub fn m(&self) {} }\n"),
    ]);
    let cfg = RustCrateConfig {
        edition: 2021,
        crate_roots: vec!["a/src/lib.rs".to_string(), "b/src/lib.rs".to_string()],
        workspace_members: vec!["a".to_string(), "b".to_string()],
        // member_in_repo_deps deliberately empty (no declared dependency).
        ..RustCrateConfig::default()
    };
    let graph = populate_rust(&fs, &cfg, None);
    let policy = RustPolicy::new(&graph, 2021);
    let a_file = file_id(&fs, "a/src/lib.rs").unwrap();
    let at_byte = byte_of(a_src, "drive");
    let from = enclosing_scope(&graph, a_file, at_byte).expect("a scope");
    let at = SourceLoc {
        file: a_file,
        byte: at_byte,
    };
    let res = resolve_path(
        &graph,
        &RawPath(vec!["b".to_string(), "Foo".to_string()]),
        NS_TYPE,
        &Anchor::use_path_2015(),
        from,
        NS_TYPE,
        &at,
        &policy,
    );
    assert_eq!(
        res.status,
        ResStatus::Unresolved,
        "no declared in-repo dep on b → the fallback declines (keep-all)"
    );
}

#[test]
fn cross_crate_use_local_module_shadows_dep() {
    // Crate a has a LOCAL `mod b_crate;` (with a `Foo`) AND declares a dep on a
    // sibling `b_crate`. The local module rib must win (P2) — never the crate root.
    //
    // MAJOR 1 (spec-review): the anchor MUST be `use_path_2015()` (a `UsePath` anchor),
    // NOT `self_mod()`. The crate fallback's eligibility gate is
    // `matches!(anchor.kind, UsePath | Bare)`, so a `SelfMod` anchor makes the fallback
    // INELIGIBLE — the test would pass vacuously (the fallback never even fires) and
    // would prove nothing about shadowing. With `use_path_2015()` + a 2021 policy the
    // fallback IS eligible, so this genuinely proves the local `mod b_crate` rib
    // shadows the cross-crate fallback. Under 2018+, `RustPolicy::anchor(UsePath, from)`
    // anchors at `enclosing_module(from)` (`rust_policy.rs` UsePath branch) — for a
    // `from` in the top-level `drive` body that enclosing module IS the crate root,
    // where `mod b_crate;` is declared, so `scope_member_lookup` finds the local rib
    // (rib_present == true) BEFORE the fallback and the fallback declines.
    let a_src = "mod b_crate;\npub fn drive() { let _ = 0; }\n";
    let fs = files(&[
        ("a/src/lib.rs", a_src),
        ("a/src/b_crate.rs", "pub struct Foo;\nimpl Foo { pub fn m(&self) {} }\n"),
        ("b/src/lib.rs", "pub struct Foo;\nimpl Foo { pub fn m(&self) {} }\n"),
    ]);
    let mut a_deps = BTreeMap::new();
    a_deps.insert("b_crate".to_string(), "b".to_string());
    let mut member_deps = BTreeMap::new();
    member_deps.insert("a".to_string(), a_deps);
    let cfg = RustCrateConfig {
        edition: 2021,
        crate_roots: vec!["a/src/lib.rs".to_string(), "b/src/lib.rs".to_string()],
        workspace_members: vec!["a".to_string(), "b".to_string()],
        member_in_repo_deps: member_deps,
        ..RustCrateConfig::default()
    };
    let graph = populate_rust(&fs, &cfg, None);
    let policy = RustPolicy::new(&graph, 2021);
    let a_file = file_id(&fs, "a/src/lib.rs").unwrap();
    // Resolve from inside crate a's Root scope (the `drive` body byte). Under
    // `use_path_2015` + 2021 the anchor is the enclosing module (= the crate root),
    // where the local `mod b_crate;` rib lives.
    let at_byte = byte_of(a_src, "drive");
    let from = enclosing_scope(&graph, a_file, at_byte).expect("a scope");
    let at = SourceLoc {
        file: a_file,
        byte: at_byte,
    };
    let res = resolve_path(
        &graph,
        &RawPath(vec!["b_crate".to_string(), "Foo".to_string()]),
        NS_TYPE,
        &Anchor::use_path_2015(), // UsePath: the fallback IS eligible, so shadowing is real
        from,
        NS_TYPE,
        &at,
        &policy,
    );
    // The local `mod b_crate` (a's own module) resolves; it is NOT b's crate root.
    // An out-of-line `mod foo;` binds to `Target::Scope` (builder `scope_target`,
    // `builder.rs:513`), NOT a `Target::Item` — so match the Scope and check its
    // file is the LOCAL module file, never b's crate root file (the fallback would
    // have produced a Scope at `b/src/lib.rs`).
    assert_eq!(res.status, ResStatus::Resolved);
    assert_eq!(res.candidates.len(), 1);
    if let Target::Scope(scope) = res.candidates[0].target {
        let module_file = graph.scope(scope).unwrap().extents[0].file;
        assert_eq!(
            module_file,
            file_id(&fs, "a/src/b_crate.rs").unwrap(),
            "the LOCAL module b_crate must win, never b's crate root (P2)"
        );
    } else {
        panic!(
            "expected the local module scope, got {:?}",
            res.candidates[0].target
        );
    }
}

#[test]
fn cross_crate_use_claimed_but_invisible_local_blocks_fallback() {
    // MAJOR 2 / spec §6.6 (REQUIRED, the load-bearing `rib_present` proof): crate a has
    // a LOCAL rib binding for `b_crate` that is CLAIMED but NOT visible at the use site,
    // AND declares a dep on a sibling crate `b_crate`. The leading segment must stay the
    // LOCAL outcome (Unresolved via claimed-but-invisible — rib_present == true), NEVER
    // b's crate root. This is the discriminating case distinct from the visible-rib test
    // above: there the rib resolves; here it is claimed-but-invisible, surfacing as
    // `Unresolved` *with* a rib present — exactly the status the fallback must NOT
    // convert (engine.rs:233-235 / the `!rib_present` guard).
    //
    // Fixture mechanism (verified against source): an inline `pub(in crate::secret) mod
    // b_crate { ... }` mints a NS_TYPE binding for `b_crate` at the crate root scope with
    // vis = (VIS_PUB_IN, restrict). The populator's `resolve_restrict` is a Phase-1 stub
    // that ALWAYS returns `None` (`rust_populator/walk/mod.rs:271-277`), so the binding's
    // `restrict` is `None`, and `RustPolicy::visible` returns `false` for VIS_PUB_IN with
    // no restrict ("pub(in) with no recorded path → fall through", `rust_policy.rs`). The
    // binding still EXISTS, so `scope_member_lookup_probed` reports `rib_present == true`
    // while `resolve_rib` returns `Unresolved` (claimed-but-invisible) — a robust
    // claimed-but-invisible rib that does not depend on where `from` sits. `secret` need
    // not exist (the stub ignores the path).
    let a_src = concat!(
        "pub(in crate::secret) mod b_crate { pub struct Foo; impl Foo { pub fn m(&self) {} } }\n",
        "pub fn drive() { let _ = 0; }\n",
    );
    let fs = files(&[
        ("a/src/lib.rs", a_src),
        ("b/src/lib.rs", "pub struct Foo;\nimpl Foo { pub fn m(&self) {} }\n"),
    ]);
    let mut a_deps = BTreeMap::new();
    a_deps.insert("b_crate".to_string(), "b".to_string());
    let mut member_deps = BTreeMap::new();
    member_deps.insert("a".to_string(), a_deps);
    let cfg = RustCrateConfig {
        edition: 2021,
        crate_roots: vec!["a/src/lib.rs".to_string(), "b/src/lib.rs".to_string()],
        workspace_members: vec!["a".to_string(), "b".to_string()],
        member_in_repo_deps: member_deps,
        ..RustCrateConfig::default()
    };
    let graph = populate_rust(&fs, &cfg, None);
    let policy = RustPolicy::new(&graph, 2021);
    let a_file = file_id(&fs, "a/src/lib.rs").unwrap();
    let at_byte = byte_of(a_src, "drive");
    let from = enclosing_scope(&graph, a_file, at_byte).expect("a scope");
    let at = SourceLoc {
        file: a_file,
        byte: at_byte,
    };
    let res = resolve_path(
        &graph,
        &RawPath(vec!["b_crate".to_string(), "Foo".to_string()]),
        NS_TYPE,
        &Anchor::use_path_2015(), // UsePath: the fallback WOULD be eligible by anchor/edition
        from,
        NS_TYPE,
        &at,
        &policy,
    );
    // The claimed-but-invisible local rib (rib_present == true) blocks the fallback, so
    // the segment stays the LOCAL Unresolved outcome — it must NOT become b's `Foo` item.
    assert_ne!(
        res.status,
        ResStatus::Resolved,
        "a claimed-but-invisible local rib must block the crate fallback (rib_present), \
         got {:?} ({:?})",
        res.status,
        res.candidates
    );
    assert!(
        !res
            .candidates
            .iter()
            .any(|c| matches!(c.target, Target::Item { .. })),
        "the fallback must not resolve into b's crate (no b::Foo item), got {:?}",
        res.candidates
    );
}

#[test]
fn cross_crate_use_external_same_name_declines_with_in_repo_member() {
    // MAJOR 3 / spec §6.7 (the key soundness case, end-to-end at the resolver): crate a
    // depends on an EXTERNAL `b` (version-only) while an UNRELATED in-repo workspace
    // member `b` ALSO exists (a real crate Root in the graph). `use b::X` from a must
    // DECLINE (stay Unresolved) — the external `b` is NOT in a's in-repo dep map, so the
    // per-crate dep gate (P3) returns None even though an in-repo crate named `b` has a
    // Root. Modeled at the resolver: `b` IS a crate root in `crate_roots` (so it has a
    // Root scope), but `a`'s `member_in_repo_deps` does NOT record `b` (an external
    // version-only dep is never captured — see Task 2 `external_version_dep_is_not_
    // recorded`). The fallback must not invent a resolution to the in-repo `b` root.
    let a_src = "use b::Thing;\npub fn drive() { let _ = 0; }\n";
    let fs = files(&[
        ("a/src/lib.rs", a_src),
        ("b/src/lib.rs", "pub struct Thing;\nimpl Thing { pub fn m(&self) {} }\n"),
    ]);
    // a declares NO in-repo dep (its real `b = "1.0"` is external → not recorded). The
    // in-repo member `b` still has a crate Root from `crate_roots`.
    let cfg = RustCrateConfig {
        edition: 2021,
        crate_roots: vec!["a/src/lib.rs".to_string(), "b/src/lib.rs".to_string()],
        workspace_members: vec!["a".to_string(), "b".to_string()],
        // member_in_repo_deps deliberately EMPTY for a (external dep, not in-repo).
        ..RustCrateConfig::default()
    };
    let graph = populate_rust(&fs, &cfg, None);
    let policy = RustPolicy::new(&graph, 2021);
    let a_file = file_id(&fs, "a/src/lib.rs").unwrap();
    let at_byte = byte_of(a_src, "b::Thing");
    let from = enclosing_scope(&graph, a_file, at_byte).expect("a scope");
    let at = SourceLoc {
        file: a_file,
        byte: at_byte,
    };
    let res = resolve_path(
        &graph,
        &RawPath(vec!["b".to_string(), "Thing".to_string()]),
        NS_TYPE,
        &Anchor::use_path_2015(),
        from,
        NS_TYPE,
        &at,
        &policy,
    );
    assert_eq!(
        res.status,
        ResStatus::Unresolved,
        "an external `b` (not in a's in-repo dep map) must DECLINE even though an \
         in-repo member `b` exists (per-crate dep gate, P3); got {:?} ({:?})",
        res.status,
        res.candidates
    );
}
```

> **Helper note:** `BTreeMap` is in scope (`rust_populate_test.rs` uses it in `files()`); `files(&[...])`, `byte_of(...)`, `populate_rust`, `RustPolicy`, `NS_TYPE`, `enclosing_scope`, `file_id`, `RustCrateConfig`, `Anchor`, `RawPath`, `ResStatus`, `SourceLoc`, `Target` are all already imported/defined at the top of that file — no new `use` line is needed.

### Step 2: Run the tests to verify they fail (RED)

```bash
cargo test --test name_resolution rust_populate_test::cross_crate_use_resolves_when_dep_declared -- --exact 2>&1 | tail -15
cargo test --test name_resolution rust_populate_test::cross_crate_use -- 2>&1 | tail -20
```

Expected (RED): `cross_crate_use_resolves_when_dep_declared` is the only RED test — it fails (`Unresolved`, not `Resolved` — the leading `b_crate` segment misses with no fallback yet). The other four are **guards that must STAY green pre-change** (they pin behavior the engine change must preserve):
- `cross_crate_use_declines_when_no_dep` — no fallback exists yet → already `Unresolved`.
- `cross_crate_use_local_module_shadows_dep` — the visible local `mod b_crate` rib already resolves (UsePath anchors at the enclosing module = crate root); post-change the eligible fallback must still be shadowed.
- `cross_crate_use_claimed_but_invisible_local_blocks_fallback` (MAJOR 2) — the claimed-but-invisible rib yields `Unresolved` pre-change (no fallback). **This is the wrong-trigger regression guard:** it stays green ONLY if Step 3's fallback gates on `!rib_present && Unresolved`; a naive `status == Unresolved`-only trigger would convert this claimed-but-invisible `Unresolved` into b's `Foo` item and FLIP it RED. So it must be observed green before AND after the engine change (it would only go red under the buggy trigger).
- `cross_crate_use_external_same_name_declines_with_in_repo_member` (MAJOR 3) — `b` is not in a's in-repo dep map → declines pre- and post-change.

### Step 3: Implement the engine wiring (minimal)

In `src/name_resolution/engine.rs`, **add a probed lookup** that returns the rib-claim boolean alongside the same `Resolution`. Replace `scope_member_lookup` (`:375-410`) with a thin wrapper over a new `scope_member_lookup_probed`:

```rust
/// Look up `(name, ns)` **within a single scope** (a path segment): explicit
/// bindings first (visibility-enforced, Pending chased), then this scope's
/// non-deferred globs. NO lexical fall-out (an anchored member lookup never
/// walks to a parent — that is what keeps a sibling-private decoy from reaching
/// an outer same-name).
fn scope_member_lookup(
    graph: &ScopeGraph,
    scope: ScopeId,
    q: &ResolveQuery,
    policy: &dyn ResolutionPolicy,
    guard: &mut CycleGuard,
) -> Resolution {
    scope_member_lookup_probed(graph, scope, q, policy, guard).0
}

/// As [`scope_member_lookup`], but ALSO reports whether an explicit rib binding
/// for `(name, ns)` was CLAIMED in this scope (regardless of visibility/outcome).
/// The boolean lets `resolve_path_guarded` distinguish a TRUE no-rib miss (where
/// the crate-root fallback may fire) from a claimed-but-invisible local rib (which
/// surfaces as `Unresolved` but must shadow the crate name — P2/BLOCKER 1). It does
/// NOT change the `Resolution` returned.
fn scope_member_lookup_probed(
    graph: &ScopeGraph,
    scope: ScopeId,
    q: &ResolveQuery,
    policy: &dyn ResolutionPolicy,
    guard: &mut CycleGuard,
) -> (Resolution, bool) {
    // Explicit bindings in this scope for (name, ns), cfg-compatible. For path
    // member lookup we do NOT gate on vis_extents byte-range (a module member is
    // visible across the whole module to a path); visibility is the `visible()`
    // policy hook. An explicit member binding SHADOWS a covering macro wildcard
    // (the wildcard is glob-tier — §4.3b Reading B), so it is checked AFTER this
    // rib, not before.
    let rib: Vec<usize> = graph
        .bindings
        .iter()
        .enumerate()
        .filter(|(_, b)| b.scope == scope && b.name == q.name && b.ns == q.ns)
        .filter(|(_, b)| cfg_compatible(&b.cond))
        .map(|(i, _)| i)
        .collect();
    let rib_present = !rib.is_empty();
    if rib_present {
        return (resolve_rib(graph, &rib, q, policy, guard), true);
    }
    // Glob tier: a covering macro wildcard poisons here (exactly like a deferred
    // glob), reached only when the rib above found no explicit member binding.
    if macro_wildcard_poisons(graph, scope, q.ns, &q.at) {
        return (poisoned(), false);
    }
    // Else this scope's globs.
    let res = match glob_lookup(graph, scope, q, policy) {
        GlobOutcome::Poison => poisoned(),
        GlobOutcome::Hit(cands) => policy.combine(cands),
        GlobOutcome::Empty => unresolved(),
    };
    (res, false)
}
```

Then **fire the fallback in `resolve_path_guarded`** for the leading segment. In the prefix-walk loop (`:338-366`), replace the body from the `let res = scope_member_lookup(...)` line (`:352`) through the `if is_last { return res; }` line (`:354`) with:

```rust
        // Single-scope member lookup AND a "was a rib claimed here?" probe (so a
        // claimed-but-invisible local cannot be overridden by the crate fallback).
        let (res, rib_present) = scope_member_lookup_probed(graph, scope, &seg_q, policy, guard);
        // Leading-segment crate-root fallback (strictly last; P2/P3): a 2018+
        // extern-prelude root resolves to the owning crate's Root scope ONLY on a
        // TRUE no-rib miss — no rib was claimed for the segment (so a local item,
        // even a deliberately-invisible one, always shadows — P2) — AND the policy's
        // anchor/edition gate + the consuming crate's per-crate in-repo dependency
        // gate pass (P3). Poison/empty-glob `Unresolved` WITH a claimed rib does not
        // qualify. `from` (the query origin) is threaded so the policy can identify
        // the consuming crate.
        let res = if i == 0 && !rib_present && matches!(res.status, ResStatus::Unresolved) {
            match policy.extern_crate_root(graph, seg, anchor, from) {
                Some(root) => Resolution {
                    candidates: vec![Candidate {
                        target: Target::Scope(root),
                        cond: CfgCond::True,
                        provenance: Default::default(),
                    }],
                    status: ResStatus::Resolved,
                },
                None => res,
            }
        } else {
            res
        };
        if is_last {
            return res;
        }
```

(`Candidate`, `Target`, `Resolution`, `ResStatus`, `CfgCond` are all already imported at `engine.rs:27-31`. `anchor` and `from` are parameters of `resolve_path_guarded` already in scope. `seg` is the loop variable from `segs.iter()` where `segs = &path.0` and `RawPath(Vec<String>)`, so `seg: &String`; `extern_crate_root` takes `&str`, so pass `seg` directly — `&String` coerces to `&str`.)

### Step 4: Run the tests to verify they pass (GREEN)

```bash
cargo test --test name_resolution rust_populate_test::cross_crate_use -- 2>&1 | tail -20
cargo test --lib name_resolution:: 2>&1 | tail -10
cargo test --test name_resolution 2>&1 | tail -10
```

Expected: all FIVE `cross_crate_use_*` tests GREEN — `cross_crate_use_resolves_when_dep_declared` (now resolves), `cross_crate_use_declines_when_no_dep` (still declines), `cross_crate_use_local_module_shadows_dep` (visible local rib still shadows the now-eligible fallback), `cross_crate_use_claimed_but_invisible_local_blocks_fallback` (the `!rib_present` guard keeps the claimed-but-invisible rib shadowing — MAJOR 2), and `cross_crate_use_external_same_name_declines_with_in_repo_member` (external `b` declines despite an in-repo `b` — MAJOR 3). The whole `name_resolution` suite and the engine `--lib` tests still pass (the probe is behavior-preserving for `scope_member_lookup`; the fallback only adds resolutions on a TRUE no-rib miss `!rib_present && Unresolved`).

### Step 5: Format + commit

```bash
cargo fmt && cargo fmt --check
```

**Host commits** (stage exactly these two files):

```bash
git add src/name_resolution/engine.rs tests/name_resolution/rust_populate_test.rs
```

Commit message:

```
feat(name-res): leading-segment cross-crate fallback in resolve_path

Add `scope_member_lookup_probed` returning (Resolution, rib_present) and fire
`policy.extern_crate_root` from resolve_path_guarded for the leading segment
(i==0) ONLY on a TRUE no-rib miss (!rib_present && Unresolved). A
claimed-but-invisible local rib (rib_present) keeps shadowing the crate name
(P2); poison/empty-glob is unchanged. On Some(root) the segment resolves to a
single scope-bearing Root candidate and the walk continues into the crate. Resolver
tests pin: resolves when the dep is declared, declines with no dep, a visible
local module shadows the dep, a claimed-but-invisible local rib still blocks the
fallback (the !rib_present guard — the wrong-trigger regression), and an external
same-name dep declines even when an in-repo member of that name exists.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
```

---

## Task 5: End-to-end collision recovery (integration)

A pure-2018+ workspace where crate `a` declares a path dep on crate `b`, with a same-name owner-`::` collision pinned by `use b_crate::Foo;`, built through the real `load_repo` + `CallGraph::build_with_scope_graph_inputs`. The capstone: pre-fix keep-all (≥2 NameOnly), post-fix one Exact via #122 (A).

> **TDD sequencing.** This test is the behavior capstone for Task 4b. Per the discipline, write and observe it RED **before** Task 4b's engine change, OR (chosen here, for a cleaner host-commit grouping) write it now as a single GREEN-verifying commit *after* 4b — but FIRST run it against the pre-4b binary to confirm it would have been RED. The Step-2 command does exactly that observation. If executing strictly task-by-task with 4b already merged, the RED was already demonstrated by 4b's resolver tests (`cross_crate_use_resolves_when_dep_declared`), which exercise the identical engine path; this test adds the load_repo + disproof end-to-end coverage.

**Files:**
- Test: `tests/integration/resolution_test.rs` (one end-to-end test, alongside the existing `scope_graph_two_crate_owner_collision_recovers_to_single_exact` `:1692`).

### Step 1: Write the test

Add at the end of `tests/integration/resolution_test.rs` (model: the existing two-crate recovery test `:1692` + the edition plan's `load_repo` end-to-end test):

```rust
#[test]
fn cross_crate_use_collision_recovers_to_single_exact() {
    use prism::call_graph::CallGraph;
    use prism::repo_loader::load_repo;
    // A pure-2018+ workspace driven end-to-end through the real loader. Crate `a`
    // (2021) declares a PATH dep on crate `b` and pins a same-name `Foo` collision
    // with `use b_crate::Foo;` (b's crate is the in-source name `b_crate`). Both
    // crate `a` and crate `b` define a `Foo` with method `m`, so the bare owner key
    // ("Foo","m") collides. Pre-fix: the leading `b_crate` segment is Unresolved ->
    // the (A) disproof's pending re-resolve declines -> keep-all (2 NameOnly).
    // Post-fix: `b_crate` resolves via crate_deps_by_root to b's lib Root -> Foo to
    // b's Foo (one in-repo item) -> the disproof prunes to one Exact.
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
        "[package]\nname = \"a\"\nedition = \"2021\"\n[dependencies]\nb_crate = { path = \"../b\" }\n",
    )
    .unwrap();
    std::fs::write(
        p.join("b/Cargo.toml"),
        "[package]\nname = \"b\"\nedition = \"2021\"\n",
    )
    .unwrap();
    // Crate a: import b's Foo and call Foo::m. Crate a ALSO defines its own Foo so
    // the bare ("Foo","m") owner key collides across the two crates.
    std::fs::write(
        p.join("a/src/lib.rs"),
        "use b_crate::Foo;\npub struct LocalFoo;\nimpl LocalFoo { pub fn m(&self) {} }\npub fn drive() {\n    Foo::m();\n}\n",
    )
    .unwrap();
    std::fs::write(
        p.join("b/src/lib.rs"),
        "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
    )
    .unwrap();
    // A second crate `c` whose Foo::m collides on the bare owner key with b's, so
    // the ("Foo","m") key holds >=2 defs and the floor is NameOnly (not Exact)
    // until the disproof prunes.
    std::fs::create_dir_all(p.join("c/src")).unwrap();
    std::fs::write(
        p.join("Cargo.toml"),
        "[workspace]\nmembers = [\"a\", \"b\", \"c\"]\n",
    )
    .unwrap();
    std::fs::write(
        p.join("c/Cargo.toml"),
        "[package]\nname = \"c\"\nedition = \"2021\"\n",
    )
    .unwrap();
    std::fs::write(
        p.join("c/src/lib.rs"),
        "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
    )
    .unwrap();

    let repo = load_repo(p).unwrap();
    let inputs = repo
        .scope_graph_inputs
        .as_ref()
        .expect("scope graph inputs");
    let cg = CallGraph::build_with_scope_graph_inputs(&repo.files, Some(inputs));
    assert!(
        cg.methods
            .get(&("Foo".to_string(), "m".to_string()))
            .map(|v| v.len())
            .unwrap_or(0)
            >= 2,
        "the bare owner key must collide across b::Foo and c::Foo (NameOnly floor)"
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Foo::m"));
    assert_eq!(out.drop, None, "the recovered cross-crate owner path must not drop");
    assert_eq!(
        out.resolved.len(),
        1,
        "the cross-crate `use b_crate::Foo` now recovers the collision to one Exact"
    );
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.resolved[0].target.file, "b/src/lib.rs");
}
```

### Step 2: Observe RED-then-GREEN

Confirm the test would be RED on the pre-Task-4b engine, then GREEN now:

```bash
# Demonstrate the pre-fix behavior (run BEFORE Task 4b's engine change if sequencing
# strictly; if 4b is already merged, this command instead confirms the post-fix GREEN
# and the resolver-level RED was already shown by 4b's cross_crate_use_resolves test):
cargo test --test integration resolution_test::cross_crate_use_collision_recovers_to_single_exact -- --exact 2>&1 | tail -15
```

Expected pre-4b (RED): `out.resolved.len()` is `2` (or `3`) NameOnly — keep-all (the leading `b_crate` is `Unresolved`, the (A) disproof declines). Expected post-4b (GREEN): `1 passed` — one Exact pointing at `b/src/lib.rs`.

### Step 3: Run the full integration suite

```bash
cargo test --test integration resolution_test:: 2>&1 | tail -10
```

Expected: the new test passes and the existing recovery/keep-all tests (`scope_graph_two_crate_owner_collision_recovers_to_single_exact`, `scope_graph_inherent_plus_trait_owner_demotes_not_drops`, `scope_graph_unresolved_owner_path_keeps_full_pool_not_drop`, `scope_graph_block_local_glob_shadow_keeps_all`) all still pass — the cross-crate fallback must not perturb the intra-crate or keep-all paths.

### Step 4: Format + commit

```bash
cargo fmt && cargo fmt --check
```

**Host commits** (stage exactly this one file):

```bash
git add tests/integration/resolution_test.rs
```

Commit message:

```
test(integration): cross-crate use collision recovers to one Exact (e2e)

End-to-end through load_repo + CallGraph::build_with_scope_graph_inputs: a pure-
2018+ workspace where crate `a` declares a path dep on `b` and pins a same-name
`Foo` owner collision via `use b_crate::Foo;`. The bare ("Foo","m") key collides
across b and c (NameOnly floor); the cross-crate leading-segment fallback lets
the #122 (A) disproof resolve Foo to b's single in-repo item and prune to one
Exact. Pins the headline recovery.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
```

---

## Task 6: Acceptance (non-code) — spec §7

Build, run the full test surface, format-check, and run the Tier-A recall gate + the ruff M2 recovery acceptance + the call-stats delta + a nav spot-check. **No commit** (verification only); record realized deltas in the PR. **Do not** stage `eval/` or `docs/eval/`.

**Files:** none.

### Step 1: Full test surface (macOS-correct)

```bash
cargo build --release 2>&1 | tail -3
cargo test --lib 2>&1 | tail -5
cargo test --test integration 2>&1 | tail -5
cargo test --test ast 2>&1 | tail -5
cargo test --test name_resolution 2>&1 | tail -5
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

Expected: 0 regression (no `ok`→`gap` flips vs the committed baseline). `--allow-stale-sut` is valid only with the immediately preceding `cargo build --release` (Step 1) in this worktree.

### Step 3: ruff M2 recovery acceptance (the real recall/precision gate — spec §5/§7)

`--quick` forces prism-only and the harness's `--corpus ruff --quick` runs prism not ruff (known harness bug), so use `--corpus ruff` WITHOUT `--quick`:

```bash
cd eval && uv run tier-a --corpus ruff --allow-stale-sut 2>&1 | tail -10
# read the run report (paths are relative to eval/ after the cd above):
RUFF_RUN=runs/$(ls -t runs | grep ruff | head -1)
grep -nE '"baseline_invalid"|"oracle_error_rate"|"sut_error_rate"' "$RUFF_RUN"
grep -c '"outcome": "regression"' "$RUFF_RUN"
```

Expected: `baseline_invalid == false`, oracle/sut error 0.0, **0 regression**.

### Step 4: Recovery delta — `call-stats` on ruff (report the +428)

```bash
./target/release/prism nav --no-cache call-stats --repo ~/code/bench-repos/ruff > /tmp/cs-ruff-after.json 2>/dev/null
grep -A7 '"recovery_typepath"' /tmp/cs-ruff-after.json | grep -E 'singleton|failopen_demote'
grep -E '"multi_target_exact_sites"' /tmp/cs-ruff-after.json
```

Expected (vs the pre-change `main` baseline, which the executor measures first by running the same command on `main`): `recovery_typepath.singleton` up ≈ **+428** (the spec's roadmap target; the post-#123 starting point is singleton ≈ 260, failopen_demote ≈ 1326), `failopen_demote` down ≈ **−428**, `multi_target_exact_sites` **unchanged** (no new collision FPs). Report the measured pair in the PR.

> **Pre-change baseline (run once, on `main`, before this branch's work — or `git stash`):**
> ```bash
> ./target/release/prism nav --no-cache call-stats --repo ~/code/bench-repos/ruff > /tmp/cs-ruff-before.json 2>/dev/null
> grep -A7 '"recovery_typepath"' /tmp/cs-ruff-before.json | grep -E 'singleton|failopen_demote'
> ```
> Report the before/after pair (the delta is the buy).

### Step 5: nav spot-check (precision audit — this touches general resolution, not only the disproof)

```bash
# Sample a few of the newly-Exact cross-crate edges in ruff and confirm they point
# at the correct sibling-crate definition (the collision counters alone don't cover
# a precision shift). Pick a couple of crate-qualified call sites and inspect callees:
./target/release/prism nav --no-cache callees --repo ~/code/bench-repos/ruff --symbol <a-cross-crate-caller> --format json 2>/dev/null | head -40
```

Expected: the sampled newly-Exact edges target the correct sibling-crate definition (spot-audit a handful). Record the sample in the PR.

> **Do not** stage `eval/` or `docs/eval/` artifacts.

### Step 6: Final review

Request an independent codex (gpt-5.5, xhigh) diff review of the branch. Fold any findings as host-committed fix-ups with the trailer. Headlines to call out: `crate_roots_by_name` + `extern crate` untouched (P5); the fallback is per-consuming-crate dep-gated (P3) and fires only on a TRUE no-rib miss (P2); recall is conditional (P1) on the correct unique resolution; +428 on ruff with `multi_target_exact_sites` unchanged.

---

## Self-review checklist (spec coverage → task)

- **§2.1 persist `crate_deps_by_root` field (`#[serde(default)]`, alongside `file_paths`)** → Task 1 Step 3 (the field on `ScopeGraph`).
- **§2.1 per-member in-repo dependency capture (PATH + WORKSPACE forms; external excluded; rename-by-in-source-name)** → Task 2 (`member_in_repo_deps` + `parse_member_in_repo_deps` + the PER-WORKSPACE-ROOT `[workspace.dependencies]` pre-scan + `owning_workspace_root`/`is_dir_ancestor` + the lexical `normalize_repo_rel` so dep targets match the Builder's member-dir keys). BLOCKER 1 (path normalization) + BLOCKER 2 (per-workspace-root, owning-workspace association) folded.
- **§2.1 `lib_root_by_member_dir` (library roots only, keyed by MEMBER DIR via `config.workspace_members`) + build `crate_deps_by_root` at `finish()` + `normalize_crate_ident`** → Task 3. Re-review BLOCKER B (key by member dir, not the library file's parent — covers explicit + nested `[lib].path`) folded.
- **§2.1 `crate_roots_by_name` / `extern crate` left unchanged** → asserted in Tasks 1/3 commit messages; no edit touches `builder.rs:48`/`:78`/`walk/items.rs:160` behavior (only ADDS the lib-root index + finish-build).
- **§2.2 `ResolutionPolicy::extern_crate_root(graph, name, anchor, from)` (default None) + RustPolicy impl (edition/anchor gate + per-crate dep map) + `crate_root_of`** → Task 4a.
- **§2.2 engine wiring: probe (`scope_member_lookup_probed` / `rib_present`) + leading-segment (`i==0`) fallback on TRUE no-rib miss only (`!rib_present && Unresolved`)** → Task 4b.
- **§2.3 `CACHE_VERSION` 17→18 + pin test** → Task 1.
- **§3 soundness / recall-safety (P1–P4)** → Task 4a tests (2015 / crate::-self::-super:: / LeadingColon / undeclared / per-consuming-crate / normalize decline cases) + Task 4b tests (resolves-when-declared / declines-no-dep / visible-local-module-shadow / claimed-but-invisible-rib-blocks-fallback [MAJOR 2] / external-same-name-declines [MAJOR 3]) + Task 5 e2e.
- **§6 tests (1–13)** → §6.1 round-trip→Task 1.1b; §6.2 path-dep resolve→Task 4b `cross_crate_use_resolves_when_dep_declared`; §6.3 workspace-dep→Task 2 `workspace_dep_records_*` + `multi_workspace_same_dep_name_resolves_per_owning_workspace` (capture) + Task 4b path (resolve); §6.4 renamed dep→Task 4a `extern_crate_root_normalizes_hyphen` + Task 2 rename-by-in-source-name; §6.5 local-shadow→Task 4b `cross_crate_use_local_module_shadows_dep` (uses `use_path_2015` so the fallback is eligible — MAJOR 1); §6.6 claimed-but-invisible→Task 4b **`cross_crate_use_claimed_but_invisible_local_blocks_fallback` (REQUIRED — the `!rib_present` wrong-trigger regression guard via a `pub(in crate::secret) mod`; MAJOR 2)**; §6.7 external-vs-in-repo→Task 2 `external_version_dep_is_not_recorded` (capture) + Task 4b **`cross_crate_use_external_same_name_declines_with_in_repo_member` (end-to-end decline; MAJOR 3)** + Task 4a `extern_crate_root_declines_undeclared_name`; §6.8 non-dependency same-name→Task 4b `cross_crate_use_declines_when_no_dep`; §6.9 anchor-kind gate→Task 4a `extern_crate_root_declines_crate_self_super_anchors` + `_declines_leading_colon`; §6.10 2015→Task 4a `extern_crate_root_declines_2015`; §6.11 lib+bin no-self-poison→Task 3 `lib_plus_bin_member_does_not_self_collide`; §6.12 e2e→Task 5; §6.13 cache pin→Task 1.1a.
- **§5 buy + §7 acceptance** → Task 6 (Tier-A matrix + ruff M2 0-regression + the +428 call-stats delta + nav spot-check + codex review).

**Placeholder scan:** no `TODO`/`...`/`TBD`; every test + edit is complete verbatim code; every command runnable. (The only `<...>` token is the `--symbol <a-cross-crate-caller>` argument in Task 6 Step 5, which is an executor-chosen ruff symbol for the manual spot-audit, not source code.)

**Type/signature consistency (verified against source):**
- `ScopeGraph.crate_deps_by_root: std::collections::BTreeMap<ScopeId, std::collections::BTreeMap<String, ScopeId>>` — same type referenced in graph.rs (Task 1), Builder field + `finish()` (Task 3), the policy hook return key/value (Task 4a), the engine candidate (Task 4b). `ScopeId` is `name_resolution::types::ScopeId`.
- `RustCrateConfig.member_in_repo_deps: BTreeMap<String, BTreeMap<String, String>>` (member dir → in-source name → NORMALIZED in-repo target member dir) — defined Task 2 (struct + Default + from_convention), consumed Task 3 (`finish()`), populated Task 2 (`parse_member_in_repo_deps`). Both keys and values are in the `normalize_repo_rel` manifest-dir spelling so Task 3's `lib_root_by_member_dir.get(..)` lookups hit; an escaping (out-of-repo) target is never recorded (re-review BLOCKER A), so every recorded value is a genuine in-repo member dir.
- `normalize_repo_rel(&str) -> Option<String>` — free fn in `repo_loader.rs` (Task 2 3b); the BLOCKER-1 lexical (pure-string) repo-relative normalizer (pop `..`, drop `.`, no trailing slash) with the re-review **BLOCKER A escape guard**: returns `None` when a `..` pops past the repo root (an out-of-repo dep), so both call sites `if let Some(target) = ...` skip it (never clamp). `Some("")` = the repo root (valid in-repo). Used for BOTH path-dep targets (`normalize_repo_rel(join_manifest_rel(member_dir, dep_path))`) and the per-workspace-root `[workspace.dependencies]` targets. One definition.
- `owning_workspace_root(&str, &BTreeSet<String>) -> Option<&str>` + `is_dir_ancestor(&str, &str) -> bool` — free fns in `repo_loader.rs` (Task 2 3b); BLOCKER-2 owning-workspace association (longest workspace-root-dir ancestor; `""` root owns all; `/`-boundary prefix match). `workspace_dep_paths: BTreeMap<String /*ws root dir*/, BTreeMap<String /*dep name*/, String /*normalized target dir*/>>` and `workspace_root_dirs: BTreeSet<String>` are the per-workspace-root pre-scan structures.
- `normalize_crate_ident(&str) -> String` — `pub(crate)` in `rust_policy.rs` (Task 3a); used by Builder `finish()` (Task 3) and `RustPolicy::extern_crate_root` (Task 4a). One definition. (Distinct from `normalize_repo_rel`: `normalize_crate_ident` is hyphen→underscore for crate IDENTIFIERS; `normalize_repo_rel` is `..`/`.` collapse for repo PATHS.)
- `crate_root_of(&ScopeGraph, ScopeId) -> Option<ScopeId>` — `pub(crate)` free fn in `rust_policy.rs` (Task 4a); used only by `extern_crate_root`. (Distinct from `RustPolicy::crate_root(&self, ScopeId)` at `rust_policy.rs:104`, which uses the policy's borrowed graph; the free helper is needed because the trait hook receives `graph` as a parameter.)
- `lib_root_member_dir(&str, &RustCrateConfig) -> Option<String>` — free fn in `builder.rs` (Task 3e); used by `create_root`. Returns the MEMBER DIR (derived from `config.workspace_members` longest-prefix match) iff `root_path` is that member's library root, gated EXACTLY: for a matched member `m`, accept ONLY `m/src/lib.rs` or the explicit `config.lib_path == Some(root_path)` override (a bare `ends_with("src/lib.rs")` is too broad — it would mis-record a `m/tools/src/lib.rs` bin/tool path; re-review round-3 MAJOR); when no member prefix matches (the single crate at the repo root, member dir `""`), accept only the bare `root_path == "src/lib.rs"` or a root `config.lib_path` override; else `None`. Keyed by member dir — **not** the library file's parent — so an explicit/nested `[lib].path` still keys by `<member>` (re-review BLOCKER B). Same spelling as `member_in_repo_deps` keys/values.
- `ResolutionPolicy::extern_crate_root(&self, &ScopeGraph, &str, &Anchor, ScopeId) -> Option<ScopeId>` — trait default in `types.rs:542-596` (Task 4a), impl in `rust_policy.rs` (Task 4a), called in `engine.rs::resolve_path_guarded` (Task 4b). `Anchor`/`AnchorKind` are `name_resolution::types`.
- `scope_member_lookup_probed(&ScopeGraph, ScopeId, &ResolveQuery, &dyn ResolutionPolicy, &mut CycleGuard) -> (Resolution, bool)` — new in `engine.rs` (Task 4b); `scope_member_lookup` becomes a wrapper returning `.0`. The fallback only runs in `resolve_path_guarded` (leading segment, `i==0`).
- `load_repo(&Path) -> Result<LoadedRepo>` (pub, `repo_loader.rs:61`); `LoadedRepo.scope_graph_inputs: Option<ScopeGraphBuildInputs>` (pub, `:39`); `ScopeGraphBuildInputs.cfg: RustCrateConfig` (pub, `call_graph.rs:127`).
- `CallGraph::build_with_scope_graph_inputs(&BTreeMap<String, ParsedFile>, Option<&ScopeGraphBuildInputs>) -> CallGraph` (pub, `call_graph.rs:419`).
- `populate_rust(&BTreeMap<String, ParsedFile>, &RustCrateConfig, Option<&BTreeSet<String>>) -> ScopeGraph` (pub, `rust_populator/mod.rs:255`); `enclosing_scope`/`file_id` are pub (`mod.rs:226`/`:207`).
- `resolve_path(graph, &RawPath, NamespaceId, &Anchor, ScopeId, NamespaceId, &SourceLoc, &dyn ResolutionPolicy) -> Resolution` (pub, `engine.rs:54`); `RustPolicy::new(&ScopeGraph, u16)`, `NS_TYPE` (pub, `rust_policy.rs:40`/`:78`).
- `site_in(&CallGraph, &str, &str) -> CallSite` (`resolution_test.rs:370`); `cg.resolve_call_site_full(&CallSite) -> ResolutionOutcome { resolved, drop }` (`resolution.rs:433`/`:765`); `ResolvedCallee.confidence: ResolutionConfidence`, `.target.file: String`; `ResolutionConfidence`/`CallGraph`/`CallSite` imported at `resolution_test.rs:1-4`.
- `CACHE_VERSION: u32` (`cpg_cache.rs:60`); pin test at `:564-568`.

**Spec-vs-source discrepancies handled (recorded for review):**
- The `ResolutionPolicy` trait lives in `src/name_resolution/types.rs:542-596`, **not** in `rust_policy.rs` (spec §2.2 said "likely in `rust_policy.rs` or a sibling"). The default `extern_crate_root` method is added there (Task 4a 3a), requiring a `use crate::name_resolution::graph::ScopeGraph;` in `types.rs` (with an inline-path fallback noted if the module-path reference objects).
- `RustPolicy` already has a private `crate_root(&self, ScopeId)` (`rust_policy.rs:104-121`) that climbs to the nearest `Root`. The spec's `crate_root_of(graph, from)` is added as a **free** helper (not reusing the method) because the trait hook receives `graph` as a parameter rather than `RustPolicy`'s borrowed `self.graph` — noted in the type-consistency list above.
- The predecessor edition-anchoring work (§ #123) is **already merged on this branch** — `repo_loader.rs` already carries `workspace_editions`, `anchoring_class_uniform`, `parse_edition`, and the `{ workspace = true }` handling (`:287-423`), and `CACHE_VERSION` is already at 17 (spec §2.3's "17→18" is from the post-#123 baseline, confirmed). Task 1 bumps 17→18. The §6.6 claimed-but-invisible case is the load-bearing `!rib_present` proof (a claimed rib sets `rib_present == true` regardless of `visible()`), and per the spec-review (MAJOR 2) it is now a **REQUIRED** dedicated resolver test (`cross_crate_use_claimed_but_invisible_local_blocks_fallback`, Task 4b), **not** optional: it is the regression guard that fails RED if the fallback is implemented with the wrong (status-only) trigger. The fixture relies on the verified source fact that `resolve_restrict` (`rust_populator/walk/mod.rs:271-277`) is a Phase-1 stub returning `None`, so a `pub(in crate::secret) mod b_crate` mints a `VIS_PUB_IN`/`restrict: None` binding that `RustPolicy::visible` rejects — a robust claimed-but-invisible rib.
- The engine's `resolve_path_guarded`/`scope_member_lookup` have MORE parameters than the spec's design-level snippet showed (`ns`, `anchor_ns`, `at`, `guard`); Task 4b's verbatim code matches the real signatures (`engine.rs:316-410`).

**Re-review BLOCKERs (2026-06-22 codex xhigh round 2) folded:**
- **BLOCKER A — `normalize_repo_rel` escape clamp (Task 2).** The original lexical normalizer `out.pop()`-ed a `..` even with nothing to pop, silently clamping an out-of-repo `../b`/`a/../../b` to an in-repo-looking dir, which both call sites recorded unconditionally — violating spec §2.1's "PATH deps that resolve to an **in-repo** member". FIXED: `normalize_repo_rel` now returns `Option<String>`, yielding `None` on a `..` underflow (escape); BOTH the path-dep call site (`parse_member_in_repo_deps`) and the workspace-dep pre-scan call site `if let Some(target) = ...` skip an escaping target (record nothing). New RED tests: `normalize_repo_rel_returns_none_when_escaping_repo_root` (unit) + `path_dep_escaping_repo_root_is_not_recorded` (capture-level escape guard from the repo-root member); the in-repo `a/../b → b` case is preserved.
- **BLOCKER B — `lib_root_member_dir` keyed by the library FILE's parent (Task 3).** The original helper returned the library file's parent dir (`<member>/src` for `[lib] path = "src/lib.rs"`, `<member>/src/inner` for a nested custom path), so `lib_root_by_member_dir` was keyed by `<member>/src` while dep-target resolution produced `<member>` → silent miss. FIXED: the helper now derives the member dir from `config.workspace_members` (longest-prefix match — the real, already-available signal `repo_loader.rs:357` populates and `crate_name_for_root` already matches against), with an exact no-member gate (`root_path == "src/lib.rs"` or a root `config.lib_path`) for the single root crate (`""`), gated on `root_path` being that member's library root (the EXACT `m/src/lib.rs`/`[lib].path` rule — re-review round-3 MAJOR). **No new data threaded through the Builder.** New RED tests: `explicit_lib_path_target_keys_by_member_dir_not_file_parent` + `nested_custom_lib_path_target_keys_by_member_dir` (both assert the dep target keys by `<member>`, not the file parent — the discriminating cases the old helper failed).
