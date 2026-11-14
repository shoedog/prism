# Item 4 — statement-level dependence over CLI and MCP (`slice_at`), gated

Spec only · 2026-10-31 · prism main `533f061a` read-only · release binary `226ff60f…` (built 2026-10-30 22:00) probed on
httpx `b5addb64`, cobra `ad460ea8`, excalidraw `0642e72c` (the tracing trial's pinned checkouts). Claim tags: **measured**
(a number from a run or an evidence file, cited), **source-verified** (read at the cited `file:line`), **judgment**.
Gate receipts: [`spec-pilots/04-gate-pilot/`](../../../analysis/2026-10-31-nav-gaps/spec-pilots/04-gate-pilot/) (script, report with per-question counts, binary
hash and commits). The 30 raw CLI outputs, the gold and the question texts stay in the private evidence store
([evidence README](../../../analysis/2026-10-31-nav-gaps/README.md)).
Cross-spec decisions, tool budget, unified seed grammar and build order: `00-README.md` (this spec's PRs there are P0,
S-1a, S-1b, S-2, S-3, S-4, and the contingent promotion S-P).

## 0. Verdict first

The mandatory gate (§2, Stage A) has been **run** with the existing CLI against the four low-ceiling DIRECTED questions.
Against the pre-registered threshold from the gap analysis (ceiling rises ≥ +0.30 on ≥ 2 of 4 questions):

| Recipe | G3 (cobra, data) | J3 (excalidraw, data) | P3 (httpx, data) | P4 (httpx, guards) | Verdict |
|---|---|---|---|---|---|
| 2026-10-22 bundle (control) | 0.25 (8/32) | 0.21 (4/19) | 0.24 (7/29) | 0.33 (1/3) | — |
| **T0** strict: one call per question-text seed | 0.44 (+0.19) | 0.16 (−0.05) | 0.17 (−0.07) | 0.67 (+0.33) | **1 of 4 → FAIL** |
| **T1** T0 + one mechanical follow-up round | 0.78 (+0.53) | 0.16 (−0.05) | 0.79 (+0.55) | 0.67 (+0.33; 1.00 with the parameter-line fix, §5.3) | **3 of 4 → PASS** |

(measured; [`pilot-report.json`](../../../analysis/2026-10-31-nav-gaps/spec-pilots/04-gate-pilot/pilot-report.json)). The difference between T0 and T1 is exactly two capabilities: following a
single-candidate NameOnly callee (`reach: nameonly`, P3: inheritance hides `self.build_request`), and reporting a
field-store stop so the walk can resume at the field's reads (G3: `c.args = a`). **Recommendation: build, but only as the
package that includes both (PR-1..PR-3 of §10); a data-only `slice_at` without them fails its own gate.** J3 does not move
under any recipe here: its chain runs through a TypeScript class static arrow property and a `debounce` rest-argument
capture, which are A2-lane anonymous-callable gaps, not dependence-query gaps (§2.4).

Disclosure: the T1 follow-up rules were written after the first exploratory probes showed those two stops; T0 is the
recipe as pre-registered. The implementer must reproduce Stage A with the frozen script before PR-1 (deterministic, 13 s
warm; §2.2).

## 1. Goal, decision served, non-goals

**Goal.** Expose the dependence prism already computes — labelled def-use edges (`CpgEdge::DataFlow(FlowConfidence)`,
`Exact` or `NameOnly(Killed|SameLine|CfgIncomplete|AliasUnstable|CallNameOnly)`), intraprocedural CFG, Exact-gated
interprocedural descent — as one statement-seeded query, `slice_at`, on the CLI and the MCP server, with a per-line reason
and confidence for every returned line, and no silent empty results.

**Decision it serves.** The tiered-review lane's open question: can a cheap model (or an orchestrator) answer data-flow and
guards questions — the one class that scored ~0.5 F1 in every trial arm with no tool used (COMPARATIVE §5.4, measured; COMPARATIVE is the tracing-trial comparative write-up in the private evidence store) —
when the tool returns the dependence rather than a call graph? The 2026-10-22 DIRECTED run showed pasted navigation
evidence does not help (T2 Δ +0.018, p≈0.31) and that on the four questions above the fixed recipe did not even find the
sites (ceiling ≤ 0.35). Stage A answers "can the substrate find them"; Stage C answers "does a model use them".

**Non-goals.** New analysis (no post-dominator tree, no new DFG edges, no runtime-mutation modelling); changing any slicing
algorithm's output; a diff-driven surface (that is `prism --diff`); census/type-reference questions (G5-class, output
behaviour); C/C++ type enrichment; J3-class anonymous-callable modelling (lane A2).

## 2. The gate (mandatory, in this order)

### 2.1 Stage A — offline ceiling check (no model runs) — RUN

**What it measures.** For each of G3, J3, P3, P4, the share of frozen gold-v2 required units whose every citation slot has
an accepted location in the set of lines a dependence query reaches — the DIRECTED harness's own "bundle recall ceiling"
(`rule_covered` in the private trial harness, `directed-results.py`, with `core_bindings` excluded per owner E1).
Roles and semantics are ignored, so this is an upper bound on what copying could score, not a score.

**Seeds (question text only, as DIRECTED required).** P4: target line `httpx/_client.py:922`, backward. P3: the parameters
of `Client.request` and `Request.__init__`, forward. G3: the parameters of `Command.SetArgs`, `execute`, `ValidateArgs`
(`ExecuteC` has none), forward. J3: the `onChange` arrow at `excalidraw-app/App.tsx:677`, forward. Gold was never read by
the recipe; it was read only to score.

**Primitives used (today's binary).** forward = `prism nav taint-reaches --source file:line` (parameter Defs are pinned
to the function start line, `src/data_flow/build/file.rs:155-198` source-verified, so the start line seeds every
parameter); backward = `prism --diff <one-line synthetic diff> --algorithm fullflow,relevant --format json` (FullFlow
walks Uses on the diff line back to their Defs, `src/algorithms/full_flow.rs`; RelevantSlice adds the enclosing predicate).

**Tiers.** T0 = the calls above, one per seed. T1 = T0 plus one mechanical round driven only by the tool's own output:
backward — FullFlow from every line the depth-1 result contained (a depth-2 data+control closure); forward — (a) a
frontier Def whose path has a field (`c.args`) → grep that field's reads in the same file → seed each (cap 5); (b)
`nav callees --depth 2` from the seed body → seed each unique in-repo function's start line (cap 15; simulates
`reach: nameonly`). No human choice enters either tier.

**Pre-registered threshold** (gap analysis §Gap 4, written 2026-10-31 00:47, before any probe): build only if the ceiling
rises by ≥ +0.30 on at least 2 of the 4 questions. Result: table in §0. Per-question detail (measured):

- **P4**: T0 reaches the `response` definition (DFG `914 response def → 922 use` **Exact**) and the predicate line 921,
  and the `stream` parameter's Def is reached too (its DFG edge to the predicate use is **Exact**), but it is reported at
  the signature line because parameter Defs are pinned there, while the gold cites a body line. With the parameter-line
  remap (§5.3) P4 is 3/3. T1's larger line set also contains early-exit checks and producer statements that do not
  answer the question: the shape the query must distinguish (§5.5).
- **G3**: forward from `SetArgs(a)` stops at `c.args = a` (282) — a receiver-field store, outside the DFG's call-edge model
  (CLAUDE.md: struct-field data paths are not call edges). T1's field-reads follow-up seeds `command.go:1102`
  (`args := c.args`) and the whole ExecuteC → Find → execute chain follows (+11 units). 7 units remain uncovered at T1:
  the walk does not follow a recursion through a helper function or an early return.
- **P3**: forward from `Client.request` stops at `self.build_request(...)` (820): `build_request` is defined on
  `BaseClient`, Python inheritance is a documented resolution gap, so the Exact-gated descent refuses. Seeding the
  single in-repo `build_request`, `_merge_headers`, `Headers.update`, `Request.__init__` (T1 rule b) recovers 18 units.
  6 units remain uncovered at T1, all behind `self.headers` property reads (property getters and setters, and the
  code they call).
- **J3**: forward from the `onChange` arrow reaches the `LocalData.save(elements, ...)` call sites and the JSX wiring and
  stops: `LocalData.save` is a `static save = (...) => {}` class property (anonymous callable owner), `nav callees`
  from the call site does not list it, and the debounced callback receives `...args`. 3/19 under both tiers.

**Reproduce.** `python3 -I gate_pilot.py <prism> <cache-base> out.json`, run from a scratch directory (script and recipe:
[`gate_pilot.py`](../../../analysis/2026-10-31-nav-gaps/spec-pilots/04-gate-pilot/gate_pilot.py) and its
[README](../../../analysis/2026-10-31-nav-gaps/spec-pilots/04-gate-pilot/README.md); it needs the private trial directory for `match.py` and gold v2, and
the pinned checkouts; 39 CLI calls; 13 s warm, excalidraw cold build 23 s). The implementer re-runs it on the same
binary hash before PR-1 and again on the PR-3 head and compares the counts with the committed report; a differing count
is a defect to explain, not a new baseline.

### 2.2 Stage B — tool check after PR-1/PR-3 (no model runs)

One `slice_at` call per Stage-A seed (`direction` as in §2.1, `control: true`, `reach: nameonly`, `depth: 2`,
`max_results: 300`) must cover ≥ T1 − 2 units per question, and P4 must cover 3/3. This is a correctness check of the
one-call tool against the substrate ceiling, not a new decision. Report lines returned and bytes alongside coverage: T1
is an over-approximation (P3's rule b seeds whole functions; the real walk follows one call), so a lower line count at
equal coverage is the expected shape.

### 2.3 Stage C — agent re-run (≈288 runs, ≈$6–9, ≈5 h), control = 2026-10-22

**Design (recommended).** The 8 data + guards questions (P3 R3 G3 J3, P4 R4 G4 J4) × 4 models (Sonnet, Haiku, gpt-6-luna,
gpt-6.1-sol, current efforts) × 3 replicates × 3 conditions = **288 runs**:

| Condition | What the model gets | Purpose |
|---|---|---|
| `none` | arm `none` exactly as 2026-10-22 | concurrent control; its paired delta against the 2026-10-22 `none` cells measures drift (expect within the per-cell SD 0.04–0.18) |
| `slice-injected` | `none` + an orchestrator-run `slice_at` result (agent_markdown, question-text seeds, the Stage-B parameters) pasted under the same one-line note as DIRECTED | isolates payload content from tool calling; compares with 2026-10-22 `injected` (old recipe) |
| `slice-tool` | the trial's `prism` arm (MCP server with `slice_at`, started with `--experimental-tools slice_at`, 00-README X3) + the skill text of §7.3 | the tool-called condition; records every tool call |

The object questions (P5 R5 G5 J5) are dropped: `slice_at` does not address a census, and DIRECTED showed their
bottleneck is output behaviour (G5: bundle held 261/262, three models summarised). Alternative: 12 × 4 × 3 × 2
(`none`, `slice-tool`) keeps the 2026-10-22 design verbatim; recommended only if the owner wants the object rows for drift.

**Scoring and pre-registered thresholds.** `score.py --gold gold/v2` (private trial harness), raw/T1/T2 via the frozen option-A path, the
directed report script extended with the third condition. Decision rule, pooled over data+guards, T2, invalid = 0:
- Ship as an agent tool: Δ(`slice-tool` − `none`) ≥ **+0.15** and the tool was called in ≥ 50 % of `slice-tool` runs of
  the models that moved.
- Verifier tool only: Δ ≥ +0.15 for Sol alone, or Δ(`slice-injected` − `none`) ≥ +0.15 while `slice-tool` ≤ +0.05 (the
  content helps, the call does not happen) → route the output to the orchestrator's validate-and-repair step (which moved
  +0.11 on 2026-10-22) and to review-routing briefs, keep the MCP surface but stop investing in adoption.
- No lift (both Δ < +0.05): keep the CLI, document `slice_at` as a verifier/author tool, close the lane.
*Reconciled (00-README X3):* until Stage C, `slice_at` is registered on MCP only under `--experimental-tools
slice_at`. "Ship as an agent tool" means promotion to the default tool list (S-P), which must fit the program's
`tools/list` cap (17,500 chars) or come with an owner decision that frees the space. "Verifier tool only" keeps it
opt-in. From the default list, dependence is reached through item 2's `dependence` section (§7.2).
Secondary, reported not gated: citation overlap with the slice output; per-model tool-call counts; wall and USD per
condition; Haiku's exploration time (DIRECTED saw injected evidence cut it from 82 s to 57 s).

**Cost.** 2026-10-22: $6.31 Claude for 288 cells, Codex subscription-billed, ~4.3 h wall. The `slice-tool` condition adds
the prism arm's ~5.7k first-turn tokens per Claude session (COMPARATIVE §6, measured) and the calls themselves; estimate
$7–9. The $10 batch cap and killed-attempt estimate from DIRECTED W1 carry over.

**Harness changes** (Python, private evidence store, ~300 LOC): a `--directed-v2` mode with three conditions; bundle generator
recipe `slice` (one `slice_at` per seed via the prism CLI, agent_markdown rendered verbatim, caps as v7); the `prism` arm
rebuilt at the PR-3 head with hash receipts; report script with a third column and the 2026-10-22 drift comparison.

### 2.4 What would have to be different this time (weighing the null result)

The 2026-10-22 run says three things (measured): evidence in the prompt did not lift scores even where it held ≥ 85 % of the
gold; cheap models summarised instead of listing; injected evidence made Haiku explore less. A dependence tool changes the
experiment only if it changes one of those mechanisms:

1. **Tool-called, not pasted.** A call happens when the model has formed the question, so the result is read as an answer
   to it rather than skimmed as context; and nothing is front-loaded, so exploration is not suppressed. But adoption was the
   trial's failure (prism called in 15/96 runs, Sonnet 1/120, `taint_reaches` never), so `slice-tool` only tests this if
   the tool description and the skill name the question shapes verbatim: "what must hold for line L to run", "where does
   this value come from / go". Stage C records calls per run; a no-call run is a `none` run and is reported as such.
2. **Payload in claim shape.** The answer contract wants data claims as `{at, function, binding, operation}` chains and
   edges `{from, to, via=argument|return|field|capture}`, and guards as `controls{at, predicate, required_value}` plus
   `data_sources{at, binding}`. The 2026-10-22 bundles were `path:line  text` lists. `slice_at`'s agent view emits each
   line with its edge (`via`), variable, confidence and hop — the model transcribes rather than re-derives (§4.4). The
   measured bottleneck was output behaviour; lowering transcription cost is the one lever a tool has on it.
3. **Small and labelled.** Default 80 lines, nearest hops first, every NameOnly edge labelled with its doubt, every stop
   named (`field_store`, `descent_refused`, `cfg_incomplete`), so the model has an explicit next step instead of a wall.
4. **Skill text that routes.** Today's skill routes guards/data-flow nowhere (it names callers/callees/ego; `taint_reaches`
   only for "a new input source near shell/SQL/path sinks"). §7.3 adds a "data flow and guards" routine: slice, read only
   the lines it names, list every hop as a claim.

If Stage C still shows no lift with all four in place, the honest reading is that cheap models do not convert dependence
evidence into claims, and the tool's consumer is the orchestrator — outcome "verifier tool" above, which the measured
+0.11 of validate-and-repair makes worth having.

## 3. Current state (source-verified unless tagged)

| Fact | Where |
|---|---|
| DataFlow edges carry `FlowConfidence::Exact \| NameOnly(FlowDoubt)`; `dfg_edge_dump` renders them as `exact` / `nameonly` + `killed\|sameline\|cfg_incomplete\|alias_unstable\|call_nameonly` (CLI `dfg-stats --edges`) | `src/cpg/types.rs` (`CpgEdge`), `src/cpg/flow_confidence.rs`, `src/navigation/queries.rs:32-60` |
| Forward DFG reachability returns the worst label on the discovering path; backward reachability is unlabelled | `src/cpg/query.rs:680-700` (`dfg_forward_reachable_labeled`), `:833` (`dfg_backward_reachable`), `:865` (`_bounded`) |
| One `Statement` node per `(file, line)`, only for statement starts inside functions; continuation lines, signature lines and blank/comment lines have none; `statement_at` is the CFG entry | `src/cpg/statement_calls.rs` (`collect_file`), `src/cpg/cfg_queries.rs:66` |
| CFG builder handles C, C++, Go, Java, JS, TS, TSX, Python, Rust; no post-dominator computation anywhere (`grep dominat src/` finds only a test name) | `src/cfg.rs` (grep of `Language::`), judgment on coverage of Lua/HCL/Bash: sequential only or none |
| Parameter Defs are pinned to the function start line (`line: start`), with the parameter's own byte span; `resolve_symbol` already recomputes the declaration line for its output | `src/data_flow/build/file.rs:155-198`, `src/reasoning/seeds.rs` (`output_loc` vs `lookup_loc`) |
| A loc seed on a parameter's own line fails: `SymbolNotFound` (P3 `_client.py:781`, J3 `App.tsx:678`, both in the v7 bundles and reproduced) | `src/reasoning/seeds.rs::resolve_loc`, `loc_seed_nodes` |
| Taint trace: Exact-gated descent and return ascent, `MAX_TAINT_DESCENT_DEPTH = 2`, boundary records, "No CFG statement node … pure-taint fallback" | `src/cpg/trace.rs:25, 440-520, 1195-1225` |
| `nodes_at` drops `Statement`/`ReturnValue` nodes; `ego` accepts `DataFlow`/`ControlFlow` edge filters but at a statement returns mostly `Contains` (click core.py:1360, hops 3: 23 Contains + 4 DataFlow + 0 ControlFlow, measured in the gap analysis) | `src/navigation/queries.rs:1898-1990`, `:2346-2368` |
| Slicing algorithms produce `SliceResult.blocks[].file_line_map: file → (line → is_diff)`; no per-line reason; `evidence` is per finding and `#[serde(skip)]`; `left_flow_core` calls the unlabelled `dfg_forward_reachable` and always includes the signature and closing lines | `src/slice.rs` (`SliceResult`), `src/diff.rs` (`DiffBlock`), `src/algorithms/left_flow.rs` |
| MCP exposes `taint_reaches` only (sources/sinks, verdicts, witness graph); seed grammar `SeedInput::{Symbol{name,file?}, Loc{file,line}}` with `deny_unknown_fields`; defaults `MAX_RESULTS_DEFAULT 50`, cap 1000, view byte cap 80 000; `shape_result` binary-searches item counts to fit | `src/mcp/tools_reasoning.rs:14-18`, `src/mcp/input.rs:10-29, 377`, `src/mcp/output.rs:214-247` |
| Tool list today: 9 tools, 14 237 JSON chars in the golden `tools/list` (taint_reaches 2 548, the largest) | `tests/fixtures/mcp/golden/lazy-default.jsonl` (measured) |
| `Reason` is an externally tagged enum rendered by `evidence_view::view_reason` (exhaustive match) and `output/navigation.rs::render` (three arms); `Evidence.reasoning` is the additive-optional precedent | `src/navigation/types.rs:50-82, 164-176`, `src/mcp/evidence_view.rs:1273`, `src/output/navigation.rs:80` |
| CLI nav grammar and the `taint-reaches` arm (`--source file:line` repeatable); `api::nav` facade exposes `callers`/`callees` only | `src/cli.rs:413-560`, `src/main.rs:308`, `src/api/nav.rs:15-75` |
| File sizes (weighted): `queries.rs` 2 534 (over the ceiling), `evidence_view.rs` 1 510, `tools.rs` 1 322, `trace.rs` 1 113, `query.rs` 953, `input.rs` 720, `seeds.rs` 476, `cli.rs` 494, `main.rs` 983 | `python3 scripts/file_size.py` (measured) |

**What changes.** New query-time modules only (`src/cpg/dependence.rs`, `src/navigation/slice_at.rs`,
`src/mcp/tools_slice.rs`, `src/mcp/slice_view.rs`), one additive `ReasoningReason` variant, one additive optional
`Evidence` field, additive seed field `variable`, a CLI arm, an MCP tool, docs and skill text. No algorithm, cache format or
existing output byte changes (goldens change only by the new tool's presence in `tools/list`). *Reconciled
(00-README X3):* with the MCP tool opt-in, the default goldens do not change at all until S-P.

## 4. Interface

### 4.1 CLI

```
prism nav slice-at --repo <dir>
    (--location <file:line> | --symbol <name> [--file <file>] [--variable <name>])
    [--direction backward|forward]      default backward
    [--control | --no-control]          default --control (backward only; forward warns and ignores)
    [--reach none|exact|nameonly]       default exact
    [--depth N]                         interprocedural crossings 0..3, default 2
    [--max-results N]                   lines 1..1000, default 80
    [--format text|json]                default text
```
Exit codes and error rendering as the other nav arms (`output::navigation::render_err`, exit 3 on `QueryError`).
`--location` and `--symbol` are an exclusive required group; `--variable` requires `--symbol`.
*Reconciled (00-README section 3):* the arm follows the unified CLI grammar of `01-find-symbol.md` §5.1: one of
`--location`/`--symbol` is required, and the two together form a callable handle; `--variable` requires `--symbol`
(alone or as part of a handle). `--symbol` accepts qualified names (01 §5.2-5.3).

### 4.2 MCP tool `slice_at` (sibling of `taint_reaches`; one new tool, list grows 9 → 10)

*Reconciled (00-README X3):* registered only under `prism-mcp --experimental-tools slice_at` (standalone; a CLI error
with `--shared`, as `--no-cache` is) until Stage C promotes it. The default list does not grow from this spec.

Arguments (unknown keys rejected, as `object()` does for every tool):

| Key | Type / range | Default | Notes |
|---|---|---|---|
| `seed` | `{"kind":"loc","file","line"}` or `{"kind":"symbol","name","file"?,"variable"?}` | required | `variable` is additive on the shared `SeedInput::Symbol` (serde default `None`), so `taint_reaches` and `nav_*` parse unchanged. *Reconciled (00-README section 3):* the seed is the unified grammar: qualified `name`s, and `{"kind":"loc","file","line","name"}` handles, which mean the symbol seed of that callable and also take `variable`; the other tools reject a present `variable` with `bad arguments` naming `slice_at` |
| `direction` | `"backward"` \| `"forward"` | `"backward"` | |
| `control` | bool | `true` | backward only; `true` with `forward` → warning `ControlIgnoredForward`, result unaffected |
| `reach` | `"none"` \| `"exact"` \| `"nameonly"` | `"exact"` | §5.4 |
| `depth` | 0..3 | 2 | interprocedural crossings; 0 = intraprocedural |
| `max_results` | 1..1000 | 80 | lines retained, nearest first (§5.6) |
| `verbosity` | `concise` \| `detailed` | `concise` | as today |
| `format`, `profile`, `snippets`, `group_by`, `max_view_bytes` | as the nav tools | canonical | agent views §4.4; allowed profiles `Audit` (default), `Graph` |

Description (≤ 1 300 chars; the budget matters, every tool definition is paid on every run): "Statement-level dependence
slice from one seed line or one variable of a function. backward answers 'what must hold, and which values, for this line
to execute as it does' (data and control dependence, nearest first); forward answers 'where does this value go'. Each
returned line carries its edge (data | control | call | return | field), the variable, the confidence (exact | nameonly +
doubt) and the hop count. Use for guards, preconditions, early exits and value provenance; do NOT use for call hierarchy
(nav_callers/nav_callees) or taint verdicts with sinks (taint_reaches). Seed grammar …; `reach: nameonly` also follows
single-candidate name-only calls (labelled). Stops (field stores, refused descents, missing CFG) are listed in
`dependence.stops` so you can seed the next call there."

### 4.3 Result shape (canonical: `Evidence`)

- `query`: `slice_at:<direction>:<seed label>`.
- `items[]`: one per retained `(file, line)`, ordered by (`distance` asc, `hop` asc, file, line). The seed line is item 0
  (`distance 0`, `why: [Reason::EnclosingFunction{..}]`), so a successful result is never empty.
  - `symbol`: the CPG node the line was reached through — `SymbolRef::Variable` (Def/Use with `path`) for data edges,
    `SymbolRef::Statement{kind:"Branch"|"Loop"|"Return"|..}` for control edges, `SymbolRef::Function` for a callee/caller
    entered by a call/return crossing.
  - `location`: the statement line; a parameter is reported at its declaration line (§5.3).
  - `score`: worst label on the discovering path (§5.5 table).
  - `why[]`: one `Reason::Reasoning(ReasoningReason::DependsOn{..})` per distinct edge that reached the line (≤ 4,
    worst-first), new variant:
    ```
    DependsOn { edge: "data"|"control"|"call"|"return"|"field",
                from: Location,            // the other endpoint of the edge
                variable: Option<String>,  // access path carried ("response", "c.args")
                confidence: "exact"|"nameonly"|"syntactic",
                doubt: Option<String>,     // killed|sameline|cfg_incomplete|alias_unstable|call_nameonly|early_exit
                hop: usize, distance: usize }
    ```
- `truncated`, `warnings[]` as today; new `ReasoningWarning` variants: `FieldStore{path, file, line}`,
  `DescentRefused{callee, file, line, reason: "nameonly"|"multi_candidate"|"external"|"depth"}`,
  `CfgIncompleteFunction{function, file}`, `SeedAdjusted{from_line, to_line}`, `NoDependence{direction}`,
  `ControlIgnoredForward`, `ControlFromSyntax`.
- `dependence` (new, `#[serde(skip_serializing_if = "Option::is_none")]`, absent from every other query so existing
  bytes are unchanged):
  ```
  DependenceSummary { direction, seeds: Vec<SymbolRef>, lines: usize, data_edges: usize, control_edges: usize,
                      crossings: usize, exact_edges: usize, nameonly_edges: usize,
                      stops: Vec<Stop{kind: "field_store"|"descent_refused"|"boundary"|"budget"|"depth",
                                      file, line, detail: String, next_seed: Option<SeedInput-shaped value>}> }
  ```
  `next_seed` is what the agent (or Stage A's T1 rule) would seed next: the field's first read for `field_store`, the
  callee's symbol for `descent_refused`. *Reconciled (00-README section 3):* in the unified grammar, a plain
  `{"kind":"loc"}` seed for `field_store` and a callable handle `{"kind":"loc", file, line, name}` for
  `descent_refused`, so the next call cannot hit `AmbiguousSymbol`.
- `reasoning`: `None` (the taint summary is sink-specific and stays untouched).

### 4.4 Agent views

`agent_markdown`, grouped by file, one line per item:
`httpx/_client.py:914  data  exact  response  ← 922  hop 0  d1   | response = self._send_handling_auth(request, auth=auth, follow_redirects=follow_redirects)`
then a `Stops` block and `next_queries` (one `slice_at` per `stops[].next_seed`, one `nav_nodes_at` per retained line
beyond the first 10, a truncation hint as today). Source text only with `snippets: line` (existing `SnippetPolicy::Line`);
default off. `agent_json` adds `edge`, `variable`, `confidence`, `doubt`, `hop`, `distance` as top-level item fields
beside the normalised location, so a data-claim chain is a field rename away from the answer contract (`via` ← `edge`,
`binding` ← `variable`, `at` ← location).

### 4.5 Errors

`QueryError` as today: `LocationOutOfRange`, `UnsupportedFile`, `NoEnclosingCallable`, `AmbiguousSymbol{candidates}`
(symbol seeds resolve through `reasoning::seeds::resolve_symbol`, same rules as `taint_reaches`: exactly one in-repo
function or ambiguity), `SymbolNotFound` when `variable` names no parameter/local Def in the callable, or a loc seed has no
Variable/Statement node after the continuation scan (§5.7). MCP `BadArguments` for depth/max_results out of range,
`variable` on a `loc` seed (*reconciled:* on a `loc` seed without `name`), unknown `direction`/`reach`. Symbol seeds
resolve through the callable selection shared with the nav tools (00-README X4), so `AmbiguousSymbol` carries
`details` and `SymbolNotFound` carries `did_you_mean` as in `01-find-symbol.md` §5.4.

## 5. Semantics (precise enough to test)

### 5.1 Seed resolution → start set

| Seed | Backward start set | Forward start set |
|---|---|---|
| `loc` on a statement line | every `Variable{Use}` node at the line (`cpg.nodes_at`), plus the `Statement` node if present | every `Variable{Def}` at the line; if none, every `Use` (a bare call argument) |
| `loc` on a parameter's declaration line | the parameter Def(s) whose `start_byte` lies on that line (lookup at the function start line, as `resolve_symbol` does) | same |
| `loc` on a continuation/blank line inside a callable | walk back ≤ 20 lines to the nearest line with CPG nodes in the same enclosing callable (`cfg_reachable_including_continuation`'s rule, `src/cpg/cfg_queries.rs:87`), warn `SeedAdjusted` | same |
| `symbol` + `variable` | the named parameter Def, or every Def of that local in the callable | same |
| `symbol` alone | the Uses on every return statement of the callable ("what does its result depend on") | every parameter Def (as `taint_reaches`) |
| anonymous callable | refused for symbol seeds (`reject_anonymous_symbol`); loc seeds select the enclosing named function as today | same |

### 5.2 Backward step (data)

From each node in the frontier follow incoming `CpgEdge::DataFlow(label)` to its Def (`reachable_backward` restricted to
DataFlow, new labelled variant `dfg_backward_reachable_labeled` in `src/cpg/dependence.rs` recording, per reached node, the
worst label on its discovering path — the mirror of `dfg_forward_reachable_with_evidence`). A reached Def's line joins the
slice; the Def's statement's own Uses are the next frontier (same line, `nodes_at`). The bounded rule of
`dfg_backward_reachable_bounded` applies: never expand out of a projection-only Def (member-only JS/TS formals). Edge
`field` is recorded when the Def is a field path on a receiver base (`self.x = …`, `c.args = …`): the walk stops there
(no field-sensitive cross-function flow exists) and a `field_store` stop is emitted with `next_seed` = the first read of
that field in the same file by text (a documented, labelled heuristic; the agent decides).

### 5.3 Parameter-line remap (required by P4)

A reached parameter Def (`line == function_start_line` and `start_byte` inside the parameter list) is reported at
`parsed.line_for_byte(start_byte)`, the parameter's own declaration line (not the signature's first line, where the Def is pinned). The CPG node is unchanged; only
the item's `location` moves. The remap is unit-tested on a multi-line Python signature and a one-line Go signature.

### 5.4 Interprocedural reach

| `reach` | Backward crossings | Forward crossings |
|---|---|---|
| `none` | none; parameter Defs and return Uses become `boundary` stops | none |
| `exact` (default) | into callers through the arg→param DataFlow edge only when the binding edge is `Exact` (and the call edge Exact); into callees through `ReturnFlow` only when the return ascent gate holds (`return_flow_ascent_allowed`) | the existing taint walk: Exact-gated descent and ascent (`descend_target`, `src/cpg/trace.rs:440-520`), `ReturnFlowMode::On` |
| `nameonly` | additionally follow `NameOnly(CallNameOnly)` binding edges when the call site resolves to exactly one in-repo callee (the single-candidate pool; multi-candidate stays a `descent_refused{multi_candidate}` stop); labelled `nameonly/call_nameonly` | additionally descend into a single-candidate NameOnly callee (the P3 inheritance case `self.build_request` → the one `build_request` in the repo); labelled the same way |

`depth` counts crossings on the discovering path; `MAX_TAINT_DESCENT_DEPTH` (`trace.rs:25`) becomes a parameter of the
trace entry points with the constant as the default (PR-3) so forward depth 3 is possible; backward depth is enforced in
`dependence.rs`. External/std callees are `boundary` stops with the callee name.

### 5.5 Control step (backward only) and labels

No post-dominator tree exists and CFGs are incomplete on real corpora (click: 5 288 of 12 781 labelled edges
`cfg_incomplete`, 544 functions without CFG, measured in the gap analysis). v1 computes control dependence syntactically:

1. **Nesting** — for each frontier line, the chain of enclosing control-flow nodes inside the callable
   (`languages::is_control_flow_node`, `ast::enclosing_branch` generalised to return the chain); the predicate line of each
   (`control_flow_condition` node's start line; for `try`/`else`/`case` arms the arm header line) joins the slice with
   `edge: control, confidence: syntactic, doubt: None`; the predicate's Uses are the next data frontier
   (`condition_variables_on_lines` already extracts them). P4: 922 → `if not stream:` 921 → `stream` → the parameter's declaration line.
2. **Early exits** — preceding sibling statements in the same block chain that are branches whose body ends in
   `return`/`raise`/`throw`/`break`/`continue`/`panic`/`os.Exit` (`StmtKind::Return` or the language's exit idioms):
   their predicate line joins with `edge: control, confidence: syntactic, doubt: early_exit`. P4: the
   `if self._state == ClientState.CLOSED: raise` guard. The trial gold treats this class as neutral/transitive; the label lets a
   consumer keep or drop it.
3. **Exceptional predecessors** — not modelled in v1 (a question may ask that preceding statements completed normally); a
   `detailed` verbosity lists the preceding call statements in the same block as `edge: control, doubt: precedes` is a
   possible v2, not proposed now.

Score table (worst label on the discovering path, as `dfg_forward_reachable_labeled` defines it):

| Edge / label | score | `confidence` / `doubt` |
|---|---|---|
| data, `FlowConfidence::Exact` | 1.0 | exact |
| control, nesting | 0.9 | syntactic |
| call/return crossing, Exact binding | 1.0 | exact |
| data, `NameOnly(Killed{kill_line})` | 0.6 | nameonly / killed (kill line in `detail`) |
| data, `NameOnly(SameLine\|CfgIncomplete\|AliasUnstable)` | 0.6 | nameonly / that doubt |
| data or crossing, `NameOnly(CallNameOnly)` (only under `reach: nameonly`) | 0.6 | nameonly / call_nameonly |
| control, early exit | 0.6 | syntactic / early_exit |
| field-store follow-up (`next_seed`), never an item | — | — |

Scores are consistent with the nav convention (Exact 1.0, NameOnly 0.6; CLAUDE.md) so the skill's "read the site before
relying on any score below 1.0" rule carries over unchanged.

### 5.6 Ordering, determinism, budgets

- BFS by `distance` (dependence steps) with `hop` as the second key; ties by (file, line) — all `BTreeMap`/`BTreeSet`.
  Two runs on one index are byte-identical (test).
- `max_results` keeps the first N items in that order (nearest first), sets `truncated: true`, and records a
  `budget` stop at the last retained distance. The 80 KB view cap (`shape_result`) applies after; it only ever drops
  the tail, so truncation never removes a nearer line than one it keeps.
- Visit cap: 20 000 CPG nodes per query (warn `ResultTruncated`); measured need on the Stage-A seeds is < 1 500.

### 5.7 Fallback when there is no CFG statement node — no silent empties

| Situation | Behaviour |
|---|---|
| Seed line has Variable nodes but no `Statement` (parameter line, multi-line statement) | data steps run; control uses the syntactic chain (which needs no Statement node); warn `ControlFromSyntax` once |
| Seed line has no CPG node at all | continuation scan (§5.1); if it finds nothing → `SymbolNotFound`, never `items: []` with success |
| Callable has no CFG (`rd_functions_without_cfg`) | data labels already say `cfg_incomplete`; add one `CfgIncompleteFunction` warning so the agent knows every edge in that function is a candidate |
| Zero edges in the chosen direction (a constant assignment; a parameter never read) | success with the seed item only, `dependence.lines == 1`, warning `NoDependence{direction}` naming what was tried (`data in: 0, control chain: 0, crossings: 0`) |
| Every crossing refused | items from the intraprocedural walk plus one `descent_refused`/`boundary` stop per refused edge with `next_seed`; the agent view prints `Stops` even when items are few |

### 5.8 Language coverage

Data steps: all 11 languages (the DFG exists for all). Control nesting: every language with `is_control_flow_node` kinds
(all 11, source-verified list in `src/languages/mod.rs:605-635`). Labels better than `cfg_incomplete` require a CFG: C,
C++, Go, Java, JS, TS, TSX, Python, Rust per `src/cfg.rs`; Lua/HCL/Bash results are mostly `nameonly/cfg_incomplete` and
say so. Interprocedural reach inherits the resolution ladder's per-language precision (Python inheritance, JS anonymous
callables, Go field-typed receivers are the known walls: P3, J3, G3 respectively in Stage A).

## 6. Can `SliceResult` carry per-line reasons without changing the algorithms?

No (source-verified). `SliceResult.blocks[].file_line_map` is `file → (line → is_diff)`; `findings`/`evidence` are per
finding and `evidence` is `#[serde(skip)]`; `left_flow_core` throws the labels away (`dfg_forward_reachable`, not
`_labeled`) and merges branch-context lines whose reason is "inside the same small branch", not an edge. Carrying reasons
would need (a) `left_flow_core`/`full_flow` to call the labelled variants and record `(line, edge, label)` and (b) an
additive `DiffBlock.line_reasons: BTreeMap<String, BTreeMap<usize, Vec<LineReason>>>` with `skip_serializing_if` — a
behaviour-preserving but algorithm-touching change (~200 LOC) with no consumer in the review path today.

**Decision:** `slice_at` does not go through `SliceResult`. It is a navigation query built from the CPG primitives
(`src/cpg/dependence.rs` + `src/navigation/slice_at.rs`) producing `Evidence` directly. The algorithms stay byte-identical,
their mutation lanes are untouched, and the per-line reason is designed once in the place that renders it. If the review
path later wants reasons, the additive `line_reasons` field above is the minimal change, and `dependence.rs` is the walk it
would call.

## 7. Relationships

### 7.1 `taint_reaches`: sibling, not extension (recommendation)

Extending `taint_reaches` with `direction: backward` was considered and rejected: its description is already the largest
(2 548 chars), its semantics are sink verdicts and sanitizer proofs, and models choose tools by name — "taint" does not
read as "what does this line depend on". `slice_at` shares `SeedInput`, `reasoning::seeds::resolve`, the taint walk for
forward mode (so forward `slice_at` without sinks is the frontier mode with per-line reasons and stops), and the
`Reasoning*` warning family. Tool-list cost: +≈1.4k chars (≈350 tokens) per agent run; the gap analysis attributes
Sonnet's +8–25 % per-run cost to unused definitions, so the description is capped and item 2 absorbs no further tools.
(The +≈1.4k is the description; the whole definition, with schema and envelope, is about 2.65k chars by the 00-README
section 2 estimate. That is why the tool is opt-in until Stage C.)
Later consolidation option (not now): retire the frontier mode of `taint_reaches` in favour of forward `slice_at` once
Stage C has run; that would shrink the list back to 9 and is a golden-transcript change with a stated reason.

### 7.2 Item 2 (composite explore)

A `dependence` section fits when the composite's seed is a location: a depth-0 backward slice (`reach: none`, `control:
true`, `max_results: 12`) rendered as the §4.4 lines, under the section's byte share, with one `next_queries` entry for
the full `slice_at`. For symbol seeds the section is the parameter → return summary (`dependence.lines`, counts only) —
cheap but low value; recommend omitting it unless item 2's budget has room. Both reuse `slice_at::run` with a `limits`
struct; no second implementation. Item 2 must not include source bodies for this section either.

*Reconciled (00-README section 4):* the entry point is `navigation::slice_at::run(&NavigationSession, &SliceRequest)
-> Result<Evidence, QueryError>`, where `SliceRequest { seed: SeedInput, direction, control, reach, depth,
max_results }` carries the §4.2 arguments (the `limits` above). It is defined in S-1a and gains control in S-1b.
Item 2's PR-5 (E-5) calls it with the depth-0 backward parameters above, for `loc` seeds only. With `slice_at` opt-in
on MCP, this section is how the default tool list reaches dependence.

### 7.3 Skill and docs

`skills/prism-code-navigation/SKILL.md`: add a row "What must hold for this line to run / where does this value come from
or go? → `slice_at`" to the tool table and a short "Data flow and guards" routine (seed the line; read only the lines it
names; list each hop as a claim with its `via`; a `stops` entry is your next seed; `nameonly` edges are candidates, read
them). `docs/MCP.md`: tools table row, a gotcha on parameter lines (reported at the declaration line; seed a signature
line to mean "all parameters") and on `field_store` stops. CLAUDE.md: one paragraph under the MCP adapter section.
*Reconciled (00-README section 5):* S-2 documents `slice_at` in `docs/MCP.md` as an experimental, opt-in tool (the
`--experimental-tools` flag row plus the gotchas above) and adds the CLAUDE.md paragraph. The default skill does not
route to a tool that is not listed: E-5 routes the "data flow and guards" routine to item 2's `dependence` section,
and S-P switches it to `slice_at` with the table row. Stage C's arm carries this section's skill text in its harness
(S-4).

## 8. Data, cache, performance

- No cache change: query-time over the cached CPG (v132) and the nav index; no new persisted structures; call-edge sidecar
  untouched. `dfg_backward_reachable_labeled` reads edges already in the graph.
- Byte compatibility: every existing output unchanged (no existing `Evidence` carries `dependence` or `DependsOn`; the
  seed field is optional). Golden transcripts change only in `tools/list` and in the new transcript lines added for the
  tool (re-bless with the reason "new tool slice_at"). *Reconciled (00-README X3, section 5):* while opt-in, the five
  default goldens do not change. S-2 pins a flag-on transcript in `tests/mcp/slice_at_test.rs`. S-P re-blesses ids 3
  and 12 and uses transcript ids 55-59.
- Performance budget: warm query ≤ 150 ms on click/httpx/cobra-size corpora and ≤ 1.5 s on excalidraw at depth 2 /
  `max_results 80` (the Stage-A forward traces, which walk more than a slice will, took tens of ms warm; measured
  indirectly: 39 CLI calls in 13 s including process start and index load). Measure with `PRISM_PROFILE` on the Stage-A
  seeds in PR-1 and record in the PR.
- Memory: no retained state; the visit cap bounds the frontier.

## 9. Test plan

**Test-first variant fixtures** (`tests/navigation/slice_at_test.rs` as a new module of the `navigation` umbrella, plus
unit tests in the new modules), enumerated before the rule is written:

| Class | Variants |
|---|---|
| Seed kinds | loc on statement; loc on parameter line (multi-line signature); loc on continuation line (adjusted + warning); loc on blank/comment (→ `SymbolNotFound`); loc past EOF (`LocationOutOfRange`); loc outside any callable (`NoEnclosingCallable`); symbol+variable param; symbol+variable local with two Defs; symbol+variable unknown (`SymbolNotFound`); symbol alone backward (returns) and forward (params); ambiguous symbol; anonymous JS arrow (symbol refused, loc selects enclosing) |
| Direction × control | backward data-only; backward data+control (nesting 1, 2 and 3 deep); backward with early exit before the seed; forward (control ignored with warning) |
| Labels carried | one fixture per `FlowDoubt`: `killed` (reassignment between def and use, kill line in detail), `sameline`, `cfg_incomplete` (function without CFG, plus the function-level warning), `alias_unstable`, `call_nameonly` (only under `reach: nameonly`); Exact chain scores 1.0; a path through one NameOnly edge scores 0.6 at every later node |
| Reach | `none` → boundary stops only; `exact` crosses an Exact arg→param edge and a return flow; NameOnly single-candidate callee refused under `exact` (stop with `next_seed`) and followed under `nameonly`; multi-candidate refused under both; external callee → boundary; depth 0/1/2 cut at the right crossing; descent depth parameter honoured at 3 |
| Field store | receiver-field store ends the walk with `field_store` stop whose `next_seed` is the first read line; the read is not an item |
| Parameter remap | Python multi-line `def f(\n a,\n b):` reports `b`'s own line; Go one-line signature unchanged; the CPG node identity is unchanged (same `start_byte`) |
| Budget / determinism | `max_results 3` keeps distance-ordered nearest three and sets `truncated` + `budget` stop; two runs byte-identical; order independent of file iteration order (two files, cross-file crossing) |
| Fallback | constant-only seed → one item + `NoDependence`; every crossing refused → items + stops; no CFG in function → labels + warning |
| Languages | the nesting/early-exit chain on Python, JS, TS, Go, Rust, Java, C (`if`/`else`, loop, `try`/`except`/`catch`, `match`/`switch`); data step smoke on Lua/HCL/Bash with the `cfg_incomplete` expectation |
| MCP | argument validation (every key, ranges, `variable` on a loc seed); canonical `Evidence` shape (`dependence` present, `reasoning` absent); agent_markdown/agent_json views; `next_queries` parser-valid and derived from stops; result cap fitting (80 KB) keeps the nearest lines; a transcript line added to `tests/mcp/transcript.rs`'s script so the goldens pin it. *Reconciled:* a flag-on transcript in `tests/mcp/slice_at_test.rs` instead; the default `tools/list` without the flag is byte-identical; `--experimental-tools` with `--shared` or an unknown name is a CLI error; a handle seed with `variable` is accepted; `variable` on `nav_callers` is rejected |
| CLI | `tests/cli/`: grammar (`--location`/`--symbol` exclusivity, `--variable` requires `--symbol`), text and json renders, exit 3 on query errors; `readme_test` parse-only coverage of the new arm |
| Negative (no regression) | `taint_reaches` and all `nav_*` goldens byte-identical except `tools/list`; `cargo test --test integration dfg_label_parity_test` unchanged; the review path (`--format json/review/sarif`) byte-identical on the existing fixtures |

**Mutation lane** `mutants/slice-at.json` (~12 mutants: direction flip, label dropped to Exact, control chain off-by-one,
early-exit mislabelled as nesting, remap disabled, nearest-first ordering inverted, `reach` gate inverted, stop omitted,
depth off-by-one, `NoDependence` suppressed); run `--since main --scope fn --authoritative` plus this lane per PR.

**Golden transcripts** change (tools/list + new lines); re-record with `PRISM_MCP_BLESS_GOLDEN=1` in PR-2 with the reason in
the commit, after an eyeball diff that nothing else moved. *Reconciled (00-README section 5):* not in PR-2 (opt-in);
in S-P if Stage C promotes the tool. The mutation lane `mutants/slice-at.json` is created in S-1a and extended in S-1b
and S-3.

**Accuracy harness.** No call-resolution change in PR-1/PR-2 → Tier-A not required. PR-3 touches `trace.rs` descent
(reach `nameonly` is opt-in and off by default, so resolution results are unchanged) → run `tier-a --matrix-only` as a
control and say so in the PR.

## 10. Slice plan

| PR | Content | Files (new unless noted) | LOC (judgment) |
|---|---|---|---|
| **PR-0** (no code) | Stage A reproduction on the frozen script; receipt (binary hash, counts) in `docs/eval/slice-at/2026-10-31-gate.md` (numbers only; the script is already committed as [`gate_pilot.py`](../../../analysis/2026-10-31-nav-gaps/spec-pilots/04-gate-pilot/gate_pilot.py) and needs the private harness to run) | `docs/eval/slice-at/` | 60 (docs) |
| **PR-1** core + CLI | `dependence.rs`: labelled backward walk, nesting/early-exit chain, parameter remap, stops, visit cap; `slice_at.rs`: seed resolution (reusing `reasoning::seeds`), `Evidence` assembly, ordering/budget; `ReasoningReason::DependsOn`, new warnings, `Evidence.dependence`; `view_reason`/`navigation.rs` arms; `api::nav::slice_at`; CLI arm; tests; mutation lane | `src/cpg/dependence.rs` (~350), `src/navigation/slice_at.rs` (~300), edits ≤ 40 lines each in `src/reasoning/types.rs`, `src/navigation/types.rs`, `src/mcp/evidence_view.rs` (one match arm), `src/output/navigation.rs`, `src/api/nav.rs`, `src/cli.rs`, `src/main.rs`; tests ~500 | ~900–1 100 incl. tests |
| ↳ *reconciled split (00-README X10)* **PR-1a (S-1a)** data | everything in PR-1 except the control step: labelled backward walk, forward through the taint walk, parameter remap, `field_store` stop and `next_seed`, ordering/budget, visit cap, `SliceRequest`/`run`, `DependsOn`, `Evidence.dependence`, warnings except the two control ones, view/render arms, `api::nav`, CLI arm, `mutants/slice-at.json`; merges after 01's S1b | as PR-1 | ~650 |
| ↳ **PR-1b (S-1b)** control | §5.5 nesting chain and early exits, `ControlFromSyntax`/`ControlIgnoredForward`, the per-language control fixtures of §9, lane extension | `src/cpg/dependence.rs` (+~150, or a new `dependence_control.rs`), tests | ~400 |
| **PR-2** MCP | `tools_slice.rs` (descriptor, handler, schema), `input.rs` parser (+`variable`, `parse_slice_at`), `slice_view.rs` (agent views, next_queries from stops), golden re-bless, `docs/MCP.md`, skill text, CLAUDE.md paragraph, MCP tests. *Reconciled (S-2):* registered only under `--experimental-tools slice_at`; no golden re-bless and no default skill row (00-README section 5) | `src/mcp/tools_slice.rs` (~200), `src/mcp/slice_view.rs` (~200), `src/mcp/input.rs` (+80), `tests/mcp/slice_at_test.rs` (~250), docs | ~600–750 |
| **PR-3** reach `nameonly` | descent/ascent through single-candidate NameOnly callees behind the `reach` flag; `MAX_TAINT_DESCENT_DEPTH` as a parameter (constant default); backward `CallNameOnly` binding edges; Stage B run and recorded | `src/cpg/trace.rs` (1 113 weighted: +60 allowed, keep the gate logic in a new `src/cpg/trace_reach.rs` ~150), `src/cpg/dependence.rs` (+60), tests ~150 | ~350–450 |
| **PR-4** (private evidence store, Python) | Stage C harness: three conditions, slice bundle recipe, prism arm rebuilt at the PR-3 head, report with drift column | private trial harness: `directed.py`, `inject/generate.py`, `evidence/directed-results.py` | ~300 |
| later | item 2 `dependence` section; optional retirement of `taint_reaches` frontier mode; exceptional-predecessor control (`doubt: precedes`); CFG post-dominators if Stage C shows guards move and syntactic control produced FPs | — | — |

Order: PR-0 → PR-1 → PR-2 → PR-3 → Stage B → PR-4 → Stage C. PR-1 and PR-2 can be reviewed together if the owner prefers
one review loop; PR-3 is separate because it touches the trace walk (dependent-lane mutation trigger: execution moves).
*Reconciled order (00-README section 6):* P0 → S-1a (developed in parallel, merged after 01's S1b) → {S-1b, S-3 in
parallel, merge-serial on `dependence.rs`} → S-2 (after S-1b) → Stage B (after S-1b and S-3) → S-4 → Stage C → S-P
only if promoted. PR-1 and PR-2 are not combined, which keeps each PR within the 2-round cap.
Review cap per PR: 2 rounds (Opus-5.5 high + gpt-6.1-sol high per the standing lane); implementer Sonnet (high) per the
2026-10-29 rule.

## 11. Risks

| Risk | Mitigation |
|---|---|
| Syntactic control dependence over-includes (an `if` whose both arms reach the seed; a loop header) and under-includes (`goto`, exceptions) | labelled `syntactic`; early exits labelled separately; Stage B/C report FP lines against the guards golds (R4/G4/J4 have 7–16 units with neutral lists) |
| `reach: nameonly` admits wrong edges on dynamic languages (the precision floor exists for a reason) | opt-in; every such line scores 0.6 with `call_nameonly`; single-candidate pools only; Stage B counts how many T1 units needed it (P3: 18 of 23) |
| Payload on long functions (G3 T0 reached 120 lines at depth 2) | nearest-first truncation; default 80; `stops.budget` tells the agent where it was cut |
| Tool-list growth and description length | one tool, ≤ 1 300 chars; measured before/after in PR-2 |
| Stage C is noisy (3 replicates, per-cell SD 0.04–0.18) | thresholds pooled over 8 questions × 4 models; paired with concurrent `none`; drift check against 2026-10-22 |
| J3-class gaps misread as `slice_at` defects | J3 stays in Stage C as a known-floor question; its stops name the anonymous callable so the readout can attribute the miss to A2 |
| `evidence_view.rs` (1 510) and `input.rs` (720) growth | one match arm and one parser each; views in the new `slice_view.rs` |

## 12. Open questions for the owner (each with a recommendation)

1. **Gate verdict.** Strict T0 fails, T1 passes, and the T1 rules were written after the first probe. Accept the
   conditional verdict (build PR-1..PR-3 as one package, Stage B confirms) or require a clean re-run first? —
   *Recommend:* accept, with the implementer's Stage-A reproduction as PR-0; the two T1 rules are generic stop-following
   rules, and they define what the tool must do, not what the gold wants.
2. **Sibling tool vs. extending `taint_reaches`.** — *Recommend:* sibling `slice_at`; revisit retiring the frontier mode
   after Stage C. *Reconciled (00-README OQ1, OQ2):* sibling, but opt-in on MCP until Stage C, and promoted only
   within the program's `tools/list` cap.
3. **Stage C design.** 8 data+guards questions × 3 conditions (recommended, isolates pasted-vs-called) or 12 × 2 (keeps the
   2026-10-22 matrix)? — *Recommend:* 8 × 3; the object rows cannot move with this tool.
4. **Control dependence v1 = syntactic nesting + early exits** (no post-dominators). — *Recommend:* yes; a CFG
   post-dominator pass is a later slice only if Stage C shows guards questions respond and syntactic control is the FP source.
5. **Depth and descent constant.** Make `MAX_TAINT_DESCENT_DEPTH` a parameter (default 2, max 3)? It changes nothing by
   default but touches the trace walk. — *Recommend:* yes, in PR-3, with Tier-A matrix control.
6. **J3 / A2.** Route the two J3 blockers (class static arrow property as a callable owner; rest-argument capture) to the
   A2 follow-up issues rather than this lane? — *Recommend:* yes; note them in `js-ts-known-gaps.md` §6 in PR-2.
7. **Default `max_results` 80** (vs the nav default 50)? — *Recommend:* 80; P4 needs 8, G3 depth-2 needs ~60–120.

## 13. Claim ledger and reproduce

Measured: §0 table and §2.1 per-question detail ([`pilot-report.json`](../../../analysis/2026-10-31-nav-gaps/spec-pilots/04-gate-pilot/pilot-report.json), [`README.md`](../../../analysis/2026-10-31-nav-gaps/spec-pilots/04-gate-pilot/README.md) for hashes and commits);
DFG labels for the P4 edges (`prism nav dfg-stats --edges` on httpx, filtered to `_client.py` 914/921/922; `README.md`);
tool-list sizes (golden transcript); 2026-10-22 numbers (the DIRECTED write-up and results, private evidence store). Source-verified: every `file:line` in §3 and §5 at main `533f061a`. Judgment: LOC,
performance budget, the Stage C cost estimate, the claim-shape mechanism in §2.4.
Caches created for the probes were deleted; no repo file was changed.
