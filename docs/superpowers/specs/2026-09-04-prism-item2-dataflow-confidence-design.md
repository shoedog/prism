# Prism roadmap item 2 — DataFlow confidence + reaching-definitions labeling, `--min-confidence`, cache v56 — design

## 1. Status

**v6.4 (2026-09-04) — Task 0b review escalation folded: the RD-relevant metric is innermost-statement mapping with distinct-statement edges, attributed before/after; nested-callable exclusion bounded by a before/after invariance control; byte control recorded as non-discriminating for CFG-universe changes (§10 Task 0b row, §11 risk 4).** v6.3 — Task 0b STOP analysed: the CFG-admissibility metric conflated three mechanisms; RD now maps endpoint lines to their enclosing statement (§4.2 step 33), Task 0b is gated on span-admissibility with nested-callable endpoints excluded (§10, §11 risk 4).** v6 — sol plan-review r1 folded (W1 finding-producer manifest, W2 Task 0b corpus thresholds, W3 cap tests, W4 base-binary custody, S1 RED shape); v5 plan v2 re-anchor defects ruled (§13); v4.1 measurement folded; v4 owner rulings folded (`~/code/tools/DECISIONS.md` A1, B2–B8, E4); scheduled.** Implementation base is the Phase 0 head `bffb847` (PR #229, branch `phase0-sarif-targets-api`): `src/cli.rs`, `src/targets/**`, `src/api/**` and `min_over(files, map, parsed)` exist there. Analysis anchors below were read at `c220525c`; the plan re-anchors them (B4).

(v3: luna second-seat folded, 8W/2S/4I/2F.)

**Recorded:** 2026-09-04 · **Exact base:** `c220525c6746d635d99a7a084791cfad4f0276d9` (`origin/main`, PR #225 merge), read via the detached review checkout `~/code/slicing-phase0-review` at spec commit `2be06ec` (all analysis code identical to base).
**Scope:** roadmap `~/code/tools/04-prism-plan-roadmap.md` §3.3 (Tier 2 scoped), §3.4 (confidence on DataFlow edges and findings), §3.6 (mode selection), §6 row 2.
**Extends:** `docs/superpowers/specs/2026-09-04-prism-phase0-sarif-targets-api-design.md` (settled v5). Phase 0 fixed the confidence vocabulary as `exact | nameonly | unlabeled`, reserved `FindingConfidence::NameOnly` for this item, named `classify_with_evidence(algorithm, quality, evidence: &EvidencePath)` as the entry point this item adds, and filed it as a §9 follow-up. The checkout's `src/finding_confidence.rs` is the v3-era implementation (`ParseQuality::min_over(files, map)`, two-arg); the settled v5 signature is `min_over(files, map, parsed)`. This design targets the v5 shape.
**Line anchors are hints against the exact base; symbols are the authority** (pipeline-lessons #6). Every claim below is cited `file:line` or `file::symbol`.
**Design granularity:** architecture, contracts, failure directions, tests and gates are binding. Names are binding where a consumer depends on them (`FlowConfidence`, `FlowDoubt`, the `--min-confidence` / `--resolution` grammar, the `dfg-stats` JSON keys, cache version 56). Everything else is the implementer's call within §9.

**[OWNER]** decisions: all answered 2026-09-04 (§12). The RD caps are set from the authorised measurement pass before Task 2 RED.

---

## 2. Problem (measured)

**2.1 `CpgEdge::DataFlow` carries no confidence, and 35 code sites assume that.**
`CpgEdge` (`src/cpg/types.rs:195-222`) declares `DataFlow` as a fieldless variant (`:198`) next to `Call(ResolutionConfidence)` / `Return(ResolutionConfidence)` (`:204-208`). `ResolutionConfidence` is `Exact | NameOnly` (`src/resolution.rs:26-30`).

Textual occurrences of `CpgEdge::DataFlow` across `src/` and `tests/`: **36**, of which 1 is a doc comment (`tests/infra/parallel_equality_test.rs:105`) — **35 code sites in 14 files**:

| Bucket | Sites | Files |
|---|---|---|
| **Production `src/`** | **14** | `src/cpg/build.rs` (4: `:578`, `:1000`, `:1325`, `:1461`), `src/cpg/query.rs` (4: `:239`, `:679`, `:745`, `:811`), `src/cpg/types.rs` (1: `:314`), `src/cpg/trace.rs` (1: `:852`), `src/algorithms/gradient_slice.rs` (1: `:110`), `src/algorithms/circular_slice.rs` (1: `:239`), `src/navigation/queries.rs` (1: `:2060`), `src/reasoning/shape.rs` (1: `:165`) |
| In-`src` test modules | 17 | `src/cpg/tests.rs` (14), `src/cpg/multiline_call_arg_tests.rs` (3) |
| `tests/` integration | 4 | `tests/infra/parallel_equality_test.rs:121`, `tests/integration/core_test.rs:873`, `tests/ast/cpg_test.rs:184`, `tests/common/mod.rs:134` |

Shape matters for the port: 10 sites are `matches!(…, CpgEdge::DataFlow)` (rewritable to `DataFlow(_)`), 6 are `add_edge`/`push` producers, **9 are `==`/`!=` value comparisons** (`cpg/tests.rs:1242,1264,1385,2614,2675`, `multiline_call_arg_tests.rs:255,289,316`, `tests/common/mod.rs:134`, `tests/ast/cpg_test.rs:184`, `tests/integration/core_test.rs:873`) which become hard compile errors once the variant takes a payload, and 2 are exhaustive `match` arms (`navigation/queries.rs:2060`, `parallel_equality_test.rs:121`). One indirect consumer, `CpgEdge::is_data_flow` (`src/cpg/types.rs:313-315`), has exactly one external caller (`tests/ast/cpg_test.rs:132`). This is the plumb-through blast radius roadmap §3.3 warns about, and it is bounded: 14 production sites, 8 files.

**2.2 The current def→use rule is shadow-aware but not order-aware.** `ParsedFile::find_path_references_scoped` (`src/ast.rs:5294-5313`) is the whole rule. For a simple path it delegates to `find_variable_references_scoped` (`:5095-5130`), which returns **every** identifier occurrence in the function whose `is_shadowed_at` check (`:5134-5158`) passes — no line comparison against `def_line` at all. For a field path it uses `collect_path_refs` (`:5315-5338`) with `line > def_line` (`:5328`). Verbatim, `src/ast.rs:5304-5309`:

> ```
> // NOTE: this filters to references AFTER def_line (`line > def_line` in `collect_path_refs`),
> // which does NOT match `find_variable_references_scoped` for simple paths — that returns ALL
> // non-shadowed references, including earlier lines (so a loop-carried `def@N → use@M (M<N)`
> // edge exists for simple paths but not field paths). `cpg/trace.rs::taint_neighbors` has a
> // field-path recovery arm that re-supplies these dropped edges for the reasoning layer; this
> // production DFG path is left as-is for byte-stability (Option C). See planA-followups Round 9.
> ```

Consequences, each constructible: a def killed by a later redefinition of the same path still reaches every downstream use (`x = a; x = b; use(x)` yields both `a`-def→use and `b`-def→use edges); a def reaches uses *textually before* it (deliberate, for loop-carried flow, but unlabeled so a consumer cannot tell a genuine back edge from a stray earlier read); shadowing is only detected when a **nested scope block** re-declares the name (`is_shadowed_at:5147-5153` requires `Language::is_scope_block(parent.kind())` and `!def_in_scope`), so Rust `let x = 1; let x = 2;` in the *same* block is invisible, and Python is invisible entirely because `Language::is_declaration_node` returns `false` for Python (`src/languages/mod.rs:240`), making `scope_has_declaration` (`src/ast.rs:5190-5209`) always `false` there. `is_scope_block` recognises exactly three node kinds — `"block" | "compound_statement" | "statement_block"` (`src/languages/mod.rs:781-788`).

**2.3 The CFG that reaching definitions needs already exists and is unused for data flow.** `cfg::build_cfg_edges` (`src/cfg.rs:29-35`) emits `CfgEdge{file, from_line, to_line}` (`:17-23`) per function via `build_function_cfg` (`:38-112`): sequential fall-through skipping terminators (`:52-67`), branch edges (`:119-137`), C/C++ `goto` (`:217`), **loop back edges** (`collect_loop_back_edges`, `:247-285` — last body statement → loop header when the body has no terminator, plus loop-header → first statement past the loop), then per-language arms for Python for/else + try (`:295`), Go defer/select (`:379`), Rust `?` + match arms (`:468`), JS/TS/Java try/catch/finally (`:530`), C/C++ switch fall-through (`:595`). Its line universe is `ParsedFile::statements_in_function` (`src/ast.rs:5778-5791`), which walks the function **body** only. The CFG is attached at Step 8 of CPG assembly (`src/cpg/build.rs:615-618`, `collect_step8_edges:1471-1497`), keyed through `stmt_index` built at Step 7 (`assemble_step7:734-780`). Nothing consults `CpgEdge::ControlFlow` when *building* DataFlow edges — Step 4 (`:549-579`) runs off `dfg.edges` alone.

**2.4 Identity is line-granular; bytes are carried but excluded.** `VarLocation::identity_key` (`src/data_flow.rs:29-41`) is `(file, function, function_start_line, line, path, kind)`; `PartialEq`/`Ord`/`Hash` all derive from it (`:48-71`) so they cannot disagree. `start_byte`/`end_byte` ride along (`:22-23`) but do not identify. Def spans are real (`PathSpan`, `src/ast.rs:160-165`, from `assignment_lvalue_spans_on_lines:3609`); **Use spans are not** — `get_use` anchors every Use at `line_start_byte` with `start_byte == end_byte`, pinned by `debug_assert_eq!` (`src/data_flow.rs:430-441`). Step 4 keys `var_index` on the same line-granular tuple (`src/cpg/build.rs:558-577`), and `CodePropertyGraph::argument_var_node_in_span`'s doc comment records the consequence: "`var_index` intentionally has one node per `(function, line, path, access)` key, so same-line same-path collisions predate this lookup and remain out of scope."

**2.5 Same-line def→use edges do not exist today.** `src/data_flow.rs:470-472` skips `ref_line == def_line` outright, and the param arm skips signature-line refs (`:451-453`). So §4.3's same-line rule is about **two Defs of one path on one line**, not about def/use pairs.

**2.6 Aliasing is function-scoped and flow-insensitive.** `build_alias_map` (`src/data_flow.rs:583-641`) folds `collect_alias_assignments` (`src/ast.rs:4263-4272`) into a `BTreeMap<String, String>` with no line dimension — last binding in line order wins, chains resolved to depth 10 / base-depth 5. `resolve_path` (`src/data_flow.rs:643-656`) rewrites `ptr.field → dev.field`. Step 4 materialises a **second** Def and a second edge set under the resolved path (`:503-527`).

**2.7 Cache and doctrine.** `CACHE_VERSION` is `55` (`src/cpg_cache.rs:161`, pinned by `:691`); the nav call-edge sidecar is `24` (`src/navigation/call_edge_cache.rs:64`, pinned by `:398`). The CPG cache persists `edges: Vec<(u32, u32, CpgEdge)>` (`src/cpg_cache.rs:206-207`, written at `:311`, read at `:527`) with **bincode 1** (`Cargo.toml:41`) — a non-self-describing format that encodes enum variants by index and structs by field order. `DataFlowGraph` is persisted whole (`src/cpg_cache.rs` `dfg` field). Doctrine: one cache transition per PR (`docs/superpowers/pipeline-lessons.md:74-78`, "Enforced twice now"); nothing below Exact feeds an asserted finding (`CLAUDE.md:174-192`); `json`/`review` bytes are pinned.

**2.8 Findings are unlabeled today.** `classify` (`src/finding_confidence.rs:118-137`) maps `needs_cpg()` (`src/slice.rs:211-231`) → `Unlabeled`, else `Exact`; `NameOnly` is never produced. Sixteen of thirty algorithms need the CPG.

---

## 3. Goals / Non-goals

**Goals.**
1. Every `CpgEdge::DataFlow` edge carries a `FlowConfidence` computed at CPG build time, from a reaching-definitions pass over the existing line-granular CFG.
2. A finding's confidence is the worst label over the edges its algorithm actually traversed (`classify_with_evidence`), so `FindingConfidence::NameOnly` becomes producible and `asserted` becomes meaningful for CPG-derived findings.
3. `--min-confidence exact|nameonly` filters at emit time for `json`, `review`, `sarif`, and the Phase 0 Task 4 `targets` consumer; default `nameonly` leaves today's output byte-identical.
4. One cache transition, 55 → 56.

**Non-goals (binding).**
1. **Changing which DataFlow edges exist in nominal mode.** This is a label-only change. No edge is added, removed, or re-endpointed by this item. Roadmap §8 names byte-pinning friction as risk #1; the mitigation *is* this constraint.
2. **SCIP / Tier 3** (roadmap §3.5, item 3): no `ExternalIndex`, no R0 rung, no overlay cache, no promotion/demotion of labels from an external index.
3. **Boundary nodes / federation** (roadmap §4, item 4).
4. **Per-byte node identity.** `VarLocation::identity_key` stays line-granular (`src/data_flow.rs:29-41`). Every endpoint group collapsed by that identity is `NameOnly(SameLine)` in v1; per-occurrence `DefId` identity is a separately authorised future item.
5. Repairing lossy anchors, `angle`/`delta` emitting nothing, the multi-run `paper` degradation, `codeFlows` in SARIF, new crate dependencies — all still Phase 0 §9 follow-ups.
6. `--resolution precise` and `--resolution auto` (§4.5).

Targets are an existing consumer only after Phase 0 Task 4 has landed. Item 2 carries the aligned evidence and runtime mode to that consumer, but does not add, size, or otherwise implement targets work; all targets acceptance claims are gated on that landing.

---

## 4. Design

### 4.1 `CpgEdge::DataFlow(FlowConfidence)` — new enum, not `ResolutionConfidence`

**Decision: a new `FlowConfidence` in `src/cpg/flow_confidence.rs`, with the doubt reason in the payload.**

```rust
/// Confidence that a DataFlow edge's definition actually reaches its use.
/// Two-valued lattice (Exact ⊐ NameOnly), same shape as `ResolutionConfidence`,
/// but a DIFFERENT producer: this is the reaching-definitions pass, not the
/// R1–R7 call ladder. Loop-carried edges are Exact — RD proved reachability
/// through a back edge; the distinction is telemetry only (§4.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FlowConfidence {
    Exact,
    NameOnly(FlowDoubt),
}

/// Why an edge could not be proven. Every variant is a reason to UNDER-assert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FlowDoubt {
    /// RD proved a redefinition of the same path kills this def before the use.
    /// `kill_line` is the lowest-numbered killing statement line on any path.
    Killed { kill_line: u32 },
    /// Two Defs of one AccessPath collapse onto one line-granular endpoint (§4.3).
    SameLine,
    /// No CFG node for the def or use line, function over the RD cap, or the
    /// function has no CFG edges at all.
    CfgIncomplete,
    /// The edge exists only through the alias map and the alias binding is
    /// re-assigned in the function, so the kill relation is undecidable.
    AliasUnstable,
    /// Step-5b arg→param edge whose resolved callee is NameOnly.
    CallNameOnly,
}

impl FlowConfidence {
    /// The lattice meet. NEVER use `std::cmp::min`: derived `Ord` puts `Exact`
    /// FIRST, so `min` returns the BEST label, not the worst. Same trap as
    /// `ParseQuality::min_over`, which uses `worst.max(..)` (`finding_confidence.rs:82`).
    pub fn worst(self, other: Self) -> Self { /* NameOnly wins; ties keep the lower kill_line */ }
    pub fn is_exact(self) -> bool;
    pub fn level(self) -> &'static str;   // "exact" | "nameonly"
}
```

**Why not reuse `ResolutionConfidence`.** (a) It is documented and produced as *call-resolution* confidence by the R1–R7 ladder (`src/resolution.rs:1-3`, `CLAUDE.md:172-190`); overloading it makes "which consult set this label" unanswerable, which is precisely the doctrine-drift failure roadmap §8 lists ("two copies of a consult drifting is a recurring finding"). (b) It cannot carry `kill_site`, which roadmap §3.3 step 3 requires. (c) Interop is one function: `From<ResolutionConfidence> for FlowConfidence` (`Exact → Exact`, `NameOnly → NameOnly(CallNameOnly)`), used at exactly one site (§4.2, Step 5b).

**Why the doubt is in the payload and not a side table.** A `BTreeMap<(NodeIndex, NodeIndex), FlowDoubt>` on `CodePropertyGraph` would be a second store that can disagree with the edge weight, and NodeIndex-keyed state is fragile across the cache round-trip. In-payload keeps one source. The settled v3 choice is `DataFlow(FlowConfidence)`; its doubt is persisted with the edge and is never skipped. Size: bincode 1 encodes the outer variant tag as `u32` plus the inner tag plus `u32` for `kill_line` — 4 bytes for `Exact`, ≤12 for the worst `NameOnly`.

**Serde / bincode consequence.** bincode is non-self-describing: adding a payload to variant index 0 of `CpgEdge` changes the encoding of **every** persisted DataFlow edge (`src/cpg_cache.rs:206-207`). There is no `#[serde(skip)]` escape — `skip` on the payload would deserialize it as `Default::default()`, silently minting a fabricated label on every cache hit, which is exactly the over-assertion the doctrine forbids. **The cache version bump is mandatory, not optional** (§4.6).

**Single-source label store.** The RD pass writes `DataFlowGraph.labels: BTreeMap<(VarLocation, VarLocation), FlowConfidence>` — a new field built alongside `forward`/`backward` (`src/data_flow.rs:557-570`). Step 4 (`src/cpg/build.rs:549-579`) *derives* each `CpgEdge::DataFlow(_)` payload from `labels`, so graph and DFG cannot disagree; §7.4 pins the equality. A line-granular key can represent a collapsed group only because every member endpoint of that group has the same `NameOnly(SameLine)` label; it must never contain `Exact` for one occurrence. `FlowEdge` (`src/data_flow.rs:85-89`) is **not** changed: it is constructed synthetically in ~10 places (`taint.rs:4930,5055,5359,5401,5473`, `cfg_queries.rs:264`, `query.rs:775`, `trace.rs:991`) that have no label to supply, and `delta_slice` (`src/algorithms/delta_slice.rs:41-68` at bffb847) diffs a `(from.file, from.line, to.file, to.line)` projection, so a new field would **not** change delta's output (corrected 2026-09-04 — v3 said "by value"; plan v2 defect 2); the decision to leave `FlowEdge` alone rests on the synthetic construction sites alone.

### 4.2 The reaching-definitions pass — `src/cpg/reaching.rs` (new, < 600 lines per `CLAUDE.md:263`)

**Signatures (owner B2).** The pseudocode in this section is binding for *semantics*, not for signatures. The Task 2 implementer proposes the exact Rust signature of the RD entry point and its private types (`DefSite`, `DefId`, `Line`, `Unavailable`) in the task report **before** writing RED tests; the controller approves or amends, and the approved shape is recorded in the plan. The `DataFlowGraph.labels` key shape and `FlowConfidence`/`FlowDoubt` remain binding (§4.1).

Per function, over the CFG lines `cfg.rs` already produces. Data structures named; ~40 lines of pseudocode.

```
INPUT   parsed: &ParsedFile, func_node
        defs:  Vec<DefSite{ id: DefId, path: AccessPath, line: usize, start_byte: usize }>
               = param occurrences (line = function start, data_flow.rs:281-291)
               ++ assignment_lvalue_spans_on_lines (ast.rs:3609)
               ++ alias-resolved twins (data_flow.rs:503-527)
        alias_map: BTreeMap<String,String>            (data_flow.rs:583-641)
        alias_stable: BTreeSet<String>                 base appears as an lvalue exactly once
OUTPUT  reach: BTreeMap<(DefId, Line), bool>, kill_at: BTreeMap<DefId, Line>

1  stmt_lines := parsed.statements_in_function(func_node)          # ast.rs:5778 — the CFG's universe
2  if defs.len() > RD_MAX_DEFS (2048) or stmt_lines.len() > RD_MAX_LINES (4096):   # caps measured 2026-09-04; RD_MAX_LINES bounds the CFG statement-line count, never the span
3      return Unavailable                                          # → every edge NameOnly(CfgIncomplete)
4  ENTRY := synthetic node at func start_line; succ(ENTRY) := {first(stmt_lines)}
5      # params are pinned to the signature line (data_flow.rs:287), which is NOT in
6      # stmt_lines because collect_statements walks the BODY (ast.rs:5785)
7  succ := adjacency from cfg::build_cfg_edges(parsed) restricted to this function's line range,
           plus the internal lexical join metadata from the CFG prerequisite in §10
8  # ---- GEN / KILL as fixed-width bit-vectors, width = defs.len() ----
9  for L in stmt_lines ∪ {ENTRY}:
10     GEN[L]  := { d.id | d.line == L }
11     KILL[L] := { d'.id | d' ∉ GEN[L] and ∃ d ∈ GEN[L] with kills(d', d) }
12 kills(d', d) :=
13     d'.path == d.path                                            # plain redefinition
14   # v1 never uses a flow-insensitive alias relation as a kill proof
15   # every alias-derived edge is retained and labeled NameOnly(AliasUnstable) at step 35
17 # ---- same-line collapse: never infer per-occurrence order in v1 ----
18 for each line-granular endpoint group (path, L) with ≥2 Defs:
19     mark every member SAME_LINE_COLLAPSED; do not kill one member with another at L
20     # VarLocation and the labels map cannot distinguish occurrences; DefId is future work.
22 # ---- worklist fixpoint over the CFG, reverse-postorder from ENTRY ----
23 IN[ENTRY] := ∅ ; worklist := [ENTRY]
24 while worklist non-empty:
25     L := pop(worklist)
26     IN[L]  := ⋃ { OUT[P] | P ∈ pred(L) }                         # back edges included ⇒ loop-carried
27     OUT'   := (IN[L] \ KILL[L]) ∪ GEN[L]
28     if OUT' ≠ OUT[L]: OUT[L] := OUT'; push succ(L) onto worklist
29 # ---- label every existing DFG edge; NEVER add or remove one (§3 non-goal 1) ----
30 for edge (def d @ Ld → use u @ Lu) in dfg.edges of this function:
30     if u is a captured read in a nested callable or deferred body: NameOnly(CfgIncomplete)
31     elif edge crosses a flagged CFG join:                         NameOnly(CfgIncomplete)
32          # try-header exception/finally, Go return→defer, or branch-arm fall-through
33     elif Unavailable or stmt_of(Lu) is None or (Ld ≠ ENTRY and stmt_of(Ld) is None): NameOnly(CfgIncomplete)   # v6.3/v6.4: stmt_of maps a DFG endpoint LINE to the start line of the INNERMOST statement whose span contains it (collect_statement_spans — after Task 0b inner statements have their own spans, so a line inside a Rust `if` body maps to its own statement, not to the wrapper's header); a def/use on a continuation line of a multi-line statement belongs to that statement. None ⇒ the line is outside every statement span of the function, or inside a nested callable body (step 30 already labels those).
34     elif d or u is in a SAME_LINE_COLLAPSED endpoint group:        NameOnly(SameLine)
35     elif edge is alias-derived:                                    NameOnly(AliasUnstable)
36     elif d.id ∈ IN[Lu]:                                             Exact
37          if Lu < Ld: telemetry dfg_label_loop_carried += 1        # proved through a back edge
38     else: NameOnly(Killed{ kill_line = min{ L | d.id ∈ KILL[L], L > Ld, L reachable from Ld } })
39          if that set is empty (killed only at its own line): kill_line = Ld
38 # ---- Step 5b (interprocedural arg→param, cpg/build.rs:990-1002) ----
39 # RD is intraprocedural. Label from the resolved callee instead:
40 #   FlowConfidence::from(resolved.confidence)  — Exact ⇒ Exact, NameOnly ⇒ NameOnly(CallNameOnly).
41 # R6MultiOwnerCandidate is already excluded upstream (build.rs:919-925); unchanged.
```

Notes that are load-bearing, not commentary:
- Step 26 needs no loop special case. `collect_loop_back_edges` (`src/cfg.rs:247-285`) already emits the back edge, so a def whose use is textually earlier lands in `IN[Lu]` iff the back edge exists — which is exactly roadmap §3.3's "loop-carried → Exact, preserving the deliberate behavior and turning it from an unexplained edge into a labeled one."
- Step 31 is the `cfg_incomplete` direction and it fires more often than it looks: continuation lines of multi-line calls are not statement lines (this is why `ContinuationScan` exists, `src/cpg/cfg_queries.rs:13-31`), and Lua / Bash / Terraform get only the generic sequential + branch + loop arms of `build_function_cfg`.
- The flagged CFG joins are mandatory safety handling, not assumptions: Python/JS/TS/Java try-header exception and finally edges originate at the try header (`src/cfg.rs:337-365,525-570`); Go synthetic return→defer edges are emitted by `build_go_edges` (`src/cfg.rs:374-402`); and the sequential loop (`src/cfg.rs:50-65`) consumes the globally sorted, line-deduplicated `statements_in_function` universe (`src/ast.rs:5778-5791`). Any DFG edge whose proof crosses one of these joins is `NameOnly(CfgIncomplete)` in v1. The negative fixtures are `dfg_reaching_cfg_try_header_negative`, `dfg_reaching_cfg_go_defer_negative`, and `dfg_reaching_cfg_branch_arm_negative` (§7.1); each asserts the offending edge exists, has a non-empty label, and is not `Exact`.
- A *capture read* is a read of an outer binding occurring inside a Python `lambda`/nested `def`/closure, Go `defer` or `go` function literal, JS/TS arrow/function, or Rust closure. All such reads are `NameOnly(CfgIncomplete)` in v1: the line CFG has no callable invocation or capture schedule. **This is a provisional v1 rule, not binding policy** (owner, DECISIONS.md B8: "'every' is strong wording … we should also be sure about Exact being downgraded to CfgIncomplete"). It is adopted only because the CFG cannot yet order a nested callable's execution, and it is expected to be refined when CFG timing work lands; a downgrade needs the same rigour as an Exact claim, so each capture fixture states *which construct* made the timing unknown, and the `dfg-stats` `CfgIncomplete` counter makes the rule's cost visible per corpus — the evidence a later refinement needs. This is timing-conservative even when a particular source happens to invoke immediately. An argument evaluated at the enclosing statement is not a deferred-body capture: `defer f(x)` evaluates `x` now and remains eligible for its normal RD label.
- Complexity: `O(|D| × |L| × iters / 64)` words. The caps at step 2 bound the worst case; exceeding them is reported, not silent (§4.7 counter).

### 4.3 Labeling rule table (binding)

| RD outcome | Label | Doubt payload | Telemetry counter |
|---|---|---|---|
| `d ∈ IN[Lu]`, `Lu > Ld` | `Exact` | — | `dfg_label_exact` |
| `d ∈ IN[Lu]`, `Lu < Ld` (proved through a back edge) | `Exact` | — | `dfg_label_exact` **and** `dfg_label_loop_carried` |
| `d ∉ IN[Lu]`, a redefinition of the same path kills it | `NameOnly` | `Killed{kill_line}` | `dfg_label_nameonly_killed` |
| ≥2 Defs in one line-granular endpoint group | `NameOnly` | `SameLine` | `dfg_label_nameonly_sameline` |
| def/use line absent from the CFG, function over cap, or no CFG edges | `NameOnly` | `CfgIncomplete` | `dfg_label_nameonly_cfg_incomplete` |
| any alias-derived edge in v1, whether or not its flow-insensitive base is re-assigned | `NameOnly` | `AliasUnstable` | `dfg_label_nameonly_alias_unstable` |
| Step-5b arg→param, resolved callee `Exact` | `Exact` | — | `dfg_label_exact` |
| Step-5b arg→param, resolved callee `NameOnly` | `NameOnly` | `CallNameOnly` | `dfg_label_nameonly_call` |

Precedence when several apply: capture/deferred-body `CfgIncomplete` > ordinary `CfgIncomplete` > `SameLine` > `AliasUnstable` > `Killed` (evaluated in the order of pseudocode lines 30-36). Rationale: a reason that says "RD could not run" must not be reported as "RD proved a kill".

### 4.4 Finding confidence = worst over the evidence path

**`EvidencePath`** (new, `src/finding_confidence.rs`) is the public API handle carried by the per-emitted-finding transport; it is not serialized independently. It retains the selected witness hops and their labels, rather than only a lossy accumulator, so a finding can be audited and an unreported bridge cannot become Exact:

```rust
#[derive(Debug, Clone, Default)]
pub struct EvidencePath {
    pub hops: Vec<EvidenceHop>,             // selected DataFlow or ResolvedCallEdge witness hops
    pub crossed_unlabeled: bool,            // an unlabelled/name-only bridge (fail to Unlabeled)
}
pub enum EvidenceHop {
    DataFlow { from: VarLocation, to: VarLocation, confidence: FlowConfidence },
    Call { edge: ResolvedCallEdge, confidence: ResolutionConfidence },
}

pub fn classify_with_evidence(algorithm: &str, quality: ParseQuality, evidence: &EvidencePath)
    -> (FindingConfidence, FindingTier);
```

Rules (each has a test in §7.2): unknown algorithm → `(Unlabeled, Candidate)` (unchanged from `classify`); `crossed_unlabeled` → `Unlabeled`; else confidence is the worst of the present axes mapped `FlowConfidence::Exact | ResolutionConfidence::Exact → Exact`, anything else → `NameOnly`; an AST-only algorithm has an empty `EvidencePath` and stays `Exact` (Phase 0 rule preserved); tier is `Asserted` iff confidence is `Exact` **and** `quality == Clean`. `classify` remains as the evidence-free entry, defined as `classify_with_evidence(a, q, &EvidencePath::unlabeled_for(a))` so the two cannot drift. `Relation` is matched exhaustively: `DataFlow` consumes its label; `AssignmentPropagation` and `RecoveredDefUse` set `crossed_unlabeled`; `CallDescent`, `ReturnInput`, and `ReturnFlow` also set `crossed_unlabeled` unless a separately proved hop is added with a regression test.

**Transport, with zero byte change.** `SliceFinding` (`src/slice.rs:23-43`) gains **no field**. `SliceResult` is public and serializable (`src/slice.rs:297-327`); v3 intentionally accepts the public source-shape change and marks it `#[non_exhaustive]`, then adds `#[serde(skip)] pub evidence: Vec<Option<EvidencePath>>`. The vector is index-aligned with `findings`: `Some(EvidencePath { hops: vec![], crossed_unlabeled: false })` is valid only for an AST-only finding, while `None` means the artifact is missing and classification must be `Unlabeled/Candidate`, never empty Exact. This skip is safe because `SliceResult` is not round-tripped through the CPG cache; the cache persists the real `CpgEdge` payload, so `CpgEdge::DataFlow(FlowConfidence)` must **not** be skipped. `ReviewRun` carries the flattened aligned `Vec<Option<EvidencePath>>`, and `ReviewOutcome::run` carries it onward. `SarifInputs` and the existing Phase 0 Task 4 targets consumer receive the same aligned pair plus runtime mode. Every projection calls one zip-preserving filter before serialization; no consumer filters findings independently.

**Per algorithm, grounded:**

- **echo** — `echo_slice::slice` traverses `callers_of_node(func_idx, 2, ConfidenceFilter::All)` (`src/algorithms/echo_slice.rs:168-173`) and `resolved_caller_edges` filtered against `R6MultiOwnerCandidate` (`:177-185`). **No DataFlow edge is ever traversed.** Record each selected `ResolvedCallEdge` and read its `ResolvedCallEdge.confidence`; `ConfidenceFilter` is only a boolean admission predicate and must not supply evidence confidence. Echo's evidence is call edges only.
- **membrane** — identical shape (`src/algorithms/membrane_slice.rs:41-54`): record each selected `ResolvedCallEdge` and its `.confidence`, never a `ConfidenceFilter` result. Call edges only.
- **provenance** — `ctx.cpg.dfg.backward_reachable(loc)` (`src/algorithms/provenance_slice.rs:589`, over `DataFlowGraph::backward_reachable`, `src/data_flow.rs:695-713`). `backward` is a `BTreeMap<VarLocation, Vec<VarLocation>>` with no edge object, so the label comes from `DataFlowGraph.labels[(prev, loc)]` per hop. Add `backward_reachable_labeled` returning the selected `EvidenceHop::DataFlow` witnesses and define `backward_reachable` as its first component — **one walk implementation**. Provenance's `flow` axis is the worst label on the selected hops from the anchor def back to the classified origin line; hops explored and abandoned do not count. Its preliminary `all_defs_of` association is a name-only, cross-function-capable selection with no DFG hop: record `crossed_unlabeled = true` whenever that branch supplies the origin. Only a verified changed-use→definition relation may start an Exact provenance artifact; an empty `all_defs_of` path is never Exact.
- **taint** — Tier 2 (`taint_forward_cfg_with_return_flow`, `src/cpg/trace.rs:968`) already records the real walk-back chain in `Trace::parents_by_root` (`:111`), so `EvidencePath::from_trace(&trace, root, sink)` walks parents and takes the worst label over hops whose `Relation` is `DataFlow` (`src/cpg/trace.rs:34,857`). Tier 1 (`taint_forward_cfg`, `src/cpg/cfg_queries.rs:197-270`) goes through `dfg_forward_reachable` (`src/cpg/query.rs:645-724`), whose BFS discards its path; add `dfg_forward_reachable_labeled` that carries a `BTreeMap<NodeIndex, FlowConfidence>` of the worst label on the discovering path, with `dfg_forward_reachable` delegating and dropping the map. The returned *set* is unchanged, so taint's output bytes are unchanged. `Relation::AssignmentPropagation` and `Relation::RecoveredDefUse` hops (`src/cpg/trace.rs:34`) are not DataFlow edges and have no RD label → they set `crossed_unlabeled`.
- **chop / leftflow / fullflow** — `chop` (`src/algorithms/chop.rs:27` → `dfg_cfg_chop`, `src/cpg/cfg_queries.rs:287` → `dfg_chop`, `src/cpg/query.rs:790-826`) returns on-path *nodes*; its `flow` axis is the worst label over DataFlow edges with both endpoints in `on_path`. `left_flow` and `full_flow` have two branches: the DFG branch (through `dfg_forward_reachable`) gets a real label, and the name-based fallback branch — `find_variable_references_scoped` called directly on the AST (`src/algorithms/left_flow.rs:104,116`; `src/algorithms/full_flow.rs:142,167`) — bypasses the DFG entirely and is unconditionally `NameOnly`.

### 4.5 `--min-confidence` and `--resolution`

```
--min-confidence exact | nameonly          (default: nameonly)
--resolution     nominal | scoped          (default: nominal)
```

**`--min-confidence`** is an **emit-time filter only** (roadmap §3.6: "One CPG build serves all rows … `--min-confidence` is a filter at emit time"). It drops findings whose `classify_with_evidence` confidence is worse than the threshold, for the finding-bearing `json`, `review`, `sarif`, and — only after Phase 0 Task 4 has landed — `targets` outputs. The filter runs before each projection, including single and multi JSON/review paths, and zips findings with their `Vec<Option<EvidencePath>>`; selection, ordering, and alignment are identical for both vectors. `nameonly` (default) admits `exact`, `nameonly` **and** `unlabeled` — an `unlabeled` finding is not *below* `nameonly`, it is ungraded, and dropping it by default would delete findings from today's output. `--min-confidence exact` admits only `exact` — the doctrine's CI gate. For `text`, `paper`, `mermaid`, and `callers`, clap rejects the flag with a clear error: those surfaces render blocks, diagrams, or caller sets without a stable finding-ownership relation, so filtering would be a silent no-op or invent an unsupported projection.

**Flag types (owner B3).** Both flags are clap `ValueEnum`s — `MinConfidence { Exact, NameOnly }` and `ResolutionMode { Nominal, Scoped }` — not validated `String`s: the user chooses from a closed set, and clap prints the possible values in `--help`. Rejection of `precise`, `auto`, or any unknown value is then the enum's own error; the roadmap-item-3 note for `precise`/`auto` lives in the flag's long help rather than in a custom error string.

**`--resolution`.** **Decision: add `nominal | scoped`, default `nominal` for every finding-bearing surface; `prism nav` and the MCP tools retain their wire shape.** `ResolutionMode` is a runtime value parsed by the CLI, stored in the API review/slice options, threaded through `run_review`/`ReviewOutcome`, and passed to `SarifInputs` and the existing Task 4 targets consumer. There is no serializer read of a fixed `RESOLUTION_MODE` constant.
- Labels are always computed at build time and always cached — one CPG serves both modes (roadmap §3.6). `--resolution` selects only what the *emitters report*. Computing RD conditionally would create two cache states for one file set, which the per-diff cache (`src/cpg_cache.rs:10-13`) cannot key.
- `scoped` ⇒ `properties.resolution_mode = "scoped"` and CPG-derived findings carry `exact` or `nameonly`. `nominal` ⇒ every CPG algorithm is forcibly projected as `unlabeled`, tier `candidate`, regardless of its evidence; AST-only algorithms retain the Phase 0 rule. Nominal is the conservative default and the byte-control mode.
- Default nominal plus `--min-confidence nameonly` admits the same finding set and preserves legacy JSON/review bytes. Targets confidence/mode assertions are not accepted until Phase 0 Task 4 has landed.
- **Nav stays as today.** `edge_kind` (`src/navigation/queries.rs:2058-2068`) keeps returning the bare string `"DataFlow"` (the arm becomes `CpgEdge::DataFlow(_)`), because `parse_ego_edges` (`:2070-2090`) parses that string back and the MCP tool schema enumerates it (`src/mcp/tools.rs:352`, and the `--edges` default `"Call,Return,DataFlow,Contains"`, `src/main.rs:313`). Changing it would break a declared enum and the nav wire shape for zero benefit; the labels reach nav operators through `dfg-stats` instead. Roadmap §3.6 wants nav at `scoped` with `nameonly` — that is satisfied: nav consumes labeled edges, it just doesn't rename them.
- **`precise` and `auto` are rejected by clap with an explicit error** naming roadmap item 3, following the Phase 0 precedent that an unknown `--format` is a clap error rather than a silent degrade (Phase 0 §2.2.3). `auto` is deferred with a written reason: with two modes and no index it would be a synonym for `scoped`, freezing a name whose meaning changes when item 3 lands.

### 4.6 Cache bump to 56, sidecar unchanged

**PartialHit label survival (owner B5 — delegated to the plan's analysis pass).** The text below says labels are recomputed after the final merged graph. The plan must carry a short analysis choosing between (a) recompute-after-merge and (b) a second label store that survives `CacheResult::PartialHit` (retained files), citing `src/cpg/build.rs:349-360` (retain / remove / merge of per-file DFGs) and `src/data_flow.rs:135-170` (lifecycle methods maintain edges/defs/uses/adjacency only), and naming the §7.5 cold / full-hit / partial-hit label-parity test as its acceptance. The sol review of the plan rules on that choice. **Plan v2's analysis (2026-09-04) chose (b), carry the labels:** `DataFlowGraph.labels` plus a per-file `rd_function_stats` map get the same file-partitioned lifecycle that `edges`/`defs`/`uses` have, because RD is a pure function of one file's parse (`build_from_refs` per file, `build_cfg_edges` over one `&ParsedFile`, `build_alias_map` function-scoped), so a retained file's labels are provably unchanged; recompute-after-merge would re-run RD over every retained file (the work `build_subset` exists to skip) and create the second consult §11 risk 2 names. Pending sol; the paragraph below states the invariant (b) must satisfy.

`CACHE_VERSION: u32 = 56` (`src/cpg_cache.rs:161`), one entry appended to the history comment: *"v56: `CpgEdge::DataFlow` carries `FlowConfidence` from the reaching-definitions pass, and `DataFlowGraph` gains the `labels` map. Label-only — the edge set is unchanged."* The pinning test at `src/cpg_cache.rs:691` moves to `56`. **Exactly one transition ships** (pipeline-lessons #10, `docs/superpowers/pipeline-lessons.md:74-78`): the fix waves of §11 amend this one entry, they do not bump again.

`DataFlowGraph.labels` has the same exact edge-membership lifecycle as `edges`, `forward`, `backward`, defs, uses, and adjacency: every build, removal, retention, and merge in the review-mode partial rebuild carries or removes the corresponding label atomically. A retained edge keeps its existing label only when its exact `(from, to)` key is retained; a recomputed edge receives the recomputed label; no merge may manufacture a default label. §7.5 and gate 10 compare cold, full-hit, and `PartialHit` artifacts after a one-file edit.

**The nav sidecar stays at 24** (`src/navigation/call_edge_cache.rs:64`, test `:398` unchanged). Justification: the sidecar persists `NavigationCallEdgeIndex` — call edges only — which this item does not touch; and its fingerprint already includes `cache_build_identity` (`:86`, from `cpg_cache::current_cache_build_identity()`), the binary's source-input hash, so **any** prism source change invalidates it automatically. Bumping it would be a second transition with no content change. Nav's CPG comes through the CPG cache (`src/navigation/cache.rs:60-61`, `CacheResult::Hit(cpg)`), so v56 already invalidates it.

### 4.7 Telemetry — `prism nav dfg-stats`

`DfgLabelStats` on `CodePropertyGraph`, exactly mirroring the `ReturnFlowStats` precedent (`src/cpg/build.rs:20-33`, field at `:136`, persisted at `src/cpg_cache.rs:213`, surfaced under `call-stats` at `src/main.rs:533-536`):

```rust
pub struct DfgLabelStats {
    pub dfg_label_exact: usize,
    pub dfg_label_loop_carried: usize,               // subset of exact
    pub dfg_label_nameonly_killed: usize,
    pub dfg_label_nameonly_sameline: usize,
    pub dfg_label_nameonly_cfg_incomplete: usize,
    pub dfg_label_nameonly_alias_unstable: usize,
    pub dfg_label_nameonly_call: usize,              // Step-5b, callee NameOnly
    pub dfg_rd_functions_over_cap: usize,
    pub dfg_rd_functions_without_cfg: usize,
}
```

Surfaced two ways. (1) `prism nav call-stats` gains an additive `"dfg_labels"` key next to `"return_flow"` — safe because `tests/cli/call_stats_test.rs` asserts structurally (`v["dropped_multi_owner"]`, `v["kinds"].is_object()`), not byte-wise. (2) A new `prism nav dfg-stats --repo <dir>` printing the same object, plus `--edges` which emits one JSON object per labeled DataFlow edge (`{from:{file,line,path,access}, to:{…}, confidence, doubt, kill_line}`, sorted by `(from, to)`) as JSONL — the exact shape of `call-stats --dump-sites` (`src/main.rs:330-340, 530-540`). `--edges` is the by-construction oracle §7 needs; without it the fixtures have nothing to read.

Lesson 16 applies (`docs/superpowers/pipeline-lessons.md:187-193`): a shipped counter that reads zero everywhere means the mechanism is inert. `dfg_label_nameonly_killed == 0` on all Tier-A corpora is a **STOP**, not a footnote.

---

## 5. Byte-stability proof plan

**Why the bytes hold.** (1) Legacy serializers do not print edge weights; new SARIF/targets fields are forced to the same nominal values in the byte-control mode. (2) The edge *set* is unchanged by construction (§3 non-goal 1); the RD pass writes labels, never `add_edge`/`remove_edge`. (3) Every traversal predicate becomes `DataFlow(_)`, which selects the identical set. (4) `--min-confidence` defaults to `nameonly`, which admits `exact`, `nameonly` and `unlabeled` — the whole set. (5) Nav's `edge_kind` string is unchanged (§4.5).

**Serde decision, made with the cache in mind:** **no `#[serde(skip)]`, no new-field-with-`default`, a real payload plus a version bump.** `skip` under bincode deserializes to `Default`, which would mint a fabricated `Exact` on every cache hit — an over-assertion the doctrine forbids, and one that no test would catch because the graph would look well-formed. A `#[serde(default)]` *new field* on `CpgEdge` is not available either: it is an enum, not a struct, and bincode has no field names to skip.

**Controls, run branch-binary vs. same-base binary in the same environment** (attribution control: a base built elsewhere is not a control):
1. `scripts/item2-byte-control.sh <base-bin> <branch-bin>`, extending `scripts/phase0-byte-control.sh`: every checked-in fixture diff enumerated by the script (`find tests/fixtures -name '*.diff' -o -name '*.patch' -o -name 'diff.json'`), plus a generated poor-parse fixture, across single algorithms `leftflow, absence, contract, echo, membrane, provenance, primitive` × formats `text, json, paper, review, mermaid`, multi sets `echo,absence,contract` and `absence,contract,primitive`, `chop,absence --format json`, `--format callers`, one `--strict-diagrams` case. **stdout + stderr + exit status** compared. Taint is excluded (documented non-byte-stability, Phase 0 §8.5). Expected: zero differing invocations. Any difference is a STOP.
2. `--format sarif` and, after Phase 0 Task 4 has landed, `prism targets` compared against the same-base binary's output (there is no committed SARIF or targets golden file; the control is base-binary output captured by `scripts/item2-byte-control.sh` — plan v2 defect 6) **with `--resolution nominal`** — proves the mode switch, not the serializer, is what changes those documents.
3. Per-binary cache-decision control: each binary with its own empty `--cache-dir`, run/run/edit/run → `(created, unchanged, changed)` for both.
4. **Same-base `prism nav --no-cache call-stats` on the Tier-A corpora**, leaf-by-leaf JSON diff, `dfg_labels` excluded from the comparison and inspected separately. Corpora from `eval/corpora.toml` committed anchors: **prism, ruff, ripgrep** (Rust); **caddy, cobra, prometheus, etcd, zap** (Go); **black, httpx, mypy** (Python). Every pre-existing key must be identical — a delta in `kind_exact` or the drop counters means DataFlow labeling leaked into call resolution, which it must not. This is the control lesson 17 (`docs/superpowers/pipeline-lessons.md:195-207`) exists for.
5. Tier-A `uv run tier-a --matrix-only --allow-stale-sut` and `--quick` (`eval/README.md:72-82`): same pass count as base, plus the new `dfg_reaching_*` cases.

---

**Behaviour commits (owner E4, 2026-09-04 — ByteStability ruling).** Byte control gates every refactor and additive commit on this branch, and v1 is label-only under it. A *designated behaviour commit* — for example flipping the default `--resolution` from `nominal` to `scoped` — may change pinned bytes when (a) Tier-A (`--matrix-only` plus the `--quick` LSP oracles) shows no recall loss and precision at or above base, (b) the affected goldens are re-blessed in that same commit with a diff review, and (c) it is a separate commit never mixed with refactor or perf work. One behaviour commit is scheduled in item 2 v1: **Task 0b** (§10), which completes the CFG statement universe (§11 risk 4) before RD is wired in; after it lands the byte-control base binary is rebuilt at its head and every later task is zero-diff against that. The default-`--resolution` flip remains a later candidate.

---

## 6. Failure directions (binding for reviewers)

Every ambiguity resolves toward `NameOnly` — under-assertion, never over-assertion.

1. **RD cannot run** (no CFG node for the def or use line, function over `RD_MAX_DEFS`/`RD_MAX_LINES`, no CFG edges in the function, degraded parse) → `NameOnly(CfgIncomplete)`. Never "assume it reaches".
2. **Same-line endpoint collapse** (two Defs share one line-granular endpoint group) → every edge touching that group is `NameOnly(SameLine)`, regardless of byte spans. Per-occurrence identity is future work.
3. **Alias-derived relation** (including a flow-insensitive single-assignment alias) → the pair is *not* treated as a kill proof in v1, and the retained edge is `NameOnly(AliasUnstable)`. Both halves matter: not killing keeps the edge, labeling it NameOnly keeps it out of asserted findings.
4. **Kill on some path, live on another** → standard RD union at line 26 keeps it `Exact`. This is correct, not a leniency: a def that reaches along any path does reach.
5. **Step-5b interprocedural** → `Exact` only when the resolved callee is `Exact`; anything else `NameOnly(CallNameOnly)`. `R6MultiOwnerCandidate` remains excluded upstream (`src/cpg/build.rs:919-925`).
6. **Traversal cannot report its edges** (`Relation::AssignmentPropagation`, `Relation::RecoveredDefUse`, the name-based `left_flow`/`full_flow` fallback) → `EvidencePath.crossed_unlabeled = true` → finding `Unlabeled`, tier `Candidate`.
7. **`--resolution nominal`** → every CPG algorithm is forcibly projected as `Unlabeled/Candidate`, even when RD evidence is Exact. The escape hatch always under-claims.
8. **CFG safety joins** → try-header exception/finally, Go synthetic return→defer, and sequential edges crossing lexical branch arms are always `NameOnly(CfgIncomplete)`; the three negative fixtures in §7.1 are mandatory.
9. **Trace relations without a proved label** → `AssignmentPropagation`, `RecoveredDefUse`, `CallDescent`, `ReturnInput`, and `ReturnFlow` set `crossed_unlabeled`; only a separately proved relation may avoid that result.
10. **A `DataFlowGraph.labels` entry missing for an existing edge** is a bug, not a default: the derivation at Step 4 uses `NameOnly(CfgIncomplete)` and increments `dfg_rd_functions_without_cfg`, so absence is visible in telemetry rather than silently `Exact`.

---

## 7. Tests

### 7.1 By-construction fixtures — `eval/fixtures/<lang>/dfg_reaching_*/`

New matrix probe `"dfg"` (`eval/tier_a/matrix.py`): add to `PROBE_TYPES` (`:27`), to `KNOWN_TOP_SECTIONS`/`EXPECT_KEYS_BY_PROBE` (`:84-90`), and — critically — an explicit `elif case.probe == "dfg"` branch in `_run_matrix_inner` (`:455-463`), whose current `else:` would otherwise run a `dfg` case as `module_deps` and pass vacuously. The probe runs `prism nav dfg-stats --repo <fixture> --edges` (§4.7) and asserts a subset:

```toml
[case]
language = "python"; capability = "dfg_reaching_killed_def"; status = "pass"; probe = "dfg"
[[expect.edges]]
from = "a.py:2:x"; to = "a.py:4:x"; confidence = "nameonly"; doubt = "killed"; kill_line = 3
```

One directory per row; each names the exact asserted label.

| Fixture | Languages | Shape | Asserted label |
|---|---|---|---|
| `dfg_reaching_killed_def` | py, go, rs, js, ts | `x = A; x = B; use(x)` | `A→use` = `nameonly/killed{kill_line = B's line}`; `B→use` = `exact` |
| `dfg_reaching_loop_carried` | py, go, rs, js, ts | `for …: use(x); x = f()` | `x-def → earlier use` = `exact`; `dfg_label_loop_carried ≥ 1` |
| `dfg_reaching_shadowed_inner` | go, rs, js, ts (C-family `is_scope_block`) | outer `x`; inner block re-declares `x`; use inside the block | inner use has **no** edge from the outer def (pre-existing `is_shadowed_at` behaviour, `src/ast.rs:5147-5153`); the outer-def→outer-use edge = `exact`. Pins that RD did not change the edge set. |
| `dfg_reaching_alias_conservative` | py, go, rs | `p = q; p.x = 1; use(q.x)` — `p` assigned once | retained alias edge = `nameonly/alias_unstable`; v1 never treats a flow-insensitive alias as an Exact kill |
| `dfg_reaching_alias_unstable` | py, go | as above but `p` re-assigned later | `nameonly/alias_unstable` |
| `dfg_reaching_same_line` | py, js, ts | `a = 1; a = 2;` on one physical line, then `use(a)` on a later line | assert a non-empty `SameLine` label set; every collapsed endpoint is `nameonly/sameline`, with no `exact` edge |
| `dfg_reaching_nonlocal_global` | py | module `x`; inner `def` with `global x` then `x = 1`; outer use after the call | assert the pre-existing absence of a module/call-boundary DFG edge; no DFG label is minted, and any finding projection through the unproved binding is `unlabeled/candidate` |
| `dfg_reaching_capture_timing` | py, go, js, ts, rs | `x=source;` captured read in Python lambda/nested def, Go `defer`/`go` literal, JS/TS arrow/function, Rust closure; then a rebind before delayed execution | every existing outer-def→capture-read edge is `nameonly/cfg_incomplete`; late-binding negatives forbid `exact` |
| `dfg_reaching_defer_argument_now` | go | `x=source; defer f(x); x=clean` | `x→defer-argument` is eligible for and asserted `exact`: the argument evaluates now, so it is not a deferred-body capture |
| `dfg_reaching_go_short_var_if` | go | `if v := f(); v != nil { use(v) }` then a later `v` at function scope | the `if`-scoped `v`-def → in-block use = `exact`; the outer `v`-def → in-block use = `nameonly` |
| `dfg_reaching_js_var_hoisting` | js | `use(v)` before `var v = 1` in the same function | `nameonly` — hoisting makes the pre-def use a real read of `undefined`, and RD over the CFG cannot prove otherwise |
| `dfg_reaching_rust_let_shadow` | rs | `let x = 1; let x = 2; use(x);` in one block | first def → use = `nameonly/killed`; second = `exact`. This is the case §2.2 shows `is_shadowed_at` misses today. |
| `dfg_reaching_cfg_gap` | py, ts | def inside a multi-line call argument list (continuation line) | `nameonly/cfg_incomplete` |
| `dfg_reaching_interproc_exact` / `_nameonly` | py, go | Step-5b arg→param with a singleton-Exact callee / a NameOnly callee | `exact` / `nameonly/call_nameonly` |

The three CFG-safety negatives are mandatory and concrete: `dfg_reaching_cfg_try_header_negative` uses `x=source; try: x=clean; raise; except: sink(x)` (Python and the equivalent JS/TS/Java shape) and asserts the existing try-header→handler/finally join is non-empty and `nameonly/cfg_incomplete`; `dfg_reaching_cfg_go_defer_negative` uses `x=source; defer f(x); x=clean; return` and asserts the synthetic return→defer reach is non-empty and `nameonly/cfg_incomplete`; `dfg_reaching_cfg_branch_arm_negative` uses `x=source; if c: x=clean; else: sink(x)` and asserts any cross-arm sequential edge is non-empty and `nameonly/cfg_incomplete`. The last fixture is enabled only after the CFG branch-arm prerequisite in §10 supplies lexical-arm metadata.

Every fixture that delivers an Exact edge has a paired NameOnly delivery case. This pairing is explicit for loop-carried flow, outer-shadow flow, and immediate-defer arguments: the negative variants must contain a non-empty emitted edge/evidence artifact and assert `NameOnly`, not merely assert that Exact is absent. For outer-shadow, `dfg_reaching_shadowed_inner_negative` asserts a non-empty NameOnly finding with `crossed_unlabeled = true` for the inner shadow boundary and separately asserts the pre-existing outer-def→inner-use edge is absent. The matrix includes fixture cases for every `Language::all()` value: Python, JavaScript, TypeScript, Tsx, Go, Java, C, Cpp, Rust, Lua, Terraform, and Bash; language coverage is not inferred from corpus anchors.

### 7.2 Unit tests

- `src/cpg/reaching.rs` in-module: GEN/KILL construction; fixpoint over a diamond (def killed on one branch → still `Exact`); fixpoint over a loop (back edge → `Exact` for a textually-earlier use); `kill_line` is the lowest reachable killing line; over-cap returns `Unavailable` with a **cap-specific reason** and increments `dfg_rd_functions_over_cap` — three decoupled tests (sol r1 W3): the line-cap fixture has > 4096 statement lines and **zero Defs** and asserts the lines reason; the def-cap fixture has > 2048 Defs within ≤ 4096 statement lines and asserts the defs reason; the long-span fixture (span > 4096 lines, few statements, few Defs) asserts *not* `Unavailable`; a function with zero CFG edges returns `Unavailable`; every member of a same-line collapsed group is `SameLine` even with distinct bytes; capture/deferred-body detection covers every language listed in §4.2; all three CFG-safety join shapes return `CfgIncomplete`; alias-derived edges remain `AliasUnstable`; `defer f(x)` remains an immediate argument read and is paired with its synthetic-edge negative.
- `src/cpg/flow_confidence.rs`: `worst()` is commutative, associative, `NameOnly` absorbing, and **`worst(Exact, NameOnly(_)) != std::cmp::min(...)`** — a test that pins the derived-`Ord` trap explicitly.
- `src/finding_confidence.rs`: `classify_with_evidence` for each row of §4.4; a missing artifact and an `all_defs_of`-selected provenance origin are `crossed_unlabeled`, never empty Exact; exhaustive tests cover `CallDescent`, `ReturnInput`, and `ReturnFlow`; `classify(a, q) == classify_with_evidence(a, q, &EvidencePath::unlabeled_for(a))` for every production algorithm string; serde round-trip `NameOnly → "nameonly"`.
- `tests/ast/dfg_test.rs` (existing file, `:1-490`): `labels` has exactly one entry per `dfg.edges` entry; a label lookup for a synthetic `FlowEdge` returns `None` rather than a default.

### 7.3 Plumb-through parity test (the one that guards §3 non-goal 1)

`tests/integration/dfg_label_parity_test.rs`: for a multi-language corpus (`src/navigation`, as `tests/infra/parallel_equality_test.rs:111` already does), build the CPG twice — once with the RD pass enabled, once with a test-only constructor that forces every Step-4 label to `NameOnly(CfgIncomplete)` — and assert that **every** production consumer selects the identical edge set in both: `strongly_connected_components` (`query.rs:239`), `dfg_forward_reachable` (`:679`), `dfg_backward_reachable` (`:745`), `dfg_chop` (`:811`), `taint_neighbors` (`trace.rs:852`), `has_incoming_dataflow` (`shape.rs:165`), `gradient_slice` (`:110`), `circular_slice` (`:239`), `is_data_flow` (`types.rs:314`), the Step-5b interprocedural floor (`parallel_equality_test.rs:109-140`), and indirect review, SARIF, mermaid, navigation, and target projections. Labels differ; sets do not.

### 7.4 Single-source parity

`tests/integration/dfg_label_parity_test.rs` also asserts: for every `CpgEdge::DataFlow(c)` in the graph whose endpoints map back to `VarLocation`s, `c == dfg.labels[(from, to)]` when the key exists, and `c` is a Step-5b label otherwise. Parity is over unique `(from, to)` edges; duplicate observations collapse via `.or_insert`, never by silently choosing a byte occurrence. Roadmap §8's "two copies of a consult" risk, closed by a test rather than a convention.

**Insert rule for `labels` (ruling 2026-09-04, plan v2 defect 5).** A duplicate `(from, to)` key — two line-granular-identical observations — stores `worst()` of the two labels. This agrees with §4.3, which makes every member of a collapsed group `NameOnly(SameLine)`; the parity test builds its own map with `.or_insert` over the graph and must still agree, which is exactly the check.

### 7.5 Cache lifecycle and CLI

`tests/integration/dfg_label_parity_test.rs` performs cold build, full cache hit, then a one-file edit causing review-mode `PartialHit`; it compares the full labeled DFG membership and every `CpgEdge::DataFlow` payload for equality with a cold rebuild of the edited tree. It exercises retained, removed, and recomputed edges, and fails on a missing or defaulted label.

`tests/cli/min_confidence_test.rs`: default nominal run byte-identical to base for every JSON/review projection on a fixture with a known `nameonly` echo finding; `--min-confidence exact` drops it while retaining aligned evidence for retained findings; `--min-confidence exact --format sarif` emits a document whose every `properties.confidence` is `"exact"`; `--resolution scoped` emits runtime `resolution_mode = "scoped"`; `--resolution nominal --format sarif` reproduces the base binary's SARIF byte-for-byte (same-base control, not a committed golden); `--resolution precise` and `--min-confidence bogus` are clap errors; `--min-confidence exact` with each of `text`, `paper`, `mermaid`, and `callers` is rejected and names that the format has no stable finding projection. Targets assertions run only after Phase 0 Task 4 has landed.

### 7.6 Per-path end-to-end confidence

For every Item 2 emitted-finding producer — the **four** CPG algorithms that construct `SliceFinding`s at bffb847: `provenance`, `taint`, `membrane`, and `echo` (Phase 0 finding inventory `~/code/tools/grounding/finding-inventory.md`, re-verified 2026-09-04 by grep; `leftflow`, `fullflow`, `relevant`, `conditioned`, `barrier`, `chop`, `delta`, `spiral`, `circular`, `vertical`, `3d`, and `gradient` construct **no** findings, and minting one would change nominal JSON/review bytes — a separate behaviour commit if ever wanted; corrected in v6 after sol r1 W1: v3–v5 listed sixteen) — add one fixture with a complete Exact witness and one with a complete NameOnly witness. Each case must emit a non-empty finding and a non-empty evidence artifact (or an explicit `crossed_unlabeled` artifact); inspect it to prove the selected DataFlow, `ResolvedCallEdge.confidence`, or conservative boundary caused the result. Assert `exact/asserted` versus `nameonly/candidate`, apply the same pre-projection filter to findings and evidence, and repeat SARIF plus targets only after Phase 0 Task 4 has landed. `--min-confidence exact` retains only the Exact case. Provenance has three cases, not two (corrected 2026-09-04 per owner B7 — the earlier wording called the bridge case `NameOnly`): its **Unlabeled** case uses the `all_defs_of` bridge and asserts `crossed_unlabeled` ⇒ `unlabeled/candidate` (§4.4: a crossed bridge is never `nameonly`); its **NameOnly** case starts from the verified changed-use→definition relation and walks at least one `NameOnly` DataFlow hop (a killed or same-line definition), asserting `nameonly/candidate`; its **Exact** case uses the same relation over Exact hops only. These are end-to-end algorithm tests, not counter or source-text checks. **Traversal-only CPG algorithms** (`chop`, `leftflow`, `fullflow`, `delta`, and the rest) are covered by labeled-walk unit tests on their reachability twins (`dfg_forward_reachable_labeled`, `backward_reachable_labeled`, chop's on-path label fold) plus §7.3 parity; no finding is minted for them.

---

## 8. Gates (controller, before review and before PR)

1. `cargo fmt --all -- --check` → exit 0.
2. `cargo clippy --all-targets --all-features -- -D warnings` → exit 0, or the pre-existing warning set diffed against the exact base **built in the same worktree**; new warnings only are blockers.
3. Focused tests (§7) GREEN; RED observed on the exact base by running `~/code/tools/bin/prism-base-c220525` with the same CLI invocations. Unit tests that cannot compile on base are recorded as "feature absent", not as RED.
4. Full suite: `cargo test --all-targets --all-features --no-fail-fast 2>&1 | tee <log>`; totals by `awk` over **every** `test result:` line of the complete log, never `tail`. Base is the item 2 branch point bffb847 (the Phase 0 head): Task 0 measures its totals in the same environment and records them (the last controller-run figure on that code is 3810 / 0 / 1 across 29 binaries + 2 doc-tests at 602a6ed, `~/code/tools/logs/closeout/full-suite-final.log`; the pre-Phase-0 `3543 / 0 / 1` at c220525c is history). The byte-control base binary is built at bffb847 as `~/code/tools/bin/prism-base-bffb847` in Task 0 **from the detached worktree `~/code/slicing-phase0-review` (checked out at bffb847)** — never by checking out paths inside the item 2 worktree, which would delete the census example added in 80d9b5e (sol r1 W4) — and rebuilt the same way at the Task 0b head after the behaviour commit. Expected: base totals + new tests, 0 failed. A failure outside this item's scope is reported, not re-baselined.
5. Byte controls §5.1–§5.3: zero differing invocations. Any difference is a STOP.
6. Same-base `prism nav --no-cache call-stats` on all eleven anchor corpora (§5.4): every pre-existing key identical. A delta that removes Exact **call** edges is a STOP (lesson 17).
7. Tier-A `uv run tier-a --matrix-only --allow-stale-sut` → base pass count + the new `dfg_reaching_*` cases, all `ok`; `uv run tier-a --quick` → no regression. Do not rebaseline.
8. **Cache-version single transition:** `git log -p --  src/cpg_cache.rs | grep CACHE_VERSION` over the branch shows exactly one `55 → 56` edit and no intermediate value; `NAV_CALL_EDGE_CACHE_VERSION` untouched at 24.
9. **Non-inert language check:** every §7.1 fixture language (py, go, rs, js, ts) has a non-empty Exact case and a non-empty NameOnly case in its fixture pack, not merely a corpus anchor; where the killed shape parses, `dfg_label_nameonly_killed > 0`. Both poles must fire per applicable fixture pack (lesson 16). Every other language is recorded in the Task 6 report with its census DataFlow-edge count (`examples/dfg_census.rs`, REPORT.md §H): Terraform has 0 DataFlow edges at bffb847 and is `gate 9 N/A — no DataFlow`; c, cpp, bash, tsx carry edges and are covered by the §7.3 parity test over their existing fixture packs, with Exact/NameOnly poles filed as a follow-up; Java and Lua are measured in Task 0. A pole is never fabricated. (Ruling 2026-09-04, plan v2 defect 1.)
10. **Cache label-parity:** §7.5 is GREEN: cold, full-hit, and `PartialHit` after the same one-file edit have identical label membership and DataFlow payloads to the cold edited-tree control. Any missing, stale, or defaulted label is a STOP.
11. **Finding-path delivery:** §7.6's producer manifest — the four CPG finding producers `provenance`, `taint`, `membrane`, `echo` — has non-empty Exact and NameOnly delivery cases, with aligned evidence and pre-projection filtering; traversal-only algorithms have labeled-walk unit tests and §7.3 parity (v6). SARIF is required; targets confidence/tier assertions are required only after the literal prerequisite “Phase 0 Task 4 landed.” Counters alone do not satisfy this gate.

---

## 9. Permitted / forbidden files

**New:** `src/cpg/flow_confidence.rs`, `src/cpg/reaching.rs`, `tests/integration/dfg_label_parity_test.rs`, `tests/cli/min_confidence_test.rs`, `eval/fixtures/<lang>/dfg_reaching_*/**`, `scripts/item2-byte-control.sh`, `docs/superpowers/plans/2026-09-XX-prism-item2-*.md`, `docs/superpowers/handoffs/2026-09-XX-prism-item2-*.md`.

**Modified:** `src/cfg.rs` and the narrow lexical-arm metadata helper in `src/ast.rs` (Task 0 only), `src/cpg/types.rs` (the `DataFlow` variant + `is_data_flow`), `src/cpg/build.rs` (Step 4 derivation, Step 5b label, `DfgLabelStats` field), `src/cpg/query.rs`, `src/cpg/trace.rs`, `src/cpg/cfg_queries.rs`, `src/cpg.rs` (re-exports), `src/data_flow.rs` (`labels` field, labeled reachability twins), `src/cpg_cache.rs` (version 56 + `labels`/`DfgLabelStats` persistence), `src/finding_confidence.rs` (`EvidencePath`, `classify_with_evidence`), `src/algorithms/{provenance_slice,taint,chop,left_flow,full_flow,echo_slice,membrane_slice,gradient_slice,circular_slice}.rs` (evidence recording only), `src/reasoning/shape.rs`, `src/navigation/queries.rs` (`edge_kind` arm + `dfg_labels` key), `src/cli.rs` and `src/main.rs` (`--min-confidence`, `--resolution`, `nav dfg-stats`), `src/output/{sarif,review,review_compact}.rs`, `src/api/run.rs`, `src/slice.rs` (`SliceResult.evidence`), `src/cpg/tests.rs`, `src/cpg/multiline_call_arg_tests.rs`, `tests/{common/mod.rs,ast/cpg_test.rs,ast/dfg_test.rs,integration/core_test.rs,infra/parallel_equality_test.rs}`, `eval/tier_a/matrix.py`, `CLAUDE.md`, and `README.md`. The Phase 0 Task 4 targets consumer is an existing interface, not an Item 2 file or work package.

**Forbidden:** edits to `src/ast.rs`'s `find_path_references_scoped`, `is_shadowed_at`, or `scope_has_declaration` (permitted AST changes: the narrow lexical-arm metadata helper required by Task 0, and — **Task 0b only** — `collect_statements` / `collect_statement_spans` / `collect_nested_statements` statement collection together with their Task 0 arm mirrors `collect_statement_arms` / `collect_nested_statement_arms`, which must change child-for-child), `src/languages/**` (**except, Task 0b only,** the per-language statement / wrapper-kind tables that `is_statement_node` / `is_control_flow_node` read — plan v2 defect 7), `src/resolution*.rs`, `src/call_graph.rs`, `src/name_resolution/**`, `src/navigation/call_edge_cache.rs` (the sidecar version is not touched), `src/mcp/**`, `Cargo.toml` dependencies (no new deps), `eval/corpora.toml`, `docs/eval/tier-a/*.json` (baselines are not rewritten). If compiled reality requires any other forbidden change, stop and amend this design.

---

## 10. Sequencing (PR-sized tasks within one branch; one PR at the end)

**Decision: label enum + plumb-through first, RD pass second — not RD behind a feature flag.** Rationale: the 35-site port is the mechanical, review-heavy half, and doing it first with a *constant* label makes the §7.3 parity test provable **before** any semantics change, cleanly separating "did the refactor change edge selection?" from "did RD change labels?". A feature flag was rejected: it creates a second code path that the byte control must cover twice, and it cannot avoid the enum change anyway.

| # | Task | Acceptance test |
|---|---|---|
| 0 | **CFG branch-arm boundaries prerequisite:** add internal lexical-arm provenance at the `src/cfg.rs:50-65` sequential-edge seam and the `src/ast.rs:5778-5791` line-granular statement universe; do not change line endpoints, CPG edge membership, or serialized bytes | `src/cfg.rs` unit tests cover a diamond and assert arm IDs; §5.1 byte control and §7.3 edge-set parity remain zero-diff; Task 0 also measures the bffb847 full-suite totals, builds `bin/prism-base-bffb847` from the detached worktree, and runs the census for Java and Lua (gate 9 disposition); the `dfg_reaching_cfg_branch_arm_negative` fixture is Task 3 staged RED / Task 6 GREEN (owner B6) |
| 0b | **CFG statement-universe completeness (designated behaviour commit, §5):** per language, a RED fixture proving a nested statement is missing from `statements_in_function`, then the minimal `collect_statements` / `collect_statement_spans` change (unwrap the language's wrapper kinds, e.g. Rust `expression_statement` around block-tailed control flow); isolated commit, no RD or refactor work mixed in; permitted files for this task only: `src/ast.rs` statement collection and the per-language statement/wrapper tables | `examples/dfg_census.rs` (extended in Task 0b with `n_dfg_edges_span_ok`: an endpoint is admissible when it lies inside ANY statement span of its function — `collect_statement_spans`, start line through end line — and endpoints inside nested callable bodies are excluded from the denominator and counted separately) re-run over the **same Tier-A anchor corpora** shows per-language **corpus-level span-admissible** DataFlow edges ≥ 90 % for every §7.1 language (the raw start-line metric is still reported, for information; v6.3 after the Task 0b STOP). **v6.4 — the metric Task 3's RD actually depends on (review escalation): map each endpoint line to its INNERMOST containing statement span; an edge is RD-usable when both endpoints map (def may be ENTRY) AND the two mapped statements are DISTINCT (an edge whose def and use collapse into one statement can only ever be `NameOnly(SameLine)`). Report this `n_dfg_edges_distinct_stmt` share per language at the base (9fbdd92) and at the Task 0b head — the before/after is what attributes the improvement to Task 0b, since `collect_statement_spans` already covered wrapper bodies before the fix. Control for the exclusion: the nested-callable edge count must be identical before and after (that mechanism is independent of the wrapper fix; a change means the exclusion absorbed wrapper-mechanism edges). Pass rule as above on `distinct_stmt` share** (was Rust 32.9 / TSX 22.5 / TS 25.5 / Go 47.9 / Python 77.1 %; the fixture packs already score 90.5–100 % and prove nothing — sol r1 W2); a language landing in [75 %, 90 %) passes only if the Task 0b report attributes ≥ 95 % of its remaining non-admissible edges to named mechanisms, each with a probe like REPORT.md §G, and the controller's ruling is recorded — **only a recorded `ACCEPT` passes; a `SECOND PASS` ruling means the wrapper fix is extended, the census re-run, and this rule re-applied before the task proceeds** (sol r2 S1); below 75 % is a STOP; every percentage is reported beside its function and edge counts, and a language whose anchor-corpus population is under 1,000 DataFlow edges (JavaScript: 36 functions / 301 edges from vendored files) is judged with that denominator in view (plan v3 note); Tier-A `--matrix-only` + `--quick` no recall loss; affected goldens re-blessed in the same commit with a diff review; the byte-control base binary is re-based to this head for every later task; still one cache transition (56) |
| 1 | `flow_confidence.rs`; `CpgEdge::DataFlow(FlowConfidence)`; all 35 sites ported to `DataFlow(_)`; Step 4 emits a constant `NameOnly(CfgIncomplete)`; Step 5b emits `From<ResolutionConfidence>`; `CACHE_VERSION = 56` | §7.3 plumb-through parity GREEN; full suite at base totals; §5.1 byte control zero diffs |
| 2 | `src/cpg/reaching.rs` — the RD pass, not yet wired into the build | §7.2 RD unit tests (diamond, loop, kill_line, caps, no-CFG) |
| 3 | Wire RD into Step 4 via `DataFlowGraph.labels`; conservative same-line, capture-timing, CFG-join, and alias rules; loop-carried counting | §7.1 `dfg_reaching_*` fixtures and expected labels committed as **staged RED** (the `nav dfg-stats --edges` oracle arrives in Task 6 — owner B6); §7.2 label unit tests GREEN; §7.4 single-source parity; §5.1 byte control still zero diffs |
| 4 | Per-finding `EvidencePath` artifacts + `classify_with_evidence`; labeled reachability twins; evidence recording in provenance / taint / chop / leftflow / fullflow / echo / membrane | §7.2 and §7.6, including Exact and NameOnly SARIF assertions; targets assertions only after Phase 0 Task 4 landed; taint output bytes unchanged |
| 5 | `--min-confidence` and `--resolution` on `slice`/`sarif`; emit-time filter and pass-through to the existing targets consumer | §7.5; §5.2 nominal-mode SARIF reproduces the base binary's output; targets acceptance is gated on Phase 0 Task 4 landed |
| 6 | `DfgLabelStats`; `nav dfg-stats` + `--edges`; `call-stats` additive key; `CLAUDE.md`/`README` truth pass | §7.1 matrix **GREEN** (owns the Task 3 staged RED — owner B6); §8.9 non-inert fixture check per language; `tests/cli/call_stats_test.rs` still GREEN |
| 7 | Closeout: full §8 gates, corpus controls, handoff, one PR | All of §8 |

**RED shape (sol r1 S1).** For every new semantic contract the RED record has two parts: the feature-absence failure on the exact predecessor (a compile error is recorded but is not the RED), then a conservative stub that compiles and fails the assertion — proving the test discriminates the semantics, not the symbol's existence. Task 7 (closeout) is exempt.

Review cap: **2 rounds**, declared before dispatch. At the cap, classify before acting — converging (fewer, smaller, non-repeating findings) ⇒ fold and disclose the extension in one line; open-class (each round surfaces new instances of the same kind, e.g. a new language's shadowing semantics every round) ⇒ park slice 3 and escalate to §12.

---

## 11. Risks

1. **Language-semantics fix waves — budget two** (roadmap §3.3). The named hazards are Python `nonlocal`/`global` (no `is_declaration_node`, `src/languages/mod.rs:240`, so RD sees module-level and function-level bindings of one name as one path), Go `:=` shadowing in `if`/`for` headers (`short_var_declaration` is a declaration node, `:249`, but the header is not inside a `"block"`, so `is_shadowed_at`'s scope walk does not see it), JS `var` hoisting (a use before the def is a real read that RD must label `nameonly`, not delete), and Rust `let` shadowing in one block (`is_shadowed_at:5150` requires `!def_in_scope`, which same-block shadowing never satisfies). Mitigation: §7.1 pins one fixture per hazard **before** slice 3, and every hazard's failure direction is `NameOnly` (§6), so a wrong answer is a lost Exact, never a false Exact.
2. **Doctrine drift — two copies of a consult** (roadmap §8). The label (graph payload vs. `dfg.labels`) and call confidence (a `ResolvedCallEdge.confidence` vs. a re-derivation) are at risk of duplication. Mitigation: §7.4 and cache gate 10 for the first; §4.4 and §7.6 require recording the existing resolved confidence for the second. Same-line groups deliberately do not consult an ordering in v1.
3. **RD performance on large functions.** Bit-vector width is defs-per-function. Generated code (protobuf marshallers, large `match` tables) can push that into the thousands, and the pass runs inside the already-parallel per-file DFG build (`src/data_flow.rs`, `par_iter`). Mitigation: hard caps `RD_MAX_DEFS = 2048`, `RD_MAX_LINES = 4096` (**measured 2026-09-04**, `~/code/tools/logs/item2-census/REPORT.md`: 92,338 functions over 14 Tier-A corpora + 14 fixture packs; worst `n_defs` 590 (Go), worst CFG statement count 331; 0 functions over either cap, 3.0× headroom above the worst generated function; worst-case per-function bit-vector allocation 4 MiB, 60 MiB across the 15-worker pool; cold DFG builds are 1–3 s against CPG builds of 20–305 s, so RD has about a whole DFG build of budget), over-cap functions counted (`dfg_rd_functions_over_cap`) not silently skipped; reverse-postorder worklist rather than round-robin iteration. `RD_MAX_LINES` bounds `stmt_lines.len()` (the statement-line universe), not the function's line span (span max is 3,296, which would leave only 1.24× headroom). The sparse `BTreeSet<DefId>` escalation trigger is not met; dense bit-vectors are right.
4. **CFG statement-universe incompleteness (measured 2026-09-04, REPORT.md §G/§H; decomposed by the Task 0b STOP report) — the binding constraint on what RD can deliver.** The raw metric "both endpoints ∈ statement start lines" conflates THREE mechanisms: (a) wrapper nodes hiding nested statements (Task 0b's job; the wrapper fix alone moved Rust 32.9 → 61.1 %, Go 47.9 → 71.4 %, TS 46.4 → 66.2 %, Python 77.1 → 80.8 %); (b) DFG endpoints on **continuation lines** of multi-line statements, which no statement-universe change can admit because the universe is start lines by construction — resolved on the RD side by `stmt_of` (step 33), not by the CFG; (c) DFG ownership attributing nested anonymous-callable bodies (lambdas, closures, arrow functions) to the enclosing named function, which the CFG correctly does not traverse — those endpoints are `NameOnly(CfgIncomplete)` by the provisional capture rule (step 30) and are excluded from the admissibility denominator. **Byte-control blind spot (Task 0b review, v6.4):** the CFG universe change re-splits sequential CFG edges and adds `CpgNode::Statement`s, yet the fixture byte control stayed 1598/1598 identical — the checked-in fixture/algorithm surface does not reach the CFG universe, so a zero diff there proves nothing about CFG correctness; the per-language RED fixtures, the census and Tier-A are the evidence for this commit, and a CFG-structure census (nodes/edges per fixture before/after) is reported as data, not a gate. Residual known gap: wrappers that are not statement nodes (Python `with`/`except`/`case`, JS `switch_case`/`catch`) contribute their bodies but not their own header line, so a use in a `with EXPR:` or `case PATTERN:` header stays outside the universe — filed as a follow-up, measured by the same census. Only 22–89 % of existing DataFlow edges have both endpoints inside the CFG's line universe on real corpora (Rust 32.9 %, TSX 22.5 %, TypeScript 25.5 %, Go 47.9 %, Python 77.1 %; the fixture packs score 90.5–100 % and cannot see it). §4.2 step 33 sends every other edge to `NameOnly(CfgIncomplete)` before RD runs. Verified Rust mechanism (controller read `src/ast.rs` `collect_statements` at bffb847): the walk recurses into a statement's body only when `is_control_flow_node(kind)`, and tree-sitter-rust wraps a block-tailed `if`/`match`/`loop` used as a statement in `expression_statement`, so the whole body is invisible; other languages' mechanisms are in REPORT.md §G. Mitigation: **Task 0b** (§10) — a designated behaviour commit (§5) that completes the statement universe per language before RD is wired in, with the census's CFG-admissible share as its acceptance metric.
4. **Byte-pin friction.** Every one of the `==`/`!=` sites (§2.1 enumerates eleven; Task 1 re-derives the census with `grep` rather than trusting a count — plan v2 defect 4) and 2 exhaustive `match` arms is a compile error, which is the *good* case; the dangerous case is a `matches!` site silently widened to `DataFlow(_)` in a context where the author meant "unlabeled only". §7.3 is the guard.
5. **`unlabeled` admitted by the default `--min-confidence nameonly`** is a deliberate compromise (§4.5) that keeps the byte-pin but means the default gate is weaker than the flag name suggests. Documented in `--help` text and in the SARIF `properties` description.
6. **Cache thrash for downstream consumers.** v56 invalidates every per-diff CPG cache and, transitively, the nav index. One-time cost, disclosed in the handoff.
7. **CFG join provenance.** Existing try-header exception/finally, Go return→defer, and globally sorted branch-arm sequential edges can create a false Exact if treated as ordinary flow. Task 0 adds internal provenance without changing line-edge bytes; §4.2 and the three negative fixtures force `NameOnly(CfgIncomplete)` at each join.

---

## 12. Open questions **[OWNER]**

**Q1 [OWNER] — ANSWERED (A1, 2026-09-04) and MEASURED:** report at `~/code/tools/logs/item2-census/REPORT.md` (prism branch commit 80d9b5e adds `examples/dfg_census.rs`). Caps set: `RD_MAX_DEFS = 2048`, `RD_MAX_LINES = 4096` over the statement-line count (§11 risk 3). The over-cap `CfgIncomplete` rule remains binding. The pass also surfaced §11 risk 4 (CFG statement-universe incompleteness), answered by Task 0b.

**Confirmed by the owner (B8):** `--min-confidence` limited to finding-bearing outputs; per-occurrence `DefId` identity is a future item; the capture-read rule is provisional (§4.2). **Delegated:** RD signatures to the Task 2 implementer (B2); PartialHit label survival to the plan's analysis pass (B5).

Payload choice, alias behavior, nominal default, exhaustive relation handling, and the labeled provenance twin are settled conservatively above: persisted `DataFlow(FlowConfidence)`, alias-derived `NameOnly(AliasUnstable)`, default nominal, unproved relations crossed-unlabeled, and one walk implementation under the byte control.

---

## 13. Review record

- **v6.4 (2026-09-04)** — Task 0b review (Opus; Approved, 0 WRONG, 3 Important SMELLs escalated): span gate near-passing before the fix (no attribution) → RD-relevant metric `distinct_stmt` with base/head before-after (§10); exclusion invariance control; byte control blind spot + header-line residual recorded (§11 risk 4); §9 carve-out names the arm mirrors. Measurement follow-up in the clone; Task 1 proceeds in parallel (independent of the universe metric).
- **v6.3 (2026-09-04)** — Task 0b STOP (sol, clone): wrapper fix implemented for 11 languages with per-language RED, but the raw start-line metric stayed under the 75 % floor (Rust 61.1, Go 71.4, JS 45.2 on 301 edges, TS 66.2, TSX 27.1; Python 80.8 middle band without attribution); two residual mechanisms reproduced by probe (continuation lines; nested-callable ownership). Ruling: the metric, not the fix, was wrong — split per §11 risk 4; RD maps endpoint lines to statements (`stmt_of`); Task 0b gated on span-admissibility. Task 0b resumes: extend the census, re-measure, then the gates.
- **v6.2 (2026-09-04)** — sol plan-review r2 (`reviews/item2-plan-r2-sol.md`, FIX, W=2 S=1, converging) folded: Task 0b middle-band decidability (only `ACCEPT` passes). Its two WRONGs are plan-only (stub closure; labeled-walk assertions) → plan v4, controller-verified (review cap reached; extension disclosed in LEDGER.md).
- **v6.1 (2026-09-04)** — plan v3 notes folded: §10 Task 0 acceptance no longer names the fixture B6 moved to Task 3/6 and now lists the base measurement, base binary and Java/Lua census; Task 0b thresholds carry denominators (small-population languages).
- **v6 (2026-09-04)** — sol plan-review r1 (`~/code/tools/reviews/item2-plan-r1-sol.md`, FIX, W=4 S=1 I=0) folded: W1 §7.6/gate 11 manifest reduced to the four actual finding producers (controller re-verified by grep); W2 Task 0b acceptance is corpus-level with a 75 % floor and attributed residuals; W3 decoupled cap tests with cap-specific reasons; W4 base binary built from the detached worktree; S1 two-part RED shape. Sol's six rulings: PartialHit (b) SOUND, `worst()` SOUND, gate 9 SOUND; Task 0b metric, cap operand, RED shape UNSOUND → fixed here.
- **v5 (2026-09-04)** — plan v2 re-anchor defects ruled: gate 9 scoped to the §7.1 fixture languages with N/A recording (1); `delta_slice` argument corrected, decision unchanged (2); gate 4 base = bffb847, base binary rebuilt in Task 0 and after Task 0b (3); site count (4); `labels` insert rule `worst()` (5); "Phase-0 goldens" = same-base binary output (6); §9 Task-0b-only exception (7); §4.6 records plan v2's PartialHit choice (b) pending sol.
- **v4.1 (2026-09-04)** — measurement pass folded: caps 2048/4096 over the statement-line count (§4.2, §11 risk 3, §12); new §11 risk 4 (CFG statement-universe incompleteness, 22–89 % admissible) with Task 0b as the designated behaviour commit (§5, §10).
- **v4 (2026-09-04)** — owner rulings folded from `~/code/tools/DECISIONS.md`: A1 (scheduled; measurement pass), B2 (§4.2 signatures by implementer), B3 (§4.5 clap value enums), B4 (plan re-anchor by controller — plan v2), B5 (§4.6 PartialHit analysis in the plan), B6 (§10 Task 3 staged RED / Task 6 GREEN), B7 (§7.6 three provenance cases), B8 (§4.2 capture rule provisional; two confirmations), E4 (§5 behaviour commits). No design change to §3/§4.1/§4.3/§4.4.

1. **WRONG FOLDED — same-line labels:** §3, §4.2–§4.3, §6, and §7.1 prohibit byte ordering and require `NameOnly(SameLine)` for every collapsed endpoint group.
2. **WRONG FOLDED — closure/deferred timing:** §4.2, §6, and §7.1 classify captured nested/deferred-body reads as `NameOnly(CfgIncomplete)` and pin late-binding negatives plus `defer f(x)` immediate-argument Exact.
3. **WRONG FOLDED — provenance empty evidence:** §4.4, §7.2, and §7.6 mark `all_defs_of` origin selection `crossed_unlabeled` and require a verified relation for Exact.
4. **WRONG FOLDED — filter surface:** §4.5 and §7.5 restrict the filter to finding-bearing formats and reject it for `text`/`paper`/`mermaid`/`callers`.
5. **WRONG FOLDED — nonlocal/global fixture:** §7.1 now asserts preserved absence/unlabeled behavior rather than minting a DFG edge.
6. **SMELL FOLDED — evidence transport:** §4.4, §7.2, and §7.6 require a per-finding conservative witness artifact; Echo/Membrane read `ResolvedCallEdge.confidence`.
7. **SMELL FOLDED — incremental cache:** §4.6, §7.5, and gate 10 require atomic label lifecycle and cold/full-hit/PartialHit parity after a one-file edit.
8. **SMELL FOLDED — finding delivery:** §7.6 and gate 11 require Exact and NameOnly end-to-end assertions for provenance, taint, Echo, and Membrane, including SARIF/targets tiers.
9. **WRONG FOLDED — evidence transport:** §4.4 defines index-aligned `Vec<Option<EvidencePath>>` through `SliceResult`, `ReviewRun`/`ReviewOutcome`, `SarifInputs`, and the existing Task 4 targets consumer; `None` is missing, not empty AST-only evidence.
10. **WRONG FOLDED — min-confidence projection:** §4.5, §7.5, and gate 11 require filtering before every JSON/review/SARIF/targets projection with findings/evidence zipped and default `nameonly` byte-preserving.
11. **WRONG FOLDED — runtime resolution:** §4.5 threads `ResolutionMode` CLI → API → `SarifInputs`/targets and forces nominal CPG algorithms to `Unlabeled/Candidate`; no fixed serializer constant remains.
12. **WRONG FOLDED — targets scope:** Goals, §4.5, §5, §7.6, gate 11, §9, and sequencing treat targets as the existing Phase 0 Task 4 consumer; acceptance is gated on “Phase 0 Task 4 landed” and no targets work is sized in Item 2.
13. **WRONG FOLDED — CFG try-header safety:** §4.2, §6, §7.1, and Task 0 name the Python/JS/TS/Java try-header exception/finally seam, force `NameOnly(CfgIncomplete)`, and provide `dfg_reaching_cfg_try_header_negative`.
14. **WRONG FOLDED — Go defer safety:** §4.2, §6, and §7.1 name `src/cfg.rs:374-402` return→defer edges, force `NameOnly(CfgIncomplete)`, and provide `dfg_reaching_cfg_go_defer_negative` alongside the immediate-argument pair.
15. **WRONG FOLDED — branch-arm sequential safety:** §4.2, §6, §7.1, and Task 0 name `src/cfg.rs:50-65` plus `src/ast.rs:5778-5791`, add lexical-arm provenance, and provide `dfg_reaching_cfg_branch_arm_negative` without changing edge bytes.
16. **WRONG FOLDED — same-line fixture/parity:** §7.1 moves the use to a later line, requires a non-empty `SameLine` set, and defines parity over unique `(from, to)` edges with duplicate collapse via `.or_insert`.
17. **SMELL FOLDED — Exact negative coverage:** §7.1 pairs every Exact fixture with a concrete NameOnly case, including loop-carried, outer-shadow, and immediate-defer negatives with non-empty delivery artifacts.
18. **SMELL FOLDED — language coverage:** §7.1 and gate 9 require fixture-based coverage for all twelve `Language::all()` values, not corpus anchors only.
19. **IMPL-DETAIL FOLDED — public result transport:** §4.4 and §9 record the public `SliceResult` source-shape decision, `#[non_exhaustive]`, aligned evidence field, and `#[serde(skip)]` safety argument because SliceResult never round-trips through the CPG cache.
20. **IMPL-DETAIL FOLDED — persisted edge payload:** §4.1, §4.4, §4.6, and §5 require the real `CpgEdge` payload to remain persisted and unsuppressed; only the non-persisted SliceResult sidecar is skipped.
21. **IMPL-DETAIL FOLDED — cache transitions:** §4.6 and Task 1 alone own the sole cache transition at `55→56`; Task 0 and later tasks do not add a transition.
22. **IMPL-DETAIL FOLDED — sidecar lifecycle:** §4.6 keeps the call-edge sidecar at `24` and §7.5 tests cold/full-hit/PartialHit parity.
23. **FOLD-INCOMPLETE FOLDED — relation exhaustiveness:** §4.4, §6, and §7.2 match `CallDescent`, `ReturnInput`, and `ReturnFlow` exhaustively as `crossed_unlabeled` unless separately proved.
24. **FOLD-INCOMPLETE FOLDED — delivery gate:** §7.6 and gate 11 enumerate every Item 2 emitted-finding producer with non-empty Exact and NameOnly cases, aligned evidence, and SARIF plus Task 4-gated targets delivery.

## v6.5 amendments (2026-09-05) — binding controller rulings after the Task 6 Tier-A adjudication

Source: `reviews/item2-task6-adjudication-astra.md` (16 disagreeing DFG matrix cases: 1 oracle wrong, 7 implementation wrong, 8 spec conflicts). Where a sentence below conflicts with earlier text, this section governs; earlier text is not rewritten in place so the history stays auditable.

1. **Nested-scope kill rule (supersedes the flat `kills(d', d) := d'.path == d.path` reading of §4.2 for nested scopes).** A definition introduced in a nested block scope (`let`/`const`/`var`-in-block, Go `:=` in an inner block, Rust `let` in an inner block, Python/JS/TS function-nested blocks that create a binding) **masks the outer binding only within that nested scope; on scope exit it does not kill the outer definition.** A subsequent assignment to the *outer* binding does kill it. Same-block replacement behaviour is unchanged. Consequences for §7.1: every `dfg_reaching_shadowed_inner` positive edge `outer-def → outer-use` is **`exact`**; every `_negative` variant is `nameonly/killed` at the line of the *outer* reassignment (e.g. `kill_line = 7`), never at the inner declaration.
1a. **Binding ownership (2026-09-05, after the Task 6a review; refines rule 1).** Kills and masking are decided per *binding*, not per containing block. Every definition and use is resolved to the binding it refers to by a lexical scope walk over the existing AST block structure (nearest enclosing declaration of that name; Go `:=` in an `if`/`for`/`switch` header declares in the header's implicit scope; a plain assignment updates the nearest enclosing declaration wherever the assignment sits). Same binding ⇒ ordinary kill semantics (an inner-block assignment to the outer binding is killed by a later outer redeclaration or assignment). A definition of an inner binding never kills the outer binding, and an assignment to an inner shadow never kills the outer definition. **Scope exit ends the inner binding:** from the closing line of its block, its definitions are dead, so an edge from an inner-binding def to a use outside the block is `nameonly/killed` with `kill_line` = the block's closing line (edges are never removed, §4.2). When the scope chain cannot resolve a binding (no lexical declaration in scope — e.g. a TypeScript/JavaScript assignment expression to an undeclared or module-level name, a Python global), labeling falls back to EXACTLY the pre-v6.5 flat-path kill equation `kills(d', d) := d'.path == d.path` for that def/use pair: the result is `exact` when no same-path definition intervenes on a reaching route, `nameonly/killed` otherwise. An unresolvable binding is NOT a `CfgIncomplete` doubt (§4.3 reserves that for "RD could not run") and never lowers a label below what the flat-path rule yields; `typescript/dfg_reaching_cfg_gap` therefore stays `exact` (rule 2). Consequence for §7.1: `go/dfg_reaching_go_short_var_if` asserts `3:v→7:v exact` and `4:v→7:v nameonly/killed@<if-block end>` in addition to the line-5 rows.
2. **Continuation lines are not CFG incompleteness (reconciles §4.2 v6.4 statement mapping with §7.1 `cfg_gap`).** A def or use on a continuation line of a multi-line statement belongs to that statement (innermost-statement mapping) and receives normal RD labeling. `typescript/dfg_reaching_cfg_gap` therefore expects **`exact`**; the §7.1 row prescribing `nameonly/cfg_incomplete` for a "def inside a multi-line call argument list" is withdrawn.
3. **`CfgIncomplete` before `Killed` is binding (§4.3 restated).** A kill may be reported only when RD proved a reaching redefinition; when no candidate reaches because the CFG is incomplete (try headers, handlers, missing exception edges), the label is `nameonly/cfg_incomplete`. The implementation's fallback that reports the def's own line as a kill (`Killed@<def line>`) is a defect; the three `cfg_try_header_negative` cases (py/js/ts) expect `nameonly/cfg_incomplete`.
4. **Definition observations are deduplicated by occurrence.** `SameLine` means two *distinct* defs of one access path collapsing onto one line-granular endpoint (§4.1). One syntactic occurrence extracted twice (Go `short_var_declaration` satisfying both the assignment and the declaration predicates) is one definition. The four Go `SameLine` disagreements are implementation defects; expectations stand as staged (`defer_argument_now` `3:x→4:x exact`; `go_short_var_if` `4:v→5:v exact`, `3:v→5:v nameonly/killed@4`; `killed_def` `3:x→5:x nameonly/killed@4`; `shadowed_inner` `3:x→8:x exact` under rule 1).
5. **Crossed evidence is `Unlabeled` (§4.4 governs; the §7.1 `go/dfg_reaching_shadowed_inner_negative` row demanding a "non-empty NameOnly finding with `crossed_unlabeled = true`" is withdrawn).** A finding whose evidence crosses that boundary is `unlabeled/candidate`; this is tested by the §7.6 producer/evidence tests (Task 4/5 already lock it), **not** by the DFG matrix. The matrix row keeps the absence assertion for `3:x→6:x` and adds the fixture's nonempty edge `3:x→9:x nameonly/killed@8`.
6. **Matrix observation surface (§7.1 binding).** DFG matrix cases read exactly two surfaces: `nav dfg-stats --edges` (JSONL, one row per labeled edge) and `nav dfg-stats` (JSON counters, §4.7). `expect.findings` is not a matrix field; finding assertions live in §7.6 evidence tests.
7. **Counter identity (§4.7 restated).** `dfg_label_loop_carried` is a *subset* of `dfg_label_exact` (an edge counts in both when its final intraprocedural label is Exact and `Lu < Ld`). Therefore **labeled edges = `dfg_label_exact` + the five mutually exclusive NameOnly counters**; summing all seven counters double-counts the loop-carried subset. Counts follow emitted edge multiplicity, never a unique-key set size.
8. **Oracle correction.** `go/dfg_reaching_interproc_nameonly` expects a parameter `p` that its source (`callee(int)`) does not declare; the case is corrected to `present = false` for `5:x→2:p`, and a new realizable fixture with a named parameter and an eligible callee is added so §7.1's `CallNameOnly` coverage stays non-empty (Step 5b labels existing edges; it never manufactures parameters).
9. **Citation fix.** The `dfg-stats` schema and counters are §4.7; §7.5 is "Cache lifecycle and CLI".

- **v6.5.2 (2026-09-05)** — amendment 1a fallback clarified after the Task 6a fix round STOP: unresolvable bindings use the pre-v6.5 flat-path equation verbatim; not a CfgIncomplete doubt.
- **v6.5.1 (2026-09-05)** — amendment 1a (binding ownership) added after the Task 6a review.
- **v6.5 (2026-09-05)** — amendments 1–9 above (controller rulings; adjudication seat: astra). No change to the lattice (§4.1), the four-producer manifest (§7.6) or the byte-control doctrine (§5).
