# Assemble Edge-Steps Parallelization (Steps 5 + 5b + 8) — Design

**Date:** 2026-06-19
**Status:** DRAFT (pre spec-review)
**Branch:** `step8-varnode-parallel`
**Predecessors:** S1.5 call-args index (#111) · Step-5b param memo (#112) · Step-7 statement-node parallelization (#114)

## 1. Motivation

After Step-7 parallelization (#114), the remaining serial cost in `assemble_graph`
is concentrated in three **edge-creation** steps. Measured on the current
post-Step-7 release binary via cold `nav --no-cache call-stats` (per-sub-step
timers, three corpora):

| sub-step | what it builds | hugo | tokio | prism |
|---|---|---|---|---|
| step5 | Call/Return edges (`resolve_call_site` per site) | 52ms | **852ms** | **1040ms** |
| step5b | Interproc DataFlow edges (resolve + arg-AST + param memo) | **417ms** | **972ms** | **1231ms** |
| step8 | ControlFlow edges (`cfg::build_cfg_edges` per file) | **588ms** | 229ms | **452ms** |
| **sum** | | **~1.06s** | **~2.05s** | **~2.72s** |

These are the three biggest remaining levers and they share one shape: a
**read-only per-unit compute** (resolve a call site / build a file's CFG edges)
followed by a **serial graph mutation** (`add_edge`). That is exactly the S1 **C2
pattern** already proven on Step 7: `ordered iter → par_iter compute → serial
deterministic apply`.

### Explicitly out of scope: Steps 2-3 (Variable nodes)

Steps 2-3 (DFG variable-node materialization) was the originally-floated target
but measurement disqualifies it: small (hugo 306ms but tokio 42ms / prism 61ms),
it is the *only* **node-creating** candidate (node-insertion order = `NodeIndex`
= cache bytes = the highest-risk class), and it has **no offloadable compute** —
it merely materializes the already-built DFG (`dfg.defs` / `dfg.uses`), so a
par-collect would only move cheap key-tuple construction off the critical path
while the serial `var_index` dedup + `add_node` remain. Bad risk/reward. Dropped.

## 2. Scope

Parallelize, behavior-**byte-identical**, the three edge-creation steps of
`CodePropertyGraph::assemble_graph` (`src/cpg/build.rs`):

- **Step 5** — Function→Function `Call`/`Return` edges.
- **Step 5b** — interprocedural arg→param `DataFlow` edges.
- **Step 8** — statement→statement `ControlFlow` edges.

Non-goals (this slice):
- Steps 1, 2-3, 4, 6, 7 (already parallel), 9 — untouched.
- **No** change to call resolution, the S3 ladder, or any edge's existence/weight.
- **No** `CACHE_VERSION` bump — the serialized bytes are unchanged.

## 3. The invariant being protected

`src/cpg/cpg_cache.rs` serializes `graph.node_indices()` and
`graph.edge_indices()` **in order**. Therefore the on-disk cache blob, and every
`NodeIndex`/`EdgeIndex`-keyed index, is a pure function of **node-insertion order
and edge-insertion order**. This slice creates no nodes, so node order is
trivially preserved; the load-bearing risk is **edge-insertion order**.

Each step today emits edges in a fully-deterministic order:

- **Step 5 / 5b** iterate `cg.calls: BTreeMap<FunctionId, BTreeSet<CallSite>>` —
  ordered by caller `FunctionId`, then by `CallSite` (BTreeSet) — and per site
  iterate `cg.resolve_call_site(site) -> Vec<ResolvedCallee>` in its returned
  (deterministic) order. Step 5 pushes `(caller→callee Call)` then
  `(callee→caller Return)` per resolved callee.
- **Step 8** iterates `files: BTreeMap<String, ParsedFile>` (ordered by path) and
  per file `cfg::build_cfg_edges(parsed) -> Vec<CfgEdge>` in its returned order.

The refactor must reproduce this exact total edge order.

## 4. Design: collect-in-order → serial apply

For each step, split the existing inline loop into a **pure collect** that returns
the pending edges as an ordered `Vec`, and a **trivial serial apply** that adds
them in that order. The collect runs in parallel over the outer ordered units;
`rayon`'s indexed `collect` is order-preserving, so flattening the per-unit
results in unit order reproduces the original total order.

```rust
type PendingEdge = (NodeIndex, NodeIndex, CpgEdge);

// Production (always compiled). Internally par_iter over the ordered units.
fn collect_step5_edges(
    cg: &CallGraph,
    func_index: &BTreeMap<(String, String, usize), NodeIndex>,
) -> Vec<PendingEdge> {
    use rayon::prelude::*;
    let ordered: Vec<(&FunctionId, &BTreeSet<CallSite>)> = cg.calls.iter().collect();
    ordered
        .par_iter()
        .map(|(caller_id, sites)| {
            let mut out = Vec::new();
            let caller_idx = match func_index.get(&(
                caller_id.file.clone(), caller_id.name.clone(), caller_id.start_line,
            )) { Some(&i) => i, None => return out };
            for site in sites.iter() {
                for resolved in cg.resolve_call_site(site) {
                    let c = resolved.target;
                    if let Some(&callee_idx) = func_index.get(&(
                        c.file.clone(), c.name.clone(), c.start_line,
                    )) {
                        out.push((caller_idx, callee_idx, CpgEdge::Call(resolved.confidence)));
                        out.push((callee_idx, caller_idx, CpgEdge::Return(resolved.confidence)));
                    }
                }
            }
            out
        })
        .collect::<Vec<Vec<PendingEdge>>>() // order-preserving over `ordered`
        .into_iter().flatten().collect()
}
```

Apply site in `assemble_graph` (replaces the inline Step-5 loop):

```rust
for (from, to, w) in Self::collect_step5_edges(&cg, &func_index) {
    graph.add_edge(from, to, w);
}
```

`resolve_call_site` is `&self` with **zero interior mutability** (audited:
`src/resolution.rs` — no `RefCell`/`Cell`/`Mutex`/`RwLock`/`unsafe`/`OnceCell`),
so concurrent calls are sound. `func_index` is built in Step 1 and is immutable
for the rest of `assemble_graph`. We parallelize the **consumption** of the
already-built resolver — *not* resolution construction (`CallGraph::build`,
Phase 3), which is a separate, deferred research question.

**Step 5b** — identical shape, par over callers, collecting `DataFlow` edges.
The only state in today's loop is the `param_cache` memo (#112). Each parallel
caller-unit keeps its **own local** param memo (`BTreeMap<(file,name,start),
Option<Vec<String>>>`) — no shared mutability. `compute_param_names` is a pure
function of `(callee_parsed, callee_id)`, so a per-caller-local memo yields
**identical param names** (hence identical edges) to the current global memo;
only the *caching scope* differs, which is a perf detail invisible to the edge
`Vec`. `var_index` is immutable during Step 5b. (The cross-caller memo sharing
that #112 added is re-evaluated in §7; the perf gate confirms the parallel build
is a net win regardless.)

**Step 8** — par over `files`; each file unit runs `cfg::build_cfg_edges(parsed)`
(read-only, file-local) and looks up `stmt_index` (immutable, built by Step 7)
to emit `ControlFlow` edges.

## 5. Proof of byte-identity (the same triple-gate as Step 7)

### 5.1 Old-order oracle — per-step collect parity (`src/cpg/tests.rs`)

For each step, freeze the **original inline loop** as a `#[cfg(test)]` reference
twin that returns the same `Vec<PendingEdge>` by direct serial iteration, and
assert the production (parallel) collect equals it:

```rust
#[test]
fn step5_parallel_edge_collect_matches_serial_reference() {
    let files = edge_fixture();            // multi-file, cross-file calls + CFG
    let cpg = CodePropertyGraph::build(&files); // source of cg + func_index
    let par = CodePropertyGraph::collect_step5_edges(&cpg.call_graph, &cpg.func_index);
    let serial = CodePropertyGraph::collect_step5_edges_reference(&cpg.call_graph, &cpg.func_index);
    assert_eq!(par, serial, "Step-5 edge sequence diverged");
    assert!(!par.is_empty(), "fixture produced no call edges");
}
```

- `collect_step5_edges_reference` / `collect_step5b_edges_reference` are the
  verbatim original loops, emitting to a `Vec` instead of calling `add_edge`.
- The oracle sources `cg` / `func_index` / `var_index` from one real
  `CodePropertyGraph::build` (test is in-crate → `pub(crate)` access). No graph
  precondition is reconstructed: the collect functions are pure reads → `Vec`.
- **Step 8** sources `stmt_index` by running `assemble_step7` on the fixture
  (deterministic); par-collect vs reference-collect use the same `stmt_index`, so
  the emitted `ControlFlow` `Vec`s are directly comparable.
- `PendingEdge` ordinals are `NodeIndex` from the same index map for both sides,
  so equality is exact (and `CpgEdge` already `#[derive(PartialEq)]`).

This is the **old-order gate**: it proves the parallel restructure reproduces the
pre-refactor edge order. `git_sha`-immune (in-memory, same binary).

### 5.2 Determinism + cache-byte gate (`tests/infra/parallel_equality_test.rs`)

The existing tests already exercise these steps end-to-end and need no change in
intent (a corpus2 already contains cross-file calls + CFG):

- `cpg_build_parallel_matches_serial_reference_in_order` — default-Rayon vs
  1-thread **full node + edge dumps** identical.
- `cache_blob_bytes_identical_serial_vs_parallel` — `cpg-cache.bin` **bytes
  identical** across thread counts.

We add a non-vacuity assert that the corpus emits Call + DataFlow + ControlFlow
edges (mirrors the Step-7 statement-node floor) so a silent corpus shrink can't
hollow out the determinism gate.

### 5.3 Tier-A backstop

`cd eval && uv run tier-a --matrix-only --allow-stale-sut` — **0 regressions**
required (CPG-construction change per AGENTS.md).

## 6. Perf gate

Cold `nav --no-cache call-stats`, branch vs `3cb0182` (current main + Step 7),
report hugo / tokio / prism wall-clock. Expected directional result: the summed
edge-step cost (hugo ~1.06s / tokio ~2.05s / prism ~2.72s) drops materially
(target ≥40% of that band across the par-eligible compute). **No regression** on
any corpus is the gate; the speedup is the reward. The param pre-pass decision
(§7) is settled by the measured tokio/prism step5b number.

## 7. Risks & mitigations

| risk | mitigation |
|---|---|
| Edge-order divergence (cache-byte break) | §5.1 old-order oracle + §5.2 cache-byte gate; order-preserving `collect` + serial apply. |
| `resolve_call_site` unsound under `par_iter` | Audited `&self`, zero interior mutability; parallelize consumption only. |
| Step-5b per-caller-local memo loses #112 cross-caller sharing | Edge parity is unaffected (param names are pure). Perf gate confirms net win; fallback = a global **immutable** param map precomputed in parallel over `cg`'s callees (no shared mutability), if measurement shows param recompute dominates. |
| Rayon nested-pool / overhead on tiny corpora | Same shared global pool as Step 7; `iter()` (1-thread pool) path is the determinism reference and stays correct. |
| Hidden ordering in `cfg::build_cfg_edges` | Read-only, returns an ordered `Vec`; unchanged — we only move *when* it runs. |

## 8. Files

- `src/cpg/build.rs` — extract `collect_step5_edges`, `collect_step5b_edges`,
  `collect_step8_edges` (production, parallel); replace the three inline loops
  with collect-then-serial-apply; add `#[cfg(test)]` reference twins.
- `src/cpg/tests.rs` — three old-order collect-parity oracles + `edge_fixture()`.
- `tests/infra/parallel_equality_test.rs` — edge non-vacuity assert.

## 9. Out-of-scope follow-ups (named, not silently dropped)

- **Resolve-once-shared** across Step 5 / Step 5b: both resolve the same sites;
  a shared per-site resolution cache would halve resolve cost (tokio step5+5b is
  ~1.8s, mostly `resolve_call_site`). Deferred — bigger refactor, changes neither
  edge set nor order if applied carefully, but its own slice.
- **`rematerialize_rust_receiver_keys` parallelization** — the next slice (Rust
  receiver post-pass: prism 2.9s / tokio 0.87s; already designed for `par_iter`
  map-then-apply).
- **Phase-3 indirect resolution** parallelization — a structural-containment
  study first (language-agnostic, hugo ~5.3s; the real research question).
