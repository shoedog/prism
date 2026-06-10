# Prism Tier 2 — `taint_reaches` (Plan B) Design

**Status:** Owner design, **revision 3 (2026-06-09)** — final sweep after three prism-wired spec-review
rounds (v3: "architecture sound and unusually well-grounded"). Witness design **(A)** locked. **Depends on
Plan A: A3 + A4 + A7.** Pure-implementation detail is pinned in `writing-plans`, noted **[→plan]**.

## 1. Context & data acquisition

Tier 2 is the seeded reasoning layer: a shared `SeedSet` + four tools. **This spec covers `taint_reaches`
only.** The DFG is retained inside the cached `CodePropertyGraph` (`build.rs:47-49`; `from_ctx` keeps
`ctx.cpg`, `navigation/mod.rs:67`; round-tripped by the cache), so `taint_reaches` runs **Plan A's A3 BFS
directly over `session.index.cpg`'s petgraph** — read-only, no rebuild, `taint_forward_cfg`/the Taint slice
untouched (Option-C).

## 2. Tool contract

```rust
pub fn taint_reaches(session: &NavigationSession, sources: &[SeedSpec], sinks: Option<&[SeedSpec]>)
    -> Result<Evidence, QueryError>;            // resolves seeds internally (B2)
```
The library fn takes **raw `SeedSpec`s and resolves them internally** (`SeedSet` is the internal resolved
type, §3); the MCP layer passes the parsed `SeedInput`s straight through. **Sinks given** → witness mode
(per-sink **tri-state** reachability + a witness per reached sink); **sinks omitted** → frontier mode (items;
top-level reachability `None`). `taint_reaches` = taint semantics; the later `dataflow_between` = generic
chop. **Reach scope v1 = intraprocedural by construction** (A3 does not traverse cross-`(file,function)`
edges; each is a `BoundaryEdge` → a **`BoundaryExited`** sink + sink-located warning). Cross-function = Phase-IP.

## 3. `SeedSet` — the shared input contract

Resolved set `SeedSet`; input element `SeedSpec`; module `src/reasoning/seeds.rs`.
```rust
pub enum SeedSpec { Loc { file: String, line: usize }, Symbol { name: String, file: Option<String> } }
pub struct ResolvedSeed { pub locations: Vec<VarLocation>, pub symbol: Option<SymbolRef>, pub origin: SeedSpec }
pub struct SeedSet      { pub seeds: Vec<ResolvedSeed>, pub warnings: Vec<Warning> }
```
- **`Loc{file,line}`** → all `Variable` locations on the exact line (taint starts from every `Variable` at a
  line, `cfg_queries.rs:149-164`); never `resolve_fn` (drops the exact line, `seed.rs:44-52`).
  **`enclosing_function` returns `None` for three distinct cases — disambiguate (B3):** missing/unindexed
  file → `UnsupportedFile`; line beyond file end → `LocationOutOfRange`; in-file but outside any function →
  empty `Evidence` + warning (a valid answer). Line with zero `Variable` nodes → `SkippedPath`.
- **`Symbol{name}` = the enclosing function; sources = PARAMETERS ONLY** (decision (b)), via
  `function_parameter_names`; each param's `Def` sits at the function start line; field-only params skipped
  (`has_bare_references`, `data_flow.rs:208-218`). **Edge cases (B3/M6):** zero-parameter function, or **all**
  params field-only/without `Def`s (empty source set) → empty `Evidence` + warning (matches "outside any
  function"); **some** params unresolved → partial `SkippedPath` naming them; resolved params proceed.
- **Wire shape:** reuse MCP `SeedInput` (`deny_unknown_fields`, `input.rs:22-27`) for v1; `TaintSeedInput`
  arrives with the (a) field-sensitive seed in v2.

## 4. Query data flow

1. Resolve `sources` (+ `sinks`) → `VarLocation`s (§3, §7).
2. **A3 BFS** over `session.index.cpg` → `Trace { frontier, parents, boundary }` (Plan A §3; `NodeIndex`
   space). `reached(sink)` is tri-state: `sink ∈ frontier` → **Reached**; reachable only via a `BoundaryEdge`
   → **BoundaryExited**; else **NotReached**.
3. **Cleansing (A4 adapter):** `cleansed_categories_for_source(source)` → sanitizer categories present in the
   source function. **Function-body presence keyed on the source, NOT path-proof.**
4. **Frontier mode (no sinks):** items = frontier nodes → `EvidenceItem { symbol, score, why:
   [Reason::Reasoning(TaintedBy{ source, sanitizers_present_in_source_fn, path_proven:false })] }`. **Score:**
   sources `1.0`; downstream items decay by **BFS depth** (closer = higher, the `gradient_slice` convention)
   **[→plan]** for the exact curve. Tie-break `(file, function, line, path, ordinal)`. **Items only.**
5. **Witness mode (sinks given):** per resolved sink, the tri-state from step 2. For **Reached** sinks, the
   witness = `parents` walk-back (invariant: never dead-ends — Plan A §3), reconstructed in `NodeIndex` space
   and converted at the shape boundary. **Node identity = full `(file, function, line, path, kind/access,
   ordinal)`** — **not** `(file,line,path)` (which would merge a `Def` and a `Use` on one line and drop the
   joining step). Edge **relation** comes from `parents` (or recomputed from the petgraph edge type), labeled
   `"DataFlow"`/`"AssignmentPropagation"`. One shortest witness per Reached sink; all witnesses **unioned into
   one `GraphPayload`** (deduped by full node identity, self-edges dropped). Each `SinkResult` links to its
   sink node via **`graph_node: Option<usize>`** (index into `GraphPayload.nodes`; `None` for
   NotReached/BoundaryExited/clipped). **Graph only.**
6. **Cleansing on multi-source sinks (m3):** **union** `sanitizers_present_in_source_fn` across all sources
   that witness the sink (so a longer-but-sanitized source isn't hidden by a shorter-but-unsanitized one).
7. **Determinism:** sort sources/sinks; neighbor order `NodeIndex::index()`; first-enqueue-wins; cycle-safe;
   summary counts are **pre-cap** (with `truncated` set if clipped). `max_results` caps **frontier items**
   (frontier mode) / **graph nodes** (witness mode) via the existing `retained_count`; `per_sink` stays
   complete after clipping (only `graph_node` may go `None`).

## 5. Placement & the shaper seam

`src/reasoning/`: `mod.rs`, `seeds.rs`, `taint_reaches.rs` (the query + A3's BFS call), `shape.rs` (the
output-shaping seam). **One public surface — no `ReasoningGraphView`.** The shaper reuses **`concise|detailed`**
(no new enum). A3's BFS and the A4 adapter are `CodePropertyGraph`/petgraph methods (reached as `ctx.cpg.…`;
`build_scoped` is on `CpgContext` and is **not** used). MCP registration is the only `mcp`-gated part.

## 6. Foundation folded in as task 0

- **Wire-size chokepoint** at `transport::write_message` (impl bodies `transport.rs:428`; `#[cfg(test)]
  InMemoryTransport` `:462`; `:68` is a call site) — already the single chokepoint for success + error frames.
  **Define, per frame class, the exact valid-JSON-RPC payload:** success / tool-`isError` / terminal over-cap
  (payload truncated with an explicit marker, envelope intact) / protocol-error (add a `_meta` path,
  `transport.rs:295-304`). Add `anthropic/maxResultSizeChars` to error `_meta` (`error.rs:162-168`). Exact
  marker bytes + truncation target **[→plan]**.
- **`default-run = "prism"`** in its **own commit**.

## 7. Error / empty-result truth table

| Situation | Result |
|---|---|
| Some seeds resolve, some fail | resolved proceed; each failure → `WarningKind::Reasoning(SeedUnresolved{spec,reason})`; not an error |
| Loc: missing/unindexed file | `QueryError::UnsupportedFile` (if all sources) / per-seed warning |
| Loc: line beyond file end | `QueryError::LocationOutOfRange` (if all sources) / per-seed warning |
| Loc/Symbol: in file, outside any function | empty `Evidence` + warning |
| Seed line with zero `Variable` nodes | `SkippedPath`; skipped |
| Symbol: zero-param / all params field-only or no `Def` | empty `Evidence` + warning |
| Symbol: some params unresolved | partial `SkippedPath` naming them; rest proceed |
| Sinks resolve, unreachable / boundary-only | tri-state `NotReached`/`BoundaryExited` + summary (valid answer) |
| **All** sources, or **all** sinks, fail | `QueryError` — **precedence by seed-input order** (first failure's kind) |

Warnings in seed-input order, deduped. Additive `Reason`/`WarningKind` variants are byte-safe (catch-all
`other =>` `navigation.rs:73`; `{:?}` render). **Quarantine reasoning growth (M4):** all reasoning concepts
nest under **`Reason::Reasoning(ReasoningReason)`** and **`WarningKind::Reasoning(ReasoningWarning)`** — one
new variant per nav enum — so nav's surface stays closed as the other three tools land. **Never add a
`QueryError` variant** (`error_text` exhaustive 5-arm match, `navigation.rs:13-23`).

## 8. Evidence vocabulary (typed contract)

```rust
pub enum Reachability { Reached, NotReached, BoundaryExited }

// additive on Evidence, #[serde(skip_serializing_if = "Option::is_none")] — byte-safe (proven by Evidence.graph)
pub struct ReasoningSummary {
    pub reachability: Option<Reachability>,   // None in frontier mode; aggregate (any Reached) in witness mode
    pub per_sink: Vec<SinkResult>,            // empty in frontier mode — agents read THIS, not the aggregate
    pub source_count: usize, pub frontier_count: usize, // pre-cap
}
pub struct SinkResult { pub sink: SymbolRef, pub reachability: Reachability, pub graph_node: Option<usize> }

pub enum ReasoningReason {                    // nested under Reason::Reasoning(..)
    TaintedBy { source: SymbolRef, sanitizers_present_in_source_fn: Vec<String>, path_proven: bool /* false v1 */ },
}
pub enum ReasoningWarning {                   // nested under WarningKind::Reasoning(..)
    SeedUnresolved { /* serialized via Warning.message/location; shape [→plan] */ },
    InterproceduralBoundary { /* sink-located; names the dropped source/edge */ },
    Cleansed { /* witness-mode: source fn has a sanitizer; not path-proof */ },
}
```
`path_proven` is always `false` in v1 — the field makes "presence ≠ path-safe" **unmissable in the wire
output**, not just the prose. Summary appears in both modes. Multi-source `boundary`: a sink Reached by A but
boundary-only from B → `reachability = Reached`, plus B's `InterproceduralBoundary` warning.

## 9. MCP surface

Library `taint_reaches`; tool **`reason_taint_reaches`** (read-only). **Registration:** `nav_v1()` stays
**==6** (test frozen); `prism-mcp`'s `run` serves `nav_v1()` + a new `reason_v1()`; a new test asserts the
**combined** registry is 7 and reasoning tools are absent from `nav_v1()`. **Schema** (matching `input.rs`):
`sources` (required `[SeedInput]`, `minItems:1`), `sinks` (optional; **empty `sinks:[]` = invalid**, omit for
frontier mode), `max_results` (int, default **[→plan]**), `verbosity` (`concise|detailed`, default `concise`).
The **tool description states the params-only scope** (taint entering F via a local/global/env read is
unrepresentable → `reached:false` is a scope artifact) and **steers agents to `per_sink`, not the top-level
aggregate**. `build_result` here = `src/mcp/output.rs:149` (≠ `spiral_slice.rs:282`).

## 10. Testing

- **`seeds.rs`:** the full §7 table — Loc 3 `None` cases, Symbol param edge cases, zero-Variable, partial.
- **`taint_reaches.rs`:** intraprocedural Reached/NotReached; frontier mode; the **`S→I→K` interior-node
  invariant**; a Go/Python source surfaces `sanitizers_present_in_source_fn` + `path_proven:false`; a
  cross-function sink → **`BoundaryExited`** + sink-located boundary warning; multi-sink union graph with
  `SinkResult.graph_node` linkage; multi-source cleansing union.
- **Evidence shape:** `ReasoningSummary` + `skip_serializing_if` keeps **nav goldens byte-unchanged**
  (`cargo test --test cli_nav_compat`); edge kinds relation-named, no `ControlFlow`.
- **MCP:** combined registry == 7, `nav_v1` == 6; smoke exchange; schema `minItems`/defaults.
- **Foundation:** oversized-**error**-frame wire-cap test (valid JSON-RPC after truncation).

## 11. Non-goals (v1) / deferred

Interprocedural → Phase-IP (consumes A3's `boundary` + Plan A A2/A5). Other 3 tools → later (reuse `SeedSet` +
shaper). (a) variable / backward provenance → v2. **Regression guard:** a test pins `taint_forward_cfg`'s
deliberate cross-`(file,function)` bypass as a contract (load-bearing for *two* consumers — the Taint slice
and, inversely, `taint_reaches`/Phase-IP), so a future "make it intraprocedural" change can't silently break
the boundary marker.

## 12. Process

Revision 3 is the final spec sweep → **`writing-plans`** (Plan A A3+A4+A7 first; Plan B gated on them) →
**plan-review** (the next adversarial gate, where the `[→plan]` implementation detail is pinned + checked).
