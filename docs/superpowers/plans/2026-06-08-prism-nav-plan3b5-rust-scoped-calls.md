# Prism Navigation Layer — Plan 3b.5: Nav-local `::`-scoped call resolution (Rust + C++)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the nav call-resolution queries (`callees`, `callers`, `module-deps`, `repo-map`) resolve **`::`-scoped calls** (`mod::fn()` in Rust, `Ns::func()` / `Class::method()` in C++) cross-file, so dispatcher-style functions are navigable (e.g. `callees run_slicing_inner`: 0 → ~30 resolved callees on this Rust repo).

**Why `::` and not "Rust":** the `.`-qualified languages (**Python, JS, TS, Go**) already resolve their package/module-qualified calls via the existing import-aware `resolve_callees_qualified` (Plan 2) — verified: Go `u.Helper()` → `u/u.go` today. The languages that fall through are the **`::`-syntax** ones: the parser's `call_function_name` stores the whole path as the callee with `qualifier = None` (no `scoped_identifier` arm), so `original_diff::slice` (Rust) and `util::helper` (C++) are both unresolved today (verified identical). This plan closes that one shared gap; the resolver is language-agnostic (`::`-triggered), so Rust and C++ are covered by the same code.

**Architecture:** Purely additive and **nav-local** (Option C). A new `src/navigation/call_resolve.rs` wraps `CallGraph::resolve_callees_qualified`: when a callee name is a `::`-scoped path (`A::b`) that the bare-name index misses, it splits the path and resolves the final segment (`b`) narrowed to files whose **stem == the preceding segment** (`A`) — the same file-stem idiom the shared resolver already uses for import qualifiers. **`call_graph.rs` is not modified**; diff-review keeps calling the unchanged shared resolver, so its goldens are byte-identical. All `CallGraph` fields are already `pub`, so the resolver reads `cpg.call_graph.functions`/`.callers`/`.calls` directly with no new accessors.

**Tech Stack:** Rust, the existing `prism::navigation` + `prism::call_graph` modules.

**Spec/context:** extends the §10 "Rust = call-derived only" note (now: `::` languages resolve module/namespace-qualified free functions via stem). Empirical: `callees run_slicing_inner@src/algorithms/mod.rs` = 41 sites / 0 resolved today; 91 distinct `mod::fn` patterns in `src/` (file-stem-resolvable) vs 201 `Type::method` (mostly stdlib/type-assoc, out of scope); C++ `util::helper()` confirmed unresolved-today and resolvable by this fix.

---

## Design decisions (locked)

1. **Scope: file-stem-resolvable `::` paths (Rust + C++).** Resolve `A::b` by looking up bare `b` in `call_graph.functions` and keeping `FunctionId`s whose **file stem == `A`** (the segment immediately before `b`). This covers Rust `mod::fn` (module == file: `original_diff::slice` → `original_diff.rs`) and C++ file-per-namespace/class `Ns::func`/`Class::method` (`util::helper` → `util.cpp`, `Foo::bar` → `Foo.cpp`). It does **not** resolve when the type/module name differs from the file stem (e.g. `CallGraph::build` — `CallGraph` lives in `call_graph.rs`) — those need a type/module→file map (deferred). The reach is **case-sensitive stem matching**, so a coincidental match resolves and a non-match returns empty; this is pinned by fixtures (T1), not emergent.
2. **Reserved path keywords are rejected.** `self`/`super`/`crate` as the *immediate* hint (e.g. `self::run`, `super::run`, `crate::run`) are path keywords, not module names — the fallback returns empty for them (guards against a stray `self.rs`/`crate.rs` false match). Multi-segment paths still use the real preceding segment: `crate::algo::run` → hint `algo` (resolves). 
3. **Strictly additive, no-regression:** the resolver first delegates to `resolve_callees_qualified`. The scoped fallback runs **only when that returns empty AND the name contains `::`**. If stem-narrowing finds no candidate it returns empty (exactly as today). Non-scoped calls and the `.`-qualified languages are unchanged; only previously-empty `::` calls gain edges.
4. **Consumers (this plan): `callees`, `callers`, `module-deps`, `repo-map`.** **`ego` is deferred** (it walks CPG `Call` edges materialized at CpgContext::build from the *shared* resolver, not query-time; surfacing scoped edges there needs nav-index CPG augmentation). **The doc-sync must warn** that after this plan `ego` returns a strictly smaller neighborhood than `callees`+`callers` for scoped-dispatch symbols.
5. **`callers` keying:** `call_graph.callers` is keyed by the **raw** callee name, so a scoped call lives under `"A::b"`, not `"b"`. `callers(b@file)` must also scan keys ending in `"::b"` and keep those whose `resolve_callees_nav` hits the target file (the identity filter prunes `other::b` → other.rs).
6. **Boundaries (documented, not fixed here):** inline modules (`mod algo { fn run() }` in `main.rs`, called `algo::run()`) won't resolve — `file_stem("main.rs") != "algo"`; `Type::method` where type ≠ file stem won't resolve; `ego` scoped edges deferred. No golden changes: the `CG` fixture is Python; diff-review uses the untouched shared resolver.

---

## File Structure

**Create:**
- `src/navigation/call_resolve.rs` — `resolve_callees_nav(cg, callee_name, caller_file, qualifier) -> Vec<&FunctionId>` (delegate + scoped fallback + reserved-keyword guard) and `scoped_caller_sites<'a>(cg, target_name) -> Vec<&'a CallSite>` (bare + `::target` keys). (< 130 lines.)
- `tests/navigation/scoped_calls_test.rs` — library tests on **Rust and C++** fixtures: positive `mod::fn`/`Ns::func` resolution; negatives (`self`/`super`/`crate`, `Type::method` non-coincidence, external `crate::fn` with a same-stem decoy); cross-stem false-positive pruning for callers; `callees`/`callers`/`module_deps`/`repo_map` see the new edges.

**Modify (additive only — no `call_graph.rs` edits):**
- `src/navigation/mod.rs` — `pub mod call_resolve;`.
- `src/navigation/queries.rs` — `direct_callees` and `direct_callers` use the nav resolver (`direct_callers` also scans scoped caller keys).
- `src/navigation/module_graph.rs` — `collect_module_edges` uses the nav resolver.
- `Cargo.toml` — `[[test]] navigation_scoped_calls`.
- `CLAUDE.md` / `docs/prism-query-layer/plan3b-module-map-followups.md` — note `::`-scoped support (Rust + C++), the ego divergence, and the remaining language-agnostic gaps.

---

## Task 1: Nav-local scoped-call resolver

**Files:**
- Create: `src/navigation/call_resolve.rs`, `tests/navigation/scoped_calls_test.rs`
- Modify: `src/navigation/mod.rs`, `Cargo.toml`

- [ ] **Step 1: Register the test target + write the failing tests**

In `Cargo.toml`, after the other `navigation_*` test targets:

```toml
[[test]]
name = "navigation_scoped_calls"
path = "tests/navigation/scoped_calls_test.rs"
```

Create `tests/navigation/scoped_calls_test.rs` with the file-local `session` helper (copied from `tests/navigation/module_graph_test.rs`, including the `create_dir_all` parent handling) and:

```rust
use prism::navigation::call_resolve::resolve_callees_nav;
use prism::navigation::{NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::sync::Arc;

fn session(files: &[(&str, &str)]) -> NavigationSession {
    let dir = tempfile::tempdir().unwrap();
    for (name, src) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, src).unwrap();
    }
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

#[test]
fn scoped_mod_fn_resolves_cross_file_rust() {
    let s = session(&[
        ("algo.rs", "pub fn run() -> i32 { 1 }\n"),
        ("main.rs", "mod algo;\nfn dispatch() -> i32 { algo::run() }\n"),
    ]);
    let cg = &s.index.cpg.call_graph;
    let resolved = resolve_callees_nav(cg, "algo::run", "main.rs", None);
    assert!(
        resolved.iter().any(|f| f.file == "algo.rs" && f.name == "run"),
        "algo::run should resolve to algo.rs::run, got {:?}",
        resolved.iter().map(|f| (&f.file, &f.name)).collect::<Vec<_>>()
    );
}

#[test]
fn scoped_ns_fn_resolves_cross_file_cpp() {
    // C++ namespace-qualified call: util::helper() -> util.cpp::helper (same `::` gap as Rust).
    let s = session(&[
        ("util.cpp", "namespace util { int helper() { return 1; } }\n"),
        ("main.cpp", "namespace util { int helper(); }\nint dispatch() { return util::helper(); }\n"),
    ]);
    let cg = &s.index.cpg.call_graph;
    let resolved = resolve_callees_nav(cg, "util::helper", "main.cpp", None);
    assert!(
        resolved.iter().any(|f| f.file == "util.cpp" && f.name == "helper"),
        "util::helper should resolve to util.cpp::helper, got {:?}",
        resolved.iter().map(|f| (&f.file, &f.name)).collect::<Vec<_>>()
    );
}

#[test]
fn scoped_call_to_wrong_stem_does_not_resolve() {
    let s = session(&[
        ("algo.rs", "pub fn run() -> i32 { 1 }\n"),
        ("main.rs", "fn dispatch() -> i32 { nope::run() }\n"),
    ]);
    let cg = &s.index.cpg.call_graph;
    assert!(resolve_callees_nav(cg, "nope::run", "main.rs", None).is_empty());
}

#[test]
fn reserved_keyword_hints_do_not_resolve() {
    // A decoy file named `crate.rs`/`self.rs` must NOT satisfy crate::run / self::run.
    let s = session(&[
        ("crate.rs", "pub fn run() -> i32 { 1 }\n"),
        ("self.rs", "pub fn go() -> i32 { 1 }\n"),
        ("main.rs", "fn d() -> i32 { crate::run() + self::go() }\n"),
    ]);
    let cg = &s.index.cpg.call_graph;
    assert!(resolve_callees_nav(cg, "crate::run", "main.rs", None).is_empty());
    assert!(resolve_callees_nav(cg, "self::go", "main.rs", None).is_empty());
}

#[test]
fn external_crate_path_does_not_resolve_without_stem_match() {
    // bincode::serialize with NO bincode-stem file -> empty (external crate, not in repo).
    let s = session(&[
        ("main.rs", "fn d() { bincode::serialize(); }\n"),
        ("other.rs", "pub fn serialize() -> i32 { 1 }\n"), // decoy: wrong stem, must not match
    ]);
    let cg = &s.index.cpg.call_graph;
    assert!(resolve_callees_nav(cg, "bincode::serialize", "main.rs", None).is_empty());
}

#[test]
fn multi_segment_scoped_path_uses_last_module_segment() {
    let s = session(&[
        ("algo.rs", "pub fn run() -> i32 { 1 }\n"),
        ("main.rs", "mod algo;\nfn d() -> i32 { crate::algo::run() }\n"),
    ]);
    let cg = &s.index.cpg.call_graph;
    let resolved = resolve_callees_nav(cg, "crate::algo::run", "main.rs", None);
    assert!(resolved.iter().any(|f| f.file == "algo.rs" && f.name == "run"));
}

#[test]
fn unscoped_resolution_is_unchanged() {
    let s = session(&[
        ("util.rs", "pub fn helper() -> i32 { 1 }\n"),
        ("main.rs", "mod util;\nuse util::helper;\nfn run() -> i32 { helper() }\n"),
    ]);
    let cg = &s.index.cpg.call_graph;
    assert!(resolve_callees_nav(cg, "helper", "main.rs", None)
        .iter()
        .any(|f| f.file == "util.rs" && f.name == "helper"));
}
```

> **Implementer note:** the exact `callee_name` strings the parser stores (e.g. `algo::run`, `crate::algo::run`, `util::helper`) are what the call graph holds for these fixtures. If a fixture stores a different spelling, inspect `cg.calls`/`cg.callers` keys and adjust the test literal — the resolver contract is "given the stored scoped name, resolve it."

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test navigation_scoped_calls`
Expected: FAIL — `prism::navigation::call_resolve` does not exist.

- [ ] **Step 3: Implement the resolver**

Create `src/navigation/call_resolve.rs`:

```rust
use crate::call_graph::{CallGraph, CallSite, FunctionId};

/// Last path segment of a `::`-scoped name (`crate::algo::run` -> `run`).
fn last_segment(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

/// Module hint: the segment immediately before the final one
/// (`crate::algo::run` -> `algo`). `None` if the name is not `::`-scoped.
fn module_hint(name: &str) -> Option<&str> {
    let mut it = name.rsplit("::");
    let _fn = it.next()?; // final segment
    it.next() // the segment before it, if any
}

/// File stem of a path (`src/algorithms/original_diff.rs` -> `original_diff`).
/// NOTE: `.rsplit('.').last()` returns the FIRST dot-component (the stem), not the
/// extension — `.last()` consumes the reverse-order iterator to its final element.
/// This deliberately matches `resolve_callees_qualified`'s existing stem idiom in
/// `call_graph.rs` (so the delegated and fallback paths compute stems identically;
/// e.g. `a.b.rs` -> `a` in both). Do not "simplify" to `rsplit_once`, which would
/// diverge (`a.b.rs` -> `a.b`).
fn file_stem(path: &str) -> &str {
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .rsplit('.')
        .last()
        .unwrap_or(path)
}

/// Nav-local callee resolution. Delegates to the shared (diff-review-shared)
/// `resolve_callees_qualified`; when that finds nothing AND the callee is a
/// `::`-scoped path, resolves the final segment narrowed to files whose stem
/// matches the module/namespace segment (Plan 3b.5 — Rust `mod::fn`, C++
/// `Ns::func`/`Class::method`). Additive: never changes a non-empty delegated
/// result, so existing behavior (incl. `.`-qualified languages) is preserved.
pub fn resolve_callees_nav<'a>(
    cg: &'a CallGraph,
    callee_name: &str,
    caller_file: &str,
    qualifier: Option<&str>,
) -> Vec<&'a FunctionId> {
    let delegated = cg.resolve_callees_qualified(callee_name, caller_file, qualifier);
    if !delegated.is_empty() {
        return delegated;
    }
    let Some(hint) = module_hint(callee_name) else {
        return delegated; // not `::`-scoped -> empty
    };
    // Path keywords are not module/namespace names; never stem-match them.
    if matches!(hint, "self" | "super" | "crate") {
        return Vec::new();
    }
    let fn_name = last_segment(callee_name);
    match cg.functions.get(fn_name) {
        Some(ids) => ids
            .iter()
            .filter(|fid| file_stem(&fid.file) == hint)
            .collect(),
        None => Vec::new(),
    }
}

/// Caller sites targeting a function named `target_name`, including scoped keys
/// (`callers` is keyed by the raw callee name, so a scoped call lives under
/// `"A::target_name"`). Bare and `::target_name`-suffixed keys are both returned;
/// the caller resolves each site to confirm the target file.
pub fn scoped_caller_sites<'a>(cg: &'a CallGraph, target_name: &str) -> Vec<&'a CallSite> {
    let suffix = format!("::{target_name}");
    let mut out: Vec<&CallSite> = Vec::new();
    for (key, sites) in &cg.callers {
        if key == target_name || key.ends_with(&suffix) {
            out.extend(sites.iter());
        }
    }
    out
}
```

Add `pub mod call_resolve;` to `src/navigation/mod.rs`.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --test navigation_scoped_calls`
Expected: PASS (all positive + negative tests). If the C++ fixture's stored `callee_name` differs (inspect `cg.calls`), adjust that one literal.

- [ ] **Step 5: Commit**

```bash
git add src/navigation/call_resolve.rs src/navigation/mod.rs \
  tests/navigation/scoped_calls_test.rs Cargo.toml
git commit -m "feat(nav): nav-local ::-scoped call resolver, Rust+C++ (Plan 3b.5 T1)"
```

---

## Task 2: Wire the resolver into `callees` + `module-deps`/`repo-map`

**Files:**
- Modify: `src/navigation/queries.rs` (`direct_callees`), `src/navigation/module_graph.rs` (`collect_module_edges`)
- Test: `tests/navigation/scoped_calls_test.rs`

- [ ] **Step 1: Write the failing tests**

Append to `tests/navigation/scoped_calls_test.rs`:

```rust
use prism::navigation::module_graph::{module_deps, repo_map};
use prism::navigation::queries;
use prism::navigation::types::SymbolRef;

#[test]
fn callees_resolves_scoped_dispatch() {
    let s = session(&[
        ("algo.rs", "pub fn run() -> i32 { 1 }\n"),
        ("main.rs", "mod algo;\nfn dispatch() -> i32 { algo::run() }\n"),
    ]);
    let ev = queries::callees(&s, Some("dispatch"), None, None, 1).unwrap();
    assert!(
        ev.items.iter().any(|it| it.symbol.as_ref().map(|s| matches!(
            s, SymbolRef::Function { file, name, .. } if file == "algo.rs" && name == "run"
        )).unwrap_or(false)),
        "callees(dispatch) should include scoped callee algo.rs::run"
    );
}

#[test]
fn module_deps_and_repo_map_include_scoped_edge() {
    let s = session(&[
        ("algo.rs", "pub fn run() -> i32 { 1 }\n"),
        ("main.rs", "mod algo;\nfn dispatch() -> i32 { algo::run() }\n"),
    ]);
    let md = module_deps(&s, "main.rs");
    assert!(md.items.iter().any(|it| it.location.file == "algo.rs"),
        "module-deps(main.rs) should include a scoped edge to algo.rs");
    // repo-map shares collect_module_edges -> the edge must appear there too.
    let rm = repo_map(&s);
    let g = rm.graph.as_ref().unwrap();
    let main_i = g.nodes.iter().position(|n| n.location.file == "main.rs").unwrap();
    let algo_i = g.nodes.iter().position(|n| n.location.file == "algo.rs").unwrap();
    assert!(g.edges.iter().any(|e| e.from == main_i && e.to == algo_i && e.kind == "ModuleDep"));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test navigation_scoped_calls callees_resolves_scoped_dispatch`
then: `cargo test --test navigation_scoped_calls module_deps_and_repo_map_include_scoped_edge`
Expected: FAIL — the scoped callee/edge is unresolved (one filter per command).

- [ ] **Step 3: Swap the callee resolver in both call sites**

In `src/navigation/queries.rs`, `direct_callees`, replace the `resolve_callees_qualified(...)` call with:

```rust
            let resolved = crate::navigation::call_resolve::resolve_callees_nav(
                &s.index.cpg.call_graph,
                &site.callee_name,
                &site.caller.file,
                site.qualifier.as_deref(),
            );
```

In `src/navigation/module_graph.rs`, `collect_module_edges`, replace the `cg.resolve_callees_qualified(...)` call with:

```rust
            let resolved = crate::navigation::call_resolve::resolve_callees_nav(
                cg,
                &site.callee_name,
                &site.caller.file,
                site.qualifier.as_deref(),
            );
```

(Leave the loop bodies otherwise unchanged.)

- [ ] **Step 4: Run to verify pass**

Run (one target per command):
`cargo test --test navigation_scoped_calls`
`cargo test --test navigation_callees`
`cargo test --test navigation_module_graph`
Expected: PASS — new tests plus all existing callees/module-graph tests (Python fixtures delegate unchanged).

- [ ] **Step 5: Commit**

```bash
git add src/navigation/queries.rs src/navigation/module_graph.rs tests/navigation/scoped_calls_test.rs
git commit -m "feat(nav): resolve scoped calls in callees/module-deps/repo-map (Plan 3b.5 T2)"
```

---

## Task 3: Wire the resolver into `callers` (+ cross-stem false-positive guard)

**Files:**
- Modify: `src/navigation/queries.rs` (`direct_callers`)
- Test: `tests/navigation/scoped_calls_test.rs`

- [ ] **Step 1: Write the failing tests (positive + cross-stem negative)**

Append to `tests/navigation/scoped_calls_test.rs`:

```rust
#[test]
fn callers_finds_scoped_dispatcher() {
    let s = session(&[
        ("algo.rs", "pub fn run() -> i32 { 1 }\n"),
        ("main.rs", "mod algo;\nfn dispatch() -> i32 { algo::run() }\n"),
    ]);
    let ev = queries::callers(&s, Some("run"), Some("algo.rs"), None, 1).unwrap();
    assert!(
        ev.items.iter().any(|it| it.symbol.as_ref().map(|s| matches!(
            s, SymbolRef::Function { name, .. } if name == "dispatch"
        )).unwrap_or(false)),
        "callers(run@algo.rs) should include the scoped dispatcher"
    );
}

#[test]
fn callers_excludes_other_stem_scoped_call() {
    // Two files define `run`; dispatcher calls other::run. callers(run@algo.rs)
    // must NOT include it — scoped_caller_sites returns a superset the identity
    // filter prunes (the Rust/C++ stem-collision guard).
    let s = session(&[
        ("algo.rs", "pub fn run() -> i32 { 1 }\n"),
        ("other.rs", "pub fn run() -> i32 { 2 }\n"),
        ("main.rs", "mod algo;\nmod other;\nfn dispatch() -> i32 { other::run() }\n"),
    ]);
    let ev = queries::callers(&s, Some("run"), Some("algo.rs"), None, 1).unwrap();
    assert!(
        !ev.items.iter().any(|it| it.symbol.as_ref().map(|s| matches!(
            s, SymbolRef::Function { name, .. } if name == "dispatch"
        )).unwrap_or(false)),
        "callers(run@algo.rs) must exclude a dispatcher that calls other::run"
    );
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test --test navigation_scoped_calls callers_finds_scoped_dispatcher`
Expected: FAIL — `direct_callers` looks up `callers["run"]` and misses `"algo::run"`.

- [ ] **Step 3: Use scoped caller scanning + the nav resolver in `direct_callers`**

In `src/navigation/queries.rs`, replace `direct_callers`'s body so it draws sites from `scoped_caller_sites` and resolves each with the nav resolver (the identity filter is unchanged):

```rust
fn direct_callers(s: &NavigationSession, target: &FunctionId) -> Vec<(FunctionId, usize)> {
    let mut out = Vec::new();
    let cg = &s.index.cpg.call_graph;
    for site in crate::navigation::call_resolve::scoped_caller_sites(cg, &target.name) {
        let resolved = crate::navigation::call_resolve::resolve_callees_nav(
            cg,
            &site.callee_name,
            &site.caller.file,
            site.qualifier.as_deref(),
        );
        if resolved.iter().any(|f| **f == *target) {
            out.push((site.caller.clone(), site.line));
        }
    }
    out
}
```

- [ ] **Step 4: Run to verify pass**

Run (one target per command):
`cargo test --test navigation_scoped_calls`
`cargo test --test navigation_callers`
Expected: PASS — scoped dispatcher found, cross-stem dispatcher excluded, all existing callers tests unchanged.

- [ ] **Step 5: Commit**

```bash
git add src/navigation/queries.rs tests/navigation/scoped_calls_test.rs
git commit -m "feat(nav): resolve scoped callers with stem-collision guard (Plan 3b.5 T3)"
```

---

## Task 4: Live-repo dogfood confirmation + docs

**Files:**
- Modify: `tests/cli/nav_compat_test.rs`, `CLAUDE.md`, `docs/prism-query-layer/plan3b-module-map-followups.md`

- [ ] **Step 1: Add the dogfood confirmation/regression test**

This is a **confirmation test** (it passes once T1–T3 are integrated, since the resolver is already live — not a red-before-green step). Append to `tests/cli/nav_compat_test.rs`:

```rust
#[test]
fn callees_resolves_scoped_dispatch_dogfood() {
    // run_slicing_inner dispatches via scoped `original_diff::slice`-style calls;
    // before 3b.5 this resolved 0 cross-file callees, now > 0.
    let out = bin()
        .args([
            "nav", "callees", "--repo", ".", "--symbol", "run_slicing_inner",
            "--file", "src/algorithms/mod.rs", "--format", "json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let resolved_cross_file = v["items"].as_array().unwrap().iter().filter(|it| {
        it["symbol"].is_object() && it["symbol"]["Function"]["file"] != "src/algorithms/mod.rs"
    }).count();
    assert!(resolved_cross_file > 0,
        "run_slicing_inner should resolve scoped algorithm callees cross-file; got {resolved_cross_file}");
}
```

- [ ] **Step 2: Run to verify it passes**

Run: `cargo test --test cli_nav_compat callees_resolves_scoped_dispatch_dogfood`
Expected: PASS. If it fails, the Task 2 `direct_callees` swap is incomplete — fix before proceeding.

- [ ] **Step 3: Doc-sync**

- In `CLAUDE.md`, update the nav note: call resolution now covers (a) `.`-qualified package/module calls for Python/JS/TS/Go (existing), (b) unqualified/`use`-imported calls, and (c) **`::`-scoped module/namespace free functions for Rust and C++** (new). Note the remaining gaps: `Type::method` where type ≠ file stem, and cross-file **method/receiver** calls (need type info) — language-agnostic.
- In `docs/prism-query-layer/plan3b-module-map-followups.md`, mark `::`-scoped `mod::fn`/`Ns::func` resolution **delivered (Plan 3b.5, Rust + C++)** and refine the remaining items to: **`Type::method` associated functions** (need type→file map), **`ego` scoped edges** (need nav-index CPG augmentation — *explicitly warn that `ego` now returns a strictly smaller neighborhood than `callees`+`callers` for scoped-dispatch symbols*), and the language-agnostic **method/receiver resolution** (needs `type_db`).

- [ ] **Step 4: Commit**

```bash
git add tests/cli/nav_compat_test.rs CLAUDE.md docs/prism-query-layer/plan3b-module-map-followups.md
git commit -m "test(nav): scoped-dispatch dogfood + docs (Plan 3b.5 T4)"
```

---

## Task 5: Full-suite green + fmt + clippy

- [ ] **Step 1: Format** — `cargo fmt` then `cargo fmt --all -- --check` (expect clean).
- [ ] **Step 2: Full suite** — `cargo test` (expect all pass; **no golden changes** — CG fixture is Python; diff-review goldens untouched). Confirm `navigation_scoped_calls` green and `cli_nav_compat`/`navigation_callees`/`navigation_module_graph`/`navigation_callers` unchanged-or-passing.
- [ ] **Step 3: Clippy** — `cargo clippy --all-targets -- -W clippy::all` (no new warnings in `call_resolve.rs`/`queries.rs`/`module_graph.rs`).
- [ ] **Step 4: Commit** any fmt-only changes (`git commit -m "style(nav): cargo fmt (Plan 3b.5 T5)" || true`).

---

## Deferred / follow-up (language-agnostic unless noted)

- **`Type::method` / associated functions** — the ~201 `Type::method` scoped calls where the type name ≠ a file stem (`CallGraph::build` → `call_graph.rs`) need a type/module→file map. (Rust + C++.)
- **Cross-file method/receiver resolution** (`obj.method()`, `ptr->method()`) — **the largest gap (~12.8k call sites here), and it is language-agnostic** (Python/JS/TS/Go/Java/Rust/C++ alike). It is unresolved because it needs receiver-type information (the `type_db`, which nav builds as `None`). This is Tier-2/precision work, not a per-language patch.
- **`ego` scoped edges** — ego walks CPG `Call` edges built at CpgContext::build from the shared resolver; surfacing scoped edges needs nav-index CPG edge augmentation at build time (nav owns its CPG copy → still Option-C-safe, but larger). After 3b.5, `ego` is strictly smaller than `callees`+`callers` for scoped symbols — documented warning.
- **Inline-module scoped calls** (`mod m { fn f }` called `m::f()` from the same file) — `file_stem(caller) != m`; resolve when inline-module spans are modeled.
- **CPG-core scoped resolution for diff-review** — making the *shared* `resolve_callees_qualified` `::`-aware would also enrich diff-review algorithms but perturbs diff-review goldens (reviewed re-baseline). Tracked separately (relates to spec §19 #2).

---

## Self-Review

**Cross-language correctness:** verified empirically — Go `u.Helper()` resolves today via the `.`-qualifier path (no change needed); C++ `util::helper()` is unresolved today (callee `util::helper`, qualifier `None`) and is resolved by this fix exactly like Rust `mod::fn`. The resolver is `::`-triggered (not language-gated), so Rust + C++ are covered by one code path; `.`-languages are untouched (delegated result is non-empty).

**No-regression / Option C:** `call_graph.rs` untouched → diff-review + Python `CG` goldens byte-identical. The fallback only adds results when the delegated call is empty and the name is `::`-scoped; reserved keywords and non-matching stems return empty. `scoped_caller_sites` returns a superset of the old `callers[target.name]` lookup, and the identity filter prunes wrong-stem callers (pinned by `callers_excludes_other_stem_scoped_call`).

**Per-task green contract:** T1 resolver + its tests (isolated, green). T2 callees+module-deps+repo-map. T3 callers + cross-stem guard. T4 dogfood confirmation + docs. T5 verify. Single-filter / whole-target test commands throughout. No non-compiling commit.

**Type consistency:** `resolve_callees_nav(&CallGraph, &str, &str, Option<&str>) -> Vec<&FunctionId>` mirrors `resolve_callees_qualified`; `scoped_caller_sites(&CallGraph, &str) -> Vec<&CallSite>`. Reads only `pub` `CallGraph` fields.
