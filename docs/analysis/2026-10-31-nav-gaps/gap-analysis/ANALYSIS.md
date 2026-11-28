# Prism capability-gap analysis — 2026-10-31

**Decision this serves:** which of the eight gaps from the 2026-10-18 tools survey (`tracing-trial/TOOLS-SURVEY.md`
"Ideas prism lacks") to integrate next, judged by value to agent users against build effort and evaluation cost.

**Method.** Read-only. Source read at `<repo>` main (533f061a); release binary probed on
five small corpora (`PROBES.md`; raw outputs in the private evidence store, not committed); prism's own measurements read from
the private evidence store (`tracing-trial`, `review-routing`, `survey`) and `docs/analysis/`. No cargo. Caches deleted.

**Claim tags.** *measured* = a number from a prism run or an evidence file (cited); *source-verified* = read in
the code at the cited file:line; *survey* = from the 2026-10-18 tools survey or the trial write-ups (vendor pages
fetched then; not re-fetched); *judgment* = my estimate. Web search was available but not used: the survey is 13
days old and covers the same vendor sources; where my knowledge adds something it is tagged *judgment*.

**The measurement context every verdict has to respect** (measured):

- Tracing trial, 6 arms x 96 runs (2026-10-21): offering a navigation server changed no score beyond the +-0.1 noise
  floor; prism was called in 15/96 runs, Sonnet called a tool in 1 of 120 tool-capable runs, `taint_reaches` was
  never called; the biggest effects in the whole trial were format breakage (204/576 answers) and the 16k-token tool
  definitions of GitNexus (COMPARATIVE sections 2-6; COMPARATIVE is the private tracing-trial comparative write-up).
- Directed injection (2026-10-22): pasting prism + LSP evidence into the prompt moved T2 F1 by +0.018 (28 cells up,
  20 down, p~0.31) even where the bundle held >=85% of the gold; validate-and-repair moved +0.11; injected evidence
  made Haiku explore less (DIRECTED.md). The bottleneck was output behaviour, not evidence.
- Tier-C Part-C (debug-fix): prism grounds citations (resolvability 5%->100% on ruff) but does not change accuracy;
  +12% cost, 2-4x input tokens. Part-D (structural blast radius, gpt-5.5): REFUTED, median delta-dR 0.0, off-arm
  saturated 6/9 tasks, prism-on 1.0-2.7x input tokens (`docs/analysis/2026-07-05-...`, `2026-08-21-...`).
- Review routing (2026-10-18): 21 findings cite prism, 7 blockers, 2 of them missed-consumer; no control arm
  (`review-routing/ANALYSIS.md` section 10).

So the prior says: a new capability earns its keep by (a) lowering the cost of the first useful call (adoption,
payload, seed friction), or (b) producing evidence an orchestrator or verifier consumes deterministically, not by
giving a cheap model one more tool it will not call. The ranking below applies that prior.

---

## Summary table

| # | Gap | Exists today | Value | Effort (PRs) | Eval cost | Recommendation |
|---|---|---|---|---|---|---|
| 1 | Lexical/semantic search ("find helpers to reuse") | Partly: `nav functions` inventory (CLI only), exact-name seeds; no search tool, no ranking, no embeddings | **M** for a symbol search (fixes measured `AmbiguousSymbol`/guessed-name friction); **L** for semantic | **S** (1 PR, ~400-600 LOC) for symbol search; L for embeddings | S (deterministic seed-resolution rate on the 24 trial questions) | **Do the symbol-search half (2nd)**; skip embeddings |
| 2 | One-call composite explore | Partly: per-tool agent views with <=5 `next_queries`, `nav_symbol_spans`; no source text, no fan-in/fan-out in one call | **M** -- the adoption lever (codegraph `explore` was the most-called tool in the trial) but blobs replaced reading and cost recall | **M** (1-2 PRs, ~600-900 LOC); composes 1, 3, 5, 6 | M (adoption harness + 20-question Haiku replicates, ~$1-2/arm) | **4th**, after 6 and 1 land; no source bodies by default |
| 3 | Shortest call path A->B | No: BFS callers/callees to depth 5, flat per-site items, no target termination or path reconstruction | **L-M** -- saves payload (120 KB at depth 3 vs a path) but inherits the precision-floor drops that break paths at dispatch | **S** (1 PR, ~300-500 LOC) | S (5 flow golds + byte comparison) | 5th; cheap, do it as a section of 2 or when a reachability consumer appears |
| 4 | Statement-level dependence over MCP | Internally yes (DFG with confidence labels, CFG, 30 slicing algorithms, JSON diff spec seeds any line); MCP exposes only `taint_reaches`; `ego` at a statement returns mostly Contains edges | **M-H** -- the only differentiated capability here; data/guards questions scored ~0.5 in every arm with no tool used; but injected slices gave no lift | **M** (2 PRs, ~700-1,000 LOC) | S gate (gold-coverage ceiling, no model runs) then M (DIRECTED rerun ~$6) | **3rd**, gated: build only if the slice raises the coverage ceiling on the 4 low-ceiling questions |
| 5 | Implementations / type hierarchy | Go: satisfier machinery + CLI `interface-manifest`; Rust: trait-keyed method index, no query; JS/Python: nothing (`class_heritage` is a guard only) | **M** for Go/Rust, **L** elsewhere; the host LSP covers it where a toolchain exists | **M** for Go+Rust (1-2 PRs, ~500-800 LOC, new type-declaration seed kind); **L** for Python/JS/TS/Java | S for Go/Rust (gopls/rust-analyzer `implementation`; Tier-A oracles do not request it today) | 6th; Go/Rust only, when a Go/Rust review consumer asks |
| 6 | Ranked repo map under a token budget | No: `nav_repo_map` is file nodes + ModuleDep edges, unranked; MCP clips to the first 50 nodes in *path order*; CLI `onboard` ranks top-12 by degree | **M-H** -- orientation is the skill's step 1 and today's default view is an alphabetical slice; prerequisite for 2 | **S-M** (1-2 PRs, ~500-800 LOC; no cache change) | S (deterministic gold-file hit rate under budget; no model runs) | **1st** |
| 7 | Framework route maps | Partly: `framework_entry` caller edges for Flask/FastAPI/Express bare-identifier handlers (no path/method); 11 framework specs for taint sources/sinks; Go/Django/Koa/Fastify/NestJS entry edges absent | **M** for web-app review only; grep gets most of it in one call | **M** (2-3 PRs, ~800-1,200 LOC, cache bump) | S for extraction correctness; value eval has no task set | 7th; only with a web-app consumer and corpus |
| 8 | SCIP export/import | No reader/writer; Tier-A already checks prism against LSP oracles; `Source::ExternalIndex` and `--resolution precise/auto` are reserved for an external index | **L** now (no consumer; import needs the toolchain prism avoids) | **L** (3+ PRs, ~1,500+ LOC) | M | **Skip**; if a Java/C++ oracle is wanted, add a SCIP reader to `eval/` (~300 LOC Python), not to the product |

---

## Ranked recommendation

1. **Gap 6 -- ranked, budgeted repo map; promote `onboard` to MCP.** Smallest change with the most visible defect
   behind it: on a 302-file repo the MCP default returns the alphabetically first 50 files (source-verified:
   `clip_graph`, `src/mcp/tools.rs:288-299`, over nodes emitted in `BTreeSet` path order,
   `src/navigation/module_graph.rs:318-333`). Deterministic eval, no cache change, and 2 needs it.
2. **Gap 1 (symbol-search half) -- `nav_find_symbol`: substring/prefix match, kind and file filters, owner, caller
   count, confidence of the best seed.** Fixes the friction both prism and serena showed in the trial (prism:
   `AmbiguousSymbol` on same-name pairs; serena: 7 of 22 `find_symbol` calls empty from guessed name paths;
   measured here: 85 candidates for `into_response`). No embeddings: the research prism already collected says
   lexical is the backbone and agents prefer keyword queries (survey, vendor-reported).
3. **Gap 4 -- `slice_at` (backward/forward statement dependence from `file:line`) over CLI and MCP, gated.** The one
   capability no peer but GitNexus claims and prism already computes. Gate before building: run the existing
   slicing CLI with a JSON diff spec on the 4 low-ceiling DIRECTED questions (G3 J3 P3 P4) and measure whether the
   slice output contains the gold units the old bundle missed. If the ceiling does not rise, the tool is for
   verifiers only and the MCP surface can wait.
4. **Gap 2 -- composite explore**, once 6 and 1 exist: it is then composition (callers d1 + callees d1 + module deps
   + span + ranked neighbourhood) under one byte budget with per-section truncation. Without 1 it hits
   `AmbiguousSymbol`; without 6 its orientation section is the alphabetical slice. Whether 2 or 4 goes third is an
   owner call between the adoption lever (2) and the unique-depth bet (4); the gate on 4 is cheap enough to run first
   either way.
5. **Gap 3** as a small standalone or a section of 2. 6. **Gap 5** Go/Rust only. 7. **Gap 7** on demand.

**Dependencies:** 1 -> 2; 6 -> 2; 3 and 5 are optional sections of 2; 4 shares the `taint_reaches` seed grammar and
the DFG confidence vocabulary; 7 and 8 are independent. Every new MCP tool adds to the ~3.4k-token tool list
(measured from the golden transcripts), and the trial attributes Sonnet's +8-25% per-run cost to unused tool
definitions: fold 3 and 5 into 2 rather than shipping five new top-level tools.

**Skip:** semantic/embedding search (no measured lift anywhere in prism's evidence, a model dependency, and an index
format problem); SCIP import/export (no consumer; Tier-A already has oracles); Python/JS/Java type hierarchy
(new per-language extraction for a question the host LSP answers where it matters).

---

## Gap 1 -- Lexical/semantic search for "find helpers to reuse"

**Current state (source-verified).**
- Seeds are exact names: `seed::resolve_fn` (`src/navigation/seed.rs`) resolves `{kind: symbol, name, file?}` and
  returns `QueryError::AmbiguousSymbol { candidates }` or `SymbolNotFound` (`src/navigation/types.rs:181-189`).
  There is no substring, prefix, fuzzy or ranked lookup on the MCP surface (`src/mcp/tools.rs:25-75` lists the
  seven nav tools; none searches).
- `prism nav functions` (`src/cli.rs:534-540`, `src/navigation/inventory.rs:15`) dumps every function record
  (file, name, lines, node kind) as JSON -- a crude lexical index, CLI only, unranked (click: 1669 records, 260 KB;
  axum: 2120).
- No FTS/BM25, no embeddings, no `find_similar_code`; a repo-wide grep for `bm25|fts5|tantivy|embedding` finds
  nothing in `src/`.

**Probe (measured).** `callers --symbol invoke` on click -> `AmbiguousSymbol` (every `invoke` in core.py);
`callers --symbol into_response` on axum -> 85 candidates. A name-only lexical pass over the inventory finds 22 click
functions containing `param` in milliseconds -- the kind of answer a reuse question wants as a starting list.

**Comparison (survey).** codegraph: SQLite + FTS5, `codegraph_search` (called 4 times of 77 in the trial);
serena: `find_symbol` by name path (its most-called tool, 22/53, with 7 empty results from guessed paths);
GitNexus: `query` = BM25 + optional embeddings (never called); Sourcegraph: search is the product; aider: no search
(repo map instead). The archived research note (`docs/archive/analysis/prism-query-layer/research-llm-codebase-
navigation.md`) records the consensus: lexical/structural is the backbone, embeddings a fallback; agents issued
4,813 keyword vs 587 NL queries on CodeScaleBench (vendor-reported).

**Value.** The trial's four reuse questions (P6 R6 G6 J6) are exactly this use case. Models found plausible helpers
*without* tools in the valid answers (none.md "Reuse"), and 9/16 reuse answers broke on format -- so the measured
problem on reuse is output, not retrieval (measured). What prism can add over grep is not recall but a ranked,
symbol-level answer with structure: kind, owner, callers count, confidence, file role (production vs test, which
`code_context.rs` already classifies) -- and a seed the next call can use. A symbol search also removes the
documented first-call failure (`AmbiguousSymbol`, retry with a location seed) that the skill asks agents to handle
manually. Value M (judgment) for symbol search; L for semantic.

**Effort.** Symbol search: **S -- 1 PR, ~400-600 LOC.** New `src/navigation/symbol_search.rs` (queries.rs is at
2,534 weighted lines, over the 2,000 ceiling: no growth there) over the existing function table + owner index +
`direct_callers` counts; ranking = exact name > prefix > substring, then caller count, production before test;
cap 50; MCP tool + CLI subcommand + skill paragraph + golden transcript. No cache change (query-time). Risks:
tool-list growth (~1.3k chars per tool, measured); Rust impl methods need the owner shown or the list is the same
85-candidate wall. Semantic search: L (embedding model, index format, privacy, cache versioning) -- not proposed.

**Cost to evaluate.** S. Deterministic: take the 24 trial questions' named seeds (and the `AmbiguousSymbol`/empty
cases logged in prism.md section 8 and serena.md section 3) and measure first-call seed resolution rate (one unique
callable in one call) before/after; threshold: >= 90% resolved in one call, versus today's exact-name behaviour.
Reuse quality itself needs manual scoring the trial has not done (gold for reuse is unscored); do not fund that for
this slice.

## Gap 2 -- One-call composite explore

**Current state (source-verified).**
- Each nav tool has agent views (`format: agent_markdown|agent_json`) with profiles `Orientation, Seed, Impact,
  Dependencies, EditContext, Graph, Audit` (`src/mcp/input.rs:61-69`) and up to 5 `next_queries` hints
  (`src/mcp/evidence_view.rs:18, 770-816`), i.e. a guided multi-call workflow, not a composite.
- `nav_symbol_spans` returns coordinates and "never echoes the callable body" (`src/mcp/tools.rs:34-40`);
  `SnippetPolicy::{None, Line, SymbolHeader}` exists for views (`src/mcp/input.rs:72-75`); no tool returns source.
- `onboard` (`src/navigation/onboarding.rs:57`) is the only composite in the codebase and is CLI-only.
- Byte budget machinery exists: `MAX_VIEW_BYTES_CAP = 80_000`, `max_results` 50/1000 (`src/mcp/input.rs:10-13`),
  `shape_result` binary-searches item counts to fit the cap (`src/mcp/output.rs:214-247`).

**Probe (measured).** What an agent assembles by hand today for "explain `Command.main`" on click: callees d1
(26 items, 17.7 KB) + callers + module-deps + symbol-spans + a Read -- four or five calls; `ego` at hops 2 with Call
edges alone is 520 nodes / 305 KB (over the cap, truncated by the server).

**Comparison (survey; trial write-ups).** codegraph `explore` (default single tool; 38 of 77 calls; one 15-26k-char
Markdown block of flow + full source) -- when Haiku used it, it *replaced* reading: +0.29 on J3, -0.19 on R3 and
-0.36 on R4 because a call graph with source does not show which argument carries a value (codegraph.md section 6).
GitNexus `context` (8 of 9 calls). Serena has no composite. Sourcegraph MCP has none.

**Value.** M (judgment, grounded): the trial's clearest adoption signal is that the composite tool got called and
the specialised ones did not (prism 25 calls, all callers/callees/nodes_at). A composite is the adoption lever
*and* the recall risk; the design must make it a map, not a reading substitute: no bodies by default, every section
capped, confidence labels kept, `next_queries` pointing at the sites to read. It does not by itself change answer
quality (DIRECTED: evidence in hand did not lift scores).

**Effort.** **M -- 1-2 PRs, ~600-900 LOC.** `src/navigation/explore.rs` composing `callers`/`callees` d1,
`module_deps` of the seed file, `symbol_spans`, a ranked neighbourhood from gap 6, optional gap 3 path to a second
seed and gap 5 implementers; one `ExploreEvidence` shape or a `sections` map inside `Evidence` (additive field,
`#[serde(skip_serializing_if)]` like `reasoning`, so existing output stays byte-compatible); MCP tool
`nav_explore`; per-section budget allocation inside the 80 KB cap; golden transcript; skill rewrite ("orient with
one call"). Risks: payload design (the only new logic), tool-description tokens, and the temptation to add source
text. Depends on 1 (seed robustness) and 6 (ranking); absorbs 3 and 5.

**Cost to evaluate.** M. (a) Adoption: `eval/adoption` (deepeval, live Claude runs) already measures whether the
skill/tool is invoked on navigation prompts -- rerun with the composite present; threshold >= 50% of runs call it on
sites/flow/object questions. (b) No harm: 20 auto-eligible trial questions x Haiku (the one adopter) x 3
replicates, prism-with-explore vs prism, ~60 runs, ~$1 and ~1 h per arm (COMPARATIVE section 10 rates); threshold:
no paired F1 drop beyond -0.05 and >= 30% fewer Read/Grep calls per run. The trial harness (`runner.py`, `score.py`,
frozen gold v2) is reusable as-is.

## Gap 3 -- Shortest call path A->B

**Current state (source-verified).** `callers_with_confidence` / `callees_with_confidence`
(`src/navigation/queries.rs:2123-2211, 2223+`) are frontier BFS to `depth` (MCP 0..5) over
`NavigationIndex::direct_callers/direct_callees` (`src/navigation/mod.rs:299-315`); results are flat per-call-site
items whose only hop information is the score decay `confidence / (1 + hop)`. No target-terminated search, no path
reconstruction, no "exact-only path" option beyond the `confidence` filter. `taint_reaches` with sinks answers
*data* reachability with a witness graph (`src/mcp/tools_reasoning.rs`), not call reachability.

**Probe (measured).** click: callees from `main` (body seed core.py:1504) depth 1/2/3 = 26/98/188 items,
17.7/62.6/120.6 KB; `Command.invoke` appears at depth 1. The reverse question "who reaches `Context.invoke`?"
returns 5 callers at depth 2 and a warning that 421 same-name receiver sites were dropped -- including
`ctx.invoke(...)` at core.py:1367, the very edge a `main -> Context.invoke` path needs. grep finds it in one hit.
So a path tool on prism's graph would answer "no path" here while the path exists; that is the precision floor
working as documented, and it bounds the tool's recall on dynamic-language dispatch.

**Comparison (survey).** GitNexus `trace` (shortest path; 0 calls in 96 runs); codegraph `explore`'s "Flow (call
path among the symbols you queried)" is a path among the queried symbols; Sourcegraph/LSP call hierarchy is
one-hop; serena has no call hierarchy on the LSP backend.

**Value.** L-M (judgment). Flow questions were already 0.88-0.96 F1 without tools (COMPARATIVE section 1); the saving
is payload (a path is hundreds of bytes versus 120 KB of depth-3 callees) and a crisper review question ("is this
sink reachable from this entrypoint"). Exact-only paths are trustworthy; NameOnly paths through `r6_single_owner`
edges (click: 1197 of them) are guesses and must be labelled.

**Effort.** **S -- 1 PR, ~300-500 LOC.** `src/navigation/call_path.rs`: bidirectional BFS over the two indexes,
k <= 3 shortest simple paths, max length 8, per-path min confidence, `exact_only` default true; output as
`Evidence` items ordered by hop with `Reason::Calls` plus a `graph` payload; CLI `nav call-path --from --to` and
MCP `nav_call_path` (or a section of gap 2). No cache change.

**Cost to evaluate.** S. The trial flow golds (P2 R2 G2 J2 and the TS flow question) are gold call chains: check the
tool returns them where prism has the edges (expected >= 4/5), report bytes versus `callees --depth 3`, and count
paths that exist by grep but not by prism (the dispatch drops) on click/flask as the recall disclosure.

## Gap 4 -- Statement-level dependence query over MCP

**Current state (source-verified).**
- The CPG carries `CpgEdge::DataFlow(FlowConfidence)`, `ControlFlow`, `Call(ResolutionConfidence)`
  (`src/cpg/types.rs:197-211`); DataFlow labels are `Exact` or `NameOnly(Killed|SameLine|CfgIncomplete|
  AliasUnstable|CallNameOnly)` and are dumped by `dfg_edge_dump` (`src/navigation/queries.rs:32-60`,
  CLI `dfg-stats --edges`).
- The slicing algorithms (`src/algorithms/`: `left_flow`, `full_flow`, `thin_slice`, `chop`, `relevant_slice`,
  ...) are statement-level backward/forward dependence over a diff; `DiffInput::from_json` (`src/diff.rs:131-146`)
  accepts `{files:[{file_path, modify_type, diff_lines:[n]}]}`, so an agent can already backward-slice any single
  line through the CLI. The result is `SliceResult.blocks[].file_line_map` (line -> is_diff_line) with no per-line
  reason (`src/slice.rs:301-317`, `src/diff.rs:177-185`); findings carry evidence paths, plain slices do not.
- MCP: only `taint_reaches` (`src/mcp/tools_reasoning.rs:14-18`). `nav_ego_graph` accepts `DataFlow` and
  `ControlFlow` edge filters (`queries.rs:2346-2368`) but `nodes_at` drops `Statement` nodes
  (`queries.rs` nodes_at: "statements/synthetic return values are not first-class nodes-at evidence").

**Probe (measured).** click core.py:1360 (`.format(...)` inside `Command.invoke`): `nodes-at` -> one Variable +
enclosing function; `ego --hops 3 --edges DataFlow,ControlFlow,Contains` -> 26 nodes, 23 Contains + 4 DataFlow + 0
ControlFlow; `taint-reaches` frontier -> "No CFG statement node at src/click/core.py:1360; using pure-taint
fallback". Repo-wide on click: 5,288 of 12,781 labeled DataFlow edges are `cfg_incomplete` and 544 functions have
no CFG. So on this Python corpus the statement-level *control* side is thin; the data side exists and is labelled.

**Comparison (survey; trial).** GitNexus `pdg_query` (`analyze --pdg`): called once, file-scoped, truncated at 50
of 305 rows before the relevant lines (gitnexus.md section 6.3). codegraph, serena, LSP, Sourcegraph: none.
CodeQL/Joern are the oracles, not agent tools.

**Value.** M-H (judgment, with the measured caveat). Data-flow and guards were ~0.5 F1 in every arm and no model
called a tool on them (COMPARATIVE section 5.4) -- the unsolved class. DIRECTED injected "a backward slice for
guards" and saw +0.03 (guards), inside noise; and 4 questions (G3 J3 P3 P4) had bundle ceilings <= 0.35, meaning the
fixed query recipe did not find the sites at all. That is the decisive measurement to make first: does a proper
slice from the question's seed raise the ceiling? If yes, the tool has a verifier use even if cheap models ignore
it; if no, prism's data flow is not yet the missing piece on those questions.

**Effort.** **M -- 2 PRs, ~700-1,000 LOC.** PR-1: `nav slice --location file:line --direction backward|forward
--depth N --exact-only` on the CLI, built on `run_slicing` with a one-line `DiffInput`, returning `Evidence` items
per included line with `why` = DataFlow/ControlFlow/Call reasons and the DFG confidence (new reason variant,
additive), caps (lines and bytes). PR-2: MCP `slice_at` with the `taint_reaches` seed grammar, golden transcript,
skill text ("guards and data flow: slice, then read"). Risks: per-line reasons do not exist in `SliceResult` today
(needs an algorithm-side witness, likely in `left_flow`/`full_flow` only); payload on long functions; presenting a
`cfg_incomplete` edge as a dependence (must carry the label); `queries.rs`/`call_graph.rs` are over the ceiling, so
new modules only. No cache change if computed at query time from the cached CPG.

**Cost to evaluate.** S gate, then M. Gate (no model runs, hours): run the CLI backward slice from each DIRECTED
question's seed lines on G3 J3 P3 P4 and compute the gold-coverage ceiling the way `DIRECTED.md` did; proceed only
if the ceiling rises materially (judgment threshold: >= +0.3 on at least 2 of 4). Then the DIRECTED harness rerun
(12 questions x 4 models x 3 reps x 2 conditions = 288 runs, ~$6, ~5 h; the 2026-10-22 run is the control);
threshold: data+guards T2 delta >= +0.15 pooled (above the 0.04-0.18 per-cell SD). If only Sol moves, record it as a
verifier tool, not a cheap-model lever.

## Gap 5 -- Implementations / type hierarchy as first-class queries

**Current state (source-verified).**
- Go: interface satisfaction is computed and used for resolution -- `go_visible_s4_implementers`
  (`src/resolution.rs:1572`), `go_proven_interface_outcome` (`:1669`), `CallGraph::interface_method_names`
  (`src/call_graph.rs:910`); the CLI `interface-manifest` (`queries.rs:1280`, Go-gated) lists per-site
  `implementers`/`dispatch_route`/`receiver_class`. There is no "implementers of interface I" query; the manifest is
  per call site, CLI only, 145 KB on gin (428 sites).
- Rust: the method index keys trait methods under the trait (`MethodKind::Trait`, `src/receiver_index.rs:373-380`;
  CLAUDE.md: dual-keyed, demoted when > 1 impl; trait targets refused for `Self::` calls). No "impls of trait T"
  query; `trait_cha` is a NameOnly resolution kind (axum: 39 sites).
- JS/TS: `class_heritage` is consulted only as a guard against inherited accessors (`src/ast.rs:3137`); no
  inheritance graph. Python inheritance is a documented resolution gap (CLAUDE.md spec section 2.4). Java: 99% exact
  gap (internal lane notes (private)).
- Seeds are callables only; a type declaration line is `LocationOutOfRange`/no seed (DIRECTED.md "Limits": prism
  "cannot use a type declaration line as a seed ... and has no type-reference query").
- The skill routes "who implements X" to `lsp-nav` explicitly (`skills/prism-code-navigation/SKILL.md`, "Use a
  different tool when").

**Probe (measured).** axum `callers --symbol into_response` -> `AmbiguousSymbol`, 85 candidates;
`callers --symbol from_request_parts --file axum/src/extract/state.rs` -> 0 (generic dispatch is invisible). gin
`callers ServeHTTP` -> Exact callers through `return_typed`/`typed_param` recovery; `interface_fanout` histogram
shows 34 sites with one implementer and a tail to 19.

**Comparison (survey; trial).** serena `find_implementations` (LSP `textDocument/implementation`; 2 calls, and it
fails on Python because pyright lacks the request); codegraph: "synthesized dynamic-dispatch links" in explore;
GitNexus: none named; Sourcegraph/SCIP: `implementation` relationships are first-class in SCIP symbol information
(judgment on SCIP's model; not re-verified).

**Value.** M for Go and Rust, where prism already proves the facts without a toolchain and could label them
(Exact when structural satisfaction is proven, else candidate); L elsewhere, where building it means a new
per-language inheritance extraction for a question the host LSP answers when it is running. The census questions
(P5 R5 G5 J5) are type *references*, a broader query than hierarchy; G5's problem was output behaviour, not
finding (DIRECTED).

**Effort.** Go+Rust **M -- 1-2 PRs, ~500-800 LOC**: a type-declaration seed kind (`SymbolRef::Type` is new; today
only Function/Statement/Variable, `src/navigation/types.rs:19-47`), `nav_implementations` over the Go satisfier
memo and the Rust `impl Trait for T` facts in the method index / scope graph, with the existing confidence
labels. Python/JS/TS/Java: **L**, new extraction and a cache bump each -- not proposed.

**Cost to evaluate.** S for Go/Rust: Tier-A's LSP clients (`eval/tier_a/lsp_client.py`, `oracles.py`) talk to
gopls and rust-analyzer but do not request `textDocument/implementation` today (grep: no hit) -- adding the request
is small; threshold: precision >= 0.95 and recall reported, on two Go and two Rust anchors.

## Gap 6 -- Ranked repo map under a token budget

**Current state (source-verified).**
- `repo_map` (`src/navigation/module_graph.rs:305-384`) emits one `GraphNode` per indexed file in `BTreeSet` path
  order plus distinct call-derived `ModuleDep` edges (Rust also gets scope-graph import edges); `items` is empty;
  no symbols, no weights, no ranking, no budget.
- MCP `nav_repo_map` takes only `max_results` (default 50, cap 1000) and `clip_graph` truncates to the *first N
  nodes* and drops edges touching the rest (`src/mcp/tools.rs:288-299`), so the default view of any repo over 50
  files is its alphabetically-first files (click: `docs/conf.py`, `examples/...` first). The byte cap is 80 KB.
- `onboard` (`src/navigation/onboarding.rs:9, 57-143`) ranks the top-12 modules by total degree and prints
  inventory + call-resolution counters in ~1.5 KB Markdown -- CLI only, not an MCP tool; the v1 design put session
  memory out of scope and did not discuss a token budget (`docs/superpowers/specs/2026-09-04-nav-onboarding-report-
  v1-design.md:25, 48`).
- Per-function caller counts are available from `direct_callers` and file role from `code_context.rs`.

**Probe (measured).** axum: 302 nodes / 804 edges, 140 KB JSON (~35k tokens) in 1.2 s; the MCP default would show
50 of 302 files in path order. click: 63 nodes / 224 edges, 33 KB. `onboard` on axum: 1.6 KB with
`axum/src/routing/mod.rs` (125 dependents) at the top -- the useful orientation signal, in the wrong surface.

**Comparison (survey; archived research note).** aider: tree-sitter def/ref tags -> file graph -> personalized
PageRank (chat files x50, mentioned identifiers x10, private x0.1) -> binary-search the definitions that fit a
1k-token budget, rebuilt per call; vendor-reported accuracy gain, not independently ablated. codegraph
`files`/`status` and GitNexus `tool_map` are not ranked maps. Serena `get_symbols_overview` is per file.

**Value.** M-H (judgment). The skill's first step is "`nav_repo_map` once for layout"; today that step on a
real repo returns an alphabetical slice, and the agent either raises `max_results` (35k tokens on axum) or skips
it. A degree- or PageRank-ranked map with the top symbols per file under a 2-4k-token default budget is what the
other tools' users get from aider and is also gap 2's orientation section. Cheap models explore less when given
large context (DIRECTED, Haiku), so the default budget must be small and the ranking must be good. This does not
change answer accuracy on the trial's question types (orientation was not measured); it fixes a defect in the one
query every session is told to make.

**Effort.** **S-M -- 1-2 PRs, ~500-800 LOC.** `src/navigation/repo_rank.rs`: PageRank (or in-degree with a
production-over-test prior) over `collect_module_edges` (module_graph.rs:82-109) plus per-file top-k functions by
caller count; new optional `budget_tokens`/`ranked` inputs on `nav_repo_map` (additive; default output
byte-identical unless requested, or make ranked the default and bless the golden with a stated reason); promote
`onboard` to `nav_onboard` reusing `build_report`. No cache change (axum's 804 edges rank in microseconds).
Docs/skill update. Risk: a `GraphNode` has no symbol list -- the ranked map needs an additive field or a new
`items` projection; keep `graph` for compatibility.

**Cost to evaluate.** S, deterministic, no model runs: for the 24 trial questions (5 corpora) and the 11 Part-D
tasks, measure the share of gold-cited files present in the ranked map at 2k and 4k tokens versus today's first-50
slice; threshold: >= 2x the current hit rate at <= 4k tokens. Later, gap 2's adoption measurement covers it.

## Gap 7 -- Framework route maps

**Current state (source-verified).**
- `framework_entry` nav-only caller edges exist for Flask, FastAPI and Express (PR #160, P9;
  `src/framework_entries/mod.rs:1-20, 82-87`; `python.rs:12-22`; `express.rs:1-50`). The record is
  `{caller, handler, site, framework}` -- no HTTP method, no path string, no middleware chain; Express handlers
  must be bare identifiers (`ExpressHandlerArg::Identifier`); member-expression handlers (`site.index`) and inline
  functions are never recorded. Koa/Fastify/NestJS/Django/DRF/Gin/gorilla/net.http have no entry edges.
- `src/frameworks/` has 11 `FrameworkSpec`s with per-file detection (`ALL_FRAMEWORKS`, `src/frameworks/mod.rs:
  166-178`) for taint sources/sinks/sanitizers; `RouteDecoratorSite` carries only line/bytes (`:160-164`).

**Probe (measured).** flask: `callers index@blog.py` -> one `<module>` caller at the decorator line, score 0.6,
`framework_entry`; 248 entries recorded repo-wide (grep of route decorators: 312 incl. docs/tests). express repo:
0 entries recorded (its examples use `site.index` and inline functions). gin: 0 (no Go support). fastapi source
tree: 27 decorator registrations by grep.

**Comparison (survey).** GitNexus `route_map`/`api_impact` (0 calls in the trial); none of serena, codegraph,
LSP, Sourcegraph. Framework-aware route listing is otherwise a CodeQL/Semgrep-rule or IDE-plugin feature
(judgment).

**Value.** M for web-application review and "where do I change endpoint X" (judgment); L for the corpora prism is
measured on (CLI tools, libraries, a React frontend). A route table is also 80% obtainable by one grep per
framework; prism's increment is handler resolution across files, method/path extraction, and linking entries to
taint sources -- the last is the differentiated part and the entry edges already feed `nav_callers`. No prism
measurement covers routes; the trial has no endpoint questions.

**Effort.** **M -- 2-3 PRs, ~800-1,200 LOC.** Path/method literal extraction per framework (Python decorators and
Express calls: easy; Gin/gorilla/net.http: medium; Django `urls.py` with dotted view strings: hard, likely
NameOnly); extend `FrameworkEntryRecord` (serialized in `CallGraph` -> `CACHE_VERSION` bump, one transition per PR);
member-expression handler resolution for Express; `nav_routes` tool + CLI; docs. Dependencies: none.

**Cost to evaluate.** S for correctness (route recall/precision against a grep-derived table on flask, fastapi,
express, gin; thresholds P >= 0.95, R >= 0.8). Value evaluation has no task set; it would need a web-app review
corpus first -- which is the real reason to defer.

## Gap 8 -- SCIP export/import

**Current state (source-verified).**
- No SCIP/LSIF reader or writer (grep over `src/`, `Cargo.toml`: nothing).
- `Source::ExternalIndex { name }` is a reserved evidence source (`src/navigation/types.rs:50-55`), rendered as
  `external_index` in views (`evidence_view.rs:1464`); `--resolution precise|auto` are "deferred to roadmap item 3
  because they require an external index" (`src/cli.rs:108-116`); the evidence-shaping design lists "external index
  provenance, such as SCIP/LSIF/Glean" as future (`docs/features/query-layer/evidence/llm-evidence-shaping-design-
  2026-06-23.md:595`).
- "Prism checked against an oracle" already exists: Tier-A compares callers/callees with rust-analyzer, gopls,
  pyright and tsserver per corpus with adjudicated verdicts (`eval/tier_a/lsp_client.py`, `oracles.py`,
  `tsserver.py`; `docs/eval/tier-a/baseline.md`), including measured oracle self-error of 0-25% and one
  oracle-invalid corpus (tokio).

**Comparison (survey; judgment).** SCIP is Sourcegraph's index format (protobuf; symbols, occurrences with roles,
relationships incl. implementation); indexers exist for TS/JS, Python, Java, Go, Rust (rust-analyzer), C/C++. SCIP
has no confidence field: an exported prism index would have to drop NameOnly edges or encode them out of band, and
SCIP occurrences are references, not call edges, so import needs an intersection with prism's call sites to become
edges (judgment on the format; not re-verified against the spec).

**Value.** L now. Export has no consumer in prism's user base (the MCP/CLI agents consume `Evidence`); import
would give compiler-grade edges for Java/C++ but requires users to run a toolchain-backed indexer, which is the
thing prism's no-toolchain posture avoids, and the roadmap already frames that option as "LSP-delegation hybrid for
immature languages" to be decided after Java measurement (`docs/analysis/prism-post-plan-roadmap.md` section 4).

**Effort.** L: protobuf dependency and schema, symbol-ID <-> `FunctionId` mapping, occurrence->call-site
intersection, a second evidence source through every view and cache path, 3+ PRs, ~1,500+ LOC (judgment).

**Cost to evaluate.** M (agreement rates per corpus, which Tier-A already produces against LSP). Cheapest useful
variant if a Java/C++ oracle is wanted: a SCIP reader inside `eval/tier_a/oracles.py` (~300 LOC Python, no product
change), which also answers whether SCIP and LSP agree before any import work.

---

## What could not be verified

- Current feature sets of serena, codegraph, GitNexus, aider and Sourcegraph beyond the 2026-10-18 survey and the
  trial write-ups (no web fetch in this session).
- Whether `SliceResult` can carry per-line reasons without algorithm changes (read as absent in `src/slice.rs:
  301-317`; the algorithm internals were not read).
- Why `callees --location src/click/core.py:1430` returned 0 items with no error (a body line at 1504 worked);
  not investigated, logged in `PROBES.md`.
- The MCP-truncated repo-map order was inferred from `clip_graph` + `BTreeSet` order, not observed through a live
  `prism-mcp` session.
- Tier-A oracle support for `textDocument/implementation`: grep found no request; the client code was not read in
  full.
- All LOC and PR counts are judgment; the file-size rule (new modules only for `queries.rs`, `call_graph.rs`,
  `tools.rs` growth) was applied but not sized against the weighted-line script for the new files.

## Reproduce

Commands and key numbers: `PROBES.md` in this directory; raw outputs are in the private evidence store (not committed), and `PROBES.md`
lists each with its command and key numbers. Prism evidence read (private store): `tracing-trial/
{TOOLS-SURVEY.md,DIRECTED.md,writeups/*.md,questions.jsonl}`, `review-routing/ANALYSIS.md`,
`survey/synthesis.md` (public copy: `docs/eval/jsts-workstream-value/surveys/synthesis.md`); repo docs `docs/analysis/2026-07-05-tier-c-partc-fair-scorecard.md`,
`docs/analysis/2026-08-21-tier-c-partd-readout.md`, `docs/analysis/prism-post-plan-roadmap.md`,
`docs/archive/analysis/prism-query-layer/research-llm-codebase-navigation.md`, `docs/MCP.md`,
`skills/prism-code-navigation/SKILL.md`, `CLAUDE.md`.
