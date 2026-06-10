I'll merge these two designs. Both architects did deep code-anchored verification and largely agree, which is strong signal. Let me adjudicate the few genuine divergences against the actual code before synthesizing — specifically the frontier-primitive choice, the MCP input-type choice, and the sanitizer fold-in scope.`taint_forward_cfg` and `cfg_reachable_lines` are **public `CodePropertyGraph` methods** (`src/cpg/cfg_queries.rs`), not buried in `taint.rs` — that's decisive for the frontier-primitive divergence. Let me read the load-bearing sections.Decisive confirmations: `taint_forward_cfg` (line 174-176) **bypasses the CFG filter for cross-function targets** — it does *not* enforce the intraprocedural boundary, so the reasoning layer must filter same-`(file,function)` itself. `dfg_forward_reachable` returns a `BTreeSet<VarLocation>` (no predecessor chain). `GraphNode { symbol, location }` has no `why` field (confirms cleansing can't ride on graph nodes). Now the three remaining divergences: MCP input type, `resolve_fn` behavior, sanitizer fold-in scope, and the output cap.I've verified the divergences against the actual code. Findings are decisive. Here is the merged design.

---

# Merged Design — Tier 2 Plan A: `taint_reaches` v1

## Approach (convergent spine — both architects chose this independently)

A new **always-compiled** `src/reasoning/` layer, peer to `src/navigation/`, exporting a pure function that borrows the warm `NavigationSession` (which already owns the whole-repo CPG with DFG+CFG and the parsed files), wraps existing CPG taint primitives, and returns the existing `Evidence` envelope. Only the MCP **tool registration** is `#[cfg(feature = "mcp")]`. This is the verbatim Option-C / additive-accessor pattern Tier 1 used — default `cargo build`/`cargo test` stay byte-for-byte unchanged. Both architects converged on every load-bearing element of this, and the code confirms each one:

- `taint_forward_cfg` and `cfg_reachable_lines` are **public `CodePropertyGraph` methods** (`src/cpg/cfg_queries.rs:105,129`) — reusable from reasoning with **no coupling to the 10k-line `taint.rs`**.
- `dfg_forward_reachable` returns a flat `BTreeSet<VarLocation>` (`src/cpg/query.rs:508`) — **no predecessor chain**, so witness reconstruction needs new code (both agree).
- `GraphNode { symbol, location }` has **no `why`/provenance field** (`src/navigation/types.rs:100`) — only `EvidenceItem` carries `why` — so cleansing cannot ride on graph nodes (Claude's catch; confirmed).
- `build_result` truncates `items` **and** `graph.nodes` under one shared `retained_count` (`src/mcp/output.rs:142,159,166`) — so **one shape per response**: frontier→items, witness→graph (both agree; confirmed).

## Component / file boundaries

```
src/reasoning/                 NEW — always-compiled, peer of src/navigation/
  mod.rs                       pub use; re-exports
  seeds.rs                     SeedSpec / ResolvedSeed / SeedSet / resolve_seed_set  (shared primitive)
  taint_reaches.rs             taint_reaches(session, request) -> Result<Evidence, QueryError>
                               + dfg_witness_path() reconstruction BFS (or place on CPG; see Decision 4)
src/lib.rs                     add `pub mod reasoning;`  (UNGATED — only `mcp` stays gated)
src/mcp/input.rs               parse reuse (see Decision 2)
src/mcp/tools.rs               reason_taint_reaches registration  (#[cfg(feature="mcp")])
src/mcp/registry.rs + tests    6 → 7 tools
Cargo.toml                     new [[test]] target for tests/reasoning_*  (targets are explicit)
```

The engine depends only on `navigation::{types, NavigationSession}`, `cpg`, and (for cleansing) `algorithms::taint` via a `pub(crate)` seam — **never** on `src/mcp/`.

## Key interfaces / types

```rust
// src/reasoning/seeds.rs — the reusable seed-set primitive (the 3 later tools inherit it)
pub enum SeedSpec {
    Loc    { file: String, line: usize },
    Symbol { name: String, file: Option<String> },
}
pub struct ProgramPoint { pub file: String, pub line: usize }
pub struct ResolvedSeed { pub point: ProgramPoint, pub symbol: Option<SymbolRef>, pub origin: SeedSpec }
pub struct SeedSet      { pub seeds: Vec<ResolvedSeed>, pub warnings: Vec<Warning> }

pub fn resolve_seed_set(s: &NavigationSession, specs: &[SeedSpec]) -> Result<SeedSet, QueryError>;

// src/reasoning/taint_reaches.rs
pub struct TaintReachesRequest {
    pub sources:     Vec<SeedSpec>,
    pub sinks:       Option<Vec<SeedSpec>>,
    pub max_results: usize,
}
pub fn taint_reaches(s: &NavigationSession, req: TaintReachesRequest) -> Result<Evidence, QueryError>;
```

I took **Claude's `SeedSpec`/`ResolvedSeed`/`SeedSet` split** (resolution returns a resolved set + warnings) over Codex's monolithic `VariableSeed` — the split is the cleaner "shared input primitive" the contract demands, and keeps resolution reusable by the later three tools. I took **Codex's `TaintReachesRequest` shape** for the top-level call. Codex's optional `path`/`access` field-sensitivity is deferred to v2 (see Decision 2 — `deny_unknown_fields` forces a new parse type the moment those fields appear).

**Additive vocabulary** (both verified safe — `src/output/navigation.rs:42` has a catch-all `other =>` arm and `WarningKind` renders via `{:?}`; **do not** add a `QueryError` variant — `error_text` is an exhaustive match):

- `Reason::TaintedBy { source: SymbolRef, cleansed_for: Vec<String> }` — **`Vec<String>` of category names** (Claude's G1 fix): the engine maps `FlowPath.cleansed_for: BTreeSet<frameworks::SanitizerCategory>` (`src/data_flow.rs:57`) to strings at its boundary, keeping `navigation::types` free of a `frameworks` dependency.
- `WarningKind::InterproceduralBoundary` — the visible cross-function deferral (constraint #4).
- `WarningKind::CleansedFlow` — witness-mode cleansing, since `GraphNode` has no `why` (Claude's G2). *(Or reuse the existing `SkippedPath`/`ResultTruncated` where they fit — these two new kinds are the only genuinely new ones.)*

## The flow

**Seed resolution — kind-split (both architects independently reached this; `resolve_fn` confirmed unsuitable):**
- `resolve_fn` routes a `Loc` seed through `enclosing_function` and returns the **function node, discarding the exact line** (`src/navigation/seed.rs:44-52`), and resolves `Symbol` against the **function** `name_index`. A taint seed is a *precise program point on a variable*, so:
  - **`Loc{file,line}`** → anchor at the **exact** `(file,line)`; do **not** call `resolve_fn`. Recover its dropped range-check by probing `enclosing_function`; `None` → per-seed warning + skip.
  - **`Symbol`** → resolve to a variable/program point (see Decision 1 — this is the one genuinely open semantic).
- **Mandatory** (both flagged, confirmed at `src/data_flow.rs:208-218`): a seed line with **zero `Variable` nodes** (field-only parameters are skipped from DFG def registration) → emit `WarningKind::SkippedPath` ("no taint-trackable variable at seed"), not a silent empty result.
- Per-seed failure degrades to a warning + skip; **all-empty** → `QueryError`.

**Frontier (sinks omitted)** — reuse the CFG-correct primitive:
1. `let paths = cpg.taint_forward_cfg(&source_points)` — handles dead-code pruning *and* the multi-line-statement continuation subtlety internally (`src/cpg/cfg_queries.rs:178-185`), so the engine never touches the private continuation helper.
2. Flatten `edge.to` to a frontier `BTreeSet<VarLocation>`. **Critical filter the engine must add itself:** `taint_forward_cfg` *deliberately lets cross-function targets bypass the CFG filter* (`src/cpg/cfg_queries.rs:174-176`) — it does **not** enforce intraprocedural scope. So drop any `edge.to` whose `(file, function)` differs from the source and emit `InterproceduralBoundary` (constraint #4 — visible, not a silent false negative).
3. Map frontier → `EvidenceItem { symbol: SymbolRef::Variable, score, source: PrismCpg, why: [TaintedBy{..}] }`; sources at score `1.0`. Render variable paths with `AccessPath::Display`, **not `Debug`** (Codex's point; nav currently leaks `Debug`). Deterministic sort by `(file, line, path)`. **Items only, no graph.**

**Witness (sinks given)** — frontier is the source of truth; witness is its chain (Claude's G3 fix, the sharpest single correction):
1. Compute the same CFG-pruned, same-function frontier set as above.
2. For each resolved sink point, test **membership in the frontier** → that is the authoritative "reached?" answer.
3. For reached sinks, reconstruct an ordered path with a **new parent-tracked BFS** (`dfg_witness_path`) **confined to the frontier node set** — it walks the same DataFlow edges + same-line assignment propagation as `dfg_forward_reachable` (`src/cpg/query.rs:534-562`) but only enqueues neighbors already in the frontier. Because it can never visit a node the CFG-pruned frontier excluded, the witness **cannot contradict the frontier** (no dead-code witness) and it needs no private CFG helper. Deterministic: sort neighbors by `NodeIndex::index()`.
4. Emit `GraphPayload`: nodes = path points, edge `kind = "DataFlow"` for real edges, `"AssignmentPropagation"` for synthetic same-line steps. **Graph only, items empty.**
5. Sinks not in the frontier because they sit across a function boundary → `InterproceduralBoundary` warning.

**State/flow:** the engine is pure — borrows `&NavigationSession` immutably (safe under single-threaded MCP dispatch), allocates only `BTree*`/`Vec`, returns an owned `Evidence`. No CPG rebuild.

**MCP handler** mirrors `nav_callers`: parse → `SeedSpec` → `reasoning::taint_reaches` → `shape_result(...)`. The existing binary-search cap already serves both shapes via `retained_count`; let the **query** apply the `max_results` witness cap and set `truncated`/`ResultTruncated` itself, then call `shape_result` so its node-count clip composes rather than clobbers (the code already composes: `src/mcp/output.rs:179`).

## Decisions + rationale (resolved divergences)

| # | Divergence | Resolution + why (code-verified) |
|---|---|---|
| Frontier primitive | Codex: own BFS over `CpgEdge::DataFlow`. Claude: reuse `taint_forward_cfg`. | **Reuse `taint_forward_cfg`** for the frontier. It is a clean public CPG method (not in `taint.rs` as Claude's citation implied), already encodes the CFG-pruning + continuation-line logic, and honors constraint #3 ("wrap, don't reimplement"). Codex's own-BFS would have to re-handle the *private* continuation helper. |
| Witness reconstruction | Both: needs new predecessor BFS. | **Keep it** — but Codex's "needs predecessors" + Claude's "confine to frontier" combine: one new `dfg_witness_path` BFS gated on frontier membership. This is the only genuinely new traversal. |
| Intraprocedural filter | Both flagged it. | **Mandatory in the engine** — confirmed `taint_forward_cfg` does *not* enforce it (`cfg_queries.rs:174-176`). |
| Output shape | Both: shared cap. | **One shape per response** — confirmed `build_result` truncates items+graph under one count. |
| Cleansing scope | Codex: defer (sink-suppression is private). Claude: fold in `apply_cleansers`. | **Partly a false conflict.** They agree v1 does no vulnerability *judgment*. The contract requires reflecting "cleansing the substrate models," so v1 **surfaces `cleansed_for` as honest metadata** (empty = "no sanitizer modeled") but never *suppresses* a reachability answer. Codex's "bigger than two functions" is right (`apply_cleansers` pulls `function_body_cleansed_for`, `is_js_ts_language`, `collect_calls`, `call_path_*`, `paired_check_satisfied`), but none of those are the sink-suppression privates he cited — those belong to the full `slice`, which v1 does not touch. So: **minimal `pub(crate)` exposure of `apply_cleansers` in place**, defer the relocation to `src/sanitizers/cleanse.rs` (Claude's Slice 0) as optional cleanup. This keeps `taint::slice` byte-for-byte and ships cleansing as a *later additive slice*, not a blocker. |

## Risks

- **R1 — Symbol-seed semantics is genuinely undecided** (Decision 1 below). Don't ship until resolved; it changes the resolver.
- **R2 — `pub(crate)` on `apply_cleansers` couples the engine to `taint.rs`.** Mitigated by deferring relocation; revisit if the coupling spreads.
- **R3 — Cleansing only fires for Go/Python/JS-TS** (`taint.rs:10656`); for C/C++/Rust `cleansed_for` is always empty. Correct and honest, but document it so empty isn't read as "verified clean."
- **R4 — `cargo fmt`/golden drift.** Run the nav golden + diff-review suites after every slice to prove Option-C.
- **R5 — New `tests/reasoning_*` files need explicit `[[test]]` targets** (Codex; `Cargo.toml` lists them) — easy to forget, silently un-run otherwise.

## Smallest shippable slices + build order (TDD)

0. **(Optional, can fold late) sanitizer access.** `pub(crate)` on `apply_cleansers` + transitive privates *in place* (not relocated). Gate: existing `algo_taxonomy_sanitizers*` unchanged. *Relocation to `sanitizers/cleanse.rs` is a separate, deferrable cleanup.*
1. **Frontier engine.** `src/reasoning/{mod,seeds,taint_reaches}`; `pub mod reasoning;`; kind-split `resolve_seed_set` + zero-Variable-node warning; frontier via `taint_forward_cfg` + intraprocedural filter. *Failing tests:* Loc source taints downstream line; Symbol seed anchors correctly; field-only param emits the warning; cross-function target dropped with `InterproceduralBoundary`.
2. **Sinks-given reachability.** Membership-test sinks against the frontier; reached ⇒ hit, unreached cross-function ⇒ boundary warning (no silent false negative).
3. **Ordered witness.** `dfg_witness_path` confined to frontier; witness mode returns `GraphPayload`. *Failing tests:* nodes ordered source→…→sink; `DataFlow` edges; a dead-code-only sink produces **no** witness (proves frontier/witness can't disagree).
4. **Cleansing metadata.** Wire `apply_cleansers`; frontier surfaces `Reason::TaintedBy{cleansed_for}`, witness surfaces `CleansedFlow` warning. *Failing test:* Go/Python source→sink through a recognized cleanser ⇒ non-empty `cleansed_for`.
5. **MCP registration (gated).** `reason_taint_reaches`; registry/smoke tests 6→7 (mcp-gated, not a default golden); new `[[test]]` target. *Failing tests under `--features mcp`:* unknown-arg rejected, empty `sources` rejected, frontier-vs-graph shape, size-cap honored.

Each slice keeps the default suite byte-for-byte; new behavior lives under `tests/reasoning_*` and the gated MCP tests.

---

## DECISIONS FOR THE OWNER

1. **Symbol-seed semantics (must decide before coding — changes the resolver).** This is the one place the two architects chose *different behavior*, not different implementations of the same behavior.
   - **(a) Symbol = variable** (Codex): scan `Variable` nodes, match `path.base`; `--source userInput` taints that variable. More precise; the natural long-term model once field-sensitive `path`/`access` seeds arrive.
   - **(b) Symbol = function, taint enters at params** (Claude): reuse `resolve_fn`, anchor at `start_line`; `--source processRequest` taints all its parameters. Cheaper, and **consistent with nav's existing `SeedInput::Symbol`**, which already means "function" everywhere in prism-mcp.
   - **Recommendation:** lean **(b)** for surface consistency in v1 (one input shape shouldn't mean "function" in nav tools and "variable" in this one), and add **(a)** as the field-sensitive extension in v2. But this is a genuine product call — if taint users primarily think in variables, (a) is more honest. *Pick before Slice 1.*

2. **MCP input type: reuse `SeedInput` vs new `TaintSeedInput`.**
   - Confirmed: `SeedInput` is `#[serde(deny_unknown_fields)]` with only `Symbol{name,file}`/`Loc{file,line}` (`src/mcp/input.rs:22-27`), and `to_triple` is **generic, not function-specific** — the function-orientation lives entirely in `resolve_fn`. So **reuse is safe for v1** (Claude). The moment you add Codex's field-sensitive `path`/`access`, `deny_unknown_fields` forces a `TaintSeedInput`.
   - **Recommendation:** reuse `SeedInput` for v1; introduce `TaintSeedInput` only when (a) above goes field-sensitive. Low-stakes, but tied to Decision 1.

3. **Cleansing relocation now vs later.** Surface cleansing metadata in v1 (contract-required) via minimal `pub(crate)` (recommended), and defer the `apply_cleansers` → `src/sanitizers/cleanse.rs` relocation. Owner may prefer to do the relocation up front if they dislike the `taint.rs` coupling (R2) — it's a clean-but-non-trivial move (~6 helpers) gated by the sanitizer suite.

4. **Witness multiplicity.** Single shortest witness per source→sink (deterministic, cheap) vs all witnesses up to `max_results` (exponential). **Recommend shortest for v1** (both architects agree).

---

**Readiness verdict:** Ready to plan after deciding **#1 (Symbol-seed semantics)** — that's the only true blocker; #2–#4 have clear recommendations and can be confirmed in passing.