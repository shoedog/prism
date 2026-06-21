# Prune Through use/re-export Imports Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Recover the same-name owner-`::` collisions that today sit at `NameOnly` only because the owner type is reached through a `use`/re-export (`use ty::CliTest; CliTest::with_file()`). Extend the one shipped disproof predicate's leading-segment gate (PR #121's ②B) so a leading type segment bound through a **single `use` chain that resolves unambiguously to exactly one scope-bearing in-repo `Item`** is treated as directly bound — letting the prune proceed (`recovery_typepath.failopen_demote` → `singleton`). Recall-safe: any ambiguous / external / poisoned / unresolved / non-scope-bearing import keeps-all.

**Architecture:** ONE production change, no engine/cache change. The disproof body (`ScopeResolution::disproves`, `src/resolution.rs:1327`) already folds `use`-imports correctly in its **final** callable step (`rust_graph_qualified_callable_edge` + `graph_target_ids`, `:1373-1380`): for `use ty::CliTest; CliTest::with_file()` the id-set is `{ty::CliTest::with_file}`, so the other `CliTest`'s method is already disproved. The *only* thing stopping the `Pending` case from reaching that step is the leading-segment directness gate `leading_segment_binds_directly` (`:1430-1433`), which returns `false` on a `Pending` binding. We add a `Pending` arm to that final `match` plus one small local helper `pending_resolves_to_single_in_repo_item` that re-resolves **the binding's own anchored import path** via the EXISTING `resolve_path` import and returns `true` iff it yields `ResStatus::Resolved` with exactly one `Target::Item { owns: Some(scope), .. }` whose `scope` maps to an in-repo `FileId` (via `graph_file_for_scope`). Everything else → keep-all (decline → no disproof → full pool retained at `NameOnly`).

**Tech Stack:** Rust, tree-sitter, `petgraph`; the existing `name_resolution` scope-graph engine (`engine::resolve_path`, `RustPolicy`, `NS_TYPE`, `BindTarget::Pending`, `Target::Item`, `graph_file_for_scope`); `cargo test`; the Tier-A accuracy harness (`eval/`, `uv run tier-a`).

---

## Premises that govern every task (from spec §1/§4)

- **P1 — recall-safety.** Never drop a real edge. The gate is **decline-only**: it can only let an *already-sound* final-step disproof proceed, or keep-all. Every uncertain import shape (`ResolvedSet`/`Ambiguous`/`Poisoned`/`Unresolved`, a `Target::External`/`Target::Local` candidate, `owns: None`, or >1 candidate) returns `false` → keep-all → full pool at `NameOnly`. A wrong drop is worse than a wrong demote.
- **P2 — the gate is not load-bearing on the disproof id-set.** The actual disproof id-set always comes from the final step (`:1373-1380`); the gate's only job is to decline on a non-single-in-repo import. The helper re-resolves the **binding's own anchored path** (the same `resolve_path` call shape as the final callable step at `:1554`) so the two follow the same `use` chain — sound for a non-bare anchor like `crate::Foo::m` where a bare lookup and the anchored path could inspect different roots.
- **P3 — no serialized change.** This is a resolution-time predicate decision. No `engine` change, no `Provenance` change, **no `CACHE_VERSION` bump**.

## Executor / commit protocol (READ FIRST)

The executor is **codex under `workspace-write`** — it can edit files and run `cargo`, but it **cannot run `git commit`**. For each task this plan SHOWS the exact commit message + the precise file set to stage; the **host** performs the commit after the task's tests pass. Do **not** stage `eval/` or `docs/eval/` artifacts in any commit. Each commit message ends with the trailer:

```
Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
```

## macOS test-runner note (READ FIRST)

`--lib`, `--test integration`, and `--test ast` run normally on this machine. A bare `cargo test --test cli` may stall at `_dyld_start`. For **CLI** tests only, compile without running, then run the freshest non-debug-artifact binary:

```bash
cargo test --test cli --no-run 2>&1 | tail -3
CLI_BIN=$(ls -t target/debug/deps/cli-* | grep -vE '\.(d|dSYM)$' | head -1)
"$CLI_BIN" --list 2>&1 | tail -5
```

`tests/ast/cpg_test.rs` is **not** covered by `--lib` or `--test integration`; it is the CHA target that bit #121 — run it explicitly with `cargo test --test ast cpg_test::`.

## Accuracy-harness reminder

This change touches call resolution (`src/resolution.rs`). There is **no** `CACHE_VERSION` bump, so warm nav caches are not auto-invalidated — clear them or pass `--no-cache` on every `call-stats` read (Task 3). Per CLAUDE.md, Task 3 runs the **required** Tier-A `--matrix-only` (pre-commit) **and** `--quick --allow-stale-sut` (pre-review) gates plus the spec §7 `--corpus ruff` M2 recall-safety acceptance (required, or explicitly host-triggered with the deferral noted in the PR — not optional); do **not** stage `eval/` or `docs/eval/` artifacts in any commit.

---

## Task 1: The `Pending` arm + helper, headline recovery (red→green) — spec §3/§4/§6.1

The single production change plus the headline red→green fixture. The existing `scope_graph_pending_import_alias_over_colliding_pool_keeps_all` (`tests/integration/resolution_test.rs:1910`) currently asserts keep-all (2 NameOnly); under the change it **flips to a single Exact** = `src/a.rs`'s `Foo::m`. We update it in place (per spec §6.1 — do not add a duplicate).

**Files:**
- Test: `tests/integration/resolution_test.rs` (rewrite the body of `scope_graph_pending_import_alias_over_colliding_pool_keeps_all` at `:1910`).
- Modify: `src/resolution.rs` — the `leading_segment_binds_directly` final `match` (`:1430-1433`) + add the `pending_resolves_to_single_in_repo_item` helper directly after `leading_segment_binds_directly` (after `:1434`).

### Step 1: Flip the headline test to assert the recovery (RED)

Replace the whole body of `scope_graph_pending_import_alias_over_colliding_pool_keeps_all`. The sources are unchanged (single crate, `mod a; mod b; use crate::a::Foo; Foo::m()` over colliding `a::Foo`/`b::Foo`); only the rename + assertions change. Rename it to `scope_graph_pending_import_alias_over_colliding_pool_recovers_to_single_exact` to describe the new behavior, and assert single Exact targeting `src/a.rs`.

Replace this exact block in `tests/integration/resolution_test.rs`:

```rust
#[test]
fn scope_graph_pending_import_alias_over_colliding_pool_keeps_all() {
    use prism::languages::Language::Rust;
    // The leading segment `Foo` binds at the call site via a named import
    // (`BindTarget::Pending`), not a direct resolved item. That uncertainty keeps
    // the full colliding owner pool rather than pruning to `a::Foo::m`.
    let sources = [
        (
            "src/lib.rs",
            "mod a;\nmod b;\nuse crate::a::Foo;\npub fn drive() {\n    Foo::m();\n}\n",
            Rust,
        ),
        (
            "src/a.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/b.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
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
        "Pending import directness keeps the full colliding pool at NameOnly",
    );
}
```

with:

```rust
#[test]
fn scope_graph_pending_import_alias_over_colliding_pool_recovers_to_single_exact() {
    use prism::languages::Language::Rust;
    // The leading segment `Foo` binds at the call site via a SINGLE named import
    // (`use crate::a::Foo;` -> `BindTarget::Pending`) that resolves unambiguously to
    // one in-repo item (`a::Foo`). The prune-through-`use` slice folds that import
    // through the engine and recovers the colliding pool to the single Exact
    // `a::Foo::m` -- the other `b::Foo::m` is disproved.
    let sources = [
        (
            "src/lib.rs",
            "mod a;\nmod b;\nuse crate::a::Foo;\npub fn drive() {\n    Foo::m();\n}\n",
            Rust,
        ),
        (
            "src/a.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/b.rs",
            "pub struct Foo;\nimpl Foo {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    assert_eq!(
        cg.methods
            .get(&("Foo".to_string(), "m".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key must collide across a::Foo and b::Foo",
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "Foo::m"));
    assert_eq!(out.drop, None, "the recovered owner path must not drop");
    assert_eq!(
        out.resolved.len(),
        1,
        "a single `use`-imported in-repo owner recovers to one candidate",
    );
    assert_eq!(
        out.resolved[0].target.file, "src/a.rs",
        "the recovered target is the imported `a::Foo`'s method, not `b::Foo`'s",
    );
    assert_eq!(
        out.resolved[0].confidence,
        ResolutionConfidence::Exact,
        "pruning through the single `use` chain mints Exact",
    );
    assert_eq!(out.resolved[0].kind, ResolutionKind::QualifiedOwner);
}
```

Run it — it must FAIL on current code (the pool is kept at 2 NameOnly, so `out.resolved.len()` is 2 and the `== 1` assert fails):

```bash
cargo test --test integration \
  resolution_test::scope_graph_pending_import_alias_over_colliding_pool_recovers_to_single_exact \
  -- --exact 2>&1 | tail -20
```

Expected: a failure at `assert_eq!(out.resolved.len(), 1, ...)` showing `left: 2, right: 1` (RED — confirms current code keeps-all on `Pending`).

### Step 2: Add the `Pending` arm + helper (minimal impl)

In `src/resolution.rs`, replace the final `match` of `leading_segment_binds_directly` (currently `:1430-1433`):

```rust
    match visible.as_slice() {
        [b] => matches!(&b.target, BindTarget::Resolved(Target::Item { .. })),
        _ => false,
    }
```

with the `Pending` arm (the `graph`, `q`, and `policy` locals are already in scope in this fn — `policy` at `:1396`, `q` at `:1410`, `at` at `:1395`):

```rust
    match visible.as_slice() {
        [b] => match &b.target {
            // Directly bound in-repo type -- unchanged (②B).
            BindTarget::Resolved(Target::Item { .. }) => true,
            // (A) slice: a single `use`/re-export chain. Fold THIS binding's own
            // import path via the engine; prune only if it resolves UNAMBIGUOUSLY
            // to one scope-bearing in-repo `Item` (Rust `use` resolution is
            // deterministic). Ambiguous / poisoned / unresolved / `Target::External`
            // / a non-scope-bearing item / multiple -> keep-all (we do NOT prune).
            BindTarget::Pending(path, anchor) => pending_resolves_to_single_in_repo_item(
                graph, path, anchor, b.scope, b.ns, &q.at, &policy,
            ),
            _ => false,
        },
        _ => false,
    }
```

Then add this helper directly after the closing brace of `leading_segment_binds_directly` (after `:1434`, before `leading_segment_has_block_local_shadow`):

```rust
/// (A) slice helper: does the leading type segment's single visible `Pending`
/// `use`/re-export binding resolve UNAMBIGUOUSLY to exactly one scope-bearing
/// in-repo `Item`? Re-resolves the binding's **own anchored import path** via the
/// same `resolve_path` call shape the final callable step uses (`resolution.rs`
/// final step), so the gate follows the same `use` chain. Returns `true` only on
/// `ResStatus::Resolved` with a single `Target::Item { owns: Some(scope), .. }`
/// whose defining `scope` maps to a known in-repo `FileId`; every other shape
/// (`ResolvedSet`/`Ambiguous`/`Poisoned`/`Unresolved`, a `Target::External`/
/// `Target::Local` candidate, `owns: None`, or >1) -> `false` -> keep-all. Note
/// there is no `External` engine *status*: externals surface as a
/// `Target::External` candidate *target* under a `Resolved` result, so we inspect
/// the candidate target, not the status (spec §3/§4).
#[allow(clippy::too_many_arguments)]
fn pending_resolves_to_single_in_repo_item(
    graph: &ScopeGraph,
    path: &RawPath,
    anchor: &Anchor,
    from: ScopeId,
    final_ns: NamespaceId,
    at: &SourceLoc,
    policy: &RustPolicy,
) -> bool {
    // `from` is the re-export author's scope (`b.scope`); `final_ns` is the final
    // segment's namespace (`b.ns`, NS_TYPE for the type binding); the prefix
    // (scope-bearing) segments use NS_TYPE.
    let res = resolve_path(graph, path, final_ns, anchor, from, NS_TYPE, at, policy);
    matches!(
        (res.status, res.candidates.as_slice()),
        (
            ResStatus::Resolved,
            [Candidate {
                target: Target::Item { owns: Some(scope), .. },
                ..
            }],
        ) if graph_file_for_scope(graph, *scope).is_some()
    )
}
```

`NamespaceId` is **not** currently imported in `src/resolution.rs` (verified) — add it to the existing `use crate::name_resolution::types::{ ... }` import list at `:11-14`. Concretely, change:

```rust
use crate::name_resolution::types::{
    Anchor, AnchorKind, BindTarget, Binding, Candidate, Edge, FileId, RawPath, ResStatus,
    ResolutionPolicy, ResolveQuery, ScopeId, SourceLoc, Span, Target, TraversalCtx,
};
```

to add `NamespaceId` (alphabetical, between `Edge` and `FileId`):

```rust
use crate::name_resolution::types::{
    Anchor, AnchorKind, BindTarget, Binding, Candidate, Edge, FileId, NamespaceId, RawPath,
    ResStatus, ResolutionPolicy, ResolveQuery, ScopeId, SourceLoc, Span, Target, TraversalCtx,
};
```

(Everything else the helper names — `resolve_path` at `:7`, `RawPath`/`Anchor`/`ScopeId`/`SourceLoc`/`ResStatus`/`Candidate`/`Target` at `:11-14`, `NS_TYPE`/`RustPolicy` at `:9`, `graph_file_for_scope` at `:1615`, `ScopeGraph` at `:8` — is already in scope; **no new `engine::resolve` import**.)

> **Note on `policy: &RustPolicy`.** `resolve_path`'s last parameter is `&dyn ResolutionPolicy`; passing `&policy` (a `&RustPolicy`) coerces to the trait object automatically because `RustPolicy` implements `ResolutionPolicy` (`rust_policy.rs:147`). The helper takes `&RustPolicy` (not `&dyn`) so the caller passes `&policy` (the local at `:1396`) directly.

Run the headline test again — GREEN:

```bash
cargo test --test integration \
  resolution_test::scope_graph_pending_import_alias_over_colliding_pool_recovers_to_single_exact \
  -- --exact 2>&1 | tail -20
```

Expected: `test result: ok. 1 passed`.

### Step 3: Format + commit

```bash
cargo fmt
cargo fmt --check
```

**Host commits** (stage exactly these two files):

```bash
git add src/resolution.rs tests/integration/resolution_test.rs
```

Commit message:

```
feat(resolution): prune owner collisions through a single in-repo `use` import

Extend the ScopeResolution disproof gate's leading-segment directness
contract (#121's ②B) with a `Pending` arm: a leading type segment reached
through a SINGLE `use`/re-export chain that resolves unambiguously to one
scope-bearing in-repo `Item` is treated as directly bound, so the existing
final callable step prunes the same-name owner collision to a single Exact.
Re-resolves the binding's own anchored import path via the existing
`resolve_path` (same call shape as the final step); every other import shape
(ResolvedSet/Ambiguous/Poisoned/Unresolved, External/Local candidate,
owns:None, or >1) keeps-all. No engine/cache change. Headline fixture
`scope_graph_pending_import_alias_over_colliding_pool_*` flips keep-all ->
single Exact.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
```

---

## Task 2: Keep-all coverage + drop/decline invariants — spec §4/§6.2-§6.6

Pin the keep-all branches (cross-crate `use` → `Unresolved`, `extern crate` → `Target::External` candidate, module glob, ambiguous re-export, the unchanged direct-binding recovery) and re-confirm the drop invariants + the trait-CHA decline. Each new fixture must provably reach the `Pending` arm (a **single** visible `Pending` binding, `visible.as_slice() == [b]`) so it pins the helper's branches — a shape that surfaces 2 visible bindings is filtered by the outer `_ => false` arm *before* the helper and would not exercise it (spec §6 preamble).

**Files:**
- Test: `tests/integration/resolution_test.rs` — add four fixtures next to the headline test (after `scope_graph_pending_import_alias_over_colliding_pool_recovers_to_single_exact`): the cross-crate `use` → `Unresolved` keep-all (1a), the `extern crate` → `Target::External` keep-all (1b), the ambiguous re-export keep-all, and the module-level glob keep-all. The glob keep-all and the direct-binding recovery already exist (`scope_graph_block_local_glob_shadow_keeps_all:1805`, `scope_graph_two_crate_owner_collision_recovers_to_single_exact:1692`) — those are re-run, not rewritten. Add a **module-level** glob keep-all to pin the "no single visible Pending for the leading segment" path (distinct from the existing block-local glob shadow, which trips ①C upstream).

### Step 1: Cross-crate `use` keeps-all (the realistic external case → `Unresolved`) + an explicit `extern crate` `Target::External` guard — spec §6.2

Two fixtures pin the two distinct fail-closed engine shapes a non-in-repo import surfaces — **both** route through the helper's `else → false → keep-all` arm, but they exercise different engine results so a future engine change can't silently start pruning either (risk §8 "pin with the external/ambiguous/poison fixtures"):

**(1a) The realistic cross-crate `use` → `Unresolved`.** A **bare-leading-ident multi-segment** `use some_external::CliTest as CliTest;` is the everyday cross-crate import shape. `some_external` is the non-final prefix segment and is **not** an in-repo crate/module, so `resolve_path_guarded` fails the prefix-segment scope lookup (`engine.rs:357-364`, non-`Resolved`/non-`Poisoned` → `_ => return unresolved()`) and the helper sees `ResStatus::Unresolved` → `false` → keep-all (full pool at NameOnly), not dropped. (This is NOT a `Target::External` candidate — see (1b). `Target::External` is produced only by an `extern crate` binding at the crate root [`items.rs:139-163`], not by a `use` chain that fails on a non-final segment.)

Add:

```rust
#[test]
fn scope_graph_pending_cross_crate_use_keeps_all() {
    use prism::languages::Language::Rust;
    // A SINGLE visible `Pending` whose multi-segment `use` chain leaves the repo
    // (`use some_external::CliTest as CliTest;`). `some_external` is not an in-repo
    // crate, so the engine fails the non-final prefix segment and returns
    // `ResStatus::Unresolved` -> the helper declines -> keep-all. We must NOT prune
    // to an in-repo `CliTest` we happen to also own. (The realistic cross-crate
    // `use` shape; the `Target::External` candidate branch is pinned separately
    // below by an explicit `extern crate`.)
    let sources = [
        (
            "src/lib.rs",
            "mod a;\nmod b;\nuse some_external::CliTest as CliTest;\npub fn drive() {\n    CliTest::m();\n}\n",
            Rust,
        ),
        (
            "src/a.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/b.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    assert_eq!(
        cg.methods
            .get(&("CliTest".to_string(), "m".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key must collide across a::CliTest and b::CliTest",
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "CliTest::m"));
    assert_eq!(out.drop, None, "an unresolved cross-crate import alias must not drop");
    assert!(
        out.resolved.len() == 2
            && out
                .resolved
                .iter()
                .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "an unresolved external `use` chain declines -> keep the full colliding pool at NameOnly",
    );
}
```

**(1b) An explicit `extern crate` → a `Resolved` + `Target::External` candidate.** This is the fixture that genuinely reaches the `Target::External` candidate keep-all branch the helper guards against. `extern crate some_external;` binds `some_external` as a Type at the crate root targeting `BindTarget::Resolved(Target::External(..))` (the crate is not in `workspace_members`, so `crate_root_named` returns `None` → `Target::External`, `items.rs:160-163`). `use some_external as CliTest;` is then a **single-segment** `Pending` whose own path (`["some_external"]`) the helper re-resolves: single segment → `is_last` → `scope_member_lookup` finds the `extern crate` binding → `ResStatus::Resolved` with a single `Target::External` candidate. The helper requires `Target::Item { owns: Some(_), .. }`, so a `Target::External` candidate does **not** match → `false` → keep-all. (Verified cleanly constructible under the `build()` harness: `from_convention` keys no crate name for the conventional `src/lib.rs` root, so `crate_root_named("some_external")` is `None`.)

Add:

```rust
#[test]
fn scope_graph_pending_extern_crate_alias_keeps_all() {
    use prism::languages::Language::Rust;
    // A SINGLE visible `Pending` (`use some_external as CliTest;`) whose own path is
    // the single segment `some_external`, bound by `extern crate some_external;` to a
    // `Target::External` candidate at the crate root (not an in-repo crate). The
    // helper re-resolves that path to `ResStatus::Resolved` with one `Target::External`
    // candidate -> not a `Target::Item` -> declines -> keep-all. This is the fixture
    // that genuinely exercises the `Target::External` candidate branch.
    let sources = [
        (
            "src/lib.rs",
            "extern crate some_external;\nmod a;\nmod b;\nuse some_external as CliTest;\npub fn drive() {\n    CliTest::m();\n}\n",
            Rust,
        ),
        (
            "src/a.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/b.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    assert_eq!(
        cg.methods
            .get(&("CliTest".to_string(), "m".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key must collide across a::CliTest and b::CliTest",
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "CliTest::m"));
    assert_eq!(out.drop, None, "an `extern crate` external alias must not drop");
    assert!(
        out.resolved.len() == 2
            && out
                .resolved
                .iter()
                .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "a `Target::External` candidate declines -> keep the full colliding pool at NameOnly",
    );
}
```

> **If `Target::External` proves not cleanly reachable in practice** (e.g. the `extern crate` binding does not survive the `build()` convention config as expected): both `Unresolved` and `Target::External` are the **same fail-closed `else → keep-all` branch** of the helper, so the (1a) `Unresolved` fixture already covers the realistic cross-crate `use` case and is sufficient for recall-safety. Keep (1b) only if it compiles and reaches the `Target::External` candidate as described; otherwise drop it and note in the PR that the `Unresolved` fixture covers the cross-crate case (the helper's `_ => false` handles both). Verify the (1b) shape empirically when implementing (assert the keep-all behavior; the comment documents the *intended* engine path).

### Step 2: Ambiguous re-export keeps-all (`Ambiguous` branch via the `Pending` arm) — spec §6.4

A **single** visible `Pending` (`use crate::facade::CliTest;`) whose target module `facade` re-exports `CliTest` ambiguously (`pub use crate::a::CliTest;` + `pub use crate::b::CliTest;`). The binding's import path resolves `Ambiguous` → helper `false` → keep-all. This pins the `Ambiguous` branch *through* the `Pending` arm (the bare "two top-level `use`s of `CliTest`" shape instead yields 2 visible bindings → `_ => false` before the helper, so it does **not** pin this branch — spec §6.4).

Add:

```rust
#[test]
fn scope_graph_pending_ambiguous_reexport_keeps_all() {
    use prism::languages::Language::Rust;
    // ONE visible `Pending` (`use crate::facade::CliTest;`) whose target module
    // re-exports `CliTest` from TWO crates ambiguously. The binding's import path
    // resolves `Ambiguous`, so the helper declines -> keep-all. Pins the
    // `Ambiguous` branch via the `Pending` arm (a single visible binding).
    let sources = [
        (
            "src/lib.rs",
            "mod a;\nmod b;\nmod facade;\nuse crate::facade::CliTest;\npub fn drive() {\n    CliTest::m();\n}\n",
            Rust,
        ),
        (
            "src/a.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/b.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/facade.rs",
            "pub use crate::a::CliTest;\npub use crate::b::CliTest;\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    assert_eq!(
        cg.methods
            .get(&("CliTest".to_string(), "m".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key must collide across a::CliTest and b::CliTest",
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "CliTest::m"));
    assert_eq!(out.drop, None, "an ambiguous re-export must not drop");
    assert!(
        out.resolved.len() == 2
            && out
                .resolved
                .iter()
                .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "an ambiguous `use` re-export declines -> keep the full pool at NameOnly",
    );
}
```

### Step 3: Module-level glob keeps-all (no single visible `Pending`) — spec §6.3

`use crate::ru::*;` at module level brings `CliTest` via a glob `Edge`, not a `Binding`, so there is no single visible `Pending` for the leading segment → the rib for `CliTest` is empty → `leading_segment_binds_directly` returns `false` at the `rib.is_empty()` early return (`:1406`) → keep-all. (This is the module-level analogue of the existing block-local `scope_graph_block_local_glob_shadow_keeps_all`, which instead trips ①C upstream — spec §6.3.)

Add:

```rust
#[test]
fn scope_graph_module_glob_import_keeps_all() {
    use prism::languages::Language::Rust;
    // Module-level `use crate::ru::*;` brings `CliTest` via a glob EDGE, not a
    // `Binding`, so there is no single visible `Pending` for the leading segment.
    // `leading_segment_binds_directly` finds an empty rib and declines -> keep-all.
    let sources = [
        (
            "src/lib.rs",
            "mod ty;\nmod ru;\nuse crate::ru::*;\npub fn drive() {\n    CliTest::m();\n}\n",
            Rust,
        ),
        (
            "src/ty.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
        (
            "src/ru.rs",
            "pub struct CliTest;\nimpl CliTest {\n    pub fn m(&self) {}\n}\n",
            Rust,
        ),
    ];
    let (cg, _) = build(&sources);
    assert_eq!(
        cg.methods
            .get(&("CliTest".to_string(), "m".to_string()))
            .map(|v| v.len()),
        Some(2),
        "the bare owner key must collide across ty::CliTest and ru::CliTest",
    );
    let out = cg.resolve_call_site_full(&site_in(&cg, "drive", "CliTest::m"));
    assert_eq!(out.drop, None, "a glob import must not drop");
    assert!(
        out.resolved.len() == 2
            && out
                .resolved
                .iter()
                .all(|c| c.confidence == ResolutionConfidence::NameOnly),
        "a module-level glob keeps the full colliding pool at NameOnly",
    );
}
```

### Step 4: Run the three new fixtures + the unchanged direct-binding + glob + drop/decline invariants

The three new fixtures must pass with the Task-1 change in place:

```bash
cargo test --test integration resolution_test:: 2>&1 | tail -25
```

Expected: the whole `resolution_test::` suite passes, including:
- `scope_graph_pending_cross_crate_use_keeps_all` (NEW, §6.2 — `Unresolved`),
- `scope_graph_pending_extern_crate_alias_keeps_all` (NEW, §6.2 — `Target::External` candidate; drop if not cleanly reachable per Step 1's note),
- `scope_graph_pending_ambiguous_reexport_keeps_all` (NEW, §6.4),
- `scope_graph_module_glob_import_keeps_all` (NEW, §6.3),
- `scope_graph_pending_import_alias_over_colliding_pool_recovers_to_single_exact` (Task 1),
- `scope_graph_two_crate_owner_collision_recovers_to_single_exact` (§6.5 — direct binding still Exact, unchanged),
- `scope_graph_block_local_glob_shadow_keeps_all`, `scope_graph_block_local_exact_use_shadow_keeps_all`, `scope_graph_macro_wildcard_shadow_keeps_all`, `scope_graph_inherent_plus_trait_owner_demotes_not_drops`, `scope_graph_unresolved_owner_path_keeps_full_pool_not_drop`, `scope_graph_non_uniform_edition_keeps_all` (§6.6 — drop/decline invariants unchanged),
- the three shipped drop invariants (the `process()` poison-suppression / owner-collision drops elsewhere in the file).

Then the CHA decline target that #121 bit — **not** in `--lib`/`--integration` (spec §6.6):

```bash
cargo test --test ast cpg_test::cha_upgrades_graph_resolved_owner_pair_to_exact -- --exact 2>&1 | tail -10
```

Expected: `test result: ok. 1 passed` (the `A::draw()` site binds `A` directly — a local `struct A` — so ②B already held; the `Pending` arm does not touch the direct path, and `Render::draw(x)` still declines to the authoritative-Exact path, not the legacy TraitCha edge).

### Step 5: Format + commit

```bash
cargo fmt
cargo fmt --check
```

**Host commits** (stage exactly this file):

```bash
git add tests/integration/resolution_test.rs
```

Commit message:

```
test(resolution): pin prune-through-`use` keep-all branches

Cover the helper's decline branches via the `Pending` arm: a cross-crate
`use` (multi-segment, non-in-repo prefix -> `Unresolved`), an `extern crate`
alias (single-segment -> a `Resolved` `Target::External` candidate), a
single-visible-`Pending` ambiguous re-export (`Ambiguous`), and a module-level
glob (no single visible `Pending`, empty rib) -- each keeps the full colliding
owner pool at NameOnly. The existing direct-binding recovery, block-local
shadow keep-alls, and the trait-CHA decline (`tests/ast cpg_test::`) are re-run
unchanged.

Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>
```

---

## Task 3: Acceptance (non-code) — spec §7

Build, run the full relevant test surface, format-check, measure the recovery signal on the real corpora, and run the Tier-A recall gate. **No commit** in this task (it is verification only); record the realized deltas in the PR description.

**Files:** none (verification).

### Step 1: Release build

```bash
cargo build --release 2>&1 | tail -5
```

Expected: `Finished `release` profile`.

### Step 2: Full relevant test surface (macOS-correct)

```bash
cargo test --lib 2>&1 | tail -5
cargo test --test integration 2>&1 | tail -5
cargo test --test ast 2>&1 | tail -5
```

Expected: each prints `test result: ok.` with `0 failed`.

CLI suite via the no-run pattern (a bare `cargo test --test cli` may stall at `_dyld_start`):

```bash
cargo test --test cli --no-run 2>&1 | tail -3
CLI_BIN=$(ls -t target/debug/deps/cli-* | grep -vE '\.(d|dSYM)$' | head -1)
"$CLI_BIN" 2>&1 | tail -5
```

Expected: the CLI test binary reports `0 failed`.

### Step 3: Format check

```bash
cargo fmt --check
```

Expected: no output (clean).

### Step 4: Recovery signal — `call-stats` on ruff / prism / ripgrep (spec §7)

There is no `CACHE_VERSION` bump, so pass `--no-cache` to force a fresh read. Capture the `recovery_typepath` and `kind_exact`/`kind_nameonly[qualified_owner]` buckets on **both** `main` (the pre-change baseline) and this branch, and report the **realized per-anchor delta** — NOT the §1 upper bound (ruff ~1,586 / ripgrep 161 / prism 15 are the `failopen_demote` ceilings; the realized recovery is the subset that additionally has a single visible `Pending` leading binding resolving to one scope-bearing in-repo `Item`, so a delta below the ceiling is expected per §1's misses and is **not** a regression).

For each corpus path `<REPO>` in `{ruff, prism, ripgrep}`:

```bash
# branch (current worktree, already built --release in Step 1)
./target/release/prism nav --no-cache call-stats --repo <REPO> --format json \
  | python3 -c 'import json,sys; d=json.load(sys.stdin); print("recovery_typepath", d.get("recovery_typepath")); print("kind_exact.qualified_owner", d.get("kind_exact",{}).get("qualified_owner")); print("kind_nameonly.qualified_owner", d.get("kind_nameonly",{}).get("qualified_owner"))'
```

To get the `main` baseline, check out `main` in a scratch worktree (or `git stash`-free second clone), `cargo build --release`, and run the same command; diff the two.

Expected direction (the acceptance signal):
- `recovery_typepath.singleton` **rises** and `recovery_typepath.failopen_demote` **falls** by the same realized amount (the `use`-imported collisions move demote → singleton);
- `kind_exact[qualified_owner]` **rises**, `kind_nameonly[qualified_owner]` **falls**;
- `recovery_typepath.pruned_multiple` and the drop classes do **not** rise (no recall loss).

Report the measured `(singleton↑, failopen_demote↓)` pair per corpus in the PR description. A non-zero rise on ruff (the corpus with the largest `use`-imported collision population) is the headline; prism/ripgrep may show small or zero deltas.

### Step 5: Tier-A recall gate — REQUIRED (spec §7 + CLAUDE.md)

This change touches **call resolution** (`src/resolution.rs`), so CLAUDE.md's
Tier-A discipline applies: `--matrix-only` before committing **and** `--quick
--allow-stale-sut` before the final review are **both REQUIRED** (`--allow-stale-sut`
only with the immediate preceding `--release` rebuild in this worktree).

**(a) Matrix gate (REQUIRED — before committing):**

```bash
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut 2>&1 | tail -30
```

Expected: matrix **0 regression** (no `ok`→`gap` flips; same validity as the committed baseline in `docs/eval/tier-a/`).

**(b) Quick gate (REQUIRED — before the final review, per CLAUDE.md):**

```bash
cd eval && uv run tier-a --quick --allow-stale-sut 2>&1 | tail -30
```

Expected: **0 regression** vs the committed baseline (no LSP-confirmed `ok`→`gap` flips). Needs `rust-analyzer`; runs in minutes.

**(c) ruff M2 real-corpus recall-safety acceptance (spec §7 — REQUIRED, or explicitly host/human-triggered, NOT optional):**

The spec's recall-safety acceptance is a `--corpus ruff` M2 — the same clean validator #121 used (ruff is pinned → valid, regression-classified). It is the headline real-corpus guard for this slice (ruff has the largest `use`-imported collision population, so it is where a wrong drop would show). Run it before the final review; if the host defers it (e.g. rust-analyzer not provisioned in this worktree), the deferral must be **explicit in the PR** and host/human-triggered — it is not silently optional:

```bash
cd eval && uv run tier-a --corpus ruff --quick --allow-stale-sut 2>&1 | tail -30
```

Expected: **0 regression**.

> **Do not** stage `eval/` or `docs/eval/` artifacts produced by these runs. Paste any flip-candidates into the PR description rather than re-baselining.

### Step 6: Final review

Request an independent **codex (gpt-5.5, xhigh)** diff review of the branch per the established loop. Fold any findings as fix-up commits (host-committed) with the same trailer. The headline acceptance to call out in the review request: the gate is decline-only (the disproof id-set still comes from the final step `:1373-1380`), `resolve_path` re-resolves the binding's own anchored path (not a bare lookup), and every non-single-in-repo import shape keeps-all (pinned by the §6.2/§6.3/§6.4 fixtures).

---

## Self-review checklist (spec coverage → task)

- **§3 the change (gate arm)** → Task 1 Step 2 (the `Pending` arm exactly per spec, `BindTarget::Resolved(Target::Item{..}) => true` unchanged + `BindTarget::Pending(path, anchor) => pending_resolves_to_single_in_repo_item(graph, path, anchor, b.scope, b.ns, &q.at, &policy)` + `_ => false`).
- **§3 the helper** → Task 1 Step 2 (`pending_resolves_to_single_in_repo_item`: reuses the existing `resolve_path` import, `final_ns`/prefix `NS_TYPE`, `ResStatus::Resolved` + single `Target::Item{owns:Some(scope),..}` + `graph_file_for_scope(graph,*scope).is_some()`; no engine/`Provenance`/`CACHE_VERSION` change).
- **§4 soundness / decline-only** → Task 1 P1/P2 premises + Task 2 §6.2/§6.3/§6.4 keep-all fixtures; the helper inspects the candidate **target** (no `External` status).
- **§6.1 headline red→green** → Task 1 (flip `scope_graph_pending_import_alias_over_colliding_pool_*` in place to single Exact = `src/a.rs`).
- **§6.2 external** → Task 2 Step 1 — (1a) cross-crate `use some_external::CliTest as CliTest;` → `Unresolved` keep-all (the realistic case), plus (1b) `extern crate some_external; use some_external as CliTest;` → a `Resolved`+`Target::External` candidate keep-all (the genuine `Target::External` branch; both are the helper's `else → false`, so (1a) suffices for recall-safety if (1b) is not cleanly reachable).
- **§6.3 glob** → Task 2 Step 3 (module-level `use crate::ru::*;`) + the existing block-local glob re-run.
- **§6.4 ambiguous re-export** → Task 2 Step 2 (single visible `Pending` via `facade` re-exporting from two crates).
- **§6.5 direct binding unchanged** → Task 2 Step 4 (re-run `scope_graph_two_crate_owner_collision_recovers_to_single_exact`).
- **§6.6 drop invariants + ①C/②B + trait-CHA decline** → Task 2 Step 4 (re-run `resolution_test::` block-local/macro/edition fixtures + `tests/ast cpg_test::cha_upgrades_graph_resolved_owner_pair_to_exact`).
- **§7 acceptance** → Task 3 (build/test/fmt + `call-stats` realized-delta on ruff/prism/ripgrep + REQUIRED Tier-A `--matrix-only` (pre-commit) + `--quick --allow-stale-sut` (pre-review, per CLAUDE.md) + the spec §7 `--corpus ruff` M2 recall-safety acceptance (required-or-explicitly-host-triggered) + codex xhigh review).

**Placeholder scan:** no `TODO`/`...`/`<fill-in>` in any code block; every fixture is complete Rust source; every command is runnable.

**Type/signature consistency (verified against source):**
- `resolve_path(graph, path: &RawPath, ns: NamespaceId, anchor: &Anchor, from: ScopeId, anchor_ns: NamespaceId, at: &SourceLoc, policy: &dyn ResolutionPolicy) -> Resolution` (`engine.rs:54`) — the helper passes `(graph, path, final_ns, anchor, from, NS_TYPE, at, policy)`; `&RustPolicy` coerces to `&dyn ResolutionPolicy`.
- `BindTarget::Pending(RawPath, Anchor)` (`types.rs:387`) — `path: &RawPath`, `anchor: &Anchor` ✓.
- `Target::Item { id, ns, owns: Option<ScopeId>, callable }` (`types.rs:366`) — matched with `owns: Some(scope), ..` ✓.
- `Candidate { target, cond, provenance }` (`types.rs:445`) — matched with `target: ..., ..` ✓.
- `ResStatus::{Resolved, ResolvedSet, Ambiguous, Poisoned, Unresolved}` (`types.rs:454`) — only `Resolved` accepted ✓.
- `Binding.scope: ScopeId`, `Binding.ns: NamespaceId` (`types.rs:401`) — `b.scope`/`b.ns` ✓.
- `graph_file_for_scope(graph, scope: ScopeId) -> Option<FileId>` (`resolution.rs:1615`) ✓.
- helper arity = 7 params (`graph, path, anchor, from, final_ns, at, policy`) → `#[allow(clippy::too_many_arguments)]` added defensively (clippy warns at >7).
- `ResolvedCallee { target: &FunctionId, confidence, kind }`, `FunctionId.file`, `ResolutionOutcome { resolved, drop }` (`resolution.rs:79,433`) — used by every fixture's assertions ✓.
- imports already present in `resolution.rs:7,9,11-14`: `resolve_path`, `NS_TYPE`, `RustPolicy`, `Anchor`, `BindTarget`, `Candidate`, `RawPath`, `ResStatus`, `Target`, `ScopeId`, `SourceLoc`, `ScopeGraph`; `NamespaceId` is **not** present and is added to the `:11-14` list (Task 1 Step 2, verified absent).
