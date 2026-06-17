# Rust Scope-Graph — Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (TDD per task,
> spec+code review between). Steps use checkbox (`- [ ]`) syntax.
>
> **Design-of-record:** `docs/superpowers/specs/2026-06-17-prism-rust-module-resolution-design.md`
> (SOUND, codex round-7 GO). This plan implements that spec's **Phase 1** (§9/§11). Read the spec for
> the data model (§1), the Rust populator+policy (§4), and the recall-safety invariants (§7) — this plan
> does not re-derive them; it slices the build and pins the TDD + the "preserve-exactly" rules.
> Supersedes the naive `2026-06-16-prism-rust-use-imports.md` (rev-2).

**Goal:** Build the language-neutral scope-graph + the Rust populator/policy + consumers so Rust
unqualified calls narrow to the real defining item (the F3 fix: `original_diff.rs`'s local `fn slice`
stops pulling in 29 algorithm files) and Rust `nav module-deps`/`repo-map` show resolved `use`/re-export
edges — **recall-safe (resolve-or-fall-through, never a wrong edge)**.

**Architecture (per spec):** a new `src/name_resolution/` module: the shared **data model + engine**
(§1/§3), a **Rust populator** (§4) + a **Rust `ResolutionPolicy`** (§4), built once per repo at
`CallGraph` build and stored on `CallGraph`; consumers in `navigation/module_graph.rs` (edges) +
`resolution.rs` (narrowing). C++/Py/TS/JS populators are **later phases** (the model is data-model-
complete for them; not built here).

**Phase-1 minimal surface (spec §11.5 — implement exactly this, no more):** core graph types · Rust
crate/module/block populator · import/re-export fixpoint · Rust anchor policy (editions + module-
boundary stop) · local/value shadow bindings (`Target::Local`) · visibility enforce-or-fall-through ·
cfg condition *carry* · glob/macro/pending **poison** (macro = wildcard) · consumer replacement
(unqualified narrowing + qualified-`::` safe fall-through). **Deferred → strict fall-through:** glob
member *expansion*, cfg *evaluation*, macro *expansion*, full qualified-path *precision*, prelude
precision, the non-Rust populators.

**The preserve-exactly recall-safety rules (spec §7 — every slice must hold these):** edge/narrow ONLY
on a `Resolved` single in-repo `Item{callable:true}`; `ResolvedSet`(>1 file)/`Ambiguous`/`Poisoned`/
`Unresolved`/`External` → fall through; per-rib local→glob→parent with **module-boundary stop** for bare
names; **wildcard** macro poison + deferred-glob poison + pending-`use` poison (never reach an outer
same-name); cfg over-approximate, never merge exclusive worlds.

---

## Task 1: Core data-model types + the `ResolutionPolicy` trait (no behavior yet)

**Files:** Create `src/name_resolution/mod.rs`, `src/name_resolution/types.rs` · Modify `src/lib.rs`
(`pub mod name_resolution;`) · Test `tests/name_resolution/types_test.rs` (+ register the umbrella target)

Transcribe the spec §1 types: `SourceLoc`, `Span`, `NamespaceId`, `VisKindId`, `EdgeKindId`, `CfgCond`
(formula + `compatible()`/`exclusive()`), `Vis`, `ScopeExtent`(+`occ` reserved), `Scope`, `ScopeKind`,
`Binding`, `BindTarget`, `Target`(Scope|Item{callable,owns}|Local|External), `Edge`, `Candidate`,
`ResStatus`, `Resolution`, `ResolveQuery`(+ reserved `ctx`), and the `ResolutionPolicy` trait
(`namespaces`, `edge_order`, `combine`, `visible(binding,q,trav)`, `anchor`, `inject`). All
`Serialize/Deserialize/Clone`, BTreeMap/sorted for determinism.

- [ ] **Step 1: Failing test** — construct a 2-scope graph by hand (a Root + a Module with one
  `Binding`), assert field access + a serde round-trip is byte-stable. Run → fails (types absent).
- [ ] **Step 2: Implement** the types + the trait (methods unimplemented/`todo!()` is fine — no engine
  yet). `CfgCond::compatible`/`exclusive` with the conservative-on-unknown rule (unit-tested).
- [ ] **Step 3: Run → pass**; `cargo build`. **Step 4: Commit** `feat(nameres): scope-graph core data model + ResolutionPolicy trait`

## Task 2: The resolution engine + the Rust policy (the recall-safety core — heaviest TDD)

**Files:** Create `src/name_resolution/engine.rs` (the language-neutral walk), `src/name_resolution/rust_policy.rs` · Test `tests/name_resolution/resolve_test.rs`

The engine (`resolve`/`resolve_path`) walks scopes inner→outer, calling policy hooks; per-rib
local→glob→parent; pending-`BindTarget` fixpoint (cycle-guarded); per-candidate results; `Poisoned`
short-circuit. The Rust policy supplies edge order, **module-boundary stop** for bare names, glob
accessibility, candidate combination, the `visible()` accessibility predicate, and edition anchors.

- [ ] **Step 1: Failing tests (hand-built graphs — the §7 recall-safety matrix).** Each asserts the
  spec rule, on a hand-constructed graph (no AST yet):
  - **module-boundary stop:** Root `fn start`; child Module `m` with a bare `start` call → `Unresolved`
    (NOT the root `start`). `super::start` from `m` → the root `start`.
  - **local shadow:** a Block with a `Target::Local` `f` + an outer `Item fn f`; bare `f` in the block →
    the `Local` (narrowing stops, no callable item).
  - **glob poison:** a scope with a deferred `Glob` edge + an outer `Item x`; bare `x` → `Poisoned` (not
    the outer `x`).
  - **pending-`use` poison:** a still-`Pending` local `use x` + an outer `x` → `Poisoned`.
  - **explicit-beats-glob; two-glob conflict → `Ambiguous`; same-target glob dedup → `Resolved`.**
  - **re-export chain + cycle:** `Pending` chain `a→b→c` resolves to `c`; an `a→b→a` cycle → guarded
    (`Unresolved`/`Ambiguous`, not a hang).
  - **cfg:** two cfg-exclusive bindings → two candidates w/ distinct `cond` (not merged); non-exclusive
    differing → `Ambiguous`.
  - **`resolve_path` longest-prefix:** `crate::engine::start` resolves `crate::engine`→scope then `start`.
- [ ] **Step 2: Run → fail.** **Step 3: Implement** the engine + Rust policy to pass each. The
  `visible()` hook (Rust: `pub`/`pub(in)`/private + glob-accessibility-not-just-public) is a policy fn,
  not an engine span check. **Step 4: Run → all pass**; confirm NO test resolves to a wrong target (only
  `Resolved`-callable narrows; all else falls through). **Step 5: Commit** `feat(nameres): resolution engine + Rust policy (module-boundary stop, poison, fixpoint, anchors)`

## Task 3: The Rust populator (AST → scope graph) + build wiring + cache

**Files:** Create `src/name_resolution/rust_populator.rs` · Modify `src/call_graph.rs` (build + store
`scope_graph`; whole-repo rebuild incl. incremental), `src/cpg_cache.rs` (widen cache key + bump
`CACHE_VERSION`), `src/ast.rs` (surface `mod`/`use`/local-binding extraction if not present) · Test
`tests/name_resolution/rust_populate_test.rs`, `tests/lang/rust/`

Build (per spec §4): crate graph (roots `lib.rs`/`main.rs`/`bin/*`/tests/examples + Cargo.toml-lite:
members, edition, `[lib]`/`[[bin]]` paths, dep `package=` renames; convention fallback) → module/block
scopes (`mod` decls via declaring-module dir; inline; `#[path]`; cfg-conditioned; `foo.rs`+`foo/mod.rs`
→ ambiguous) → bindings (items in {Type,Value,Macro}; `use`→`Binding(Pending)`; `pub use`→Public;
**local value/pattern bindings → `Target::Local`**; macros → wildcard-poison marker) → `Glob` edges.
Stored on `CallGraph`; **whole-repo rebuild after any incremental merge** (a `mod`/`#[path]`/manifest
change reshapes unchanged files); cache key += manifests + file-set/existence + cfg + stable IDs.

- [ ] **Step 1: Failing tests** — parse multi-file Rust fixtures, assert the built graph + an end-to-end
  resolve: `mod engine; // engine.rs: pub fn start(){}` + `use crate::engine::start; fn g(){ start() }`
  → `resolve` finds `engine.rs::start`. Also: `foo.rs`+`foo/mod.rs` both → ambiguous; inline `mod`;
  `#[cfg]` dup mods → conditioned; a `let f` shadows a free `fn f`.
- [ ] **Step 2: Run → fail.** **Step 3: Implement** the populator + the build/store wiring + the cache
  widening (`CACHE_VERSION` bump + a pin/round-trip test) + whole-repo incremental rebuild (mirror the
  Go-embedding recompute). **Step 4: Run → pass**; `cargo test --test name_resolution` + `--test ast` (cache) green. **Step 5: Commit** `feat(rust-nameres): Rust populator + CallGraph wiring + cache (vN)`

## Task 4: Consumers — module-deps edges + unqualified narrowing + qualified fall-through

**Files:** Modify `src/navigation/module_graph.rs` (Rust resolved `use`/re-export edges), `src/resolution.rs` (unqualified narrowing via the graph; qualified-`::` graph-or-fall-through) · Test `tests/navigation/module_graph_test.rs`, `tests/integration/resolution_test.rs`

- [ ] **Step 1: Failing tests** — (a) module-deps: `use crate::util::helper;` → resolved edge to
  `util.rs`; external `use std::…` → `UnresolvedModule`/external label (replaces the Rust
  call-derived-only contract). (b) unqualified narrowing: two `fn process` (engine.rs/other.rs); a file
  with `use crate::engine::process; process()` → narrows to engine.rs only; **recall guards:** no-import
  → unchanged; cross-module bare name → fall through (module-boundary stop); `let`-shadowed name → no
  edge. (c) **qualified-`::` safety:** a qualified Rust call the graph can't resolve → falls through
  (NOT the legacy `resolution.rs:~532` stem/owner heuristic emitting a wrong edge).
- [ ] **Step 2: Run → fail.** **Step 3: Implement** the consumers via the graph; **disable the legacy
  qualified Rust heuristic where the graph is authoritative** (graph-resolve or fall through — spec §10).
  Edge/narrow only on `Resolved` single in-repo `Item{callable}`. **Step 4: Recall-guard tests pass +
  no Python/JS/Go module-deps regression** (those consumers unchanged; the graph is Rust-only in Phase 1). **Step 5: Commit** `fix(rust): module-deps + unqualified narrowing via the scope graph; qualified-:: safe fall-through`

## Task 5: Verification (the F3 demo + Tier-A + recall-safety)

- [ ] **Step 1:** `cargo build --release`.
- [ ] **Step 2: the F3 demo** — `target/release/prism nav module-deps --file src/algorithms/original_diff.rs`:
  confirm the local `fn slice` no longer pulls in all 29 `pub fn slice` files, and Rust `use` edges now
  resolve. Paste before/after.
- [ ] **Step 3: Tier-A** — `cd eval && uv run tier-a --matrix-only --allow-stale-sut` (no regression);
  `--quick --allow-stale-sut` (prism, Rust): the `callers/C-method` recall (the F3 receiver-typing class
  baseline.md flagged at 0.121) should **improve or hold**, precision must **not regress**. Paste flips.
- [ ] **Step 4:** full `cargo test` · `--features mcp` build · `cargo fmt --check`.
- [ ] **Step 5: Commit** any baseline/doc note if matrix flips (expected precision-positive / recall-up).

## Final verification (before PR)
- [ ] `cargo fmt --check` · full `cargo test` · `cargo build --release` · `--features mcp`
- [ ] Tier-A `--matrix-only` no regression; F3 demo fixed; Rust `nav module-deps` resolved edges
- [ ] **Recall-safety audit:** grep the consumer paths — every narrow/edge is gated on `Resolved`
  single-callable; every other status falls through. No path resolves to a wrong target.
- [ ] PR body: F3 before/after + the Tier-A Rust delta + the scope-graph module map + "Phase-1 surface
  only; later phases (glob expansion, cfg eval, macros, full qualified, C++/Py/TS/JS) fall through."

## Slicing into PRs (owner's "Phase 1 = several PRs")
Tasks 1+2 (core types + engine/policy, the recall-safety heart) = **PR-1**; Task 3 (populator + cache) =
**PR-2**; Task 4 (consumers) = **PR-3**; Task 5 folds into PR-3's verification. PR-1/2 add the subsystem
with **no consumer change** (zero behavior change until PR-3 wires it), so they're low-risk to land
incrementally; PR-3 is the behavior-changing one (Tier-A gated).

## Risks / watch-items
- **Recall (headline):** the whole value is dropping *wrong* edges; a bug could drop *right* ones.
  Mitigation = the §7 invariants as a test matrix (Task 2) + the recall guards (Task 4) + the Tier-A
  precision-must-not-regress gate + the recall-safety audit.
- **Replacing live behavior:** Task 4 changes real resolution (`resolution.rs`/`module_graph.rs`).
  Tier-A `--quick` is the regression net; the legacy qualified heuristic must be disabled *only* where
  the graph is authoritative (else recall loss).
- **Cache correctness:** whole-repo rebuild + the widened key (manifests/file-set/cfg). A missed input
  ⇒ stale resolution. Test the incremental path (a `mod` add/remove changes unchanged files' resolution).
- **Scope creep:** implement ONLY the §11.5 surface; everything else falls through. Resist expanding
  glob/cfg/macro/qualified beyond *poison/representation* in Phase 1.
