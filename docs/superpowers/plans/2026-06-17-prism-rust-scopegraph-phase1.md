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
> **Rev 2** (folds the codex gpt-5.5 xhigh plan-review round 1, CHANGES-REQUESTED → 7 findings): pins
> macro-wildcard-poison, visibility enforce-or-fall-through, broadened local bindings, a **positive
> qualified-`::` recall gate**, the glob Phase-1 boundary (engine supports non-deferred globs; the Rust
> populator **poisons** real `use a::*`), the cache stale-topology test (promoted from watch-item), and a
> **fail-open** populator — plus the do-now edition-anchor/consumer-guard/`ResolveQuery.at` items.
>
> **Rev 3** (folds round 2, CHANGES-REQUESTED → 4 MAJOR + 1 MINOR, "no scope creep"): corrects the
> re-export privacy test (a `pub use` of a *private* item falls through; only a *public* item in a
> *private module* resolves through the facade), adds the **`authoritative_for(callsite)` contract** +
> a shared `graph_callable_edge` helper + three-state authority tests (full / absent / fail-open-poisoned
> graph), pins **block-local `use` extents** (before/inside/after) and the **nested out-of-line module**
> declaring-dir rule, makes the incremental path **replace** (never merge stale) the scope graph, and
> adds use-group/alias/`extern crate`/workspace/`[lib]`-`[[bin]]`/`pub use`-chain/`while let` coverage.
>
> **Rev 4** (folds round 3, CHANGES-REQUESTED → 4 MAJOR + 1 MINOR — integration plumbing, not
> architecture): adds **`ScopeGraphBuildInputs`** (repo root + whole file-set + manifest hashes + cfg +
> `complete` flag) threaded through `repo_loader`→nav/CPG→`CallGraph`→cache, since the current build
> surface skips `Cargo.toml` and passes only parsed source; makes **`authoritative_for` completeness-aware
> per-graph AND per-site** (a subset/diff/scoped build is `complete=false` → authoritative for no site, so
> it can't masquerade); **splits the consumer helpers** — `graph_callable_edge` (call narrowing) vs
> `graph_module_dep_edge` (module-deps resolves named/`pub use`/glob imports *regardless of callable*);
> broadens local bindings to **multi-binding/non-first-identifier patterns**; adds the **outer-same-name
> visibility decoy**. Phase-1 graph is authoritative only on **whole-workspace** builds (where F3 lands).
>
> **Rev 5** (folds round 4, **APPROVE-WITH-NITS** — "no fall-through/poison-rule blocker; stays inside
> §11.5"): asserts cfg multi-candidate **status** (`ResolvedSet`, not just distinct conds), adds the
> remaining crate-root shapes (`src/bin/<n>/main.rs`/`benches/`/autobins), splits **fail-open into
> graph-wide (`complete=false`) vs local-region poison**, adds a **module-deps glob-edge** test (edge to
> the glob's module without member expansion — coexists with call-poison), pins the **positional `let`
> extent** (a call before `let f` doesn't see the local), and corrects PR-2's claim to **"no consumer
> behavior change," NOT byte-for-byte** (storing `scope_graph` + the `CACHE_VERSION` bump change the cache
> blob by design). **Plan converged — ready for implementation pending owner go.**
>
> **Rev 6** (folds round 5, CHANGES-REQUESTED → 2 MAJOR + 2 MINOR — two new edges the rev-4/5 folds
> exposed): **(F1)** adds a `CallSite.kind: Call | MacroInvocation` discriminator + namespace-correct
> `graph_callable_edge` + a guard so a `m!()` invocation can never edge to a Value `fn m`; **(F2)**
> replaces the ambiguous "empty/poisoned" wording with one crisp **region state machine** — `Modeled` /
> `Poisoned` (modeled-but-hazardous, incl. parse-failed *target*) / `Unmodeled` (parse-failed
> *containing* file) — with **authority keyed on the call SITE's scope, not the target** (a poisoned
> target → `Poisoned` *result* → fall through, not a non-authoritative site); **(F3)** re-export cycle is
> terminal `Poisoned` with an outer-`a` decoy; **(F4)** adds `pub(crate)` cross-workspace-crate
> visibility; do-now: manifest-only + file-existence-only cache tests.
>
> **Rev 7** (folds round 6, **APPROVE-WITH-NITS** — "preserves §7, stays within §11.5, handles the repo
> integration risks"; nits purely additive, no new contracts): adds an explicit External/Scope/
> non-callable same-name **engine** shadow decoy (Task 2), `tests/`/`examples/` crate-root fixtures
> (Task 3), `CallSite.kind` `#[serde(default)]` + identity/order caveat, and rewords Task 5 to **paste
> Tier-A flips into the PR body, not re-baseline**. **CONVERGED** — two consecutive APPROVE-WITH-NITS, the
> second additive-only; ready for implementation pending owner go.

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
    only; `pub(in path)` visible only inside `path`; **`pub(crate)` (round-5 F4):** visible anywhere in
    the *defining* crate, but NOT through a sibling workspace crate (a cross-crate `use other_crate::m::f`
    where `f` is `pub(crate)` in `other_crate` → not visible → fall through) unless an actually-`pub`
    facade re-exports it. **Re-export privacy (round-2 F1 — two distinct cases, do not conflate):** (i) a `pub use` of a *private* item → the item is still not visible across
    the re-export → fall through (`Unresolved`), NEVER a wrong edge to the private item (a `pub use` does
    not launder privacy); (ii) a *public* item living in a *private module*, re-exported via `pub use`
    from an accessible scope → **resolved through the re-export** (the item itself is `pub`; only its
    module *path* is private — the facade pattern). A glob only re-exports `pub` members
    (glob-accessibility, not just "an edge exists"). **Outer same-name decoy (round-3 F5):** an
    inaccessible explicit binding/re-export at the resolving rib + an outer callable of the *same name* →
    the engine must NOT continue outward to the outer callable; it falls through (`Unresolved`). An
    inaccessible inner binding wins the name-resolution race and then fails visibility — it does not
    silently fall back to a wrong outer target.
  - **explicit External/Scope/non-callable shadow decoy (round-6 nit 1 — engine shadow rule §3.4+§7):**
    an inner explicit binding to a non-callable target + an outer callable of the *same name* — `use ext::x`
    (→ `External`), or `use crate::m as x` (→ `Scope`), or an inner non-callable `Item x` (a type/const) —
    each shadows outward, so resolving `x` returns `External`/`Scope`/non-callable and the engine must NOT
    reach the outer callable `x`. (Task 4(d) pins the *consumer* no-edge; this pins the *engine* shadow.)
  - **glob (engine, non-deferred only):** **explicit-beats-glob**; **two-glob conflict → `Ambiguous`**;
    **same-target glob dedup → `Resolved`**. NOTE (F5): these exercise engine combination over glob
    edges whose members are *already known* (non-deferred) — they are NOT Rust `use a::*` member
    expansion (that is deferred; the Rust populator poisons it — Task 3). State this in the test module
    doc so the engine support is not mistaken for scope creep.
  - **re-export chain + cycle:** `Pending` chain `a→b→c` resolves to `c`; an `a→b→a` cycle **with an outer
    `a` decoy** → terminal **`Poisoned`** (the still-pending import poisons — round-5 F3), NOT a hang and
    NOT the outer `a`.
  - **cfg:** two cfg-exclusive bindings → a multi-candidate **`ResolvedSet`** with distinct `cond` (not
    merged, not collapsed to one); non-exclusive differing → `Ambiguous`. Assert the **status** (round-4
    nit 1) — a multi-candidate result must make the call consumer fall through (never mint an edge), never
    silently pick or drop a world (spec §7).
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
`scope_graph`; whole-repo rebuild incl. incremental; + a `CallSite.kind: Call | MacroInvocation`
discriminator — round-5 F1, with `#[serde(default)]` so old caches deserialize, and excluded from
call-site identity/ordering unless intended since `CallSite` is cached + ordered — round-6 do-now),
`src/queries.rs` (tag the `macro_invocation` capture at `:173` so the discriminator is set), `src/cpg_cache.rs` (widen cache key + bump `CACHE_VERSION`), `src/ast.rs` (surface
`mod`/`use`/local-binding extraction if not present), `src/repo_loader.rs` (load `Cargo.toml`/manifests +
the whole file-set, not just supported-source — currently skipped at `:277`),
`src/cpg/build.rs`/`src/navigation/mod.rs` (thread the build inputs into `CallGraph::build` — currently
pass only parsed source files at `cpg/build.rs:134`/`navigation/mod.rs:35`), `src/cpg/context.rs`
(subset/scoped builds → no authoritative graph, `:160`) · Test
`tests/name_resolution/rust_populate_test.rs`, `tests/lang/rust/`

**Build inputs (round-3 F1 — do-now).** §11.5's crate graph and §8's cache key need inputs the current
build surface does not pass. Add a first-class `ScopeGraphBuildInputs { repo_root, all_file_paths (for
existence/`mod`-target checks), manifest_hashes (Cargo.toml/workspace), cfg/features, complete: bool }`
threaded through `repo_loader` → nav/CPG → `CallGraph::build` → cache. The diff-review path parses only
diff files (`main.rs:615`) and scoped CPG filters to a subset (`cpg/context.rs:160`) — those builds set
`complete=false`, so **the `scope_graph` is omitted / marked incomplete and is NOT authoritative** (legacy
resolution unchanged there). Phase 1's graph is authoritative only on **whole-workspace** builds (nav
`module-deps`/`repo-map` + full CPG) — which is exactly where the F3 fix lands. The new inputs are
**purely additive** (a side channel only the populator reads): the existing parsed-supported-source file
set, hashing, and every existing consumer are unchanged, so PR-2 changes **no consumer behavior**
(resolution/nav output unchanged; only the cache blob shifts — see Fail-open).

Build (per spec §4): crate graph (roots `lib.rs`/`main.rs`/`src/bin/*.rs`/`src/bin/<n>/main.rs`/`benches/`/
`tests/`/`examples/` + autobins/autobenches + Cargo.toml-lite: members, edition, `[lib]`/`[[bin]]` paths,
dep `package=` renames; convention fallback) → module/block
scopes (`mod` decls via declaring-module dir; inline; `#[path]`; cfg-conditioned; `foo.rs`+`foo/mod.rs`
→ ambiguous) → bindings (items in {Type,Value,Macro}; `use`→`Binding(Pending)`; `pub use`→Public;
**all local value/pattern bindings → `Target::Local`** — `let`, fn params, closure args, and `for`/
`match`/`if let`/`while let` pattern bindings (F3); real `use a::*` → **deferred-glob poison** (no member
expansion in Phase 1 — F5); name-introducing macros (`macro_rules!`, attribute/`macro!()` invocations)
→ **wildcard-poison marker** over the affected scope/range — F1) → `Glob` edges. Stored on `CallGraph`;
**the whole `scope_graph` is REPLACED (rebuilt from all parsed files) after any incremental merge — never
merge stale scope data** (a `mod`/`#[path]`/manifest change reshapes unchanged files; the partial-hit
flow at `main.rs:718` hands back a cached `CallGraph` + changed files, so the graph must be recomputed
whole, not subset-merged — round-2 do-now); cache key += manifests + file-set/existence + cfg + stable IDs. **Fail-open (F7), using the Task-4 region state machine (round-4 nit 3 / round-5 F2):** a *graph-wide*
failure (unparseable `Cargo.toml`/workspace, crate-graph build error) → whole graph **`complete=false`
(non-authoritative)**, legacy NOT disabled repo-wide. A *local* failure is classified by the same
`Poisoned` vs `Unmodeled` rule: a parse-failed **target** module or a `mod foo;` whose file is missing →
the *target* region is **`Poisoned`** (a lookup into it falls through, but the call site's own modeled
scope stays authoritative); a parse-failed **containing** file → that region is **`Unmodeled`** (sites in
it are not authoritative → legacy runs). Never a panic, never aborts CPG construction. **PR-2 is "no consumer behavior change," NOT byte-for-byte identical** (round-4 do-now):
storing `scope_graph` + bumping `CACHE_VERSION` (`cpg_cache.rs:52`) changes the cache blob's bytes by
design — what is unchanged is resolution/nav *output* (no consumer reads the graph until PR-3).
**Do-now:** Rust import evidence lives ONLY on the scope graph — do NOT write Rust into
`CallGraph.imports` (`ast.rs:498` `extract_imports` intentionally skips Rust; Python/JS/Go import
behavior stays untouched).

- [ ] **Step 1: Failing tests** — parse multi-file Rust fixtures, assert the built graph + an end-to-end
  resolve:
  - **end-to-end:** `mod engine; // engine.rs: pub fn start(){}` + `use crate::engine::start; fn g(){ start() }`
    → `resolve` finds `engine.rs::start`. Also `foo.rs`+`foo/mod.rs` both → ambiguous; inline `mod`;
    `#[cfg]` dup mods → conditioned.
  - **nested out-of-line module (round-2 F4 — declaring-module directory):** `src/foo.rs` containing
    `mod bar;` resolves the child to `src/foo/bar.rs`, NOT `src/bar.rs` (spec §4 declaring-module-dir
    rule). Cover both `src/foo.rs`+`src/foo/bar.rs` and the `src/foo/mod.rs`+`src/foo/bar.rs` shapes.
  - **local bindings (F3 + round-3 F4) — all forms become `Target::Local` and shadow a free `fn f`:**
    `let f = …; f()`; `fn g(f: impl Fn()) { f() }` (param); `(0..n).for_each(|f| f())` (closure arg);
    `for f in xs { f() }`; `match x { Some(f) => f() , … }`; `if let Some(f) = x { f() }`;
    `while let Some(f) = it.next() { f() }`. **Multi-binding / non-first-identifier patterns (round-3 F4 —
    the common wrong-edge path):** `let (_, f) = pair; f()` (tuple, f not first); `let Point { y: f, .. } = p; f()`
    (struct field); `match m { (_, f) => f() }`; closure `|(_, f)| f()` (destructured arg). Each → the
    `Local`, never the free fn.
  - **block-local `use` extent (round-2 F3/vis_extents):** `fn g(){ before(); { use crate::m::before; before() } before() }`
    where the block `use` brings a *different* `before` into the inner block only → the **inside** call
    resolves to the imported `m::before`; the **before** and **after** calls do NOT (the `use`'s
    `vis_extents` cover only the block — proves `ResolveQuery.at` maps to the right rib and the import
    does not leak out of its block). Spec §1 byte-qualified `ScopeExtent`/`vis_extents` (:173/:351).
    **Positional `let` extent (round-4 do-now):** with a free `fn f` and `fn g(){ f(); let f = …; f() }` —
    the call **before** `let f` does NOT see the later local (resolves to the free `fn f`); the call
    **after** resolves to the `Local`. The local's shadow must not extend backward before its `let`.
  - **use-group / alias / extern crate / workspace / crate-roots (round-2 F5 + round-4 nit 2):**
    `use a::{b, c::d, self}` (nested + `self` group), `use a::b as c; c()` (alias → `Pending` to `a::b`),
    `extern crate foo as bar;` (crate alias), a workspace member + a `package = "x"` dep rename,
    `[lib]`/`[[bin]]` non-convention roots, AND the remaining crate-root shapes — `src/bin/<n>/main.rs`,
    `tests/`, `examples/`, `benches/`, and `autobins` — each builds the expected bindings/roots (resolve
    where in-repo, poison/extern where not).
  - **macro wildcard poison (F1):** a fixture with `m!(); f()` where an outer `f` exists → the populator
    marks the macro range wildcard-poison → `resolve(f)` in range = `Poisoned` (a call before the macro is
    unaffected).
  - **real glob poison (F5):** `use other::*; thing()` where `other` has a `pub fn thing` → the populator
    emits **deferred-glob poison** (NO member expansion) → `resolve(thing)` = `Poisoned`, never a synthetic
    edge to `other::thing`.
  - **visibility (F2 — both re-export cases):** (i) `use crate::m::private_fn; private_fn()` where
    `m::private_fn` is private → not visible across the module boundary → `resolve` falls through
    (`Unresolved`), no edge to the private item; (ii) `mod m { pub fn f(){} }` (with `m` private) +
    `pub use crate::m::f;` from the crate root + a call to the re-exported `f` → **resolves to `m::f`**
    (public item, private module path — the facade must still resolve).
  - **cache stale-topology (F6 — promoted from watch-item):** build a 2-file graph, then (a) ADD a
    `mod new;` + `new.rs` and (b) REMOVE a `mod`, each via the incremental path; assert the rebuilt graph
    matches a from-scratch build (the cache key change forces a whole-repo rebuild; an unchanged sibling's
    resolution updates). Same for a `#[path="…"]` target add/remove. **Manifest-only + file-existence-only
    (round-5 do-now):** a `Cargo.toml` edit (edition / `[lib]` path) with NO source change, and adding/
    removing a file an existing `mod` references — both must invalidate (today's cache hashes source maps
    only, `cpg_cache.rs:119`/`:311`; the widened key must catch these).
  - **fail-open (F7) — the three region cases (round-5 F2):** (1) `mod missing;` whose file is absent →
    `missing`'s region is **`Poisoned`** (a `use crate::missing::x` falls through; the containing module
    still resolves its own items); (2) a **target** module that parses with errors → **`Poisoned`** target
    (same); (3) a **containing** file that parses with errors → that file's region is **`Unmodeled`**
    (sites in it are not authoritative). Plus graph-wide: malformed `Cargo.toml` → whole graph
    `complete=false`. Assert no panic, CPG build succeeds, and the rest of the graph still resolves.
- [ ] **Step 2: Run → fail.** **Step 3: Implement** the populator + the build/store wiring + the cache
  widening (`CACHE_VERSION` bump + a pin/round-trip test) + whole-repo incremental rebuild (mirror the
  Go-embedding recompute) + the fail-open guards. **Step 4: Run → pass**; `cargo test --test name_resolution` + `--test ast` (cache) green. **Step 5: Commit** `feat(rust-nameres): Rust populator + CallGraph wiring + cache (vN)`

## Task 4: Consumers — module-deps edges + unqualified narrowing + qualified fall-through

**Files:** Modify `src/navigation/module_graph.rs` (Rust resolved `use`/re-export edges via
`graph_module_dep_edge`), `src/resolution.rs` (unqualified narrowing + qualified-`::` graph-or-fall-through
via `graph_callable_edge`) · the two shared helpers + `authoritative_for` live where both can reach them
(e.g. `src/name_resolution/consumer.rs` or on `CallGraph`) · Test `tests/navigation/module_graph_test.rs`, `tests/integration/resolution_test.rs`

The call-site→scope mapping is explicit (do-now): the consumer builds each `ResolveQuery.at` from the
captured `CallSite.start_byte/end_byte` (`call_graph.rs:28`) so block-local shadows and block-scoped
`use` resolve at the right rib — not just caller-function/file granularity.

**The graph-authority contract (round-2 F2 / round-3 F2 / round-5 F2 — define BEFORE disabling any legacy
path).** The legacy qualified Rust heuristic (`resolution.rs:532`/`:566` owner/stem ladder) is disabled
*only* for sites an authoritative graph covers. **Authority is keyed on the call SITE's containing scope,
NOT the target** — a poisoned/missing *target* yields a `Poisoned`/`Unresolved` *result* (→ fall through),
it does not make the site non-authoritative. `authoritative_for(callsite)`:
1. **per-graph:** `scope_graph.complete == true` (a whole-workspace build per F1 — diff/scoped/subset/
   skeleton builds are `complete=false` → graph omitted/incomplete → authoritative for NO site); AND
2. **per-site:** a **modeled** containing scope/rib is found for the site's `CallSite.start_byte` (the
   site's own file parsed and was populated).
The crisp region state machine (round-5 F2 — one definition, used by Task 3 AND Task 4):
- **`Modeled`** — scopes built for this region; resolve normally.
- **`Poisoned`** — modeled but *hazardous*: a real glob, an unexpanded macro, a pending `use`, OR a
  parse-failed **target** module (we know a name *might* live here but can't confirm). A site whose
  containing scope is `Modeled` but whose lookup lands in `Poisoned` → authoritative, result `Poisoned`.
- **`Unmodeled`** — no scopes built: a parse-failed **containing** file, or a region the populator never
  reached. A site here is **not** authoritative.
Then:
- **authoritative + `Resolved` single in-repo callable** → emit the graph edge (legacy skipped).
- **authoritative + any other status** (`Poisoned`/`Ambiguous`/`ResolvedSet`/`Unresolved`/`External`) →
  **fall through, legacy skipped** (the graph considered it and declined; do not let the heuristic guess).
- **NOT authoritative** — `complete=false` graph (`build_skeleton` `call_graph.rs:147`; direct-subset
  incremental `call_graph.rs:1009`) OR the site sits in an `Unmodeled` region → **legacy heuristic runs
  unchanged** (no recall loss; PR-3 changes nothing for uncovered sites).

**Two distinct consumer helpers (round-3 F3 — do-now; do NOT force module-deps through callable-only
logic).** Call narrowing and module-deps resolve *different* binding kinds:
- `graph_callable_edge(site) -> Option<Target>` — for `resolution.rs` call narrowing; `Some` only on
  authoritative + `Resolved` single in-repo `Item{callable}`. **Namespace-correct (round-5 F1):** a `f()`
  site resolves in the **Value** namespace; a `m!()` **macro invocation** (captured at `queries.rs:173`)
  resolves in the **Macro** namespace (Phase-1: `macro_rules!`/macros are poison/deferred → fall through),
  so a macro invocation can NEVER edge to a Value `fn m`. This needs a **`CallSite` syntax discriminator**
  (a `kind: Call | MacroInvocation`, set by the extractor — do-now; `CallSite` carries only name/bytes/
  qualifier today, `call_graph.rs:22`) so the consumer can pick the namespace from `start_byte`.
- `graph_module_dep_edge(import) -> ResolvedImport` / a resolved-import traversal — for
  `module_graph.rs`; resolves named `use`/`pub use` re-export **and glob** bindings via `resolve`
  *regardless of callable* (a `use crate::m;`, a type-only import, a re-export-only dep are all real
  module edges — `module_graph.rs:85` is call-derived-only today, `:186` uses the old `CallGraph.imports`
  labels). Same authority/fall-through rules; different target predicate.

- [ ] **Step 1: Failing tests** —
  - **(a) module-deps (via `graph_module_dep_edge`, NOT callable-only — round-3 F3):**
    `use crate::util::helper;` → resolved edge to `util.rs`; a **type-only** `use crate::types::Config;`
    (non-callable) → resolved edge to `types.rs` (proves module-deps does NOT go through the callable-only
    helper); external `use std::…` → `UnresolvedModule`/external label (replaces the Rust
    call-derived-only contract). Also a **`pub use` re-export chain** (`a` `pub use`s `b::thing`; a third
    module imports it from `a`) → module-deps shows the resolved edge to the defining file, not a dangling
    re-export hop (round-2 F5). **Glob module-edge (round-4 nit 4):** `use crate::prelude::*;` → a
    module-deps edge to `prelude` **without expanding members** — the glob is a real module dependency for
    `module-deps` even though its members stay *poisoned* for call resolution (Task 3 real-glob-poison);
    the two coexist (deps-edge yes, call-narrowing no).
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
  - **(c″) graph-authority states (round-2 F2 / round-3 F2 / round-5 F2):** for the SAME
    `crate::engine::start()` site — (1) **complete** whole-workspace graph covering the file → graph edge
    (legacy skipped); (2) NO graph (skeleton / `scope_graph` absent) → legacy edge unchanged (no recall
    loss); (3) **incomplete/subset** graph (`complete=false`) → NOT authoritative → legacy edge unchanged
    (round-3 F2 trap: a subset must not masquerade as authoritative); (4) the **target** `engine` is
    `Poisoned` (parse-failed / missing `mod`) but the call site's own scope is `Modeled` → authoritative,
    result `Poisoned` → fall through (NO edge, legacy skipped); (5) the **containing** file is `Unmodeled`
    (parse-failed) → NOT authoritative → legacy edge unchanged. Assert `authoritative_for` true only for
    (1)+(4); (2)+(3)+(5) byte-identical to pre-PR-3. This pins the round-5 F2 site-vs-target rule (a
    poisoned target ≠ a non-authoritative site).
  - **(d) callable-edge fall-through guards (do-now) — `graph_callable_edge` only:** every
    non-`Resolved`-single-callable result avoids a **call** edge/narrowing — assert each of:
    `Resolved(Target::External)` (e.g. `std`), `Resolved` non-callable `Item` (a type/const),
    `Resolved(Target::Scope)`, `ResolvedSet` (>1 file), a cfg-exclusive multi-candidate result, and
    `Poisoned` → NO call edge. (Note: a non-callable `Item` is NOT an error for module-deps — it is a
    valid `graph_module_dep_edge`; the two helpers diverge here by design — round-3 F3.)
  - **(e) macro-invocation guard (round-5 F1):** `fn m(){} fn g(){ m!(); }` → the `m!()` site produces
    **NO call edge** to the Value `fn m` (it resolves in the Macro namespace, Phase-1 poison → fall
    through), with legacy skipped when graph-authoritative. Asserts the `CallSite` syntax discriminator is
    wired so a macro invocation never does a Value lookup.
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
- [ ] **Step 5:** **Paste any Tier-A flips/regressions into the PR description — do NOT re-baseline**
  (per CLAUDE.md; expected precision-positive / recall-up). Re-baselining is a separate human-triggered
  step, not part of this PR.

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
  Tier-A `--quick` is the regression net; the legacy qualified heuristic is disabled *only* where the
  graph is authoritative — pinned by the **completeness-aware `authoritative_for(callsite)` contract**
  (per-graph `complete` AND per-site rib found) + the four-state authority tests (Task 4 c″): a
  subset/skeleton/incomplete graph is authoritative for NO site → legacy byte-for-byte (no recall loss,
  and no subset masquerading as authoritative — round-3 F2); a covered-but-poisoned site falls through
  (no wrong-edge guess). The two helpers — `graph_callable_edge` (call, **namespace-correct** so `m!()`
  never edges to a Value `fn m` — round-5 F1) and `graph_module_dep_edge` (module-deps, non-callable
  imports too) — share the authority/fall-through contract but resolve different binding kinds (round-3
  F3). The `Modeled`/`Poisoned`/`Unmodeled` region state machine (round-5 F2) keys authority on the call
  *site's* scope, so a poisoned/missing target falls through without disabling legacy for unmodeled sites.
- **Cache correctness:** whole-repo rebuild + the widened key (manifests/file-set/cfg). A missed input
  ⇒ stale resolution. **Pinned by the Task-3 stale-topology test (F6)** — a `mod`/`#[path]` add/remove
  must change unchanged files' resolution to match a from-scratch build.
- **Scope creep:** implement ONLY the §11.5 surface; everything else falls through. Resist expanding
  glob/cfg/macro/qualified beyond *poison/representation* in Phase 1.
