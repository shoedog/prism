# CPG Substrate Analysis — Improvements to Make Before (and After) the Tier-2 Reasoning Layer

**Date:** 2026-06-10
**Method:** Dogfooded analysis — prism's own nav layer run against this repo (repo-map,
module-deps, nodes-at, callers; release binary rebuilt from `main` @ `df2d914`), a sampled
CPU profile of the cold CPG build, targeted fixture experiments, full reads of the Tier-2
specs/plans, and file:line-verified source reads of the substrate. Sub-agent survey claims
were independently re-verified where load-bearing; one was falsified (see F8 note on macros).
**Scope question:** Which CPG/PDG/CFG/EOG/DFG/AST improvements should land **before**
implementing Plan B (`taint_reaches`, currently ON HOLD pending the podman migration),
because they are cheaper now or materially de-risk the reasoning layer — plus cost/benefit/
risk for the larger refactors.

---

## 1. Executive summary

The substrate is in good shape architecturally — the `cpg/` split (PR #92), the A3/A4/A7
reasoning seams, the `Reason::Reasoning` quarantine, and the E12 type-provider trait are
clean, deliberate seams. The Tier-2 specs are unusually honest about substrate limits
(`path_proven:false`, `BoundaryExited`, params-only scope).

But **Plan B rev-4 contains two whole slices that exist only to compensate for substrate
identity gaps** (Slice 5's AST ordering oracle; Slice 3d's function-node identity), and one
substrate defect Plan B does *not* compensate for (call-resolution fan-out polluting Step-5b
arg→param edges and `BoundaryEdge` naming). The HOLD window is the cheapest moment that will
ever exist to fix the identity gaps: after Plan B ships, the workarounds are pinned by wire
shapes and regression tests and become permanent.

**Recommended before Plan B implementation resumes (≈ 2 weeks total):**

1. **S1 — Memoize `all_functions()` + parallelize the build** (perf; 27s cold → target <8s).
   This is sequenced *first* because it neutralizes the "no `CACHE_VERSION` bump" constraint
   that Plan A imposed for cold-rebuild-fragility reasons, making the identity fixes cheap.
2. **S2 — Node-identity hardening: byte ranges on `Variable`/`Statement` nodes + span-keyed
   function identity** (one batched cache bump). Deletes Plan B Slices 5 and 3d.
3. **S3 — Call-resolution precision floor: local-definition preference + receiver-call
   conservatism + resolution-confidence scoring.** Fixes the verified `slice`→29-files and
   `e.target()`→`taint.rs` false edges; makes v1 `BoundaryExited` verdicts truthful.
4. **S4 — Scope-honesty warnings** (per-function unmodeled-construct detection) so a false
   `NotReached` can never silently read as a safety proof.

**Defer to Phase-IP (interprocedural):** Rust `use`-import extraction, receiver-type call
resolution via E12 providers, A2/A5 as already contracted.
**Defer indefinitely:** full EOG, SSA, basic-block CFG restructure — byte-range identity +
(later) control-dependence edges capture most of their value at a fraction of the cost.

---

## 2. Verified findings

Each finding lists evidence. Severity is for the stated goal: accuracy of prism-as-LLM-
substrate first, speed second. Language priority: Rust > Go > Python > JS/TS > C > C++.

### F1 — No EOG / intra-line ordering: node identity is line-granular  **[critical for Tier-2]**

`CpgNode::Variable { path, file, function: String, line, access }` — no byte offset, column,
or ordinal (`src/cpg/types.rs:29-35`); dedup key is `(file, function, line, path, access)`
(`src/cpg/build.rs:198, 220-271`). `Statement` nodes are likewise `(file, line, kind)`.

Consequences:
- Same-line def/use order is unrecoverable from the graph. Plan B rev-4 documents this
  exactly (plan §Slice 5: "node rank can NEVER recover byte order") and builds an **entire
  AST-based occurrence oracle** (`order.rs`), a CFG-cycle carve-out (`line_on_cfg_cycle`),
  a `SameLineOrderView` trait, conservative-keep warnings, and a trio of round-6/round-9
  regression fixtures — all to keep a corrupt backward same-line witness out of the wire.
- Multiple occurrences of one variable on a line collapse to a single node, so a witness
  step can be ambiguous between occurrences.
- `cfg_valid` admits same-line targets unconditionally (`cpg/trace.rs:439` per plan), which
  is why the oracle has to gate *both* `RecoveredDefUse` arms.

This is the highest-leverage "before, not after" item. See §3/R1.

### F2 — Function identity is a `(file, name)` string  **[critical for Tier-2]**

`func_index: BTreeMap<(String, String), NodeIndex>`, last-writer-wins (`src/cpg/build.rs:197,
211`); `Variable.function` is a bare name string. Same-named functions in one file — Rust
`impl A { fn f }` / `impl B { fn f }`, C++ overloads, nested fns — conflate. Plan B Slice 3d
compensates with `containing_function_node` (name-filter + innermost-span + string fallback)
and has to *document a deliberate asymmetry* (`forward_reachable_in_function` stays
name-keyed). The nav layer already disambiguates with `SymbolRef.ordinal`; the CPG does not.
Note: `data_flow.rs` iterates real function AST nodes during extraction, so recording the
function's span (or node id) on each `VarLocation` is cheap at the source.

### F3 — Unqualified call resolution returns every same-named function repo-wide  **[high; verified false positives]**

`resolve_callees_qualified` falls through to "all non-static same-named definitions"
(`src/call_graph.rs:654ff`). Demonstrated on prism itself:

- `original_diff.rs:48` calls its **local** `fn slice` → module-deps reports dependencies on
  **all 29 other algorithm files** that define `pub fn slice` (verified via
  `nav module-deps --file src/algorithms/original_diff.rs`).
- `src/cpg/build.rs:466` calls petgraph's `e.target()` (external trait method) → resolved to
  the unrelated private `fn target` at `src/algorithms/taint.rs:1276`, producing a phantom
  `cpg/build.rs → algorithms/taint.rs` module edge.

Blast radius: nav `callers`/`callees`/`ego`/`module-deps`/`repo-map` precision; the
call-graph-consuming slices (membrane, echo, circular, vertical); **and Tier-2** — Step 5b
builds arg→param `DataFlow` edges per *resolved callee* (`build.rs:327-405`), so fan-out
creates false interprocedural edges today. In Plan B v1 those surface as spurious
`BoundaryExited` verdicts and mis-named `InterproceduralBoundary` warnings (A3 doesn't
traverse them, so no false `Reached`); in Phase-IP they become **false `Reached`** unless
fixed first. Plan B contains no compensation for this.

Also verified: **Rust `use` imports are not extracted at all** ("Python/JS/TS/TSX/Go extract
imports; Rust/Java/C/C++ do not" — `src/navigation/module_graph.rs:127`), so import-aware
narrowing never fires for the #1-priority language, and repo-map's
`UnresolvedModule` warning covers only fixture languages.

### F4 — No PDG: control-dependence edges don't exist  **[medium; safely additive later]**

CFG edges are statement-level reachability only; no post-dominator analysis anywhere
(confirmed across `cfg.rs`, `cpg/cfg_queries.rs`). `cfg_valid` is an explicit per-node
over-approximation — Plan A §3 frames v1 Evidence honestly as a "data-flow path." Guard
detection ("is this sink dominated by the null-check / sanitizer branch?") is approximated by
enclosing-block heuristics in algorithms. Docs already estimate dominator analysis at ~1 week
(`docs/cpg-phase6-cfg-plan.md` follow-on). The `Reason::Reasoning` quarantine (Plan B §7/M4)
means adding `guarded_by`-style annotations later is wire-safe — so this is valuable but
**not** gating.

### F5 — Cold build: 27s for ~150k LOC, single-threaded, with a verified quadratic hotspot  **[high for speed goal]**

Measured: cold `nav` query 27.7s; warm 0.4s (this repo, 319 source files). A sampled profile
attributes the dominant share to `assemble_graph → ParsedFile::all_functions` re-running the
whole-file tree-sitter Functions query. `all_functions()` is recomputed at 8 call sites
across build phases — including **inside Step 5b per resolved call site**
(`src/cpg/build.rs:349`) and 6 times across `call_graph.rs` phases (`:75,:101,:162,:189,
:567,:596`) — never memoized (`src/ast.rs`). With F3's fan-out multiplying resolved callees,
this goes superlinear. No rayon anywhere (`Cargo.toml`).

Knock-on: the 27s cold rebuild is *why* Plan A pinned "no `CACHE_VERSION` bump" (cold ~27s
vs ~30s ACP handshake fragility, Plan A §2) — i.e., a perf defect is constraining schema
evolution. Fixing F5 first converts F1/F2's cache bump from "fleet-wide risk" to "minor cost."
Also note: incremental rebuild does **not** re-run Phase-3 indirect-call resolution
(`src/cpg/build.rs:158-184`), so indirect edges go stale under the incremental path — a
separate small correctness bug.

### F6 — Rust (priority #1) specific gaps

- **`?` operator**: no CFG early-return edge, no error-channel flow — A5 holds the contract
  (Plan A §9), deferred Phase-IP. A clean `reached:false` on `?`-laden code is scope, not
  safety; S4's honesty warning should name this until A5 lands.
- **`use`-import extraction absent** (F3) — unqualified-call narrowing impossible.
- **Trait-object / `dyn` dispatch**: RustTypeProvider handles nominal `impl Trait for Type`
  (E12 Phase 5); dynamic dispatch and `Type::method`-where-stem-differs remain the documented
  nav gaps (CLAUDE.md).
- **Closures**: not extracted as function nodes; flows through closure bodies attribute to
  the enclosing fn (acceptable for taint, wrong for callers/callees granularity).
- **Macros — verified OK at the DFG level**: identifier uses inside `println!`/`format!`
  token trees *are* extracted (fixture-verified: `x` is a `Use` on both macro lines). What's
  missing is macro *semantics* (e.g., `write!(buf, …, x)` mutating `buf`), which is minor.
  An earlier survey claim that macro contents are invisible to the DFG is **false**.
- Rust has the **lowest test-matrix coverage (69%) of any priority language** (TEST_GAPS.md)
  while being both priority #1 and the dogfood language. Raise alongside any substrate work.

### F7 — Go gaps

`defer` is CFG-modeled; interface satisfaction is structural (E12). Missing: `go` statement
spawn semantics (concurrency invisible), channel send/recv data flow (taint through a channel
is a false `NotReached` — S4 should name channel ops), `select` data flow. The `v, err := f()`
idiom works (multi-target assignment fixed, PR D); `if err != nil` guard reasoning waits on F4.

### F8 — Python / JS-TS gaps

Python: comprehension bindings (deferred Gap 7), `with` CFG entry/exit, decorator semantics.
JS/TS: **no call/data edge through `await`/`.then()`** — modern async taint chains break
(false `NotReached`); optional-chaining short-circuit not in CFG (parsing/AccessPath side was
fixed in PR D); HOC/callback registration unresolved (partially mitigated by
`callback_dispatcher_slice`). Import maps exist for both; TS structural typing done (E12);
`.d.ts` ingestion still deferred.

### F9 — C / C++ gaps

The March–April C/C++ program closed the worst (sinks/sources, RAII, function-pointer Levels
0-2, static disambiguation, goto paths, CHA+RTA, tree-sitter struct fallback). Remaining:
preprocessor blindness (ERROR-node quality grades exist as mitigation), C++ exception-unwind
and destructor CFG edges, template instantiation collapse, `setjmp`/`longjmp`. These are
lower priority by the stated language order and mostly waiting on type-info-driven work.

### F10 — Where the seams are good

- `cpg/` split: `types/build/context/query/trace/cfg_queries` — reasoning reads the petgraph
  through `trace.rs` only.
- Output shaping: `shape.rs` + `shape_result`/`build_result` chokepoints; additive
  `Evidence.reasoning` proven byte-safe.
- Enum quarantine: `Reason::Reasoning(..)`/`WarningKind::Reasoning(..)` keeps nav's wire
  surface closed while reasoning grows.
- Type enrichment: `TypeProvider`/`DispatchProvider` traits, 7 providers, RTA filter.
- Cache: per-file SHA-256 + grammar fingerprint + version tag; clean invalidation story.

### F11 — Where the seams are weak

- **Identity types** (`VarLocation`, `(file,name)` function keys) are woven through
  `data_flow` → `cpg` → `navigation` → `reasoning`; every month adds consumers (this is the
  cost driver behind doing F1/F2 *now*).
- **Two call resolvers** — shared `resolve_callees_qualified` (diff-review + Step 5b) and
  nav-local `resolve_callees_nav` — already diverge on `::`-scoped fallback; precision fixes
  must land in the shared one to benefit Tier-2.
- **`taint.rs` is >10k lines** with reasoning reaching into its internals
  (`cleansed_categories_for_source`, `taint.rs:10680`) — the A4 layering inversion is real,
  tracked, and paired with A2; the `src/sanitizers/` directory already exists as the target.
- **`ParsedFile` has no cached symbol table** — every consumer re-queries tree-sitter (F5).
- **Incremental rebuild skips Phase 3** (stale indirect call edges).

---

## 3. The sequencing answer

### Do BEFORE Plan B implementation resumes (the HOLD window)

| # | Item | Cost | Why now, not after |
|---|------|------|--------------------|
| **S1** | Memoize `all_functions()` (+ `function_parameter_names`) per `ParsedFile`; rayon the per-file parse/DFG passes; precompute a per-file function-span index | 3–5 days | Pure win, no wire risk (BTreeMap merges keep determinism). Expected 27s → <8s cold based on profile dominance. **Unlocks S2**: removes the cold-rebuild fragility that made `CACHE_VERSION` bumps taboo (Plan A §2), and fixes the ~27s-vs-30s ACP handshake fragility in the a2a-bridge orchestration. |
| **S2** | **Node-identity hardening** (one batched `CACHE_VERSION` bump): add byte-range/ordinal to `Variable` + `Statement` nodes (thread spans through `VarLocation` extraction in `data_flow.rs`, which already holds the AST nodes); key function identity by span (fix `func_index` last-writer-wins; put the containing function's span/node-id on `VarLocation`) | 4–7 days | Deletes Plan B **Slice 5 entirely** (ordering oracle, `SameLineOrderView`, `line_on_cfg_cycle`, conservative-keep machinery → a trivial byte comparison) and **Slice 3d** (function-node identity + its documented asymmetry). After Plan B ships, these workarounds are pinned by the serialized witness wire shape and regression fixtures — unwinding them then means re-validating the first reasoning wire contract. Also fixes overload conflation for nav and every future tool. |
| **S3** | **Call-resolution precision floor** in the *shared* resolver: (a) if the caller's file defines the callee, resolve to it alone; (b) never resolve receiver-method calls (`x.m()` where `x` isn't an imported module) to cross-file free functions without type confirmation — prefer unresolved over wrong; (c) encode resolution confidence in the existing `score`/`why` channel (scores are uniformly 1.0 today, `module_graph.rs:18`) instead of dropping recall | 2–3 days | Kills the verified `slice`-fan-out and `e.target()` false-edge classes. Makes v1 `BoundaryExited` verdicts and `InterproceduralBoundary` warnings name *real* boundaries (Plan B has no compensation for this). Goldens will drift in the improving direction — re-bless with review; this is far cheaper before Plan B's §11 boundary-bypass regression guard pins current behavior. |
| **S4** | **Scope-honesty warnings**: per traversed function, detect unmodeled constructs (Rust `?`/closures crossing the flow, Go `go`/channel ops, JS `await`/`.then`, Python comprehensions) and emit a `WarningKind::Reasoning(..)` coverage note | 1–2 days | The reasoning layer's value to LLMs is *trustworthy* tri-state answers. A `NotReached` caused by an unmodeled construct must be distinguishable from a proven non-flow. Extends the established honesty pattern (`path_proven:false`, `BoundaryExited`); requires only a per-language node-kind blocklist. Doing it in Plan B v1 sets the truth-in-output norm for the other three tools; the warning telemetry also *prioritizes* future per-language DFG work with real dogfood data. |

Net: **≈2 weeks**, after which Plan B Slices 3d/5 are re-planned down to near-nothing (one
re-review round on a smaller plan — likely a net schedule *gain* vs implementing and
maintaining the oracle).

### Do BEFORE Phase-IP (interprocedural reasoning), not before Plan B

| Item | Cost | Rationale |
|------|------|-----------|
| Rust `use`-extraction + crate-internal module-path→file resolution (then C++ `using`) | 3–5 days | Required for trustworthy interprocedural traversal in the #1 language; also turns repo-map's `UnresolvedModule` (v1) into resolved edges. |
| Receiver-type call resolution wired through E12 `DispatchProvider`s (the documented `Type::method`/cross-file-receiver gap) | 1–3 weeks | The remaining call-precision tier; language-agnostic per CLAUDE.md. Gate Phase-IP traversal on confidence labels from S3c in the meantime. |
| A2 `compute_bindings` extraction + characterization fixture; A5 Rust `?` overlay edge | as contracted (Plan A §9) | Already correctly sequenced by the specs. |
| Fix incremental rebuild skipping Phase 3 | 1–2 days | Correctness bug; matters more as nav cache usage grows. |

### Valuable but safely AFTER (additive)

- **Control-dependence (PDG-lite) edges** (~1 week): post-dominator per function CFG, new
  edge kind. Enables guard detection, a real `conditioned_slice` upgrade, and eventually
  `path_proven:true` claims. The M4 quarantine makes the wire growth additive.
- **Per-language DFG coverage** (channels, await-chains, comprehensions): drive order from S4
  warning telemetry rather than guessing.
- **`taint.rs` decomposition / sanitizer relocation into `src/sanitizers/`** (2–4 days,
  mechanical, fixture-pinned): pairs with A2 as already planned.

### Recommend NOT doing

- **Full EOG / sub-statement CFG nodes**: high cost (node-model redesign, graph-size blowup),
  and byte-range identity (S2) captures the witness-correctness value that motivated it.
- **SSA / kill–strong-update**: rewrite-scale; revisit only if frontier-mode over-reporting
  measurably hurts agent workflows (Plan B followups note it; keep it there).
- **Basic-block CFG restructure**: no current consumer needs it; statement-level + dominators
  suffice.
- (Reinforced by `docs/prism-query-layer/research-llm-codebase-navigation.md`: the value of
  graph navigation to LLM agents is **precision** — RepoGraph/LocAgent-class gains assume
  true edges, and the research flags that wrong context actively degrades agents. S3 buys
  more agent-visible accuracy per day than any modeling expansion.)

---

## 4. Large-refactor cost/benefit/risk register

| Refactor | Cost | Benefit | What NOT doing it blocks | Risk | Mitigation |
|---|---|---|---|---|---|
| **R1. Node-identity hardening (S2)** | 4–7 d + 1 re-review round of Plan B | Exact same-line ordering (kills the only "certify-corruption" failure mode Plan B's reviews found); overload-correct witnesses; simpler Plan B; correct nav for overloads | Permanent `order.rs` oracle + asymmetry debt; every future tool (`dataflow_between`, provenance v2) inherits line-granular ambiguity | Golden-output drift; cache invalidation; touching `VarLocation` ripples through 4 layers | Land after S1 (cheap cold rebuild); keep wire additive (`skip_serializing_if`); Option-C proof matrix (`cli_nav_compat` byte-identical); characterization tests on DFG edge sets *before* refactor; single batched `CACHE_VERSION` bump |
| **R2. Call-resolution precision program (S3 → use-imports → type-confirmed dispatch)** | 2–3 d / 3–5 d / 1–3 w staged | Agent-trustworthy edges; truthful boundaries; Phase-IP soundness; repo-map that reflects real architecture | Phase-IP false `Reached`; LLM consumers learn to distrust nav output (precision failures are *visible* — both demo bugs were found in minutes of dogfooding) | Over-tightening creates false negatives | Tier the fix: drop only provably-wrong edges (local-def case); for the rest, *label* confidence via existing `score`/`Source` channels rather than delete; per-language goldens |
| **R3. Build perf (S1 + Step-5b/Phase-3 algorithmic cleanup)** | 3–5 d | 27s → target <8s cold; removes schema-evolution deterrent; MCP cold-start UX | Cache bumps stay scary; orchestration handshake fragility persists; repo-scale ceiling stays low | Parallelism nondeterminism | Deterministic merge into BTreeMaps; goldens; keep single-threaded path behind a flag for bisection |
| **R4. Control-dependence edges (PDG-lite)** | ~1 w | Guard-aware reasoning; conditioned/relevant slice upgrades; future `path_proven` strengthening | "Is the sink guarded?" stays heuristic across all 27 algorithms | Low — additive edge kind, no output change until consumed | Land edges + queries first, consume behind `detailed` verbosity later |
| **R5. Full EOG / SSA / basic blocks** | 3–6 w each | Marginal over R1+R4 | — | Node-model rewrite touches all 27 algorithms | **Don't.** Reassess only with concrete consumer evidence |
| **R6. `taint.rs` split + sanitizer relocation** | 2–4 d | Closes the A4 layering inversion debt; per-language sanitizer growth stops compounding in a 10k-line file | Sanitizer coverage work gets riskier over time | Low | `algo_taxonomy_sanitizers*` fixtures byte-pinned (already the proven A4 technique) |
| **R7. Per-file incremental indexing (E13 Phase-2 style)** | 3–4 w | O(changed) re-index for big repos | Multi-repo/HTTP roadmap scale ceiling | NodeIndex stability; merge complexity | Defer until a real >1M LOC dogfood target exists; S1 likely makes whole-repo rebuild fast enough for current scale; fix the Phase-3 staleness bug now regardless |

---

## 5. Component verdicts (the user's checklist)

- **AST parsing** — solid; tree-sitter + ERROR-node quality grades. Watch: C/C++ preprocessor
  (mitigated, not solved). Macro token-tree identifier extraction works (verified).
- **CFG** — good statement-level coverage incl. language handlers (for/else, defer, match,
  try/finally, goto, switch fall-through). Gaps: Rust `?` (A5), C++ unwind/destructors,
  async/`go` spawn edges. No basic blocks — fine.
- **DFG** — field-sensitive (AccessPath, k=5), alias-map, param-binding convention is
  load-bearing and documented (`data_flow.rs:221-227`). Core defect: line-granular identity
  (F1). No SSA/kill — acceptable, over-approximating.
- **PDG** — does not exist; R4 is the path; safely additive later.
- **EOG** — does not exist; S2 (byte ranges) is the 80/20 substitute; full EOG not worth it.
- **Call graph** — strong indirect-call story (Levels 0-2, dispatch tables, static
  disambiguation, CHA+RTA) but a weak *precision floor* for the unqualified/receiver common
  case (F3) — that floor is what agents actually see.

## 6. Per-language priority snapshot (accuracy deltas, biggest first)

| Language | Biggest accuracy lever now | Second |
|---|---|---|
| Rust | S3 + `use`-imports (call precision); A5 `?` edge at Phase-IP | Closure extraction; raise 69% test matrix |
| Go | Channel/`go` flow honesty (S4 now, modeling later) | Pointer-vs-value receiver method sets |
| Python | Comprehension bindings | `with`-CFG, decorator semantics |
| JS/TS | `await`/Promise-chain flow (S4 now, edges later) | Callback/HOC resolution |
| C | (largely done) setjmp/signal honesty | va_list taint |
| C++ | Exception/destructor CFG edges | Template instantiation identity |

---

## 7. Suggested order of operations

```
HOLD window (now):      S1 perf  →  S2 identity (one cache bump)  →  S3 precision floor  →  S4 honesty
                        └─ re-plan Plan B: Slice 5 → byte-compare; Slice 3d → deleted; 1 review round
Plan B implementation:  as planned (smaller), gated on cli_nav_compat byte-identity as before
Phase-IP prep:          Rust use-imports → type-confirmed dispatch → A2/A5 → fix incremental Phase-3
After:                  control-dependence edges → per-language DFG coverage (telemetry-driven) → R6 split
Not planned:            full EOG, SSA, basic blocks, R7 until scale demands
```
