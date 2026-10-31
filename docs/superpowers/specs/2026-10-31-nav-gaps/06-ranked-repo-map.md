# Ranked repo map under a token budget (gap item 6)

Date: 2026-10-31
Status: draft for owner review
Base: main `533f061a`
Companion spec: `01-find-symbol.md` (item 1). Item 2 (composite explore) composes both; section 5.9 names what it calls.
Cross-spec decisions, tool budget, seed grammar and build order: `00-README.md` (this spec's PRs are R1 = S1 and R2 = S2; R1 depends on the shared catalog PR N0).

**Claim tags.** *measured* = a number from a prism run or an evidence file, cited; *source-verified* = read in the code
at the cited `file:line` on `533f061a`; *judgment* = my estimate or design choice. Pilot scripts, inputs list and recorded outputs:
[`spec-pilots/pilot-06-01/`](../../../analysis/2026-10-31-nav-gaps/spec-pilots/pilot-06-01/) (`RUN.txt` there says how to regenerate the large inputs; `RESULTS.md`
holds each `*.out` section and a crosswalk from the tables below; the focused-ranking rows need the private evidence store).

---

## 1. Goal and the decision it serves

**Goal.** Make the one orientation call every agent session is told to make (`nav_repo_map` with no arguments,
`skills/prism-code-navigation/SKILL.md`, "Orient before you read", step 1) return a useful map: files ranked by how
central they are in the call/import graph, each with its most-called symbols and their spans, packed into a small
token budget, optionally personalised to the files or symbols the agent already knows it cares about.

**Decisions this serves.**

1. *Whether the no-argument `nav_repo_map` default changes* from "first N files in path order" to the ranked map
   (decided by the no-harm gate D1, section 9.3).
2. *Whether `focus` ships and item 2 builds its orientation section on it* (decided by gate D2 on large repositories,
   section 9.3). If D2 fails, item 2 uses callers/callees neighbourhoods for orientation instead of the map.

This is an orientation fix. It is not expected to change answer accuracy on the tracing-trial question types: the trial
and the directed-injection run both found output behaviour, not evidence, to be the bottleneck (gap analysis,
"measurement context"). The value claim is narrower: the default view stops being an alphabetical slice, and the map
fits a budget that cheap models tolerate.

## 2. Non-goals

- No source text in the map (no elided code lines as aider renders). The composite-explore lessons (codegraph
  `explore` replaced reading and cost recall) apply; spans let the agent read what it chooses.
- No git recency signal in v1 (owner question 4).
- No new cache, sidecar or schema version; everything is computed at query time.
- No session memory, no "files in chat" state: the MCP server is stateless per call; the caller passes `focus`.
- No change to CLI `repo-map` default output or to `onboard` output (both keep their byte contracts).
- No fuzzy symbol matching inside `focus`; fuzzy search is item 1.

## 3. Current state

- `repo_map` emits one `GraphNode` per indexed file in `BTreeSet` path order plus distinct `ModuleDep` edges; `items`
  is empty; nodes carry no symbols, weights or ranks (source-verified: `src/navigation/module_graph.rs:305-384`, node
  order from `files: BTreeSet<&String>` at `:326-340`). Edges come from `collect_module_edges` (`:87-109`), which
  keeps per-edge call reasons with `max_score` (Exact 1.0 / NameOnly 0.6) but no per-edge count, plus Rust scope-graph
  import edges (`:310-323`).
- MCP `nav_repo_map` takes `max_results` (default 50, cap 1000) and view options only (source-verified:
  `src/mcp/input.rs:349-375`, schema `src/mcp/tools.rs` `repo_map_schema`). `clip_graph` keeps the first N nodes and
  drops edges touching the rest (source-verified: `src/mcp/tools.rs:288-299`). Agent views take the first
  `item_limit` nodes of the same order (source-verified: `src/mcp/evidence_view.rs:544-546`). So every view of a
  repository with more than 50 files shows its alphabetically first 50 files.
- CLI `onboard` ranks the top 12 modules by total degree (dependencies + dependents), path ascending on ties
  (source-verified: `src/navigation/onboarding.rs:9, 143-196`). It is CLI-only (source-verified: `src/mcp/tools.rs:25-75`
  registers seven nav tools; `onboard` is not one).
- Per-function inputs exist: incoming resolved calls per `FunctionId` (`NavigationIndex::direct_callers`,
  `src/navigation/mod.rs:299`), method owners (`CallGraph::method_owners`, `src/call_graph.rs`, serialized in the CPG
  cache), file role (`code_context::classify_file`, `src/navigation/code_context.rs:17`).
- Sizes (measured, pilot): httpx 60 files / 197 edges, cobra 36 / 141, ripgrep 102 / 358, excalidraw 628 / 2,973;
  axum 302 / 804 and 140 KB JSON (gap-analysis probe). Today's MCP default (first 50 nodes + their edges, canonical
  JSON) is 9.4-13.0 KB on the four pilot corpora, about 2.3-3.2k tokens at 4 bytes/token (measured, the `budget.out` section of [`RESULTS.md`](../../../analysis/2026-10-31-nav-gaps/spec-pilots/pilot-06-01/RESULTS.md)).

**What changes:** a ranking module, a budget packer and a compact rendering; `nav_repo_map` gains a ranked mode that
becomes the no-argument default; the CLI gains `repo-map --ranked`.

## 4. Shared MCP-surface decision (items 6 and 1)

The same text appears in both specs.

> **Rule: extend an existing tool when the question is the same; add one tool only for a new question; keep one seed
> grammar.** Item 6 answers the question `nav_repo_map` already answers ("where am I, what matters here?"), so it
> extends `nav_repo_map` and adds no tool. Item 1 answers a new question ("what is named like X?") that needs an input
> no tool takes (a name pattern), so it adds exactly one tool, `nav_find_symbol`, and puts the seed-friction fixes
> (qualified names, disambiguation hints, a `name` field on location seeds) into the existing seeded tools rather than
> into new ones. `onboard` is not promoted to its own tool; its counts move into the ranked map's header.
>
> *Reconciled (00-README X1, X2):* across all four specs the default tool list grows by one tool, item 2's
> `nav_explore` (9 to 10). Item 1's search reaches MCP as `nav_explore`'s `query` seed; the standalone
> `nav_find_symbol` is contingent (F2). Budget: the `tools/list` payload grows from 13,540 JSON chars (measured from
> `tests/fixtures/mcp/golden/*.jsonl` by the gap analysis) to at most 17,500 chars for the whole program, enforced
> by a test that F1b adds; item 6's share is at most +900 chars (new `nav_repo_map` properties and description),
> item 1's +150 (the seed `name` property on six inlined seed copies), `nav_explore` +2,750.
>
> Alternative considered: fold find-symbol into `nav_repo_map` as a `query` parameter (tool count stays 9). Rejected
> on judgment: one tool answering two different questions makes the description longer and tool choice worse, and the
> trial's adoption signal favoured a tool whose name says what it does. Owner question 1 in item 1's spec.

## 5. Design

### 5.1 Interface

**CLI** (new flags on the existing subcommand; default output unchanged):

```text
prism nav [--no-cache | --cache-dir DIR] repo-map --repo PATH --ranked
    [--budget-tokens N]          # default 2000, 256..20000
    [--focus-file PATH]...       # repeatable, at most 20
    [--focus-symbol NAME]...     # repeatable, at most 20; exact or qualified name (5.3)
    [--symbols-per-file K]       # default 3, 0..10
    [--format text|json]         # text = the markdown map; json = RankedRepoMap
```

Without `--ranked`, `repo-map` is byte-identical to today. `--ranked` with no other flag gives the unfocused map.

**MCP** `nav_repo_map`, new optional properties:

| Property | Type | Default | Limits | Meaning |
|---|---|---|---|---|
| `mode` | `"ranked" \| "graph"` | inferred (below) | | `graph` = today's behaviour |
| `budget_tokens` | integer | 2000 | 256..20000 | budget for the content text (5.5) |
| `focus` | object `{files?: string[], symbols?: string[]}` | none | each list <= 20, strings <= 512 bytes | personalisation (5.3) |
| `symbols_per_file` | integer | 3 | 0..10 | symbol lines per file entry |
| `format` | existing enum | `agent_markdown` in ranked mode | | `canonical_json` and `agent_json` both render the result as compact JSON |

**Mode inference** (keeps every explicit existing call byte-identical): an explicit `mode` wins; otherwise, if any of
the existing graph/view properties (`max_results`, `profile`, `snippets`, `group_by`, `max_view_bytes`) is present the
mode is `graph`; otherwise it is `ranked`. Ranked-only properties combined with `mode: "graph"`, or graph-only
properties combined with `mode: "ranked"`, are `bad arguments` naming the conflicting property. So only the
no-argument call changes, and that call is the defect.

**Result.** A dedicated result, like `nav_symbol_spans` (`SymbolSpans`) and CLI `onboard` (`ProjectOverview`), not
`Evidence`: the map's value is one budgeted text block, and `Evidence` has no place for per-file symbol lists without
new reason variants that every other view would then have to handle (judgment).

```rust
// src/navigation/repo_rank.rs (new)
pub struct RankedRepoMap {
    pub schema_version: &'static str,          // "1.0"
    pub query: String,                         // "repo-map:ranked"
    pub project: ProjectHeader,                // files, skipped, functions, languages, call counts (from onboard)
    pub ranking: RankingInfo,                  // formula id "ppr-sym-v1", damping, focus_applied: bool
    pub focus: Option<ResolvedFocus>,          // files used, symbols -> files, unresolved inputs
    pub budget: BudgetInfo,                    // requested, used (estimated), estimator "bytes_div_3"
    pub files: Vec<RankedFile>,                // shown files, rank order
    pub omitted_files: usize,                  // indexed files not shown
    pub warnings: Vec<Warning>,                // existing Warning type
}
pub struct RankedFile {
    pub rank: usize,                           // 1-based
    pub file: String,
    pub score: f64,                            // normalised so rank 1 = 1.0, rounded to 4 decimals on output
    pub role: CodeRole,                        // production | test
    pub language: String,                      // onboarding label set
    pub in_degree: usize,                      // distinct dependent files
    pub out_degree: usize,                     // distinct dependency files
    pub focus: bool,                           // pinned by focus
    pub entry: bool,                           // contains an entry point (5.4); display only in v1
    pub symbols: Vec<MapSymbol>,
    pub symbols_omitted: usize,
}
pub struct MapSymbol {
    pub symbol: SymbolRef,                     // existing Function variant: file, name, lines, bytes
    pub owner: Option<String>,                 // CallGraph::method_owners
    pub callers_exact: usize,                  // direct incoming edges, Exact
    pub callers_name_only: usize,              // direct incoming edges, NameOnly
}
```

`structuredContent` is the `RankedRepoMap` JSON. `content_text` is the rendering selected by `format`: the markdown
map (default) or compact single-line JSON. The budget governs `content_text` (5.5). Defaulting to markdown departs
from the "agent views are opt-in" notice (`VIEW_NOTICE`, `src/mcp/tools.rs:22`), which is about `Evidence` views; the
ranked map is not `Evidence`, its canonical form is always in `structuredContent`, and the notice text gets one
clause saying so (judgment: a no-argument call has to return the budgeted text, or the default stays unusable).
*Reconciled (00-README X7):* the clause is generic ("tools that return a dedicated result default to agent_markdown;
their structuredContent is that result"), so it also covers `nav_explore`, which uses the same `format` vocabulary.
`VIEW_NOTICE` is composed into `initialize`'s `instructions` (`src/mcp/transport.rs:513`), so S2 also re-blesses
golden id 1.

**Markdown rendering** (fixed order; one line per file, one indented line for its symbols):

```text
# Repo map: axum (302 files, 2114 functions) - ranked by call/import centrality, tests x0.1
Calls: 2385 exact, 994 name-only, 5952 unresolved. Budget 2000 tokens (est.), used 1946.
1. axum/src/routing/mod.rs  in 125, out 17
   Router::route L210-240 (40 exact, 12 name-only) | Router::new L120-131 (31, 0) | Router::merge L300-322 (9, 2)
2. axum/src/test_helpers/test_client.rs  in 73, out 3  [test]
   ...
-- 251 more files not shown; raise budget_tokens or pass focus.
```

A symbol line gives everything needed to build the item 1 seed handle (`{kind:"loc", file, line, name}`), so the
map's symbols are directly usable as seeds for `nav_callers`/`nav_callees`.

**Errors.** Bad arguments (out-of-range integers, more than 20 focus entries, wrong types, mode conflicts) use the
existing `bad arguments` result. Unknown focus files or unresolvable focus symbols are warnings, never errors; the map
falls back to the unfocused ranking for the unresolved part. An index with zero files returns an empty `files` list
and the header.

### 5.2 Ranking semantics

One formula for both modes, a personalised PageRank on a weighted, symmetrised file graph (id `ppr-sym-v1`):

1. **Nodes:** every indexed file (`s.repo.files`, the same node set as `repo_map`).
2. **Forward weights:** for each ordered pair `(a, b)`, `a != b`, `w(a,b)` = sum over resolved cross-file call sites
   from `a` to `b` of 1.0 (Exact) or 0.6 (NameOnly), plus 1.0 if a Rust scope-graph import edge `a -> b` exists.
   This needs one change in `collect_module_edges`: keep the summed weight beside `max_score` (additive field;
   `repo_map`/`module_deps` output unchanged).
3. **Symmetrise:** `W(a,b) = w(a,b) + 0.5 * w(b,a)`. A file is central both when many files depend on it and, more
   weakly, when it depends on many (the pilot's degree ranking used both directions and was the best unfocused order).
4. **Personalisation** `p(f)`: 1.0 for production files, 0.1 for test files (`classify_file(f).is_test`). With focus:
   multiply by 50 for a focus file, by 10 for a file that defines a focus symbol (aider's chat-file and
   mentioned-identifier multipliers). `p` is normalised to sum 1.
5. **Iteration:** `r0 = p`; 100 fixed iterations of `r' = (1-d) p + d (sum_j r_j W(j,.)/out_W(j) + dangling * p)`,
   `d = 0.85`, dangling mass (files with no outgoing weight) redistributed by `p`. Fixed count, no early stop.
6. **Order:** score descending (`f64::total_cmp`), then production before test, then path ascending. Focus files are
   pinned first, in the order given, before the ranked list.

The pilot ran this formula (Python, file-level) and found it equal to total degree for the unfocused case
(section 9.2). It is chosen over plain degree because the same computation supports focus; degree cannot be
personalised without a second mechanism (judgment).

### 5.3 Focus resolution

- `focus.files`: exact repository-relative paths (same normalisation as `nav_module_deps` `file`). Not indexed ->
  warning `SkippedPath` with the skip reason when known (same text as `module_deps`, `module_graph.rs:132-160`).
- `focus.symbols`: each string is resolved exactly, never fuzzily: (a) a callable whose name equals the string; else
  (b) a qualified spelling `Owner.m`, `Owner::m`, `(*Owner).m`, `module.f` split as in item 1 section 5.3 and matched
  on owner key, trait key or file stem. The defining files of all matches get the x10 multiplier. More than 20
  defining files -> the symbol is ignored with a warning ("too common; pass a file or a qualified name"). No match ->
  warning; item 1's `nav_find_symbol` is the tool for partial names.
- Duplicates are removed preserving first occurrence. The resolved focus is echoed in `ResolvedFocus`.

### 5.4 What each file entry shows

- Path, role, `in_degree`/`out_degree` (distinct files over the merged `ModuleDep` edge set, exactly the counts
  `onboard` prints), `[test]`, `[focus]`, `[entry]` flags.
- Up to `symbols_per_file` symbols, chosen by: named in focus first; then entry symbols; then
  `5 * callers_exact_from_other_files + 3 * callers_name_only_from_other_files` descending; then total callers
  descending; then start line ascending. Anonymous callables (`anon_owner::is_anonymous_owner_name`) and module
  owners (`is_module_owner_name`) are never listed. Owner shown as `Owner::name` (Rust) or `Owner.name` (others).
- **Entry points** (display flag only in v1): Rust `fn main` in `src/main.rs` or `src/bin/*.rs`; Go `func main` in a
  file whose package clause is `main`; Python files named `__main__.py` or containing a module-level
  `if __name__ == "__main__":`; files holding the handler of a recorded `framework_entry` record. JS/TS entry points
  need `package.json`, which is not indexed: not detected in v1. Whether entries should also boost the ranking is an
  ablation in the eval (section 9.4), not part of `ppr-sym-v1`.

### 5.5 Budget packing and token estimate

- **Estimator:** `est(s) = ceil(utf8_bytes(s) / 3)`. Deliberately pessimistic for path-heavy text (the pilot used
  3.5 bytes/token; code tokenisers vary). The eval checks it against a reference tokenizer (section 9.3, D1c).
- **Packing:** render the header and footer first and reserve their cost. Walk files in rank order; for each, try
  the entry with `k = symbols_per_file` symbol entries, then `k-1`, down to the bare file line; take the first that
  fits. Stop at the first file whose bare line does not fit. The shown files are therefore always a prefix of the
  ranking (testable, and what an agent expects from "top N").
- `budget.used` is the estimate of the final `content_text`; `omitted_files` and the footer say how many were cut.
- In `canonical_json` format the cost of each entry is its compact-JSON bytes, so fewer files fit (judgment: about
  2-3x fewer); the markdown default is what the budget is designed for.
- The 80 KB wire cap still applies (`shape_structured_value`, `src/mcp/output.rs:256-273`). At the 20,000-token
  maximum, content text is at most about 60 KB, and `structuredContent` duplicates it; if the wire cap would be
  exceeded the packer is rerun with the budget halved until it fits, and a `ResultTruncated` warning records it.

### 5.6 Determinism

- Inputs are `BTreeMap`/`BTreeSet`-ordered (`collect_module_edges`, `s.repo.files`); the PageRank loop iterates nodes
  in path order with a fixed iteration count and fixed summation order, so scores are bit-identical for the same index
  (judgment, verified by the double-build and thread-count tests in section 8).
- Ties: section 5.2 step 6; symbols: section 5.4. Output rounding happens only at serialisation.
- Nothing depends on wall time, git, environment or hash iteration order.

### 5.7 Confidence handling and language coverage

- NameOnly edges count at 0.6, matching the score convention used everywhere else (`confidence_score`,
  `module_graph.rs:29-34`). Dropped sites (multi-owner collisions, unknown names) contribute nothing; the header shows
  the resolution counters so the reader knows how sparse the graph is. Caller counts are split Exact/NameOnly.
- Coverage follows the call graph: languages with sparse resolution (C/C++ incomplete, Java exact gap, JS/TS high
  unresolved share; internal lane notes (private), per-language maturity) get flatter rankings that degrade toward the test prior plus
  path order. This is disclosed in the header counters, not hidden. Isolated files (97 of 302 on axum, measured,
  the `axum-onboard.md` row of [PROBES.md](../../../analysis/2026-10-31-nav-gaps/gap-analysis/PROBES.md)) rank by personalisation only.

### 5.8 `onboard`: promote versus extend

Extend `nav_repo_map`; do not add `nav_onboard`. `onboard`'s facts are counts (inventory, languages, call
resolution) plus a top-12 degree list; the ranked map header carries the counts (one or two lines, ~60 tokens) and the
ranked list supersedes the top-12. A separate tool would cost tool-list tokens on every run for content that is a
subset of the map (judgment, shared rule in section 4). CLI `onboard` stays byte-identical; switching its module list to
the new ranking is a possible follow-up that changes its output contract and needs its own decision.

### 5.9 Reusable API for item 2

Item 2 calls these, not the MCP handler:

```rust
pub struct FocusSpec { pub files: Vec<String>, pub symbols: Vec<String> }
pub struct FileRanking { pub entries: Vec<RankedFileCore>, pub focus: Option<ResolvedFocus>, pub warnings: Vec<Warning> }
pub fn rank_files(s: &NavigationSession, focus: &FocusSpec) -> FileRanking;       // full ranking, no budget
pub struct PackOptions { pub budget_tokens: usize, pub symbols_per_file: usize, pub format: MapFormat }
pub fn pack_ranked_map(s: &NavigationSession, ranking: &FileRanking, opts: &PackOptions) -> RankedRepoMap;
pub fn render_ranked_map(map: &RankedRepoMap, format: MapFormat) -> String;
pub fn estimate_tokens(text: &str) -> usize;                                       // shared estimator
pub struct RankedFileCore { pub rank: usize, pub file: String, pub score: f64, pub role: CodeRole,
    pub language: String, pub in_degree: usize, pub out_degree: usize, pub focus: bool, pub entry: bool }
                                                         // = RankedFile without the symbol list
```

Item 2's `module` section calls `rank_files` with an empty `FocusSpec` and reads the seed file's `RankedFileCore`
(00-README section 4); it does not use `focus`, so gate D2 does not affect it.

The per-symbol facts (owner, caller counts, role, language) come from `navigation::symbol_catalog::SymbolCatalog`,
shared with item 1. *Reconciled (00-README X5):* the catalog is defined by item 1 (`01-find-symbol.md` section 5.10)
and built in its own PR, N0, before S1; it carries the cross-file caller counts section 5.4 ranks by.

## 6. Comparison with aider's repo map (ideas only)

What aider does (survey of 2026-10-18 and the archived research note; not re-verified): tree-sitter def/ref tags per
file; a file graph whose edges are identifier references, weighted by reference count with multipliers (identifiers
mentioned in chat x10, files in chat x50, private names x0.1, very common identifiers down-weighted); personalised
PageRank; rank spread to definitions; a binary search on the number of definitions that fits a token budget (default
about 1k tokens, larger when no files are in chat); rendering of the definitions' lines with surrounding structure;
recomputed each turn from chat state.

| Idea | Prism v1 | Why |
|---|---|---|
| Personalised PageRank, chat x50 / mention x10 | adopt as `focus` | same purpose; caller supplies focus explicitly |
| Private-name x0.1 | replace with test-file x0.1 | prism already classifies file role; underscore-private is language-specific |
| Edges from name references | use resolved call edges with confidence | more precise; sparser where resolution drops sites (disclosed) |
| Down-weight identifiers defined in many files | not needed | prism already drops multi-owner collisions |
| Budget by binary search over definitions | greedy prefix packing per file | simpler, prefix property is testable; same effect |
| Render source lines of definitions | no; spans only | map, not a reading substitute (codegraph lesson) |
| Chat-state coupling | no | stateless MCP |

## 7. Data, cache and performance

- **Cache:** none. No `CACHE_VERSION` or call-edge sidecar change. Inputs are already in the cached CPG, call graph
  (`method_owners`) and nav call-edge index. Existing outputs stay byte-compatible except the no-argument MCP call.
- **Cost:** `collect_module_edges` is already computed by `repo_map`; PageRank is `100 * |E|` multiply-adds (axum 804
  edges, excalidraw 2,973; a kubernetes-size repository is judged at under 10^6 edges). Symbol facts are one pass over
  function nodes and incoming edges.
- **Budget (pre-registered):** warm-index ranked map p95 <= 50 ms on repositories up to 1,000 files and <= 500 ms on
  the largest stress corpus in `docs/analysis` profiles (kubernetes), excluding index load; measured in slice S1 with
  `PRISM_PROFILE`. Memory: no persistent structure beyond the shared `SymbolCatalog` (item 1 section 7 bounds it).

## 8. Test plan (test-first variant fixtures)

Enumerate the input classes as fixtures before writing the ranking (internal lane notes (private): the test-first variant matrix rule):

- **Graph shapes:** star hub; chain; cycle; two disconnected components; only isolated files; self-file calls
  (excluded); hub that is a test file (test prior); Rust scope-graph import-only edge (weight 1.0); NameOnly-only
  edges (0.6); mixed languages; files with zero functions.
- **Focus:** existing file; unknown file (warning, unfocused fallback); focus on a test file; symbol unique; symbol in
  2-20 files; symbol in more than 20 files (ignored, warning); unknown symbol; qualified spellings (`Owner.m`,
  `Owner::m`, `(*Owner).m`, `module.f`); duplicate entries; 21 entries (bad arguments).
- **Budget:** whole repository fits; first file fits only bare; exact boundary (entry cost equals remaining budget);
  256 minimum; 20,000 maximum against the wire cap (halving path); multibyte paths (bytes, not chars); very long
  identifiers (clamped by `output::clamp_user_text`); `symbols_per_file` 0.
- **Symbols:** anonymous callables and `<module>` never listed; owner rendering per language; entry flag per rule in
  5.4 (positive and negative case per language).
- **Determinism:** two independent builds byte-identical; run at `RAYON_NUM_THREADS=1` and `4` (internal lane notes (private): check CI
  before review); file insertion order permuted.
- **Compatibility:** `mode: "graph"` and every legacy-property call byte-identical to today's output (assert against
  the pre-change rendering); CLI `repo-map` without `--ranked` unchanged; `onboard` unchanged.
- **Prefix property:** for every fixture, shown files equal the first `len(files)` of `rank_files`.
- **Golden transcripts:** `tests/fixtures/mcp/golden/*.jsonl` change in two places, re-blessed in S2 with the reason
  in the commit: id 3 (`tools/list`: new `nav_repo_map` properties and description) and id 4 (`nav_repo_map {}` now
  returns the ranked map). Add one transcript line for a focused, budgeted call. *Reconciled (00-README section 5):*
  also id 1 (the `VIEW_NOTICE` clause); new steps use transcript ids 35-39; S2 merges after F1b and before E-4.
- **Negative:** budget 0 or 20,001; `focus` not an object; `symbols_per_file` 11; mode conflicts.

## 9. Evaluation

### 9.1 Metric and sets

**Metric:** gold-file hit rate = |gold files among the shown files| / |gold files|, per question, macro mean, at
budgets of 1,000 / 2,000 / 4,000 estimated tokens. Deterministic, no model runs.

- **Set A (orientation, small and mid-size repositories):** the 24 tracing-trial questions (private evidence store) on 5 corpora (httpx,
  ripgrep, cobra, excalidraw, secbench-node-r1) at the trial pins; gold = repository-relative paths cited as
  `path:line` in gold v2, filtered to indexed files. Focused runs derive focus mechanically from the question text:
  backticked identifiers of 3+ characters -> `focus.symbols`; spelled paths -> `focus.files`.
- **Set B (large repositories):** the 11 runnable Tier-C Part-D tasks (`eval/tier_c/issues/structural.toml`; gold in
  `eval/tier_c/gold/<task>/gold.json`, sites with adjudication `real`, 5-22 files per task, measured) at their pinned
  SHAs; focus = the task's `symbol` plus the `def_site` file.
- **Baselines:** B0 = today's MCP default (first 50 nodes in path order, canonical JSON; report its token cost);
  B1 = path order packed into the same budget with the same rendering (isolates ranking from format);
  B2 = `onboard` total-degree order packed the same way.

### 9.2 Pilot (measured; calibration only, not the decision run)

File-level Python re-implementation over CLI `repo-map` and `call-stats --dump-sites` output, 21 Set A questions
(secbench not run; it is not under the allowed corpora), no symbols. `@K` = top K files.

| Order | @12 | @25 | @50 |
|---|---:|---:|---:|
| path order (today's MCP order) | 0.400 | 0.581 | 0.810 |
| `onboard` total degree | 0.458 | 0.648 | 0.835 |
| degree x test prior | 0.487 | 0.711 | 0.841 |
| `ppr-sym-v1`, unfocused | 0.487 | 0.694 | 0.833 |
| `ppr-sym-v1`, focus from question text | 0.596 | 0.750 | 0.851 |

At an estimated 2k-token budget with 3 symbols per file the compact map holds all of httpx (60 files), cobra (36),
94 of ripgrep's 102 and 57 of excalidraw's 628 files (measured estimate, the `budget.out` section of [`RESULTS.md`](../../../analysis/2026-10-31-nav-gaps/spec-pilots/pilot-06-01/RESULTS.md)). At equal budget:

| | all 21 | excalidraw (3 questions) |
|---|---:|---:|
| B0 today, first 50 | 0.810 | 0.378 |
| B1 path order @2k | 0.915 | 0.406 |
| ranked @2k | 0.902 | 0.311 |
| focused @2k | 0.908 | 0.353 |
| B1 path order @4k | 0.930 | 0.511 |
| ranked @4k | 0.935 | 0.547 |
| focused @4k | 0.941 | 0.589 |

**What the pilot says (measured, with caveats):**

1. On Set A the gain over today comes from **compaction**: a 2k-token compact map shows whole repositories that
   today's 50-node JSON slices. Ranking adds nothing measurable at 2k on these corpora; on excalidraw path order even
   wins at 2k, because the gold files of some questions sit in a directory that sorts near the top alphabetically
   (an artifact, n = 3).
2. Ranking and focus help at small file counts (@12: 0.400 -> 0.596) and on excalidraw at 4k (0.511 -> 0.589).
3. **The gap analysis's threshold (">= 2x the current hit rate at <= 4k tokens") is unreachable on Set A**: today's
   first-50 order already scores 0.810 pooled, and 2 x 0.810 > 1. By the steering rule, a gate shown unreachable by
   its own measurement is a design defect: it is replaced below, and this replacement is owner question 6.
4. Set A cannot discriminate ranking from path order; only large repositories can. Hence Set B decides.

### 9.3 Pre-registered decision gates (run on the S1 CLI, before S2)

- **D1 - ship ranked as the no-argument MCP default.** All of: (a) Set A unfocused ranked @2k >= B1 @2k - 0.02
  pooled; (b) Set B unfocused ranked @2k >= B1 @2k + 0.10 pooled; (c) the default call's content text measured with a
  reference tokenizer (o200k or the Anthropic token-count endpoint; stated in the run record) is <= 2,000 tokens on
  >= 95% of the 13 Set A + B corpora (5 + 8). If (a) and (c) pass but (b) fails, ship the compact budgeted map in **path
  order** as the default (the compaction gain is real), keep ranking behind `mode`, and record ranking as unproven.
- **D2 - ship `focus` and let item 2 build orientation on it.** Set B focused @2k >= 0.50 pooled and >= unfocused
  @2k + 0.20 pooled, with no Set B task below its unfocused value by more than 0.10. Otherwise `focus` stays CLI-only
  and item 2 orients from callers/callees.
- Report per task and per corpus, never pooled only; show B0's token cost beside every row.

### 9.4 Ablations (reported, no gate)

Entry-point boost x3 in `p`; reverse weight 0 versus 0.5; NameOnly weight 0 versus 0.6. Any of these becomes part of a
`ppr-sym-v2` only with a separate owner decision (internal lane notes (private): measure cut yield first).

**Cost:** Set A minutes; Set B needs index builds of django, hugo, ruff, guava, mypy, prometheus, caddy and
TypeScript at the task SHAs (the TypeScript cache was 2.2 GB in Part-D; measured there), so it runs in a host-quiet
window (internal lane notes (private): host quiet window), about 1-2 hours serial (judgment). The harness is a Python script under `eval/`
(no private data committed; question/gold paths are arguments).

## 10. Slice plan

| PR | Content | Files | LOC (judgment) |
|---|---|---|---|
| S1 (R1) | `repo_rank.rs` (weights, PPR, focus, symbol choice, packer, renderers), `collect_module_edges` weight field, CLI `repo-map --ranked` flags, fixtures and unit tests, eval harness `eval/repo_map_rank/`; consumes `SymbolCatalog` from N0 (00-README X5) | new `src/navigation/repo_rank.rs`, `src/navigation/module_graph.rs` (+10), `src/cli.rs` (+20), `src/main.rs` (+25), `src/output/` renderer (new file) | ~600 (300 code, 200 tests, 100 harness) |
| gate | run D1/D2 on S1 (Set B in the host-quiet window shared with item 1's M1/M3, 00-README X12), record in the PR or a follow-up note | - | - |
| S2 (R2) | MCP: mode inference, new properties, dedicated result, `VIEW_NOTICE` clause, golden re-bless (ids 1, 3, 4; new ids 35-39), skill step 1 and table row, `docs/MCP.md` row; stays within its +900-char allotment under the size test F1b adds | new `src/mcp/tools_repo_map.rs` (handler moves out of `tools.rs`), `src/mcp/input.rs` (+40), `tools.rs` schema (+15) | ~350 |

Cross-spec order (00-README section 6): N0 → S1 (parallel with item 1's F1a/F1b and item 4's S-1a) → gate → S2
(golden merges F1b → S2 → E-4).

File-size rule (source-verified with `scripts/file_size.py`): `src/navigation/queries.rs` 2,534.5 weighted lines is
not touched; `src/mcp/tools.rs` 1,322.5 and `src/mcp/output.rs` 1,689 get only schema lines (the handler goes to a new
module, so `tools.rs` shrinks slightly); `module_graph.rs` 349.5 and `input.rs` 720 stay well under 1,000. Mutation
gate: `--since main --scope fn --authoritative` plus any lane the PR adds (CLAUDE.md tier 1); no tier-2 trigger
(no resolution change). Tier-A: not triggered (no call-resolution or CPG change).

## 11. Risks

- **Ranking unproven on small repos** (pilot): mitigated by gate D1's path-order fallback; the compaction gain does
  not depend on ranking.
- **Sparse graphs** (JS/TS 92% unresolved, C/C++, Java): rankings flatten. Disclosed in the header counters; the
  Set B TypeScript and Java tasks measure it.
- **Default change surprises a client** relying on the no-argument JSON graph: no known consumer reads it
  programmatically (the skill reads it as orientation; judgment). `mode: "graph"` restores it.
- **Estimator error**: bytes/3 may over- or under-count for some scripts; D1(c) measures it with a real tokenizer.
- **Eval overfitting**: parameters are fixed a priori (aider's multipliers, d = 0.85, prior 0.1); the pilot only
  calibrated, and Set B was not used to pick anything.

## 12. Owner questions (each with a recommendation)

1. **Change the no-argument `nav_repo_map` default to the ranked map?** Recommend yes, subject to D1; re-bless golden
   ids 3 and 4 with that reason.
2. **Default budget?** Recommend 2,000 tokens (under today's 2.3-3.2k first-50 payload, and small enough for cheap
   models that explore less with large context, DIRECTED).
3. **Promote `onboard` to an MCP tool?** Recommend no: its counts go into the map header (section 5.8).
4. **Git recency as a signal?** Recommend no for v1 (needs git history, changes with checkout state; no measurement).
5. **Entry points in the ranking?** Recommend display-only in v1 and the ablation in 9.4.
6. **Replace the gap analysis's ">= 2x at <= 4k" threshold** (unreachable on Set A, section 9.2 item 3) with D1/D2?
   Recommend yes.

## Appendix: reproduce the pilot

[`spec-pilots/pilot-06-01/`](../../../analysis/2026-10-31-nav-gaps/spec-pilots/pilot-06-01/): `goldfiles.py` (gold v2 -> files; needs the private evidence store),
`rank.py`/`rank2.py`/`rank3.py`/`atbudget.py` (orders and hit rates), `budget.py` (token fit), `RESULTS.md` (the `*.out`
outputs quoted above, with a crosswalk), `RUN.txt` (binary, corpus SHAs, regeneration commands). The binary was the release
build of 2026-10-30 at or near `533f061a`; no cargo was run; probe caches were deleted.
