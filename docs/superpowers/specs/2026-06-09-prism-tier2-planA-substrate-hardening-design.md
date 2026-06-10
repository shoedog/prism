# Prism Tier 2 — Plan A: Substrate Hardening (Design)

**Status:** Owner design, **revision 3 (2026-06-09)** — final sweep after three prism-wired spec-review
rounds; v3 verdict **"sound to plan."** Witness design **(A)** locked. **Plan A gate = A3 + A4 + A7** (A6 is
non-gating hygiene; A2/A5 are Phase-IP). Input to `writing-plans`. Pure-implementation detail (exact JSON
bytes, category-string casing) is pinned in the plan, noted inline as **[→plan]**.

**Goal:** Make the substrate sound enough that Tier-2 `taint_reaches` v1 produces non-misleading Evidence —
**without changing one byte of diff-review or nav output.**

## 1. Scope

Slice 1 (split `src/cpg.rs`) is **merged** (PR #92). Plan A is the **three gating slices** intraprocedural
`taint_reaches` v1 consumes:
- **A3** — the witness/frontier engine (single inline-CFG-filtered predecessor BFS).
- **A4** — one reasoning adapter over the existing sanitizer seam.
- **A7** — the `src/reasoning/` scaffolding + the output-shaping seam.

**Non-gating:** **A6** (DFG/CPG push-guard) is separable opportunistic hygiene — A3 carries its own
push-time guard (below), so A6 buys the gate nothing and only widens the golden-proof surface; land it
on its own when convenient. **Phase-IP (interprocedural; renamed from "Plan A.5"):** **A2**
(`compute_bindings` extraction + `Precision`) and **A5** (Rust `?` overlay edge) — neither has a v1 consumer;
§9 holds their contracts.

## 2. Architecture — overlay-only, no overlay data structure

Tier-2 reasoning is an **ephemeral, read-only computation over the production `CpgContext` petgraph** — there
is **no `ReasoningGraphView`**. "Overlay-only" is the principle (no persisted mutation), not a structure.
- **Substrate engine pin:** Tier-2 builds on the `CpgContext` **petgraph** engine. `DataFlowGraph`'s separate
  `forward` map + `forward_reachable` (`data_flow.rs:479`) are **legacy** for Tier-2.
- **Option-C by construction for A7;** A4 changes `taint.rs` visibility only (no behavior) and is proven by
  the §7 matrix. No `CACHE_VERSION` bump (a bump forces a fleet-wide cold rebuild → worsens the measured
  ~27s-cold vs 30s-ACP-handshake fragility).

## 3. A3 — the witness/frontier engine (invariant by construction)

A single **predecessor-tracking forward BFS** over the petgraph from the source `VarLocation`s, following
**DataFlow + same-line assignment-propagation** edges (`query.rs:535-562`) — **never `ControlFlow`** edges —
applying the CFG filter **inline**.

**`cfg_valid(source, target)` — defined exactly (B1):**
- If the CPG has **no CFG edges** (`has_cfg_edges()` false): fall back to pure taint — valid iff `target` is
  in the **same `(file, function)`** as `source` (no CFG pruning), matching `taint_forward_cfg`'s fallback
  (`cfg_queries.rs:133-135`).
- Else, with `cfg_set = cfg_reachable_lines(source)` (which **excludes the source's own line**,
  `cfg_queries.rs:99-105`): valid iff `target` is same-`(file,function)` **and** (`target.line == source.line`
  — same-line always included, matching `:171-185`) **or** `target.line ∈ cfg_set` **or**
  `cfg_reachable_including_continuation(target.line, cfg_set)`.
- The check is **per-node** (target reachable from source), an explicit **over-approximation** — not pairwise
  along the path; v1 Evidence is framed honestly as a "data-flow path" (§5), not a proven control-flow path.

**Traversal & outputs.** BFS in **`NodeIndex` space**; a neighbor is enqueued only if `cfg_valid` holds,
recording its parent at **push time** (A3 carries its **own push-time `visited` guard** — the existing
engines dedup at *pop*, so "first-enqueue-wins" cannot reuse them). Neighbors sorted by `NodeIndex::index()`;
**first enqueue wins** the parent slot. When both a DataFlow and a same-line edge reach one target,
**DataFlow wins the parent slot** and the parent carries that relation label. A neighbor in a **different
`(file, function)`** (a Step-5b arg→param edge) is **not traversed** — recorded as a `BoundaryEdge`.

```rust
pub enum Relation { DataFlow, AssignmentPropagation }
pub struct BoundaryEdge { pub root: NodeIndex, pub from: NodeIndex, pub to: NodeIndex } // names the dropped source
pub struct Trace {
    pub frontier: BTreeSet<NodeIndex>,
    pub parents:  BTreeMap<NodeIndex, (NodeIndex, Relation)>, // child -> (parent, relation); relation also recomputable from petgraph edge
    pub boundary: Vec<BoundaryEdge>,
}
```
- **Invariants (by construction):** `reached(sink) ≡ sink ∈ frontier`; every `frontier` member has a
  dead-end-free witness via `parents` walk-back. Frontier = "reachable via a path of per-node-CFG-valid
  intraprocedural def-use steps" — tighter than `taint_forward_cfg`'s node-wise over-approximation.
- **Reachability is tri-state** (consumed by `shape.rs`/Plan B): `Reached` (sink ∈ frontier), `NotReached`
  (sink resolved, not in frontier, no boundary), **`BoundaryExited`** (sink only reachable through a
  `BoundaryEdge` — "indeterminate, stopped at a call boundary," **never** rendered as "safe").
- **Conversion:** witness stays in `NodeIndex` space; convert to `VarLocation`/`file:line`/`AccessPath` only
  at the `shape.rs` boundary (`to_var_location`, `query.rs:568`) — a `VarLocation`-keyed map would be lossy.
- **Forward-only** (backward provenance = v2); **additive** (new method beside `dfg_forward_reachable` /
  `taint_forward_cfg`; their 6 call sites + the Taint slice untouched).

## 4. A4 — one reasoning adapter over the sanitizer seam (consult, not populate)

`active_recognizers()` is already populated (python 5, js_ts 3, path 2); `SHELL_RECOGNIZERS` is empty by
design (Go uses sink-time AST+CFG); production `apply_cleansers` (`taint.rs:10645`, called `:10848`) already
enriches cleansing. The raw primitive hardcodes empty (`cfg_queries.rs:189-199`).
- **Deliverable: one reasoning-facing adapter** — `cleansed_categories_for_source(source: VarLocation) ->
  Vec<String>` — not two raw internals. It is `pub(crate)`, wraps `function_body_cleansed_for` over the
  **frozen** recognizer set, and defines the `source==sink` and empty-witness cases (`apply_cleansers` bails
  on empty `FlowPath` fans; the adapter must not). Category-string vocabulary + casing **[→plan]**.
- **Cleansing is function-body sanitizer *presence keyed on the source function*, not path-proof.** The
  output **shape encodes this** (§5) — `sanitizers_present_in_source_fn` + `path_proven: false` — so an agent
  cannot misread it. Do **not** populate the global registry; honest-empty for C/C++/Rust stays.
- **Layering inversion** (reasoning reaching into `taint.rs`, whose helpers depend on taint-local
  `collect_calls`/`call_path_*`/`is_js_ts_language`) is **minimal and temporary**; its relocation into
  `src/sanitizers/` is **paired with A2** and tracked by a **dated/issue obligation [→plan]**, not left open.
- **Proof:** `algo_taxonomy_sanitizers*` fixtures byte-unchanged.

## 5. The output-shaping seam (`shape.rs`; tunable while dogfooding)

Reuses the existing **`concise|detailed`** verbosity (no new enum); `detailed` adds per-step relations +
cleansing annotations. Defaults follow established practice:
- **Edge kinds named by relation:** `"DataFlow"` / `"AssignmentPropagation"` — **never** `"TaintFlow"` or
  `"ControlFlow"` (the BFS follows no CFG edges).
- **Witness = an ordered simple path**, each step self-describing (`file:line`, `AccessPath::Display` not
  `Debug`, relation); summary text says **"data-flow path"** (not "the path the taint takes") + a one-line
  **field-insensitive / over-approximate** caveat.
- **Cleansing labeled in the shape:** `sanitizers_present_in_source_fn: [...]` + `path_proven: false` —
  never a bare `cleansed_for: ["xss"]` that reads as path-safe.
- **Tri-state output:** `Reached` → witness graph; `NotReached` → `reached:false` + summary; **`BoundaryExited`
  → its own shape** (summary + the `BoundaryEdge`s), distinct from `NotReached`.

## 6. Owner decisions (resolved)

| Decision | Resolution |
|---|---|
| Overlay | **Overlay-only principle, no overlay data structure** |
| Witness engine | **(A) single inline-CFG-filtered predecessor BFS**, own push-time guard, NodeIndex-space |
| Plan A gate | **A3 + A4 + A7**; A6 non-gating hygiene; A2 + A5 = Phase-IP |
| Reachability | **Tri-state**: Reached / NotReached / **BoundaryExited** |
| Cleansing shape | `sanitizers_present_in_source_fn` + `path_proven:false` (weakness in the **shape**) |
| A4 surface | **one adapter** `cleansed_categories_for_source`; relocation paired w/ A2 (dated obligation) |
| Edge kinds | relation-named (`DataFlow`/`AssignmentPropagation`); no `ControlFlow`/`TaintFlow` |
| Substrate engine | `CpgContext` petgraph; `DataFlowGraph` reachability = legacy |

## 7. Testing — exact targets + proof matrix

New reasoning tests live in **`tests/reasoning_*.rs`** with **explicit `[[test]]` targets** registered in
`Cargo.toml` (mirror `cli_nav_compat`, `Cargo.toml:494-496`); A3/A6 construction tests go in the
`src/cpg/tests.rs` unit module (run via `cargo test`).
- **A3 (test-first, must-pass-before-done):** the **`S→I→K` interior-node case** (membership ⇔ dead-end-free
  witness); same-line `x = source` propagation (proves the start-line-exclusion landmine is handled);
  no-path; `BoundaryExited`; absent-CFG fallback; deterministic parent on DataFlow-vs-same-line tie.
- **A4:** `algo_taxonomy_sanitizers*` fixtures byte-unchanged; adapter category output for a Go/Python source.
- **Byte-identity proof matrix:** assert `cargo test --test cli_nav_compat` (nav + LeftFlow diff-review
  goldens) + `cargo fmt --check`. **Do NOT** claim byte-identity via the aggregate `review` preset — it is
  not byte-stable (Taint nondeterministic, `nav_compat_test.rs:17-22`). A4 touches only `taint.rs` *symbol
  visibility*, so its blast radius is "does it still compile + fixtures unchanged," not a reachability change.
- **Acceptance fixture:** a multi-step intraprocedural def-use chain *with* a real sanitizer marker (gate not
  met by source≈sink-adjacent witnesses alone).

## 8. Hand-off to Plan B (enumerated Rust contracts)

Plan B owns the normative `src/reasoning/` surface and consumes these A3 types **by name:** `Relation`,
`BoundaryEdge`, `Trace { frontier, parents, boundary }`, the tri-state reachability enum, and the A4 adapter
`cleansed_categories_for_source`. The additive `Evidence` summary field name and its serde schema are pinned
in Plan B §8. Plan B is gated on **A3 + A4 + A7**.

## 9. Phase-IP / hygiene contracts (deferred; specify before scheduling)

- **A2 — `compute_bindings` extraction** (`Production`-only dedup of Step 5b, `build.rs:327-405`), pinned by a
  characterization fixture snapshotting ordered `CpgEdge::DataFlow` pairs (first-call-on-line, `.`/`->`
  truncation, Use-before-Def, callee-range, `ast.rs:2734-2750,2925-2969`). `Precision::Overlay` field-sensitive
  mode + the A4→`src/sanitizers/` relocation land here.
- **A5 — Rust `?` overlay edge:** specify edge type, synthetic exit-target, AST detection, direction, relation
  label, consuming traversal, + a Rust-`?` Taint golden. Note: a clean `reached:false` on a `?`-laden Rust
  flow that crosses the error channel reflects **scope, not proof of safety** (relevant to the Rust dogfood).
- **A6 — push-guard hygiene:** if landed, name every affected function and whether the guard is `visited`-only
  vs a separate `enqueued` set; output-neutral.
