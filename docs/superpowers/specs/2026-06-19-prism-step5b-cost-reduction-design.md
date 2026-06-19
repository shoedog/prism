# Prism Step 5b Serial-Cost Reduction (Design)

**Rev 2 — 2026-06-19. Status: PLAN-READY (codex gpt-5.5 xhigh spec-review folded).**

> **Rev 2 (codex spec-review — SOUND-WITH-CONCERNS → PLAN-READY, no blockers):** folded 4 MINORs.
> §3.1 memo now caches **`Option<Vec<String>>`** — the closure subsumes the current `let Some(info) = …
> else { continue }`, so a callee whose `FunctionInfo` isn't found caches `None` and skips the site (the
> miss-behavior is explicit, not hidden in an `unwrap`). §3.2/§5 Slice-2 acceptance strengthened: a
> **serial-reference edge-order oracle** (capture the pre-refactor Step-5b edge sequence, assert the parallel
> build reproduces it byte-for-byte) — the existing `parallel_equality_test.rs` proves thread-count
> determinism but NOT equivalence to the old serial order. §1 wording softened (removing serial work *can*
> raise the ratio; the re-measure gate is the safeguard). Codex independently confirmed Slice-1 memo purity
> (no per-site leak) and that `cpg_cache.rs` serializes `edge_indices()` directly (edge order is byte-significant).

**Goal:** Close the s1-followups gate-9 cold-build ratio gap (cold-hugo user/wall **1.42 → ≥1.5**, post-S1.5)
by cutting the serial cost of `assemble_graph` Step 5b — **memoization first, parallelization only if needed.**

**One-liner:** the directive was "parallelize Step 5b," but a confirming profile shows the dominant Step-5b
cost is **redundant `all_functions()` reconstruction (10.9× on hugo)**, not raw CPU — so the first, lowest-risk
lever is a per-callee param-name memo (compute-only, no cache-byte concern), with parallelization as a gated
follow-on. Removing serial work can raise the user/wall ratio too (it cuts both user and wall, so the magnitude
depends on the `all_functions()` share — the re-measure gate decides), at a fraction of the risk of parallelizing.

---

## §1. Context & profile (measure-first)

S1.5 (PR #111, on `main`) removed the Step-5b `collect_call_args` quadratic walk (cold hugo 86→18s, 4.7×), but
cold-hugo user/wall is **1.42**, under the s1-followups item-9 ≥1.5 target. Two independent design passes (this
author + codex gpt-5.5 xhigh) plus a confirming profile (env-gated `Instant` timers, reverted) measured the
post-S1.5 cold build:

| phase (cold) | hugo | tokio |
|---|---|---|
| `build_session` total | 19.3s | 6.1s |
| `DataFlowGraph::build` | 1.14s | 0.11s |
| `CallGraph::build` | 5.60s | 1.82s |
| **`assemble_graph` (serial)** | **12.0s (62%)** | 3.9s |
| — of which **Step 5b** | **5.0s** | 0.5s |
| `call_stats` query (telemetry) | **0.05s** | 0.36s |
| **Step 5b `all_functions()`** | **35,898 calls / 3,302 distinct callees (10.9×)** | 5,744 / 1,433 (4×) |

Findings that shape this design:
1. **`call_stats` telemetry is negligible (0.05s).** The gate-9 1.42 reflects the *build*, not the query —
   a measurement concern codex raised, now refuted. The gate is valid.
2. **`assemble_graph` is the serial dominator (62% of the cold build), entirely serial (Steps 1–9).** Step 5b
   is its largest single step on hugo (5.0s, ~42%) but corpus-varying (tokio 0.5s, 13%).
3. **The dominant Step-5b cost is redundant work, not CPU pressure.** Step 5b calls
   `callee_parsed.all_functions()` (full callee-file tree-Node reconstruction) **once per resolved call site** —
   35,898 times for only 3,302 distinct callees on hugo. ~91% of those reconstructions are redundant.

## §2. Scope

**Slice 1 (this design's primary): per-callee param-name memoization** — compute-only, behavior-preserving,
**no edge-order / cache-byte impact.** Cuts the 10.9× `all_functions()` redundancy.

**Slice 2 (gated follow-on): parallelize Step 5b** (parallel-compute → ordinal-sorted serial apply) — **only if
Slice 1 + re-measure does not reach gate-9 ≥1.5.** Carries the cache-byte-parity invariant (higher risk).

**Out of scope:**
- The other serial `assemble_graph` steps (1/2-3/4/5/6/7/8/9 ≈ 7s on hugo) — the node-creation steps assign
  `NodeIndex` in order (load-bearing for cache bytes); a separate, harder effort. Noted as the next ceiling.
- `CallGraph::build` internals (Phase 3, scope-graph, Go passes) — separate.
- No behavior change: `call_argument_texts_at` output, DFG edges, resolution, and **edge insertion order** are
  byte-identical before/after (both slices).

---

## §3. Design

### §3.1 Slice 1 — per-callee param-name memo (lazy, in-loop)

The normalized param-name list Step 5b computes is a **pure function of `(callee.file, callee.name,
callee.start_line)`** plus the immutable `callee_parsed`: it does `callee_parsed.functions().find(...)`
(→ `info`), `callee_parsed.all_functions().find(...).map(function_parameter_occurrences)` with an
`info.param_names` fallback, then the Python `self`/`cls` slice gate (which depends on `info.owner` +
`callee_parsed.language` — also per-callee). Same callee ⇒ same final `Vec<String>`.

So memoize the **final** param-name vector per callee key. In `assemble_graph` Step 5b (`src/cpg/build.rs:428`):

The memo caches **`Option<Vec<String>>`**, subsuming the current `let Some(info) = … else { continue }`
(`src/cpg/build.rs:446`): `None` = the callee's `FunctionInfo` was not found ⇒ **skip this site** (no edges),
exactly as today; `Some(names)` = the final post-self/cls-slice param list.

```rust
// before the Step 5b loop:
let mut param_cache: BTreeMap<(String, String, usize), Option<Vec<String>>> = BTreeMap::new();

// inside the loop, replacing the current `let Some(info) = … else { continue }` (:446) THROUGH the
// normalized_param_names + self/cls computation (~:446–500):
let cache_key = (callee_id.file.clone(), callee_id.name.clone(), callee_id.start_line);
let cached = param_cache.entry(cache_key).or_insert_with(|| {
    // EXACTLY the current logic, returning Option<owned Vec<String>>:
    //   1. info = callee_parsed.functions().find(name == callee_id.name && start_line ==)?  // None => skip
    //   2. normalized = all_functions().find(...).map(function_parameter_occurrences)
    //                     .unwrap_or_else(|| info.param_names.clone())
    //   3. final = if normalized.first() in {"self","cls"} && info.owner.is_some() && lang == Python
    //                { normalized[1..].to_vec() } else { normalized }
    //   Some(final)
    compute_param_names(callee_parsed, callee_id)
});
let param_names: &[String] = match cached {
    Some(names) => names,
    None => continue, // callee FunctionInfo not found — same as today's `else { continue }`
};
```

`compute_param_names` is the current `:446–500` logic verbatim, returning `Option<Vec<String>>` (`None` on the
`info`-not-found path; the value is **owned** since the cache holds it). The arg→param loop borrows the cached
slice.

**Behavior preservation:** the cached value (incl. the `None` skip) is computed by the unchanged logic and is a
pure function of the key, so first-write-wins memo ≡ recompute-each-time (codex-verified: no `caller`/`site`/
`arg_texts` input leaks in). `graph.add_edge(from, to, DataFlow)` is untouched — same edges, **same insertion
order** ⇒ byte-identical cache, identical Tier-A. The only change is *skipping redundant recomputation* (incl.
re-searching for a missing callee). No `NodeIndex`, no edge-order, no `CACHE_VERSION` concern.

**Note:** the per-site `param_idx` lookup (`callee_id.start_line..=end_line` scan × `var_index`, `:529`) stays
per-site and **unchanged** (it depends on `var_index`, not the param names) — out of Slice 1's scope.

### §3.2 Slice 2 — parallelize Step 5b (gated on a re-measure)

Only if Slice 1 + re-measure is short of gate-9 ≥1.5. Parallel-compute → ordinal-sorted serial apply (the S1
C1/C2 + receiver-post-pass pattern):

1. **Enumerate** the current iteration order into jobs with explicit ordinals:
   `Vec<Job { caller_ord, site_ord, caller_id: &FunctionId, site: &CallSite }>` from
   `for (caller_id, sites) in &cg.calls { for site in sites { … } }`.
2. **Parallel map** (`rayon`, read-only over `cg`, `files`, `var_index`, `param_cache` built read-only first):
   each job → `Vec<Edge { caller_ord, site_ord, resolved_ord, param_ord, from: NodeIndex, to: NodeIndex }>`,
   dropping candidates exactly where the serial code `continue`s. `resolved_ord`/`param_ord` = enumerate
   indices over `resolve_call_site(site)` and the param list.
3. **Barrier + sort** all `Edge`s by `(caller_ord, site_ord, resolved_ord, param_ord)`.
4. **Serial apply** in that order: `graph.add_edge(from, to, CpgEdge::DataFlow)`. No dedup (duplicates, if any,
   are current behavior).

For Slice 2, `param_cache` must be precomputed read-only before the parallel map (a `BTreeMap` built in a
serial pre-pass over distinct callees, then shared `&`), since `or_insert_with` mutation isn't `par_iter`-safe.

---

## §4. Determinism, cache, Option-C

- **Slice 1:** trivially safe — it changes *nothing* the graph or cache sees (same edges, same order). Tier-A
  + cache bytes identical by construction.
- **Slice 2:** the ordinal sort makes the serial apply emit a **byte-identical edge order** regardless of
  rayon's collect order. s1-followups item 2 + codex's `cpg_cache.rs` read confirm petgraph node/edge
  insertion order IS cache-byte-significant (it serializes `edge_indices()` directly; S2 hardened identity but
  did not remove insertion-order serialization). **Acceptance (two distinct properties):** (i) *thread-count
  determinism + cache-byte parity* — extend `tests/infra/parallel_equality_test.rs`, widening its corpus to a
  Step-5b-heavy repo (s1-followups item 7); AND (ii) **equivalence to the pre-refactor SERIAL order** — the
  parity test above proves the parallel build agrees with itself across thread counts, NOT that it matches
  today's serial loop. So add a **serial-reference edge-order oracle**: capture the Step-5b `DataFlow`-edge
  sequence from the unchanged serial loop (a `#[cfg(test)]` reference, the S1.5 frozen-oracle pattern) and
  assert the parallel build reproduces it exactly. (ii) is the real cutover guard; (i) alone is insufficient.
- **Send/Sync (Slice 2):** `resolve_call_site` is `&self` read-only (`src/resolution.rs`); `ParsedFile` is
  `Send+Sync` (existing `par_iter` in `CallGraph`/`DataFlowGraph` proves it); the `CallArgsIndex` `OnceLock`
  is concurrent-first-touch-safe. The map captures only immutable `&cg`/`&files`/`&var_index`/`&param_cache`.
- **Option-C:** `cli_nav_compat` byte-identical (both slices).

---

## §5. Verification & gates

- **Slice 1:** Tier-A `--matrix-only` 0 regressions; `cargo test --lib` + the existing CPG/parity tests green;
  **re-measure** cold-hugo user/wall + absolute time (prism/tokio/hugo). Decision gate: if ≥1.5, Slice 2 is
  unnecessary (record + stop); if not, proceed to Slice 2.
- **Slice 2:** BOTH parity properties green — (i) the extended `parallel_equality_test.rs` (thread-count
  determinism + cache-byte parity) AND (ii) the **serial-reference edge-order oracle** (parallel build ==
  pre-refactor serial Step-5b edge sequence); Tier-A 0 regressions; re-measure gate-9.
- **Verification-scope override (macOS host):** full `cargo test` / `--test cli` / `--test frameworks` stall
  at `_dyld_start`; use `cargo test --lib`, `cargo test --test integration <filter>`, `cargo test --test infra
  <filter>` (the parity test), `fmt`, `clippy -p prism --lib`, `build`. The orchestrator runs Tier-A + perf.
- Report before/after numbers in the PR; perf-only ⇒ do not re-baseline Tier-A.

---

## §6. Risks

| risk | mitigation |
|---|---|
| Slice 1 memo diverges from per-site recompute | value is a pure fn of the key; closure is the verbatim current logic; a unit test asserts memoized == recomputed for a multi-callee fixture. |
| Slice 1 doesn't reach ≥1.5 | gated re-measure → Slice 2. (Likely: 10.9× redundancy removal is large + raises the ratio.) |
| Slice 2 cache-byte divergence | ordinal sort + the exact-order/cache-byte parity test (the S1 gate pattern). |
| Slice 2 rayon hazard | read-only map; `param_cache` precomputed read-only; captures no `&mut`. |
| Over-scoping into node-creation steps | explicitly out of scope (§2); they carry the `NodeIndex`-order constraint. |

---

## §7. Execution

- Branch `step5b-parallelize` (off `main` `f6499f8`, S1.5 included).
- Codex implement(high)/review(xhigh) loop, TDD. Slice 1 first → re-measure (orchestrator) → Slice 2 only if gated in.
- File map: `src/cpg/build.rs` (Step 5b: the memo in Slice 1; the compute→apply refactor in Slice 2);
  `tests/infra/parallel_equality_test.rs` (Slice 2 parity, extend). No other files.
- Plan: `docs/superpowers/plans/2026-06-19-prism-step5b-cost-reduction.md` (next step).
