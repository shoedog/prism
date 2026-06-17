# Rust Scope-Graph — Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (TDD per task,
> spec+code review between). Steps use checkbox (`- [ ]`) syntax.
>
> **Design-of-record:** `docs/superpowers/specs/2026-06-17-prism-rust-module-resolution-design.md`
> (SOUND, codex round-7 GO). This plan implements that spec's **Phase 1** (§9/§11). Read the spec for
> the data model (§1), the Rust populator+policy (§4), and the recall-safety invariants (§7) — this plan
> does not re-derive them; it slices the build and pins the TDD + the "preserve-exactly" rules.
> Supersedes the naive `2026-06-16-prism-rust-use-imports.md` (rev-2).
>
> **Rev 2** (folds the codex gpt-5.5 xhigh plan-review, CHANGES-REQUESTED → the 7 findings): pins
> macro-wildcard-poison, visibility enforce-or-fall-through, broadened local bindings, a **positive
> qualified-`::` recall gate**, the glob Phase-1 boundary (engine supports non-deferred globs; the Rust
> populator **poisons** real `use a::*`), the cache stale-topology test (promoted from watch-item), and a
> **fail-open** populator — plus the do-now edition-anchor/consumer-guard/`ResolveQuery.at` items.

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
  spec rule, on a hand-constructed graph (no AST yet). Every case that is meant to fall through MUST
  assert the *status* (`Unresolved`/`Ambiguous`/`Poisoned`/`ResolvedSet`), never the outer same-name
  target:
  - **module-boundary stop:** Root `fn start`; child Module `m` with a bare `start` call → `Unresolved`
    (NOT the root `start`). `super::start` from `m` → the root `start`.
  - **local shadow:** a Block with a `Target::Local` `f` + an outer `Item fn f`; bare `f` in the block →
    the `Local` (narrowing stops, no callable item).
  - **glob poison (deferred):** a scope with a *deferred* `Glob` edge + an outer `Item x`; bare `x` →
    `Poisoned` (not the outer `x`).
  - **macro wildcard poison (F1):** a scope carrying an *unexpanded* name-introducing macro marker
    (`m!(); …`) + an outer `Item f`; bare `f` in that scope/range → `Poisoned` (the wildcard could have
    introduced `f` — never fall through to the outer `f`). Assert a call *outside* the macro's range is
    unaffected.
  - **pending-`use` poison:** a still-`Pending` local `use x` + an outer `x` → `Poisoned`.
  - **visibility enforce-or-fall-through (F2):** the `visible()` policy hook, hand-built:
    `private`/`pub(self)` item in module `m`, resolved *from a sibling* → not visible → fall through
    (`Unresolved`), NOT a wrong edge to the private item; `pub(super)` visible from the parent's subtree
    only; `pub(in path)` visible only inside `path`; a `pub use` re-export visible across the re-export
    where the underlying item is private. A glob only re-exports `pub` members (glob-accessibility, not
    just "an edge exists").
  - **glob (engine, non-deferred only):** **explicit-beats-glob**; **two-glob conflict → `Ambiguous`**;
    **same-target glob dedup → `Resolved`**. NOTE (F5): these exercise engine combination over glob
    edges whose members are *already known* (non-deferred) — they are NOT Rust `use a::*` member
    expansion (that is deferred; the Rust populator poisons it — Task 3). State this in the test module
    doc so the engine support is not mistaken for scope creep.
  - **re-export chain + cycle:** `Pending` chain `a→b→c` resolves to `c`; an `a→b→a` cycle → guarded
    (`Unresolved`/`Ambiguous`, not a hang).
  - **cfg:** two cfg-exclusive bindings → two candidates w/ distinct `cond` (not merged); non-exclusive
    differing → `Ambiguous`.
  - **edition anchors (do-now):** 2015 — a bare `use foo::…` anchors at the crate root; 2018 — `::x`
    anchors the extern-prelude and a bare `x` with no in-scope binding falls through (extern-prelude is
    not an in-repo `Item`); a `let`/item `x` shadowing an extern-prelude name resolves to the local, not
    the prelude. Assert via the policy `anchor` hook on hand-built graphs tagged 2015 vs 2018.
  - **`resolve_path` longest-prefix:** `crate::engine::start` resolves `crate::engine`→scope then `start`.
- [ ] **Step 2: Run → fail.** **Step 3: Implement** the engine + Rust policy to pass each. The
  `visible()` hook (Rust: `pub`/`pub(super)`/`pub(in path)`/private + glob-accessibility-not-just-public)
  is a policy fn, not an engine span check; un-enforced/unknown visibility falls through, never resolves
  a not-visible target. **Step 4: Run → all pass**; confirm NO test resolves to a wrong target (only
  `Resolved`-callable narrows; all else falls through). **Step 5: Commit** `feat(nameres): resolution engine + Rust policy (module-boundary stop, poison, visibility, fixpoint, anchors)`

## Task 3: The Rust populator (AST → scope graph) + build wiring + cache

**Files:** Create `src/name_resolution/rust_populator.rs` · Modify `src/call_graph.rs` (build + store
`scope_graph`; whole-repo rebuild incl. incremental), `src/cpg_cache.rs` (widen cache key + bump
`CACHE_VERSION`), `src/ast.rs` (surface `mod`/`use`/local-binding extraction if not present) · Test
`tests/name_resolution/rust_populate_test.rs`, `tests/lang/rust/`

Build (per spec §4): crate graph (roots `lib.rs`/`main.rs`/`bin/*`/tests/examples + Cargo.toml-lite:
members, edition, `[lib]`/`[[bin]]` paths, dep `package=` renames; convention fallback) → module/block
scopes (`mod` decls via declaring-module dir; inline; `#[path]`; cfg-conditioned; `foo.rs`+`foo/mod.rs`
→ ambiguous) → bindings (items in {Type,Value,Macro}; `use`→`Binding(Pending)`; `pub use`→Public;
**all local value/pattern bindings → `Target::Local`** — `let`, fn params, closure args, and `for`/
`match`/`if let`/`while let` pattern bindings (F3); real `use a::*` → **deferred-glob poison** (no member
expansion in Phase 1 — F5); name-introducing macros (`macro_rules!`, attribute/`macro!()` invocations)
→ **wildcard-poison marker** over the affected scope/range — F1) → `Glob` edges. Stored on `CallGraph`;
**whole-repo rebuild after any incremental merge** (a `mod`/`#[path]`/manifest change reshapes unchanged
files); cache key += manifests + file-set/existence + cfg + stable IDs. **Fail-open (F7):** any
malformed/missing input (unparseable Cargo.toml, missing `mod` target, parse error) yields an empty or
poisoned region, never a panic / never aborts CPG construction — PR-2 must be byte-for-byte behavior-
neutral. **Do-now:** Rust import evidence lives ONLY on the scope graph — do NOT write Rust into
`CallGraph.imports` (`ast.rs:498` `extract_imports` intentionally skips Rust; Python/JS/Go import
behavior stays untouched).

- [ ] **Step 1: Failing tests** — parse multi-file Rust fixtures, assert the built graph + an end-to-end
  resolve:
  - **end-to-end:** `mod engine; // engine.rs: pub fn start(){}` + `use crate::engine::start; fn g(){ start() }`
    → `resolve` finds `engine.rs::start`. Also `foo.rs`+`foo/mod.rs` both → ambiguous; inline `mod`;
    `#[cfg]` dup mods → conditioned.
  - **local bindings (F3) — all forms become `Target::Local` and shadow a free `fn f`:** `let f = …; f()`;
    `fn g(f: impl Fn()) { f() }` (param); `(0..n).for_each(|f| f())` (closure arg); `for f in xs { f() }`;
    `match x { Some(f) => f() , … }`; `if let Some(f) = x { f() }`. Each → the `Local`, never the free fn.
  - **macro wildcard poison (F1):** a fixture with `m!(); f()` where an outer `f` exists → the populator
    marks the macro range wildcard-poison → `resolve(f)` in range = `Poisoned` (a call before the macro is
    unaffected).
  - **real glob poison (F5):** `use other::*; thing()` where `other` has a `pub fn thing` → the populator
    emits **deferred-glob poison** (NO member expansion) → `resolve(thing)` = `Poisoned`, never a synthetic
    edge to `other::thing`.
  - **visibility (F2):** `use crate::m::private_fn; private_fn()` where `m::private_fn` is private → the
    re-export/use is not visible across the module boundary → `resolve` falls through (`Unresolved`), no
    edge to the private item.
  - **cache stale-topology (F6 — promoted from watch-item):** build a 2-file graph, then (a) ADD a
    `mod new;` + `new.rs` and (b) REMOVE a `mod`, each via the incremental path; assert the rebuilt graph
    matches a from-scratch build (the cache key change forces a whole-repo rebuild; an unchanged sibling's
    resolution updates). Same for a `#[path="…"]` target add/remove.
  - **fail-open (F7):** a fixture with a `mod missing;` whose file is absent, and one with malformed
    `Cargo.toml` → the populator returns a graph (empty/poisoned region) and CPG build succeeds; assert no
    panic and the rest of the graph still resolves.
- [ ] **Step 2: Run → fail.** **Step 3: Implement** the populator + the build/store wiring + the cache
  widening (`CACHE_VERSION` bump + a pin/round-trip test) + whole-repo incremental rebuild (mirror the
  Go-embedding recompute) + the fail-open guards. **Step 4: Run → pass**; `cargo test --test name_resolution` + `--test ast` (cache) green. **Step 5: Commit** `feat(rust-nameres): Rust populator + CallGraph wiring + cache (vN)`

## Task 4: Consumers — module-deps edges + unqualified narrowing + qualified fall-through

**Files:** Modify `src/navigation/module_graph.rs` (Rust resolved `use`/re-export edges), `src/resolution.rs` (unqualified narrowing via the graph; qualified-`::` graph-or-fall-through) · Test `tests/navigation/module_graph_test.rs`, `tests/integration/resolution_test.rs`

The call-site→scope mapping is explicit (do-now): the consumer builds each `ResolveQuery.at` from the
captured `CallSite.start_byte/end_byte` (`call_graph.rs:22`) so block-local shadows and block-scoped
`use` resolve at the right rib — not just caller-function/file granularity.

- [ ] **Step 1: Failing tests** —
  - **(a) module-deps:** `use crate::util::helper;` → resolved edge to `util.rs`; external `use std::…`
    → `UnresolvedModule`/external label (replaces the Rust call-derived-only contract).
  - **(b) unqualified narrowing:** two `fn process` (engine.rs/other.rs); a file with
    `use crate::engine::process; process()` → narrows to engine.rs only. **Recall guards:** no-import →
    unchanged; cross-module bare name → fall through (module-boundary stop); `let`-shadowed name → no
    edge; a `private_fn` imported across a module boundary where it is private → no edge (F2 consumer side).
  - **(c) qualified-`::` SAFETY (negative):** a qualified Rust call the graph can't resolve → falls
    through (NOT the legacy `resolution.rs:~532` stem/owner heuristic emitting a wrong edge).
  - **(c′) qualified-`::` RECALL (positive — F4):** `crate::engine::start()`, `self::start()`, and
    `super::start()` that the Phase-1 graph CAN authoritatively resolve → the **correct** edge is emitted
    (proves disabling the legacy heuristic did not silently drop all `::` recall). This is a golden
    fixture: a small Rust crate with known-correct `::` edges, asserted exactly (precision-not-regress
    alone is necessary-but-not-sufficient).
  - **(d) consumer-guard fall-through (do-now):** every non-`Resolved`-single-callable result avoids an
    in-repo edge — assert each of: `Resolved(Target::External)` (e.g. `std`), `Resolved` non-callable
    `Item` (a type/const), `Resolved(Target::Scope)`, `ResolvedSet` (>1 file), a cfg-exclusive
    multi-candidate result, and `Poisoned` → NO edge / narrowing.
- [ ] **Step 2: Run → fail.** **Step 3: Implement** the consumers via the graph; **disable the legacy
  qualified Rust heuristic where the graph is authoritative** (graph-resolve or fall through — spec §10).
  Edge/narrow only on `Resolved` single in-repo `Item{callable}`. **Step 4: (c′) + recall-guard tests
  pass + no Python/JS/Go module-deps regression** (those consumers unchanged; the graph is Rust-only in
  Phase 1). **Step 5: Commit** `fix(rust): module-deps + unqualified narrowing via the scope graph; qualified-:: safe fall-through`

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
  Mitigation = the §7 invariants as a test matrix (Task 2) + the recall guards (Task 4) + the **positive
  qualified-`::` golden gate (Task 4 c′)** so disabling the legacy heuristic can't silently zero `::`
  recall + the Tier-A precision-must-not-regress gate + the recall-safety audit. (Tier-A precision alone
  is necessary-but-not-sufficient — the golden fixtures pin recall directly.)
- **Replacing live behavior:** Task 4 changes real resolution (`resolution.rs`/`module_graph.rs`).
  Tier-A `--quick` is the regression net; the legacy qualified heuristic must be disabled *only* where
  the graph is authoritative (else recall loss).
- **Cache correctness:** whole-repo rebuild + the widened key (manifests/file-set/cfg). A missed input
  ⇒ stale resolution. **Pinned by the Task-3 stale-topology test (F6)** — a `mod`/`#[path]` add/remove
  must change unchanged files' resolution to match a from-scratch build.
- **Scope creep:** implement ONLY the §11.5 surface; everything else falls through. Resist expanding
  glob/cfg/macro/qualified beyond *poison/representation* in Phase 1.
