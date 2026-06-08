# Prism Navigation Layer — Plan 3a: nav cache + grammar fingerprint (v3, review-hardened ×2)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the whole-repo `NavigationIndex` an exact-hit on-disk cache (keyed by a `build.rs`-generated grammar-version fingerprint + repo identity) in a **prism-owned** store, so `prism nav` queries don't rebuild the CPG every call — and close the stale-tree-after-`cargo update` bug for both caches.

**Architecture:** Additive (no analysis-logic edits; nav query output is identical hit or miss). A `build.rs` emits `GRAMMAR_FINGERPRINT` from `Cargo.lock`'s `tree-sitter-*` versions. `cpg_cache` gains `grammar_fingerprint` + a `skip_policy_version` constant in its validity key (`CACHE_VERSION` 2→3). The nav cache lives in a **prism-owned XDG dir keyed by canonical repo path** (never inside the analyzed repo). `NavigationIndex::build_cached` loads/saves there, deriving the nav-local indexes from the (cached or fresh) CPG.

**Tech Stack:** Rust, `build.rs`, the existing `cpg_cache` (bincode + SHA-256), `dirs` (XDG cache dir), Plan 1 `LoadedRepo`/`NavigationIndex`, `CpgContext::build_with_cached_cpg`.

**Spec:** §9 (cache), **R1-B4 / R2-m2** (build.rs grammar fingerprint). **Builds on:** Plans 1+2 (merged).

## 0. Plan-review disposition (round 1)

| Finding | Resolution | Task |
|---|---|---|
| B1 invalidation test edits a file `load_cache` ignores; structs private | in-crate `#[cfg(test)]` unit test in `cpg_cache.rs`: build a `CpgCache` with a wrong fingerprint, serialize to `cpg-cache.bin`, assert `Miss` | T2 |
| M2 cache key dropped §9 fields | enumerate the v1 nav key; add a `skip_policy_version` const; `repo_root_id` realized via the cache **dir path** (T3); document deferrals (graph_profile, type_db=None, supported_languages) | T2/T3 |
| M3 `target/` cache dirties foreign repos + violates `repo_root_id` | prism-owned `dirs::cache_dir()/prism/nav/<hash(canonical repo root)>/` | T3 |
| M4 tests don't prove a hit / nav-query equivalence | assert `cpg-cache.bin` exists after build 1; add a `callers`/`ego` query equal across forced-miss vs forced-hit | T3 |
| M5 `--no-cache` plumbing under-specified | `build_session(repo, no_cache)`; `no_cache: bool` on `NavArgs`; thread through all four arms; flag precedes the subcommand | T4 |
| M6 CLI test only checks dir exists; uses `--repo .` | tempdir repo+cache; assert `--no-cache` writes nothing + cached vs uncached output byte-equal | T4 |
| m7 cite R3-m2 | corrected to R1-B4 / R2-m2 | header |
| m8/m9 Task1 failure reason / tautological assert | corrected below | T1 |
| m10 file add/remove nav test missing | added | T3 |
| m11 doc-sync (build script, cache layout) | CLAUDE.md note | T4 |

## Grounding (verified)

- `cpg_cache.rs`: `CACHE_VERSION=2`; private `struct CpgCache { version, prism_version, file_hashes, has_type_db, graph }`; `save_cache(cpg, file_hashes, has_type_db, cache_dir) -> Result<()>`; `load_cache(current_hashes, has_type_db, cache_dir) -> CacheResult{Hit(CodePropertyGraph)|PartialHit{..}|Miss}` — reads/bincode-deserializes `cpg-cache.bin`, **not** the json meta. `compute_file_hashes`.
- `cpg.rs`: `CpgContext::build_with_cached_cpg(files, cpg, type_db) -> CpgContext`.
- `LoadedRepo { root, files, file_hashes, .. }`; `NavigationIndex::build` derives `line_range_index`/`name_index` from `Function` nodes.

## File Structure

| File | Responsibility |
|---|---|
| `build.rs` (new) | `GRAMMAR_FINGERPRINT` from `Cargo.lock` |
| `src/cpg_cache.rs` (modify) | `grammar_fingerprint` + `skip_policy_version` in key; `CACHE_VERSION` 3; in-crate invalidation test |
| `src/navigation/cache.rs` (new) | XDG/repo-id cache dir; `NavigationIndex::build_cached` |
| `src/navigation/mod.rs` (modify) | `from_ctx` refactor; `pub mod cache;` |
| `src/main.rs` (modify) | `build_session(repo, no_cache)`; `NavArgs.no_cache` |
| `Cargo.toml` (modify) | `dirs` dependency |
| `tests/navigation/cache_test.rs` (new) | hit detection, equivalence, content + add/remove invalidation, `--no-cache` |

---

## Task 1: `build.rs` grammar fingerprint

**Files:** Create `build.rs`; Test `tests/navigation/cache_test.rs` (registered here).

- [ ] **Step 1: Register target + failing test**

`Cargo.toml`: `[[test]] name = "navigation_cache"` / `path = "tests/navigation/cache_test.rs"`.
```rust
// tests/navigation/cache_test.rs
#[test]
fn grammar_fingerprint_is_present() {
    assert!(!env!("GRAMMAR_FINGERPRINT").is_empty(), "build.rs must emit GRAMMAR_FINGERPRINT");
}

#[test]
fn fingerprint_has_real_grammar_input() {
    // R2-M4: not a tautology — the fingerprint must derive from actual tree-sitter-* versions.
    let lock = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.lock")).unwrap();
    let n = lock.lines().filter(|l| l.trim().starts_with("name = \"tree-sitter")).count();
    assert!(n >= 1, "expected >=1 tree-sitter-* crate in Cargo.lock (build.rs panics otherwise)");
}
```

- [ ] **Step 2: Run to verify it fails** — `cargo test --test navigation_cache` → FAIL: `GRAMMAR_FINGERPRINT` compile-time env var is not set (build.rs missing), so `env!` fails to compile.

- [ ] **Step 3: Implement `build.rs`** (FNV-1a over the sorted `tree-sitter-*` versions in `Cargo.lock`; no extra deps)

```rust
// build.rs
fn main() {
    println!("cargo:rerun-if-changed=Cargo.lock");
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let lock = std::fs::read_to_string(format!("{manifest}/Cargo.lock")).unwrap_or_default();
    let mut entries: Vec<String> = Vec::new(); // "name@version" — Vec, not a map, so dup grammars don't collapse (R2-m11)
    let mut cur: Option<String> = None;
    for line in lock.lines() {
        let l = line.trim();
        if let Some(r) = l.strip_prefix("name = \"") { cur = r.strip_suffix('"').map(String::from); }
        else if let Some(r) = l.strip_prefix("version = \"") {
            if let Some(n) = &cur { if n.starts_with("tree-sitter") {
                if let Some(v) = r.strip_suffix('"') { entries.push(format!("{n}@{v}")); } } }
        }
    }
    entries.sort();
    entries.dedup();
    assert!(!entries.is_empty(), "build.rs: no tree-sitter-* crates in Cargo.lock — fingerprint would be a constant");
    let joined = entries.join(";");
    let mut h: u64 = 0xcbf29ce484222325;
    for b in joined.bytes() { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    println!("cargo:rustc-env=GRAMMAR_FINGERPRINT={h:016x}");
}
```

- [ ] **Step 4: Run to verify it passes** — `cargo test --test navigation_cache -- grammar_fingerprint_is_present` → PASS.
- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat(nav): build.rs grammar-version fingerprint"`

---

## Task 2: Fingerprint + skip-policy version in the cache key

**Files:** Modify `src/cpg_cache.rs` (incl. an in-crate test).

- [ ] **Step 1: Write the failing in-crate unit test** (B1 — `load_cache` reads `cpg-cache.bin`, so inject there)

```rust
// in src/cpg_cache.rs, under #[cfg(test)] mod tests { ... }
#[test]
fn wrong_grammar_fingerprint_misses() {
    let dir = tempfile::tempdir().unwrap();
    let hashes: BTreeMap<String, String> = BTreeMap::from([("a.py".into(), "h".into())]);
    // Construct a CpgCache directly with a deliberately-wrong fingerprint and serialize to cpg-cache.bin.
    let bad = CpgCache {
        version: CACHE_VERSION, prism_version: env!("CARGO_PKG_VERSION").into(),
        grammar_fingerprint: "deadbeef".into(), skip_policy_version: SKIP_POLICY_VERSION,
        file_hashes: hashes.clone(), has_type_db: false,
        graph: SerializedCpg { nodes: vec![], edges: vec![], call_graph: CallGraph::empty(), dfg: DataFlowGraph::empty() },
    };
    std::fs::write(dir.path().join("cpg-cache.bin"), bincode::serialize(&bad).unwrap()).unwrap();
    assert!(matches!(load_cache(&hashes, false, dir.path()), CacheResult::Miss));
}
```
> The test is `#[cfg(test)] mod tests` *inside* `cpg_cache.rs` (`use super::*`), so it reaches the private `CpgCache`/`SerializedCpg`. `CallGraph::empty()`/`DataFlowGraph::empty()`/`CodePropertyGraph::empty()` exist (`cpg.rs:638-648`). **Confirm `SerializedCpg`'s exact fields against `cpg_cache.rs:69`** and adjust the literal if they differ. The corrected fingerprint check (#1) returns `Miss` before `reconstruct_cpg` touches this graph, so an empty graph is safe.

- [ ] **Step 2: Run to verify it fails** — `cargo test --lib cpg_cache::tests::wrong_grammar_fingerprint_misses` → FAIL (fields don't exist yet).

- [ ] **Step 3: Implement** — in `src/cpg_cache.rs`:
  1. `const CACHE_VERSION: u32 = 3;` (doc: `// - v3: grammar_fingerprint + skip_policy_version`) and `pub const SKIP_POLICY_VERSION: u32 = 1;`.
  2. Add `grammar_fingerprint: String` and `skip_policy_version: u32` to `struct CpgCache` (and `CacheMeta` if it must mirror — match what `load_cache` validates).
  3. `save_cache`: `grammar_fingerprint: env!("GRAMMAR_FINGERPRINT").into(), skip_policy_version: SKIP_POLICY_VERSION`.
  4. `load_cache`: insert **immediately after the `has_type_db` invalidation check (~`cpg_cache.rs:267`) and BEFORE the file-hash comparison that returns `Hit` (~`:270`)** — NOT after the hash block, or the check is dead on the exact-hit path and the whole feature no-ops (R2-B1): `if cache.grammar_fingerprint != env!("GRAMMAR_FINGERPRINT") || cache.skip_policy_version != SKIP_POLICY_VERSION { return CacheResult::Miss; }`.

  **v1 nav key (documented):** `version`, `prism_version`, `grammar_fingerprint`, `skip_policy_version`, `file_hashes`, `has_type_db`, plus `repo_root_id` realized as the cache **directory** (T3). Deferred from spec §9: `graph_profile` (single profile in v1, §61-62), `type_db_key` (nav builds with `type_db=None`), `supported_languages` (stable; the file set already reflects it).

- [ ] **Step 4: Run to verify it passes** — `cargo test --lib cpg_cache` and `cargo test --test ast_cpg_cache` → PASS (old caches gracefully `Miss` on the version bump).
- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat(cache): grammar + skip-policy fingerprints in cache key (v3)"`

---

## Task 3: `NavigationIndex::build_cached` in a prism-owned store

**Files:** Create `src/navigation/cache.rs`; modify `src/navigation/mod.rs`, `Cargo.toml` (`dirs`); Test `tests/navigation/cache_test.rs`.

- [ ] **Step 1: Write the failing tests** (real hit detection + nav-query equivalence + invalidation)

```rust
// add to tests/navigation/cache_test.rs
use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::sync::Arc;

fn write(dir: &std::path::Path, name: &str, src: &str) { std::fs::write(dir.join(name), src).unwrap(); }

#[test]
fn build_cached_writes_then_hits_with_equal_query_output() {
    let repo_d = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    write(repo_d.path(), "a.py", "def helper():\n    return 1\n\ndef run():\n    return helper()\n");
    let repo = Arc::new(load_repo(repo_d.path()).unwrap());

    let idx_miss = Arc::new(NavigationIndex::build_cached_in(&repo, cache.path())); // miss -> builds + saves
    assert!(cache.path().join("cpg-cache.bin").exists(), "build 1 must write the cache");
    let idx_hit = Arc::new(NavigationIndex::build_cached_in(&repo, cache.path()));  // hit -> loads

    // R2-M5: cover all index paths reconstruct_cpg rebuilds — callees (forward edges),
    // callers (reverse edges), ego (location_index) — all identical across miss and hit.
    let s_miss = NavigationSession { repo: repo.clone(), index: idx_miss };
    let s_hit = NavigationSession { repo: repo.clone(), index: idx_hit };
    assert_eq!(queries::callees(&s_miss, Some("run"), None, None, 1).unwrap(),
               queries::callees(&s_hit, Some("run"), None, None, 1).unwrap());
    assert_eq!(queries::callers(&s_miss, Some("helper"), None, None, 1).unwrap(),
               queries::callers(&s_hit, Some("helper"), None, None, 1).unwrap());
    assert_eq!(queries::ego_graph(&s_miss, Some("run"), None, None, 1, &["Call"]).unwrap(),
               queries::ego_graph(&s_hit, Some("run"), None, None, 1, &["Call"]).unwrap());
}

#[test]
fn file_add_invalidates() {
    let repo_d = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    write(repo_d.path(), "a.py", "def f():\n    return 1\n");
    let _ = NavigationIndex::build_cached_in(&load_repo(repo_d.path()).unwrap(), cache.path());
    write(repo_d.path(), "b.py", "def g():\n    return 2\n");           // add a file
    let idx = NavigationIndex::build_cached_in(&load_repo(repo_d.path()).unwrap(), cache.path());
    assert!(idx.name_index.contains_key(&("b.py".into(), "g".into())));  // rebuilt
}
```

- [ ] **Step 2: Run to verify it fails** — `cargo test --test navigation_cache` → FAIL (`build_cached_in` missing).

- [ ] **Step 3: Implement** — add `dirs = "5"` to `Cargo.toml`; create `src/navigation/cache.rs`:

```rust
// src/navigation/cache.rs
use crate::cpg::CpgContext;
use crate::cpg_cache::{self, CacheResult};
use crate::navigation::NavigationIndex;
use crate::repo_loader::LoadedRepo;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// Prism-owned cache dir for this repo: <user cache>/prism/nav/<hash(canonical root)>/.
pub fn nav_cache_dir(repo: &LoadedRepo) -> PathBuf {
    let canon = std::fs::canonicalize(&repo.root).unwrap_or_else(|_| repo.root.clone());
    let mut h = Sha256::new(); h.update(canon.to_string_lossy().as_bytes());
    let id = format!("{:x}", h.finalize());
    let base = dirs::cache_dir().unwrap_or_else(std::env::temp_dir);
    base.join("prism").join("nav").join(&id[..16])
}

impl NavigationIndex {
    /// Default cache location (prism-owned, per-repo).
    pub fn build_cached(repo: &LoadedRepo) -> Self {
        let dir = nav_cache_dir(repo);
        let _ = std::fs::create_dir_all(&dir);
        Self::build_cached_in(repo, &dir)
    }
    /// Cache in an explicit dir (tests / `--cache-dir`).
    pub fn build_cached_in(repo: &LoadedRepo, cache_dir: &Path) -> Self {
        let _ = std::fs::create_dir_all(cache_dir);
        let has_type_db = repo.type_db.is_some();
        match cpg_cache::load_cache(&repo.file_hashes, has_type_db, cache_dir) {
            CacheResult::Hit(cpg) => Self::from_ctx(CpgContext::build_with_cached_cpg(&repo.files, cpg, repo.type_db.as_ref())),
            _ => {
                let ctx = CpgContext::build(&repo.files, repo.type_db.as_ref());
                let _ = cpg_cache::save_cache(&ctx.cpg, &repo.file_hashes, has_type_db, cache_dir);
                Self::from_ctx(ctx)
            }
        }
    }
}
```
Refactor `mod.rs`: extract the index derivation into `pub(crate) fn from_ctx(ctx: CpgContext) -> NavigationIndex`; `build` becomes `Self::from_ctx(CpgContext::build(&repo.files, repo.type_db.as_ref()))`. Add `pub mod cache;`. `PartialHit` is treated as a miss (v1 exact-hit). `Evidence`/`EgoGraph` already derive `PartialEq` (Plan 2), so the equality assert compiles.

- [ ] **Step 4: Run to verify it passes** — `cargo test --test navigation_cache` → PASS.
- [ ] **Step 5: Commit** — `git add -A && git commit -m "feat(nav): build_cached in a prism-owned per-repo cache store"`

---

## Task 4: Wire the CLI (+ `--no-cache`) + doc-sync

**Files:** Modify `src/main.rs`, `CLAUDE.md`; Test `tests/cli/nav_compat_test.rs`.

- [ ] **Step 1: Write the failing test** (injected `--cache-dir`; assert `--no-cache` writes nothing + output transparency)

```rust
// append to tests/cli/nav_compat_test.rs
#[test]
fn nav_cache_writes_and_no_cache_does_not() {
    let repo = tempfile::tempdir().unwrap();
    std::fs::write(repo.path().join("a.py"), "def helper():\n    return 1\n\ndef run():\n    return helper()\n").unwrap();
    let r = repo.path().to_str().unwrap();
    // cached run with an explicit --cache-dir writes the cache:
    let cdir = tempfile::tempdir().unwrap();
    let cached = bin().args(["nav","callees","--repo",r,"--cache-dir",cdir.path().to_str().unwrap(),"--symbol","run","--format","json"]).output().unwrap();
    assert!(cached.status.success());
    assert!(cdir.path().join("cpg-cache.bin").exists(), "cached run must write the cache");
    // --no-cache run with its own FRESH --cache-dir leaves it empty, output identical:
    let ndir = tempfile::tempdir().unwrap();
    let nocache = bin().args(["nav","--no-cache","callees","--repo",r,"--cache-dir",ndir.path().to_str().unwrap(),"--symbol","run","--format","json"]).output().unwrap();
    assert!(nocache.status.success());
    assert!(!ndir.path().join("cpg-cache.bin").exists(), "--no-cache must not write");
    assert_eq!(cached.stdout, nocache.stdout, "cached vs uncached output must be byte-equal");
}
```
> Uses a tempdir repo (never `--repo .`) and tempdir cache dirs, so no whole-prism build, no writes into a live tree, and no race on a shared store.

- [ ] **Step 2: Run to verify it fails** — FAIL (`--no-cache` unknown / `build_session` not cache-aware).

- [ ] **Step 3: Implement** — in `src/main.rs`: change `build_session(repo: &Path)` → `build_session(repo: &Path, no_cache: bool, cache_dir: Option<&Path>)`: load the repo, then `if no_cache { NavigationIndex::build(&loaded) } else { let dir = cache_dir.map(|p| p.to_path_buf()).unwrap_or_else(|| prism::navigation::cache::nav_cache_dir(&loaded)); NavigationIndex::build_cached_in(&loaded, &dir) }`. Add `#[arg(long)] no_cache: bool` and `#[arg(long)] cache_dir: Option<PathBuf>` to `NavArgs` (both precede the subcommand: `prism nav --no-cache callees …`). Thread `nav.no_cache` + `nav.cache_dir.as_deref()` into all four arms' `build_session(...)` calls.

- [ ] **Step 4: Run to verify it passes (full suite + goldens)** — `cargo test` → all green. The cache is output-transparent, so Plan 1/2 byte-for-byte goldens are unchanged.
- [ ] **Step 5: Doc-sync** — in `CLAUDE.md`: add `dirs` + the `build.rs` grammar fingerprint to **Dependencies**, and a line on the prism-owned per-repo nav cache (`<cache>/prism/nav/<repo-id>/`) under the CPG/architecture notes.
- [ ] **Step 6: Commit** — `git add -A && git commit -m "feat(nav): cache nav index per-repo in a prism-owned store (+ --no-cache/--cache-dir); doc-sync"`. *Optional, non-gating (outside the per-task green contract):* `time cargo run -q -- nav callers --repo <repo> --cache-dir /tmp/navc --symbol <fn>` twice — the 2nd run is faster (cache hit). Use an explicit `--cache-dir` so it doesn't touch the user's default store.

---

## Done / handoff

After Plan 3a: `prism nav` reuses an on-disk whole-repo index across runs from a prism-owned per-repo store, invalidated on content / file add-remove / Prism-version / **grammar-version** / skip-policy change — and the grammar fix also hardens the per-diff review cache. **Plan 3b** = `module-deps`/`repo-map`; **Plan 3c** = MCP.

## Round 2 disposition
**Closed:** B1 fingerprint-check placement pinned before the hash-hit return (T2.3.4); B2 `--no-cache` test inspects an injected `--cache-dir` (T4.1); B3 real `SerializedCpg` literal (T2.1); M4 discriminating fingerprint test + build.rs `name@version`/panic-if-empty (T1); M5 `callers`/`ego` hit-vs-miss equivalence (T3.1); m8/m10 below.
**Decisions (documented, not deferred):** `CacheMeta` (the JSON debug file) is **non-authoritative** — `load_cache` validates only the bincode `CpgCache`, so `CacheMeta` need NOT carry the new fields; leave it as-is (M6). File *remove* invalidation hits the same `cached_keys != current_keys → Miss` branch as file *add*, so T3's add test covers it; a separate remove test is optional (m8). `nav --no-cache`/`--cache-dir` gate the **nav** store specifically (the same-named `ReviewArgs` flag is a separate command tree disabled under `nav` by `args_conflicts_with_subcommands`) — note this in the Task 4 doc-sync (m10).

## Confirm at execution
- `dirs` major version available offline (`5`); else `directories`/`dirs-next`.
- `Evidence`/`EgoGraph`/`QueryError` derive `PartialEq` for the equality asserts (Plan 2 added them).
- `SerializedCpg` exact field names (`cpg_cache.rs:69`) for the T2 literal.
