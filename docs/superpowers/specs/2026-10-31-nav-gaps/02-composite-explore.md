# Composite explore (`nav_explore`), with call path A→B and implementations folded in

Date: 2026-10-31
Status: draft for owner review (items 2 + 3 + 5 of the 2026-10-31 nav-gap list)
Base read: main `533f061a` (read-only; no cargo run)
Claim tags: *measured* (a number from a prism run or an evidence file), *source-verified* (read at the
cited file:line), *judgment* (estimate). Unlabelled design text is a proposal.
Cross-spec decisions, tool budget, unified seed grammar and build order: `00-README.md`. This spec's PRs there are
E-1a, E-1b, E-2, E-3a, E-3b, E-4 and E-5. `nav_explore` is the program's only new default MCP tool (X1).

## 1. Goal, the decision it serves, non-goals

**Goal.** One MCP tool that answers "what is this symbol, who uses it, what does it use, how far does a change
ripple" in a single budgeted call, with two optional deeper sections, the shortest call path to a second symbol
and the implementations/hierarchy of a trait or interface, so that an agent's first useful prism call costs one
round trip instead of four or five and returns a *map*, not a reading substitute.

**Decision it serves.** The gap analysis found that the only arm whose tool got adopted in the tracing trial was
the one with a composite (`codegraph_explore`, 38 of 77 calls; prism 25 calls in 96 runs, all `nav_callers`,
`nav_callees`, `nav_nodes_at`; *measured*, COMPARATIVE §2; COMPARATIVE is the tracing-trial comparative write-up in the private evidence store), and that the composite also cost recall when its
source blobs replaced reading (R3 Haiku −0.19, R4 −0.36; *measured*, `codegraph.md` §6, private evidence store). The decision this spec
sets up is: **does a source-free, budgeted composite raise prism's adoption and lower per-run tool/Read traffic
without the recall loss?** Section 9 pre-registers the thresholds. Items 3 and 5 ride along because each is a
section whose standalone MCP tool would cost tool-list tokens on every run for a capability called rarely
(GitNexus `trace`: 0 calls in 96 runs; serena `find_implementations`: 2; *measured*, COMPARATIVE §2).

**Non-goals.**
- No source bodies in any section, in any version of this tool (see §4.2 for the evidence). A one-line header is
  the most source text explore returns.
- No embeddings, no text search: seed search is item 1's job (`01-find-symbol.md`); explore consumes its interface.
- No ranked repo map: item 6's job (`06-ranked-repo-map.md`); explore consumes its ranking signal.
- No statement-level dependence: item 4's job; explore reserves a `dependence` section and defers its contract
  (*reconciled:* the contract is `slice_at::run`, spec 04 §7.2; wired in PR-5).
- No JS/TS, Python or Java type hierarchy in v1 (§6.4 gives the reasons and the cheap follow-up).
- No change to existing tools' output bytes, to the CPG cache format or to the nav call-edge sidecar.

## 2. Current state (source-verified unless tagged)

**What an agent has today.**
- Seven nav tools plus `taint_reaches` and `refresh_index` (`src/mcp/tools.rs:25-75`,
  `src/mcp/tools_reasoning.rs:14-21`). Each nav tool returns one `Evidence` (`src/navigation/types.rs:168-178`),
  with optional agent views (`format: agent_markdown|agent_json`, profiles `Orientation, Seed, Impact,
  Dependencies, EditContext, Graph, Audit`, `src/mcp/input.rs:61-69`) and at most five `next_queries`
  (`MAX_NEXT_QUERIES = 5`, `src/mcp/evidence_view.rs:18`, built at `:770-816`). `nav_symbol_spans` returns
  coordinates only and "never echoes the callable body" (`src/mcp/tools.rs:34-40`); `SnippetPolicy::{None, Line,
  SymbolHeader}` exists for views (`src/mcp/input.rs:72-76`). No tool returns fan-in and fan-out together.
- Budget machinery: `MAX_RESULT_CHARS = 80_000` wire cap with a 12,000 floor (`src/mcp/output.rs:11-12`);
  `shape_result` binary-searches the retained item count to fit (`src/mcp/output.rs:188-250`, the search at `:233`);
  `MAX_RESULTS_DEFAULT = 50`, cap 1000, `MAX_VIEW_BYTES_DEFAULT = 12_000` (`src/mcp/input.rs:10-13`).
- *Measured* (gap analysis [PROBES.md](../../../analysis/2026-10-31-nav-gaps/gap-analysis/PROBES.md), click corpus): the hand-assembled "explain `Command.main`" bundle is callees
  depth 1 (26 items, 17.7 KB) + callers + module-deps + symbol-spans + a Read, four or five calls; `ego` at hops 2
  with Call edges is 520 nodes / 305 KB, over the cap.

**Callers and callees.** `callers_with_confidence` / `callees_with_confidence`
(`src/navigation/queries.rs:2123-2204`, `:2223-2320`) are frontier BFS for `hop in 0..depth` (MCP `depth` 0..5,
`DEPTH_MAX = 5`, `src/mcp/input.rs:7`) over `NavigationIndex::direct_callers/direct_callees`
(`src/navigation/mod.rs:299-313`), which read `NavigationCallEdgeIndex { outgoing_by_caller, incoming_by_target }`
(`:109-112`). Edge records are `IndexedIncomingCall { caller, callee_name, call_site_line, start_byte, end_byte,
qualifier, confidence, kind }` and `IndexedOutgoingCallSite { ..., resolved: Vec<IndexedResolvedTarget { target,
confidence, kind }> }` (`:74-106`). One item per call site; `visited` only gates recursion; the only hop information
is `score = confidence_score / (1 + hop)`. There is no `target` parameter, no parent pointer, no path
reconstruction: the fact "BFS to depth 5, no target termination or path reconstruction" is confirmed.
*Measured*: click callees from `main` at depth 1/2/3 = 26/98/188 items, 17.7/62.6/120.6 KB.

**Seeds.** `seed::resolve_fn` (`src/navigation/seed.rs:113-170`): location > symbol; a symbol seed is an exact name
match over `name_index` with optional file filter; 0 hits → `SymbolNotFound`, >1 → `AmbiguousSymbol { candidates }`
(`src/navigation/types.rs:181-189`). Seeds are callables only: `SymbolRef` has `Function | Statement | Variable`
(`:20-47`). A type declaration line is not a callable, so `nav_callers`/`nav_symbol_spans` on `class HTTPTransport`
or `struct Command` lines returned `LocationOutOfRange` in the DIRECTED injection (*measured*, the DIRECTED write-up,
§Limits, private evidence store).

**Implementations machinery.**
- Go: `CallGraph.interface_impls: BTreeMap<(interface name, method name), Vec<FunctionId>>` (`src/call_graph.rs:901`),
  filled by `apply_go_interface_dispatch` from `GoTypeProvider::compute_interface_dispatch(&live)` where `live =
  live_types::go_admission_live_set(files)` (`src/call_graph.rs:4005-4049`): the set is **RTA-pruned to constructed
  types**. `interface_method_names` (`:910`), `go_interface_declarations: BTreeMap<GoOwnerIdentity,
  BTreeSet<GoInterfaceDeclaration { defining_file, methods, method_signatures, embedded_types, generic,
  dispatchable }>>` (`:1188`, `src/go_owner_partition.rs:43-50, 63`) and `go_method_declarations:
  BTreeMap<GoOwnerIdentity, BTreeSet<GoMethodDeclaration>>` (`:1190`, `go_owner_partition.rs:64`) are persisted in
  the CPG. The only query is the CLI `interface-manifest` (`src/navigation/queries.rs:1281`, Go-gated, per call
  site: `dispatch_route`, `receiver_class`, `implementers`, `implementer_identities {file, name, package_clause,
  package_dir, span}`); *measured*: 428 sites / 145 KB on gin. No "implementers of I" query exists.
- Rust: `method_facts: BTreeMap<FunctionId, MethodFacts { kind: MethodKind::{Inherent, Trait(owner_key)}, .. }>`
  (`src/call_graph.rs:591-611, 922`) is filled per method from the enclosing `trait_item` or the `impl_item`'s
  `trait` field (`src/receiver_index.rs:366-381`, `src/languages/mod.rs:1429-1443`); the owner index keys methods
  by `(owner, name)` (`extension_methods`, `src/call_graph.rs:876`). An impl block's self type is resolved through the
  scope graph only for return typing (`resolve_impl_self_return`, `src/receiver_index.rs:204-219`). No trait → impls
  or type → traits query; empty impl blocks (`impl Marker for T {}`) leave no method fact at all.
- JS/TS: `class_heritage` is read only as a guard against inherited accessors (`src/ast.rs:3137`) and by a
  single-base-class helper used in resolution (`src/languages/mod.rs:1338-1375`); Python has the same helper over
  `superclasses`. Neither builds an inheritance relation. Java: no extraction.
- Oracles: Tier-A's `LspOracle` requests `callHierarchy/incomingCalls|outgoingCalls` and `documentSymbol` only
  (`eval/tier_a/oracles.py:292-371`); its `capability_probe` requires call hierarchy. **Correction to the brief:**
  `textDocument/implementation` *is* requested in the repo, by `eval/tools/dispatch_oracle.py` (gopls, def→impl route:
  `textDocument/definition` at the method token, then `textDocument/implementation` at the declaration, results
  mapped to receiver types via `documentSymbol`; `:1-130`). It is a per-dispatch-site oracle for Go only; Tier-A's
  per-corpus runs do not request it. Adding it is small (§9.4).

**Corpus sizes for the implementations section** (*measured*, grep on the bench checkouts): axum 30 trait
declarations, 103 `impl IntoResponse for`, 60 `impl FromRequestParts<..> for`; ripgrep 9 `impl Sink for`; gin 19
interface declarations; cobra 6 `PositionalArgs` validators.

**What changes.** New query modules under `src/navigation/` (explore, blast radius, call path, type index,
implementations), new output renderers, one CLI subcommand family and one MCP tool. `queries.rs` (2,534 weighted
lines), `call_graph.rs` (9,970), `resolution.rs` (4,186) and `tests/mcp/transport_tests.rs` (2,249) are over the
2,000 ceiling and are not touched (*measured*, `scripts/file_size.py`).

## 3. Interface

### 3.1 MCP: one tool, `nav_explore`

Read-only, `Evidence`-adjacent but **dedicated result** (`ExploreResult`, like `nav_symbol_spans` returns
`SymbolSpans`), because sections have heterogeneous shapes and per-section budgets; the item records inside list
sections reuse `SymbolRef`, `Location`, the resolution-kind strings and the Exact/NameOnly vocabulary so an agent that
learned `nav_callers` reads them unchanged.

```jsonc
{
  "seed":      { "kind": "symbol", "name": "ServeHTTP", "file": "gin.go" }   // or {"kind":"loc","file":..,"line":..}
                 // or {"kind":"query","text":"serve http"} when item 1 has landed (§8)
  "sections":  ["definition","callers","callees","blast_radius"],           // default; see table
  "to":        { "kind": "symbol", "name": "Next" },                          // required iff "path_to" requested
  "depth":     3,                 // blast-radius depth, 1..5, default 3
  "k":         1,                 // path_to: number of paths, 1..3
  "max_len":   6,                 // path_to: maximum hops, 1..8
  "confidence": "all",            // "exact" | "all"; NameOnly items are kept but labelled under "all"
  "snippets":  "none",            // "none" | "line": one call-site line per caller/callee item (≤160 bytes)
  "max_bytes": 8000,              // budget for the content text, 2000..24000
  "format":    "agent_markdown"   // "agent_markdown" | "json"
}
```

*Reconciled (00-README sections 3 and 1):* `seed` and `to` take the unified seed grammar owned by spec 01:
qualified `name`s (`Engine.ServeHTTP`) and callable handles `{"kind":"loc","file","line","name"}`. The `query` kind is
`{"kind":"query","text","file"?}`, where `file` is 01's glob filter. It is accepted on `seed` only, never on `to`. It
is parsed by explore's own input parser, so the shared `SeedInput` enum, and golden id 14, are unchanged. It is
enabled only if 01's M1 gate passes. `format` uses the vocabulary shared with the ranked `nav_repo_map` (X7):
`agent_markdown` (default), `canonical_json` or `agent_json`, where both JSON values render the canonical
`ExploreResult` as compact JSON. That replaces `"json"`. The `"json"` spelling remains the CLI `--format json`.

Sections (names are the `sections` vocabulary; unknown names are `bad arguments`, naming the vocabulary):

| Section | Default | Needs | Content |
|---|---|---|---|
| `definition` | always on | – | spans (outer/name/body, from `symbol_spans`), header line, role (production/test), language, line count, direct caller and callee counts, (Go/Rust) owner type and trait/interface membership when known |
| `callers` | on | – | ranked direct callers, one row per call site |
| `callees` | on | – | ranked direct callees plus an `unresolved` roll-up |
| `blast_radius` | on | – | transitive callers to `depth`: per-hop function and file counts, totals (all / Exact-only), top files |
| `module` | off | item 6 | the seed file's rank and top dependencies/dependents |
| `path_to` | off | `to` | item 3: k shortest call paths seed→`to` |
| `implementations` | off | Go/Rust seed | item 5: implementers / impls / overriders / overridden, or `implements` for a type or method seed |
| `dependence` | off | item 4 | reserved: backward statement dependence from a `loc` seed; returns `status: "unavailable"` until spec 04 ships |
| `candidates` | automatic | – | emitted instead of the other sections when the seed is ambiguous (§4.1) |

Argument rules: `to` without `path_to` → bad arguments; `path_to` without `to` → bad arguments; `k`/`max_len` without
`path_to` are accepted and ignored (documented) so a client can keep one argument template; `implementations` on a
Python/JS/TS/Java seed is **not** an error: the section carries `status: "unsupported_language"` and a one-line reason,
and the other sections are answered. `snippets: "line"` is the only source text beyond the header (§4.2).

Result envelope (`structuredContent`; the `content` text is the budgeted Markdown view unless `format: "json"`):

```jsonc
{
  "schema_version": "explore/1",
  "query": "explore:ServeHTTP@gin.go",
  "seed": { "symbol": SymbolRef, "role": "production", "language": "go", "header": "func (engine *Engine) ServeHTTP(w http.ResponseWriter, req *http.Request) {" },
  "sections": {
    "definition":   { "symbol_span": Location, "name_span": Location|null, "body_span": Location|null,
                      "lines": 14, "direct_callers": 31, "direct_callees": 6,
                      "owner": "Engine", "member_of": [{"kind":"interface","name":"http.Handler","confidence":"exact"}] },
    "callers":      { "total": 31, "shown": 18, "truncated": true, "exact": 29, "nameonly": 2,
                      "items": [ { "symbol": SymbolRef, "site": {"file":"gin_test.go","line":88}, "confidence":"exact",
                                   "kind":"typed_param", "role":"test", "score": 1.0, "snippet": null } ] },
    "callees":      { "total": 6, "shown": 6, "truncated": false, "items": [ ... same row shape, "site" in the seed file ... ],
                      "unresolved": { "sites": 3, "names": [ {"name":"w.WriteHeader","sites":2}, ... ] } },
    "blast_radius": { "depth": 3, "per_hop": [ {"hop":1,"functions":31,"files":9}, {"hop":2,"functions":40,"files":12}, {"hop":3,"functions":12,"files":5} ],
                      "total_functions": 83, "total_files": 17, "exact_only_functions": 80,
                      "node_cap": 5000, "truncated": false,
                      "top_files": [ {"file":"gin_test.go","functions":22,"role":"test"}, ... ] },
    "path_to":        { ... §5 ... },
    "implementations": { ... §6 ... }
  },
  "budget": { "max_bytes": 8000, "used_bytes": 7421, "allocation": {"callers":2800,"callees":2800,"blast_radius":900,"definition":600} },
  "warnings": [ Warning ],          // existing kinds: Collision, ParseQuality, ResultTruncated, StaleIndex, AmbiguousSymbol
  "next_queries": [ NextQuery ]     // ≤ 5, parser-valid, pointing at sites to read or to re-seed
}
```

`_meta` carries the existing keys (`prism/schema_version`, view keys, stale-index and refresh keys) plus
`prism/explore_budget_used` (integer). No new retryable status: warming/indexing/build_failed behave as for every tool.

**Errors.** Seed errors are the existing `QueryError`s rendered by `query_error_result`, with one change: a symbol seed
that is ambiguous returns a **successful** result whose only section is `candidates` (§4.1) rather than an
`AmbiguousSymbol` error; `LocationOutOfRange` on a type-declaration line becomes a type seed when the type index has
that line (§6.1), else the existing error. Bad arguments are reported as today (`bad arguments: ...`).

**Tool description budget.** ≤ 900 characters including the snapshot hedge (today's seven nav descriptions are
714–2,548 chars each, 13,540 total, about 3.4k tokens; *measured*, [PROBES.md](../../../analysis/2026-10-31-nav-gaps/gap-analysis/PROBES.md)). The description names the sections
in one list and gives one example; the `sections` vocabulary and the argument rules live in the input schema
`description` fields, which clients do not always render, and in `docs/MCP.md`. *Reconciled (00-README section 2):*
the whole definition (description, schema with the `query` seed and `to`, envelope) is allotted +2,750 chars under
the program's 17,500-char `tools/list` cap, which F1b's test enforces. It is estimated at 2,724.

### 3.2 CLI

```text
prism nav [--no-cache|--cache-dir DIR] explore --repo PATH
    (--symbol NAME [--file F] | --location file:line)
    [--sections a,b,c] [--to-symbol NAME [--to-file F] | --to-location file:line]
    [--depth N] [--k N] [--max-len N] [--confidence exact|all] [--snippets none|line]
    [--max-bytes N] [--format markdown|json|text]

prism nav call-path --repo PATH (--from-symbol .. [--from-file ..] | --from-location ..)
    (--to-symbol .. [--to-file ..] | --to-location ..) [--k N] [--max-len N] [--confidence exact|all] [--format json|text]

prism nav implementations --repo PATH (--symbol NAME [--file F] | --location file:line)
    [--direction implementers|implements|both] [--format json|text]
```

*Reconciled (00-README section 3):* as in every seeded CLI arm, `--location` and `--symbol` together form a callable
handle (spec 01 §5.1), and `--symbol` accepts qualified names. `explore` also takes `--query TEXT [--file GLOB]` for
`query` seeds.

`call-path` and `implementations` stay callable standalone **on the CLI only**: a clap variant costs no agent tokens,
the eval scripts (§9.3, §9.4) want one section per invocation, and the review-routing bridge runs the CLI, not MCP.
They call the same functions the explore sections call; there is no second implementation. On MCP they exist only as
sections of `nav_explore` (owner question Q2).

`--format text` for `explore` is the Markdown view; `json` is the canonical `ExploreResult`; exit codes follow
`render_err` (`src/output/navigation.rs:3`).

## 4. Semantics

### 4.1 Seed resolution

Order: `loc` → callable enclosing the line (`seed::resolve_fn`); if none and the line is a type declaration line in the
type index (§6.1) → type seed; else the existing error. `symbol` → item 1's `resolve_seed` when available (§8), else
`resolve_fn`; on `AmbiguousSymbol`, explore returns `candidates`: up to 20 rows `{symbol, role, direct_callers,
owner?, language}` ranked by item 1's ordering when available, else production before test, then direct caller count
descending, then file/line; `next_queries` are the top 5 as `nav_explore` calls with a `loc` seed. This is the
measured first-call failure (`AmbiguousSymbol` on click `invoke`, 85 candidates on axum `into_response`, Haiku P5
not retrying; *measured*) handled inside the tool instead of by the skill text. `query` seeds (item 1's search) are
accepted only once item 1 ships; before that they are `bad arguments` naming the two supported kinds.

*Reconciled (00-README section 4):* explore starts after 01's S1b, so item 1 is always present. Symbol, `loc` and
handle seeds go through `navigation::seed::resolve_fn` as S1b extends it (qualified names, handles, the shared
selection). On `AmbiguousSymbol { candidates, details }` the `candidates` rows are `{symbol, language,
...CandidateDetail}` (01 §5.4: owner, trait key, kind, role, overload stub, caller counts, `seed` handle) in 01's
rank order. The production-first fallback order above is no longer needed. `query` seeds call
`symbol_search::find_symbols(&NavigationSession, &SymbolQuery { query: text, file, mode: Auto, max_results: 20, .. })`:
a top match alone in its best `t_name + q` tier is explored, otherwise its matches become `candidates`
(`SymbolMatch::seed()` gives each handle). `next_queries` for candidates are `nav_explore` calls with **handle**
seeds, the top 5. `query` seeds ship only if 01's M1 query-only rate is at least 80%. Otherwise they stay
`bad arguments`.

### 4.2 Source text: header line yes, bodies never, call-site lines opt-in

Decision: `definition.header` is the first line of the callable (or the `name_span` line when a decorator wrapper
precedes it), trimmed to 200 bytes. No body excerpt exists in v1, not even behind a flag. `snippets: "line"` adds the
one call-site line to each caller/callee row (≤ 160 bytes, trimmed), default off.

Evidence for the decision (*measured*, tracing trial): codegraph's explore returned 15–26k-character blocks of
flow plus full source; when Haiku used it, it opened no file (R3: 10 tool calls, 0 Read, F1 0.33 vs 0.52 in `none`;
R4: 3 calls vs 14, 0.47 vs 0.83), losing exactly the argument hand-off units a call graph cannot show. The gains the
same tool produced (R1 Sol +0.40, J3 Haiku +0.29) came from the *flow* and the same-name disambiguation, not from the
source. Injected evidence made Haiku explore less (DIRECTED). So the payload must make reading cheaper to target, not
replace it: every row carries a `file:line` to read, `next_queries` point at the sites, and the only source text is
what is needed to pick the right site (the header gives parameter names; the opt-in call-site line shows which argument
is passed). Whether `snippets: "line"` should become the default is an eval arm (§9.2, arm C), not a guess.

### 4.3 Ranking (callers, callees)

Total order on rows, applied before truncation so truncation takes the tail:

1. confidence: Exact before NameOnly;
2. role of the *other* endpoint's file: production before test (`code_context::classify_file`,
   `src/navigation/code_context.rs:18`);
3. importance: item 6's `symbol_weight` of the other endpoint when available (§8), else its direct caller count
   (callers section) or the seed's call-site count to it (callees section), descending
   (*reconciled:* no `symbol_weight` exists; the key is `5 * callers_exact + 3 * callers_name_only` of the other
   endpoint from `SymbolCatalog`, the weighting 01 §5.5 and 06 §5.4 use, then the call-site count for callees);
4. tie-breaks: file ascending, start line, call-site line, call-site start byte, resolution kind string, name,
   qualifier — the same keys `sort_evidence_items` uses today (`src/navigation/queries.rs:2096-2111`).

`score` is the existing `confidence_score(edge.confidence)` (1.0 / 0.6); explore adds no decay because every row is
hop 1. `kind` is `ResolutionKind::as_str()` unchanged. `exact`/`nameonly` counts are over the full set, not the shown
subset. Callees: an unresolved site (empty `resolved`) is not a row; it is counted in `unresolved.sites` and grouped
by callee name (top 10 names by site count, then name).

### 4.4 Blast radius

BFS over `incoming_by_target` from the seed with a visited set; hop h = callers of hop h−1 functions not seen before;
`per_hop[h].functions` = newly visited functions at hop h, `files` = distinct files among them. Two passes: all
edges, and Exact-only edges (`exact_only_functions`). Stop expanding when `visited` reaches `node_cap` (5,000; CLI
`--node-cap`, MCP fixed) and set `truncated: true` with the hop at which it stopped. `top_files` = up to 10 files by
visited-function count, then path. The dropped-collision warning for the seed name is attached (same
`collision_dropped_site_locations` call the callers query makes, `src/navigation/mod.rs:329`), because a dropped
receiver site is a caller the radius cannot see. The section never lists functions beyond the counts and files;
an agent that wants names raises `depth` on `nav_callers`, and `next_queries` offers that call.

### 4.5 Budget fitting

`max_bytes` bounds the **content text** (Markdown or JSON view). Allocation: `definition` is fitted first and never
truncated (≤ 600 bytes by construction). The remainder is split among the requested list sections by fixed weights
(callers 30, callees 30, path_to 20, implementations 20, module 10, blast_radius 10, candidates 100), renormalized
over the sections present. Each list section is fitted by binary search on its retained row count against its share,
the same way `shape_result` fits items (`src/mcp/output.rs:233-250`); a section that uses less than its share returns
the difference to a pool, and one second pass gives the pool to the sections that were truncated, in the weight
order. `truncated`/`total`/`shown` are per section. The retained rows in `structuredContent` are exactly the rows in
the text, so both views agree; the whole wire result then passes through the existing 80 KB cap
(`shape_structured_value`), which with `max_bytes ≤ 24,000` and the measured 2–3× Markdown→JSON ratio is never the
binding constraint (*judgment*; a test pins it with the largest fixture). `max_bytes` default 8,000 ≈ 2k tokens:
large enough for about 40–60 rows across sections at ~90–120 bytes per Markdown row, small enough that the DIRECTED
finding (cheap models explore less with large context) is respected. The default is an owner question (Q5).

### 4.6 Determinism and confidence

All collections are `BTreeMap`/`BTreeSet` or totally ordered `Vec`s; no `HashMap` iteration reaches output; `f32`
scores compare through the row order above, never alone. The same snapshot, arguments and binary produce identical
bytes. Confidence is never upgraded: a NameOnly edge stays `nameonly` in every section, the path label is the minimum
over its edges, and an implementation found by name only is `nameonly`.

### 4.7 Markdown view (shape, not prose)

```markdown
# ServeHTTP — gin.go:569 (production, go) · 31 callers (29 exact) · 6 callees · blast radius d3: 83 fns / 17 files
`func (engine *Engine) ServeHTTP(w http.ResponseWriter, req *http.Request) {`
member of: http.Handler (exact)

## Callers (18 of 31 shown; 2 name-only)
- gin.go:633 `RunListener` ← exact typed_param
- gin_integration_test.go:88 `testRequest` ← exact typed_param (test)
...
## Callees (6)
- context.go:55 `reset` exact qualified_owner
- (unresolved: 3 sites: w.WriteHeader ×2, ...)
## Blast radius (depth 3)
hop1 31 fns/9 files · hop2 40/12 · hop3 12/5 · total 83 fns in 17 files (80 exact-only)
top files: gin_test.go 22 (test), routes_test.go 9 (test), gin.go 7
## Next
nav_explore {"seed":{"kind":"loc","file":"gin.go","line":633}}
nav_callers {"seed":{"kind":"symbol","name":"ServeHTTP","file":"gin.go"},"depth":2}
```

Rows are `file:line name ← confidence kind (role)`; nothing else. The renderer lives in `src/output/explore.rs`
and is shared by the CLI `--format markdown|text` and the MCP content text, so the golden and CLI tests pin one
renderer.

## 5. Item 3: shortest call path A→B (`path_to`, `prism nav call-path`)

### 5.1 Algorithm

Inputs: `from` (the explore seed), `to`, `k` (1..3, default 1), `max_len` (1..8, default 6), `confidence`.

1. **Exact pass.** Bidirectional BFS: forward from `from` over `outgoing_by_caller` keeping only
   `IndexedResolvedTarget` entries with `confidence == Exact`, backward from `to` over `incoming_by_target` with the
   same filter, alternating the smaller frontier, each side bounded by `ceil(max_len/2)` and both together by
   `max_len`; record `dist_from[n]` and `dist_to[n]`. Unresolved sites are never traversed. If the frontiers meet,
   `L = min over meeting nodes of dist_from + dist_to`.
2. **Enumeration.** With `L` known, enumerate simple paths by depth-first walk from `from`, expanding an edge
   `cur → n` only when `dist_from[n] == dist_from[cur] + 1` and `dist_from[n] + dist_to[n] ≤ L + slack`, where
   `slack = 0` for `k = 1` and `1` for `k > 1` (so k > 1 may return paths of length L and L+1, never longer); the
   on-path set keeps paths simple. Neighbours are visited in the total order (target file, start line, call-site
   line, start byte), so the first `k` paths found are deterministic. The walk stops after 10,000 expansions and sets
   `truncated: true`.
3. **Fallback pass.** If `confidence == "all"` and the exact pass found nothing, repeat 1–2 over all resolved edges;
   every returned path then carries `min_confidence: "nameonly"` and the section `status` is `found_nameonly`.
   Under `confidence: "exact"` there is no fallback.
4. **Cycles and self.** Simple paths only; `from == to` means "shortest cycle through A": step 1 starts the forward
   search from A's callees and the backward search from A's callers, and a path of length 1 is A→A recursion.
   Recursive edges elsewhere are ordinary edges the on-path set prevents from repeating.
5. **Parallel sites.** Two call sites between the same pair are one edge; the edge reports the lowest call-site line
   and `site_count`.

Complexity: bounded by `max_len` and the frontier sizes; on the measured click graph depth-3 callees from `main` had
188 items, so a 6-hop bidirectional search is tens of thousands of edge reads at most (*judgment*; the expansion cap
makes the worst case a flagged `truncated`, never a timeout).

### 5.2 Result

```jsonc
"path_to": {
  "from": SymbolRef, "to": SymbolRef,
  "status": "found" | "found_nameonly" | "not_found" | "not_found_within_max_len" | "truncated",
  "k": 1, "max_len": 6, "exact_pass": {"expanded": 412, "met": true}, "all_pass": null,
  "paths": [ { "hops": 3, "min_confidence": "exact",
               "edges": [ {"from": SymbolRef, "to": SymbolRef, "site": {"file":"core.py","line":1367}, "site_count": 1,
                           "confidence": "exact", "kind": "qualified_owner"} ] } ],
  "dropped_sites_near_endpoints": 421   // collision-dropped same-name receiver sites for `to`'s name, when > 0
}
```

Payload: one 3-hop path is ≈ 400–600 bytes of JSON and three Markdown lines, against 120.6 KB for depth-3 callees on
click (*measured*). The recall bound is the precision floor, stated in the result: on click the edge
`Command.invoke → Context.invoke` at `core.py:1367` is one of 421 dropped same-name receiver sites (*measured*,
[PROBES.md](../../../analysis/2026-10-31-nav-gaps/gap-analysis/PROBES.md)), so `main → Context.invoke` reports `not_found` with `dropped_sites_near_endpoints: 421`; the Markdown view
prints "no path through resolved edges; 421 same-name receiver sites near the target were not attributed" so an
agent reads it as unknown, not absent (the skill already teaches this reading for callers).

## 6. Item 5: implementations and type hierarchy (`implementations`, `prism nav implementations`)

### 6.1 Type seeds and the type index

A per-session, lazily built `TypeIndex` (`OnceLock` on `NavigationIndex`, like `resolved_call_edges`) over Go and
Rust files only in v1: one tree-sitter walk per file collecting `TypeDecl { file, name, kind: interface|struct|
trait|enum|type_alias, name_span, decl_span, module_scope (Rust) | package_dir+clause (Go), generic: bool }`, plus for
Rust every `impl_item` as `ImplBlock { file, self_type_text, trait_path: Option<String>, generic_params: bool,
span, method_fids }` and for Go the `go_interface_declarations` / `go_method_declarations` already in the CPG. The
walk is the one `rust_type_aliases_by_module` already does for `type_item` (`src/receiver_index.rs:12-50`), widened to
the other item kinds; it is query-time only, so **no CPG cache or sidecar change** (CPG stays v132). Identity: Rust
types and traits resolve their path through the scope graph (`resolve_type_path_to_type_scope`,
`src/resolution_identity`) when it answers `TypeKey::InRepo` → `exact`; otherwise name → `nameonly`. Go identity is
`GoOwnerIdentity` (package dir + clause + name), which the manifest already emits.

`SymbolRef` gains no variant in v1 (adding one forces every exhaustive match in views and renderers to change);
explore's type seed is its own `TypeRef { file, name, kind, start_line, end_line, start_byte, end_byte }` inside
`ExploreResult.seed`. When a type seed is used, `callers`/`callees`/`blast_radius` are answered for the type's methods
collectively only if cheap (Go: the owner's method set; Rust: the inherent and trait impls' methods), else those
sections report `status: "not_applicable_to_type"`; v1 does the latter and `definition` + `implementations` only
(owner question Q6).

### 6.2 Queries

| Seed | `implementers` direction | `implements` direction |
|---|---|---|
| Go interface `I` | types whose method set includes every method of `I` (embedded interfaces expanded through `embedded_types`, recursively, cycle-guarded), each `{type: TypeRef, confidence, live: bool, receiver: value|pointer, missing: []}` | – |
| Go concrete type `T` | – | interfaces `T` (or `*T`) satisfies, same comparison |
| Go method `T.m` | – | interface methods `I.m` for every `I` that `T` satisfies and that declares `m` |
| Rust trait `Tr` | impl blocks `impl Tr for X` → `{type: X (TypeRef or raw text), confidence, blanket: bool, generic: bool, span, methods: n}`; empty impls included (from `ImplBlock`), derive/macro-generated impls not visible (stated) | – |
| Rust type `X` | – | traits with an `impl Tr for X` block (nominal self types; a blanket impl that could cover `X` is listed as `nameonly, blanket`) |
| Rust trait method `Tr::m` (declared in `trait_item`) | overriders: `m` in every `impl Tr for X` block (`method_facts[fid].kind == Trait(key)` ∧ name); `has_default: bool` on the trait side | – |
| Rust impl method `m` in `impl Tr for X` | – | overridden: the trait's declaration of `m` with `has_default`; plus the trait itself |
| Any other language | `status: "unsupported_language"` | same |

Go satisfaction is computed at query time from `go_interface_declarations × go_method_declarations` (method names
and, when `GoMethodDeclaration` carries a comparable signature string, `method_signatures` equality → `exact`; name
match only → `nameonly`). **Verify at implementation:** `GoMethodDeclaration`'s fields were not read for this spec;
if it has no signature text, v1 ships name-set satisfaction labelled `nameonly` and a follow-up adds signatures.
`live` comes from `go_interface_live_types`; the section reports the full in-repo satisfier set, not the RTA-pruned
one, because the navigation question is "who implements this", and marks the non-live ones so a reviewer knows
`interface_impls` (and therefore `nav_callers` through dispatch) does not route to them. Generic interfaces
(`generic: true`) and Rust traits with generic parameters on the trait itself are answered with
`status: "unsupported_generic"` and the raw impl list labelled `nameonly`.

Result:

```jsonc
"implementations": {
  "status": "ok" | "unsupported_language" | "unsupported_generic" | "not_a_type_or_method",
  "language": "rust", "seed_kind": "trait", "direction": "implementers",
  "total": 9, "shown": 9, "truncated": false, "exact": 8, "nameonly": 1,
  "items": [ { "type": TypeRef, "file": "crates/searcher/src/sinks.rs", "line": 540, "confidence": "exact",
               "blanket": false, "generic": false, "role": "production", "methods": 3 } ],
  "notes": ["derive and macro-generated impls are not indexed", "1 blanket impl listed as nameonly"]
}
```

Ordering: Exact before NameOnly, production before test, then file, line. Cap 200 rows before budget fitting.

### 6.3 Languages in v1 and why

Go and Rust only: the facts exist in the CPG (Go) or are one AST walk away with scope-graph identity (Rust), and the
measured consumers are Go/Rust review lanes (the dispatch-precision gate, the Rust trait-method ambiguity
`into_response` ×85). Where the host LSP runs, `lsp-nav` already answers this; prism's increment is answering it from
the index with the same confidence labels the call edges carry, on checkouts with no toolchain.

### 6.4 Out: JS/TS, Python, Java (and the cheap version if a consumer appears)

- JS/TS and Python have no inheritance relation in prism; a syntactic `extends`/base-class subclass list is cheap
  (~150 LOC per language, name-only, no import resolution) but answers "subclasses of X", which no trial question
  asked: the census questions (P5, J5) are type *references*, and their failure was output behaviour (DIRECTED). Python
  inheritance is also a resolution gap, so a name-only hierarchy would outrun the call graph it sits beside.
- Java: no extraction and a 99% Exact gap; a hierarchy without resolved calls is not navigation.
- Trigger to revisit: a review consumer that asks "who overrides this method" on a JS/TS or Python corpus, measured
  the way §9.4 measures Go/Rust (pyright has no `implementation` request, so the Python oracle would be tsserver-style
  class scanning or a model adjudication; stated as a cost, not planned).

## 7. Data, cache and performance

- No CPG cache bump (v132 stays), no nav call-edge sidecar change, no change to any existing tool's bytes: the
  `TypeIndex`, Go satisfier table and blast-radius/path searches are query-time over persisted facts.
- Session memory: `TypeIndex` is O(types + impl blocks); unloaded with the index at idle like everything on
  `NavigationIndex`. Under the shared daemon it is per provider (built under the read lock's `OnceLock`, once).
- Budgets (warm, after the first explore on a corpus; to be recorded in each PR against the five probe corpora):
  explore default sections ≤ 150 ms; blast radius ≤ 50 ms at `node_cap` 5,000; `path_to` ≤ 100 ms or `truncated`;
  `TypeIndex` first build ≤ 2 s on a 5k-file Go/Rust corpus (*judgment*; measured in PR-3 on ruff and kubernetes-size
  trees under `PRISM_PROFILE`, and the walk is parallel per file with rayon like the CPG build).
- Golden transcripts: all five `tests/fixtures/mcp/golden/*.jsonl` contain the `tools/list` response, so **adding
  `nav_explore` changes every golden** (re-bless with `PRISM_MCP_BLESS_GOLDEN=1`, reason: new tool) and
  `tests/mcp/smoke_test.rs:44-48` moves from 9 to 10 tools. Nothing else in the goldens changes.
  *Reconciled (00-README section 5):* PR-4 re-blesses ids 3 and 12 (the unknown-tool error lists the available
  tools) and adds transcript ids 40-49. It merges after 01's S1b and 06's S2, which change id 3 first; 06's S2
  also owns the `VIEW_NOTICE` clause (id 1), so PR-4 does not touch it. The `registry.rs` tool-list unit test
  changes with the smoke count.

## 8. Assumed interfaces (to reconcile with `01-find-symbol.md`, `06-ranked-repo-map.md`, spec 04)

| From | Assumed signature | Used for | Fallback if the other spec differs or is not shipped |
|---|---|---|---|
| item 1 | `navigation::find_symbol::resolve_seed(&NavigationSession, name: &str, file: Option<&str>) -> Result<ResolvedFn, QueryError>` returning a unique best hit when the ranking separates it, else `AmbiguousSymbol` | §4.1 symbol seeds | `seed::resolve_fn` |
| item 1 | `find_symbol::search(&NavigationSession, &SymbolQuery { text, kind, file, limit }) -> Vec<SymbolHit { symbol, score, role, direct_callers, owner, match_kind }>` | `query` seeds; `candidates` ranking | `query` seeds rejected; candidates ranked by role, caller count, file/line |
| item 6 | `navigation::repo_rank::symbol_weight(&NavigationSession, &FunctionId) -> f32` (deterministic, in [0,1]) | §4.3 rank key 3 | direct caller count |
| item 6 | `repo_rank::file_rank(&NavigationSession) -> BTreeMap<String, RankedFile { rank, score, in_degree, out_degree }>` | `module` section | section reports `status: "unavailable"` |
| item 4 | `reasoning::slice_at::backward(&NavigationSession, file, line, &SliceOpts) -> Evidence` | `dependence` section for `loc` seeds | section reports `status: "unavailable"` |
| this spec → others | `navigation::explore::{explore, ExploreOpts, ExploreResult}`, `navigation::call_path::shortest_paths`, `navigation::implementations::query`, `navigation::type_index::TypeIndex` | item 6 may reuse `blast_radius` counts as a rank signal; item 1 may reuse `TypeIndex` for type-kind hits | – |

Reconciliation rule: if `01` or `06` lands with other names, explore adapts in its own PR; neither spec should
block on the other. The `candidates` section is explore's own even if `01` ships a richer search.

**Reconciled interfaces (00-README X9, section 4).** The rows above are superseded by the names the other specs
define:

| From | Actual interface | Used for |
|---|---|---|
| item 1 (S1b) | `navigation::seed::resolve_fn` with the shared callable selection: qualified names, handles; `QueryError::AmbiguousSymbol { candidates, details: Vec<CandidateDetail> }`, `SymbolNotFound { did_you_mean }`; `seed::{SeedHandle, resolve_handle}` | §4.1 symbol, `loc` and handle seeds; `candidates` rows |
| item 1 (S1a) | `navigation::symbol_search::{find_symbols, SymbolQuery, SymbolMatch, SymbolSearchResult}`, `SymbolMatch::seed()` | `query` seeds (gated on 01's M1) |
| item 1 (N0) | `navigation::symbol_catalog::{SymbolCatalog, CatalogEntry}` (`callers_exact`, `callers_name_only`, owner, role, language) | §4.3 rank key 3; `definition` owner |
| item 6 (S1) | `navigation::repo_rank::rank_files(&NavigationSession, &FocusSpec::default()) -> FileRanking { entries: Vec<RankedFileCore { rank, file, score, role, language, in_degree, out_degree, focus, entry }>, .. }` | `module` section (the seed file's entry; no focus, so 06's gate D2 does not apply; default-on only if 06's D1 passes) |
| item 4 (S-1a/S-1b) | `navigation::slice_at::run(&NavigationSession, &SliceRequest { seed, direction: Backward, control: true, reach: None, depth: 0, max_results: 12 }) -> Result<Evidence, QueryError>` | `dependence` section for `loc` seeds (PR-5) |

## 9. Evaluation (pre-registered)

### 9.1 Adoption (existing harness, live Sonnet, deepeval)

`eval/adoption` measures whether the skill and a nav tool are invoked on navigation prompts
(`nav_invocation_pass5_rate`; probes in `eval/adoption/goldens/probes.toml`, iteration loop in its README). Add six
explore-shaped probes against the same `tier_c` repo: "explain what `X` is and who depends on it", "how does `A`
reach `B`", "what would break if `X` changed (how far)", "who implements `I`" (a Go or Rust fixture), one ambiguous
name, one type-declaration line. `expected_tools = ["nav_explore"]` for the first three; `["nav_explore",
"nav_callers"]` for impact. Threshold: `nav_invocation_pass5_rate ≥ 0.5` on the six new probes and no existing
probe's rate falls by more than 0.1 (existing probes keep their expected tools; a run that calls `nav_explore`
instead of `nav_callers` on an existing impact probe counts as a pass only if the goldens are updated in the same PR,
stated explicitly). Cost: one round of the existing suite (minutes, account capacity; *judgment* ≈ $2–3).

### 9.2 Efficiency and no-harm on the trial questions (cheap-model replicates)

Harness: the tracing-trial runner (private evidence store: `runner.py`, `score.py`, frozen gold v2, isolation unchanged). Questions: the 20
auto-eligible ones (sites, flow, data, guards, object ×4 corpora; reuse excluded as unscored). Model: Haiku, the one
adopter (9/24 prism runs; *measured*), 3 replicates; plus Sol ×1 as the strong-model check (subscription-billed, no
USD). Arms, same session, same prism binary:

- **P0** prism as shipped today (control, re-run so the control shares the environment);
- **P1** prism + `nav_explore` (default arguments, `snippets: none`), skill text updated to "orient with one call";
- **C** (only if P1 passes T1–T3) prism + `nav_explore` with `snippets: "line"` as the tool default.

60 Haiku runs per arm at the measured $0.245 per 24 Haiku runs ≈ $0.6–1.0 per arm, about 1 h serial each
(*measured* rates, COMPARATIVE §6, §10). Metrics per run from the existing transcripts: arm tool calls by name,
built-in calls by kind (READ/SEARCH), input tokens (first turn and total), F1 against gold v2, format validity.

Thresholds (decided before the run):
- **T1 adoption in use:** `nav_explore` called in ≥ 40% of Haiku runs on sites/flow/object questions (today prism
  overall 38%, concentrated there) and ≥ 1 Sol run in 20.
- **T2 efficiency:** on paired runs where P1 called `nav_explore`, median READ+SEARCH built-in calls ≤ 0.7× the P0
  pair; median total input tokens ≤ 1.1× P0 (explore's bundle must not cost more than the calls it replaces).
- **T3 no harm, pooled:** mean paired ΔF1 (P1 − P0) over the 20 questions × 3 replicates ≥ −0.05.
- **T4 no harm where composites failed before:** mean paired ΔF1 on the 8 data + guards questions ≥ −0.05, and no
  question with Δ ≤ −0.15 in ≥ 2 of 3 replicates. This is the codegraph failure mode; T4 failing while T2 passes
  means the tool is substituting for reading, and the remedy is tried in arm C and the skill text, not by adding
  source.
- **Decision:** T1 ∧ T2 ∧ T3 ∧ T4 → ship as default-recommended in the skill; T3/T4 fail → keep the tool, skill
  recommends it only for sites/flow/object questions, record the loss; T1 fails → the composite is not an adoption
  lever for prism either and the owner decides whether the tool stays (the token cost is ~400 tokens per run).
- Noise disclosure: single-cell noise is ±0.1 F1 (*measured*); three replicates on 20 questions give a pooled SD of
  roughly 0.03–0.04 on the mean (*judgment*), so T3's −0.05 is at the edge of detectability; the eval reports the
  interval, and a result inside noise is "no harm detected", not "no harm".

### 9.3 Path correctness (deterministic, $0)

Script `eval/tools/call_path_gold_check.py` (~120 LOC, stdlib): for each flow gold (P2, R2, G2, J2) and the call-edge
chain of J3, take the gold's first and last `from`/`to` units as A and B, run `prism nav call-path --format json`
with `--confidence exact` then `all`, and check that every `required_unit` edge `(from, to, site)` appears in some
returned path. Report per gold: found/not, hops, `min_confidence`, JSON bytes, and bytes of `callees --depth 3` from
A for comparison. Threshold: required edges recovered on ≥ 4 of 5 golds **where prism has the edges** (an edge absent
from `callees --depth 1` of its `from` is a resolution gap, listed separately, not a path defect); path payload ≤ 2 KB
per gold. Recall disclosure (not gated): on click and flask, 10 grep-found `a.m()` → `Class.m` edges sampled from the
`Collision` warning sites, how many break a path; expected to be most of them (the 421-site example), reported as the
floor the tool inherits.

### 9.4 Implementations correctness (LSP oracle, $0 plus adjudication)

Oracle: gopls and rust-analyzer `textDocument/implementation`. Tier-A today requests call hierarchy only
(`eval/tier_a/oracles.py:292-371`); `eval/tools/dispatch_oracle.py` already drives gopls `implementation` per
dispatch site. What it takes to add a per-declaration implementations oracle:

1. `LspOracle.implementations(rel_path, line, character)` (~20 LOC): one `textDocument/implementation` request after
   the existing readiness wait; results mapped to declarations with `documentSymbol` (Go receiver types via
   `_split_receiver`, already there; Rust `impl Tr for X` symbols appear as `impl` entries in rust-analyzer's
   `documentSymbol`, whose name text gives `X` — verify on the first run).
2. A capability probe per server, like `capability_probe`: rust-analyzer answers `implementation` on a trait name
   (impl blocks) and on a trait method (impl methods); gopls on an interface name and on an interface method
   (*judgment* from the servers' documented capabilities; the probe makes it measured).
3. `eval/tools/implementations_oracle.py` (~300 LOC, reusing `dispatch_oracle`'s client setup, build-tag handling
   and identity mapping): seeds = every interface/trait declaration the `TypeIndex` reports (gin 19 interfaces, axum
   30 traits; sample 30 when more), for each: prism `nav implementations --format json` vs the oracle set, both
   restricted to in-repo declarations; per row `sound | over_approx | recall_gap | oracle_unresolved`, and the
   override-proof rule from the dispatch oracle (an oracle failure is `oracle_unresolved`, never a prism verdict).
4. Corpora: gin and caddy (Go), ripgrep and axum (Rust) — all pinned in `eval/corpora.toml` with oracle commands.

Thresholds: precision of `exact`-labelled rows ≥ 0.95 per language (every `over_approx` row adjudicated and
explained before the number is reported); recall ≥ 0.80 for Go non-generic interfaces and Rust nominal impls,
reported separately for blanket impls, derive-generated impls and generic interfaces (excluded from the recall
denominator but counted and shown). Trait-method `overriders` are checked on 10 methods per Rust corpus the same way.
Cost: hours of machine time, ~2 h adjudication (*judgment*).

## 10. Test plan

Test-first variant fixtures (one Go, one Rust, one Python/JS fixture generator each in `tests/common/mod.rs`), then
the implementation. Each class below is a named test; the negative/edge cases are listed with their expected result.

**Seeds.** unique symbol → sections answered; same-name pair → `candidates` with two rows, no other sections, two
`next_queries` with `loc` seeds; `loc` in a body → enclosing callable; `loc` on a Go `type X interface {` line and on
a Rust `pub trait T {` line → type seed (`definition` + `implementations`); `loc` on a blank line past EOF →
`LocationOutOfRange`; anonymous owner name → `SymbolNotFound` with the existing hint; symbol in a test file → `role:
test` in `seed`; `query` seed before item 1 → bad arguments naming the supported kinds (*reconciled:* `query` seed
with 01's M1 gate failed → bad arguments; `query` kind on `to` → bad arguments; a handle seed → that exact callable; a
qualified symbol seed → resolved; ambiguous symbol → `candidates` rows carrying 01's `details` and handle
`next_queries`).

**Callers/callees ranking.** Exact and NameOnly callers of one seed → Exact first; a production and a test caller
with equal confidence → production first; two production Exact callers with different fan-in → higher first; full
tie → file/line/site order; callees with two unresolved sites of the same name → one `unresolved.names` row with
`sites: 2`; `confidence: "exact"` → NameOnly rows absent and counted.

**Blast radius.** chain D→C→B→A seeded at A, depth 3 → per_hop [1,1,1], total 3; diamond (B and C both call A, D
calls both) → D counted once; cycle A←B←A → terminates, B counted once, A not re-counted; a NameOnly edge in hop 2 →
`exact_only_functions` lower than `total_functions`; `node_cap` 3 (test-only option) on a 5-caller fixture →
`truncated: true` at hop 1; a Collision-dropped site for the seed name → warning attached.

**Budget.** `max_bytes` 2,000 with 60 callers → `definition` complete, callers truncated, `shown < total`, bytes ≤
2,000; callers using less than its share and callees over → callees receives the pool; the JSON `structuredContent`
rows equal the Markdown rows; largest fixture at `max_bytes` 24,000 → wire result under the 80 KB cap without the
terminal over-cap path; `max_bytes` 1,999 and 24,001 → bad arguments; identical input twice → identical bytes.

**Path.** direct edge → 1 hop; 3-hop chain → the chain; two equal-length chains with `k: 2` → both, deterministic
order, `k: 1` → the one that sorts first; the only route through a NameOnly edge → `confidence: "all"` gives
`found_nameonly` with `min_confidence: nameonly`, `confidence: "exact"` gives `not_found`; a cycle on the route →
simple path returned; `from == to` with recursion → 1-hop self path, without → shortest cycle or `not_found`; a route
through an unresolved site → `not_found`; `max_len` 2 on a 3-hop chain → `not_found_within_max_len`; parallel sites
→ `site_count: 2` and the lowest line; Collision-dropped sites at `to` → `dropped_sites_near_endpoints` populated;
expansion cap (test-only knob) → `truncated`.

**Implementations.** Go: interface with value-receiver, pointer-receiver and embedded-interface satisfiers → three
rows with `receiver` set and the embedded path expanded; a type missing one method → absent, listed in a
`near_miss` note only when exactly one method is missing (cheap and useful; drop if it complicates PR-3); same-named
interfaces in two packages → two distinct seeds, no cross-listing; a generic interface → `unsupported_generic`;
`live: false` for a type with no construction site; `implements` from a concrete type and from a method. Rust: trait
with two nominal impls, one empty impl, one blanket impl → four rows, the blanket `nameonly, blanket: true`; trait
method → impl methods, `has_default` true/false; impl method → its trait declaration; same-named traits in two
modules → scope-graph identity separates them; `impl Display for X` seeded by `Display` → `SymbolNotFound` with a
"not an in-repo trait" hint; a derive-only impl → absent and the `notes` line present. Python class seed →
`unsupported_language`, other sections answered.

**MCP.** input parsing (every argument's range, unknown section, `to` without `path_to`, `path_to` without `to`,
`format` values); `_meta` keys; `nav_explore` on a warming index → the existing retryable result; goldens re-blessed
with the stated reason; smoke count 10; `tools/list` description length ≤ 900 chars (a unit test pins it so the
description cannot creep).

**CLI.** `explore`, `call-path`, `implementations` in `tests/cli/`: json and text/markdown outputs, exit codes,
`--format json` parses as `ExploreResult`, `--confidence exact`.

**Coverage matrix.** New test files are added to the three `all_test_files` copies in
`tests/integration/coverage_test.rs` only if they contain `fn test_*` names in the algorithm × language form; the
explore tests are navigation tests under `tests/integration/` and `tests/cli/` and do not enter the matrix.

## 11. Slice plan

Order recommended: PR-1 → PR-2 → PR-4 → PR-3 → PR-5, so the adoption measurement (needs MCP) starts before the
implementations work; the `nav_explore` description written in PR-4 already names the `implementations` section, so
PR-3 changes no golden. LOC are weighted lines, code plus tests, *judgment*.

| PR | Scope | New files | Touched files | LOC |
|---|---|---|---|---|
| PR-1 explore core (CLI) | `ExploreOpts/ExploreResult`, seed resolution incl. `candidates`, `definition`, ranked `callers`/`callees`, `blast_radius`, budget fitting, Markdown/JSON renderers, CLI `explore` | `src/navigation/explore.rs` (~350), `src/navigation/blast_radius.rs` (~120), `src/navigation/explore_budget.rs` (~150), `src/output/explore.rs` (~200), `tests/integration/explore_test.rs` (~350), `tests/cli/explore_test.rs` (~80) | `src/navigation/mod.rs` (+5), `src/output/mod.rs` (+2), `src/cli.rs` (+45), `src/main.rs` (+40) | ~1,300 |
| PR-2 call path | bidirectional BFS, enumeration, fallback, CLI `call-path`, `path_to` section, gold-check script | `src/navigation/call_path.rs` (~300), `tests/integration/call_path_test.rs` (~250), `eval/tools/call_path_gold_check.py` (~120) | `explore.rs` (+40), `cli.rs` (+30), `main.rs` (+25) | ~750 |
| PR-3 implementations | `TypeIndex`, Go satisfaction, Rust impl scan, `implementations` section, CLI, oracle script, Tier-A wrapper | `src/navigation/type_index.rs` (~250), `src/navigation/implementations.rs` (~300), `tests/integration/implementations_test.rs` (~300), `eval/tools/implementations_oracle.py` (~300) | `explore.rs` (+40), `navigation/mod.rs` (+10, the `OnceLock`), `cli.rs` (+30), `main.rs` (+25), `eval/tier_a/oracles.py` (+25) | ~1,300 |
| PR-4 MCP tool + docs | `nav_explore` descriptor, input parsing, `next_queries` for explore, golden re-bless, smoke count, `docs/MCP.md`, `SKILL.md` ("orient with one call"), `CLAUDE.md` tool list, adoption probes | `src/mcp/tools_explore.rs` (~200), `src/mcp/input_explore.rs` (~150), `src/mcp/explore_view.rs` (~120, keeps `evidence_view.rs` from growing), `tests/mcp/explore_tool_test.rs` (~200) | `src/mcp/mod.rs`, `registry` wiring (+10), `tests/mcp/smoke_test.rs` (9→10), 5 goldens, docs, `eval/adoption/goldens/probes.toml` (+6) | ~750 |
| PR-5 (after spec 04) | `dependence` section wired to `slice_at` | – | `explore.rs` (+60), tests (+80) | ~150 |

*Reconciled split and order (00-README X10, section 6).* PR-1 and PR-3 are over about 800 LOC, so each is split in
two to fit the 2-round review cap. The rest is unchanged:

| ID | Scope | Depends on |
|---|---|---|
| E-1a (PR-1a) | `ExploreOpts/ExploreResult`, seed resolution including `candidates` and `query` seeds (§4.1 reconciled), `definition`, ranked `callers`/`callees`, `blast_radius`, JSON output, CLI `explore --format json`, `mutants/explore.json` (ranking comparator); ~800 | 01 S1b, 06 S1, 01's gate (M1) |
| E-1b (PR-1b) | `explore_budget.rs`, `src/output/explore.rs` Markdown renderer, `module` section (06 `rank_files`), CLI markdown/text, budget tests, lane extension (fitter); ~550 | E-1a |
| E-2 (PR-2) | as PR-2 above, with `mutants/call-path.json` | E-1b |
| E-3a (PR-3a) | `type_index.rs` (Go and Rust declarations, `OnceLock`), type seeds, Rust implementations and overriders, CLI `implementations`, the Rust half of `implementations_test.rs`; ~650 | E-1b |
| E-3b (PR-3b) | Go satisfaction, the Go half of the tests, `eval/tools/implementations_oracle.py`, `eval/tier_a/oracles.py` (+25), the §9.4 run; ~650 | E-3a |
| E-4 (PR-4) | as PR-4 above; golden merges run 01 S1b → 06 S2 → E-4 | E-1b |
| E-5 (PR-5) | as PR-5 above; also routes the skill's "data flow and guards" routine to the `dependence` section (00-README section 5) | E-1b, 04 S-1b |

E-2, E-3a/E-3b, E-4 and E-5 can run in parallel after E-1b. E-4 still comes before the implementations work in
merge priority, as recommended below.

Then the eval runs: §9.3 inside PR-2's description, §9.4 inside PR-3's, §9.1 and §9.2 after PR-4 merges (one
report under `docs/analysis/`, raw runs in the private evidence store), owner decision per §9.2.

Mutation gate per CLAUDE.md tier 1 on every PR (`--since main --scope fn --authoritative`); PR-1 adds a lane
`mutants/explore.json` for the budget fitter and the ranking comparator, PR-2 a lane for the path enumeration
pruning conditions (these are the places an equivalent-looking mutant survives). No tier-2 trigger: resolution results,
oracles and execution context are unchanged.

## 12. Risks

| Risk | Effect | Mitigation / where it is measured |
|---|---|---|
| The composite replaces reading (the codegraph R3/R4 pattern) | recall loss on data and guards | no bodies; rows are read targets; `next_queries`; T4 gates the recommendation; arm C tests the call-site line |
| Tool-list growth | ~400 tokens on every run of every prism user | one tool, ≤ 900-char description pinned by test; no standalone MCP tools for items 3 and 5 |
| Precision-floor drops break paths and radii (click: 421 dropped sites) | `not_found`/under-count where the truth exists | result states the dropped count; status vocabulary distinguishes "no route through resolved edges" from "no route"; recall disclosure in §9.3 |
| Rust trait identity by name across modules; blanket and macro-generated impls | wrong or missing impl rows | scope-graph identity when resolvable, `nameonly` otherwise; blanket flagged; derive gap stated in `notes`; §9.4 measures |
| Go `GoMethodDeclaration` may lack signature text | name-set satisfaction over-approximates | labelled `nameonly` until signatures are compared; §9.4 precision gate on `exact` rows only |
| `TypeIndex` cost on very large trees | first explore slow under a shared daemon | parallel walk, Go/Rust files only, built once per provider; budget measured in PR-3 |
| Default content format `agent_markdown` departs from the other tools' canonical-JSON default | client surprise; one more thing to document | owner question Q1; `format: "json"` restores the convention per call |
| Budget fitting renders repeatedly | latency | binary search is O(log rows) renders per section; sections ≤ 200 rows |
| Golden re-bless hides unrelated drift | a wrong byte accepted as "new tool" | re-bless in PR-4 alone, with the diff limited to the `tools/list` line, checked in review |

## 13. Open questions for the owner (each with a recommendation)

1. **Content format default.** Recommend `agent_markdown` as `nav_explore`'s default content text (the tool exists
   to be read by an agent; one less knob, and the trial logged `bad arguments: view controls require format`
   retries). Alternative: canonical JSON like the other tools, with the Markdown behind `format`. *Reconciled
   (00-README OQ4):* the same default and `format` vocabulary as the ranked `nav_repo_map`.
2. **Standalone MCP tools for path and implementations.** Recommend no: CLI-only standalone, MCP as sections. The
   measured call counts for peers' equivalents (0 and 2 in 96 runs) do not justify ~800 tokens per run.
3. **Source text.** Recommend header line only, `snippets: "line"` opt-in, no body excerpt ever; let arm C decide
   whether the call-site line becomes the default.
4. **Section defaults.** Recommend `definition, callers, callees, blast_radius`; `module` off until item 6 lands,
   then on by default if its budget share stays under 10%.
5. **Budget default 8,000 bytes / cap 24,000; blast-radius depth 3 / `node_cap` 5,000.** Recommend as specified;
   both are per-call overridable. (00-README OQ5 asks whether to express the budget as `budget_tokens` with 06's
   shared estimator, so the two dedicated-result tools use one unit.)
6. **Type seeds and the call sections.** Recommend v1 answers `definition` + `implementations` only for a type seed
   (Q in §6.1); the per-type aggregate of callers is a follow-up if a consumer asks.
7. **Eval budget.** Recommend Haiku ×3 and Sol ×1 for §9.2 (≈ $1 per Haiku arm); add Sonnet ×1 (≈ $1.6, 24 runs)
   only if T1 passes on Haiku, since Sonnet called a navigation tool once in 120 trial runs.
8. **Order.** Recommend PR-1 → PR-2 → PR-4 → PR-3, so adoption is measured before the implementations work; if the
   owner wants the Go/Rust review lanes served first, PR-3 before PR-4 costs nothing but time. *Reconciled
   (00-README section 6, OQ24):* superseded by the cross-spec order. E-2, E-3a/b, E-4 and E-5 run in parallel after
   E-1b.
