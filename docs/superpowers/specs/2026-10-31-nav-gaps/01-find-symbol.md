# Find-symbol search without embeddings (gap item 1)

Date: 2026-10-31
Status: draft for owner review
Base: main `533f061a`
Companion spec: `06-ranked-repo-map.md` (item 6). Item 2 (composite explore) composes both; section 5.10 names what it
calls. Cross-spec decisions, tool budget, build order: `00-README.md`. This spec owns the unified seed grammar
(sections 5.1-5.3; summary in 00-README section 3) and `SymbolCatalog` (section 5.10).

**Claim tags.** *measured* = a number from a prism run or an evidence file, cited; *source-verified* = read in the code
at the cited `file:line` on `533f061a`; *simulated* = computed by a Python re-implementation of the proposed rules over
prism's CLI output (non-binding); *judgment* = my estimate or design choice. Pilot scripts and recorded outputs:
[`spec-pilots/pilot-06-01/`](../../../analysis/2026-10-31-nav-gaps/spec-pilots/pilot-06-01/) (`seeds.tsv`, `seeds-today.tsv`, `findsim2.py`, the `findsim2.out`
section of `RESULTS.md`, `RUN.txt`).

---

## 1. Goal and the decision it serves

**Goal.** Remove the first-call failure of name-based navigation and give agents a ranked, structured answer to "what
is named like X?": substring, prefix, camel/snake-word and bounded fuzzy matching over callables, with kind, file,
owner, language and role filters, where every result carries a seed handle that the other nav tools accept unchanged.

**Decisions this serves.**

1. *Seed friction:* whether fixes inside the existing seeded tools (qualified names, a `name` field on location seeds,
   ranked and explained disambiguation) bring first-call seed resolution to >= 90% on the trial seeds and Part-D def
   sites. That is slice S1, with no new MCP tool.
2. *Discovery:* whether to expose the search as the one new MCP tool `nav_find_symbol` (slice S2). It also serves as
   item 2's seed step for partial names. The gate between S1 and S2 is in section 8.4.

## 2. Non-goals

- No embeddings or semantic search (gap analysis: no measured lift anywhere in prism's evidence; a model dependency;
  an index-format problem).
- No BM25 over doc comments in v1. Assessed in section 5.9: not worth it now.
- No type, class, interface or trait *declarations* as results. Seeds are callables only
  (`SymbolRef::{Function, Statement, Variable}`, source-verified `src/navigation/types.rs:19-48`); a type seed is item
  5's explore-local `TypeRef` (`02-composite-explore.md` section 6.1; *reconciled, 00-README X6:* no `SymbolRef::Type`
  in v1). v1 does match owner names (section 5.5).
- No statements, variables or fields; no cross-repository search; no ranking by git history.
- No change to diff-review output or to any existing successful query result.

## 3. Current state

- **Seeds are exact names.** `seed::resolve_fn` filters `name_index` with `n == name` and an optional exact `file`;
  0 hits gives `SymbolNotFound`, 2 or more give `AmbiguousSymbol { candidates }` in node-index order
  (source-verified: `src/navigation/seed.rs:113-160`). A location seed resolves to the innermost named function
  containing the line, and wins over `symbol` when both are given (`seed.rs:119-131`;
  `NavigationIndex::enclosing_function`, `src/navigation/mod.rs:579-620`). MCP seeds are
  `{kind:"symbol", name, file?}` or `{kind:"loc", file, line}` with `deny_unknown_fields`
  (source-verified: `src/mcp/input.rs:25-38`).
- **Disambiguation help is fixed text.** The MCP error keeps at most 20 candidates (`MAX_ERROR_CANDIDATES`,
  `src/mcp/error.rs:7`) and always says "add the `file` field to the symbol seed" (`error.rs`, `actionable_sentence`).
  Candidates carry no owner, kind or caller count. The advice is wrong whenever the candidates share a file:
  click `invoke` has 7 candidates, 5 in `src/click/core.py`; httpx `send --file httpx/_client.py` is still ambiguous
  (`Client.send`, `AsyncClient.send`); click `main --file src/click/core.py` returns 3 candidates, two of them
  `@t.overload` stubs (measured: the `click-callers-invoke.json` row of [PROBES.md](../../../analysis/2026-10-31-nav-gaps/gap-analysis/PROBES.md), raw output in the private evidence store; [`seeds-today.tsv`](../../../analysis/2026-10-31-nav-gaps/spec-pilots/pilot-06-01/seeds-today.tsv); [`RUN.txt`](../../../analysis/2026-10-31-nav-gaps/spec-pilots/pilot-06-01/RUN.txt)).
- **Qualified spellings fail.** `Client.send` gives `SymbolNotFound` (measured); by construction every qualified
  spelling fails, since callable names never contain the separator (source-verified: exact `n == name` match).
- **Scale of the wall:** axum `into_response` has 85 candidates (32 in `axum-core`, 16 `axum-extra`, 15
  `axum-macros`, 11 `axum`, 11 `examples`; measured, the `axum-callers-into_response.json` row of [PROBES.md](../../../analysis/2026-10-31-nav-gaps/gap-analysis/PROBES.md)).
- **`nav functions` is CLI-only** and unranked: it reloads the repository without the nav cache
  (`crate::repo_loader::load_repo`) and prints file, name, lines and node kind, but no owner (source-verified:
  `src/navigation/inventory.rs:15-41`; `src/cli.rs` `Functions`; not registered in `src/mcp/tools.rs:25-75`).
- **Facts already available for ranking:** owner per method (`CallGraph::method_owners`) and trait dual-keying
  (`CallGraph::methods`, keyed `(owner_key, method)` with trait impls also under the trait name; source-verified
  `src/call_graph.rs`, `CallGraph` fields); `FunctionInfo.owner` and `kind_id` (`src/ast.rs:192-206`); direct
  callers per `FunctionId` with confidence (`NavigationIndex::direct_callers`, `src/navigation/mod.rs:299`); file role
  (`code_context::classify_file`). Function identity `(file, name, start_line)` is the CPG's own key
  (`anon_owner::owner_key` at `src/cpg/build.rs:961`).
- **Gaps that bound recall:** Rust trait method *declarations* without a body are not indexed (measured:
  the `Sink::matched` trait method declaration in `crates/searcher/src/sink.rs` is absent from the ripgrep inventory while its five impls are
  present). Anonymous callables are reachable only by location (`seed.rs:34-45`).
- **Today's resolution rate on the trial seeds** (measured, [`seeds-today.tsv`](../../../analysis/2026-10-31-nav-gaps/spec-pilots/pilot-06-01/seeds-today.tsv); 17 single-callable seeds named in 14
  trial questions on httpx, ripgrep, cobra, excalidraw; secbench seeds not run; R5 `Sink::matched` is a set query and
  type names are out of scope):

| First call | Unique and correct |
|---|---:|
| bare name, as an agent types it (`request`, `send`, `run`) | 9/17 |
| name + the file the question names, when it names one | 13/17 |
| best single call the question allows (location seed where a `file:line` is given) | 14/17 |
| qualified spelling as written in the question (`Client.send`) | 0 (SymbolNotFound by construction) |

**What changes.** S1: a shared callable catalog and matcher; qualified symbol seeds; a `name` field on location
seeds; ranked, explained disambiguation in errors; CLI `nav find-symbol`. S2: the MCP tool `nav_find_symbol`.

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
> trial's adoption signal favoured a tool whose name says what it does. Owner question 1 below.

## 5. Design

### 5.1 The seed handle: a `name` on location seeds

Every search result, every disambiguation candidate and every item 6 map symbol carries

```json
{"kind": "loc", "file": "httpx/_client.py", "line": 879, "name": "send"}
```

`line` is the callable's `start_line`. The MCP `loc` seed variant gains an optional `name` (string, at most 512 bytes);
the schema change is one property on the shared `seed_schema` used by `nav_symbol_spans`, `nav_callers`,
`nav_callees` and `nav_ego_graph`. Resolution with `name` present:

1. Candidates are the functions in `file` whose name equals `name` (`name_index[(file, name)]`).
2. If one candidate starts on `line`, it is the seed. `(file, name, start_line)` is the CPG function key, so this is
   unique; if two nodes ever share it, the result is `AmbiguousSymbol` (defensive; 0 such duplicates in the four
   pilot corpora, measured over the FunctionTable inventory).
3. Otherwise, the innermost candidate containing `line` is the seed (so a body line plus the name also works).
4. Otherwise `SymbolNotFound { seed: "loc:<file>:<line>#<name>" }` with the hints of section 5.4.

*Reconciled (00-README X4, X13):* the property is also added to `taint_reaches`' `sources` and `sinks` seed schemas
(six inlined copies in all, +150 `tools/list` chars), and to item 4's `slice_at` and item 2's `nav_explore` when
they land. A `loc` seed with `name` is a callable handle everywhere: in the node-level tools (`taint_reaches`,
`slice_at`, which resolve through `reasoning::seeds`) it means exactly what a symbol seed resolved to that callable
means (its parameters as sources; item 4's symbol-seed start sets, including `variable`).

Without `name`, location seeds behave exactly as today. **Round-trip guarantee:** for every named, non-anonymous
callable `f`, `resolve(handle(f)) == f`. It holds by construction (step 2) and is tested over every fixture function
and checked on all functions of the eval corpora (M2, section 8).

CLI: `--location F:L --symbol N` together mean the same handle (today location silently wins and `--symbol` is
ignored; behaviour change: a mismatching name is now an error instead of being ignored). `symbol-spans` relaxes its
`conflicts_with` accordingly.

### 5.2 Qualified symbol seeds in the existing tools

`{kind:"symbol", name:"Client.send"}` (and the CLI `--symbol`) resolves as follows:

1. Exact name match first, exactly as today, so a callable whose indexed name contains a separator keeps working.
2. If there is no exact match and the string contains a qualifier separator, parse it (section 5.3), keep candidates
   whose name equals the parsed name and whose qualifier penalty `q <= 1` (owner, trait key, file stem or directory
   match), apply `file` if given, and drop Python `@overload` stubs when exactly one non-stub remains (section 5.5).
3. One candidate -> resolved. Several -> `AmbiguousSymbol` with ranked, explained candidates. None ->
   `SymbolNotFound` with hints.

This is exact matching on structure, never fuzzy: a nav query never silently runs on a symbol the agent did not name.

*Reconciled (00-README X4):* steps 1-3 and the handle selection of section 5.1 live in one callable-selection
routine in `navigation::seed`, called by both `seed::resolve_fn` (nav tools, item 2) and
`reasoning::seeds::resolve_symbol` (`taint_reaches`, item 4's `slice_at`; source-verified: it does its own
`functions_named` lookup today, `src/reasoning/seeds.rs:213-300`). So qualified names, handles, `details` and
`did_you_mean` behave the same in every seeded tool.

### 5.3 Query parsing (shared by search, qualified seeds and item 6 focus)

- Go method spellings `(*T).m` and `(T).m` are rewritten to `T.m`.
- The string is split on `::`, `.`, `#` and `->`; the last segment is the **name pattern**, the others are
  **qualifiers**. Only the last qualifier is used in v1 (on the pilot it was sufficient for every seed; judgment).
- `query` must be 1-200 bytes after trimming; an empty name segment is `bad arguments`.

### 5.4 Disambiguation in errors

`QueryError::AmbiguousSymbol` gains an additive field, serialized only when non-empty:

```rust
AmbiguousSymbol {
    candidates: Vec<SymbolRef>,                                   // now in search-rank order (5.5)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    details: Vec<CandidateDetail>,                                // same order and length as candidates
}
pub struct CandidateDetail {
    pub owner: Option<String>, pub trait_key: Option<String>,
    pub kind: SymbolKind,                                         // function | method
    pub role: CodeRole, pub overload_stub: bool,
    pub callers_exact: usize, pub callers_name_only: usize,
    pub seed: SeedHandle,                                         // section 5.1
}
```

`SymbolNotFound` gains `did_you_mean: Vec<CandidateDetail>` (at most 5, match tiers 0-4 of section 5.5, skipped when
empty). The MCP cap of 20 stays, but it now keeps the 20 best-ranked candidates instead of the first 20 by node index.

The actionable sentence becomes computed (`error.rs`): candidates in more than one file -> "add `file`"; all in one
file -> "use a location seed with name, e.g. {kind:"loc", file, line, name} from `details[i].seed`"; owners differ ->
"or qualify the name, e.g. `Owner.method`". Behaviour change: the `AmbiguousSymbol` payload order and text change
(not covered by the golden transcripts, which contain no ambiguous call; source-verified by grep of
`tests/fixtures/mcp/golden/`); unit tests asserting the old order are updated with that reason.

### 5.5 Matching, filters and ranking (`symbol_search.rs`)

**Corpus:** every named callable in the nav CPG, excluding anonymous callables and module owners
(`anon_owner::is_anonymous_owner_name`, `is_module_owner_name`). Per callable the shared `SymbolCatalog` stores:
`FunctionId`, byte span, owner (`method_owners`), trait keys (other keys under which `methods` lists it), kind
(`method` if it has an owner, else `function`), language label, role, `overload_stub` (Python function decorated with
`overload` or `typing.overload`/`t.overload`), path depth, lowercase name, name words, and direct caller counts split
Exact/NameOnly.

**Name tiers** `t_name` (lower is better):

| Tier | Rule (case-insensitive unless stated) | Example query -> name |
|---:|---|---|
| 0 | exact, case-sensitive | `send` -> `send` |
| 1 | equal after normalisation (lowercase; drop `_`, `-`; strip Rust `r#`) | `get_user` -> `getUser` |
| 2 | prefix; or glob match when the pattern contains `*` or `?` | `merge` -> `merge_headers` |
| 3 | word prefix: each query word (camel/snake/digit split) is a prefix of a later name word, in order; or the query letters are the initials of consecutive name words | `rend elem` -> `getRenderableElements`; `gre` -> `getRenderableElements` |
| 4 | substring | `escape` -> `backslash_escape` |
| 5 | fuzzy: subsequence, query >= 3 chars, matched window <= 3 x query length | `rdrEl` -> `renderElement` |

`match` controls tiers: `exact` -> 0-1; `prefix` -> 0-3; `auto` (default) -> 0-4, plus tier 5 only when tiers 0-4
produce fewer than `max_results` matches; `fuzzy` -> 0-5.

**Qualifier penalty** `q` (only when a qualifier is present; last qualifier `L`, compared case-sensitively, then
case-insensitively): 0 if `L` equals the owner or a trait key; 1 if `L` equals the file stem, the parent directory
name, or the Go package directory; otherwise 4 (listed after every qualified match, not removed).

**Filters** (applied before ranking): `kind` (`function | method | any`); `file` glob over the repository-relative path
(`**` any characters, `*` any except `/`, `?` one character except `/`; no wildcard = exact path or directory prefix);
`owner` (owner or trait key, exact); `language` (onboarding labels: python, javascript, typescript, tsx, go, java, c,
cpp, rust, lua, terraform, bash); `role` (`production | test | any`, default `any`).

**Order:** sort key `(t_name + q, overload_stub, role [production first], -(5*callers_exact + 3*callers_name_only),
path_depth, file, start_line)`. Displayed `score = round4(1 / (1 + t_name + q))`; items are emitted already sorted,
so the existing score-then-location sorting of Evidence consumers does not reorder ties differently (ties carry equal
scores and are emitted in key order; the view layer keeps input order for equal scores, which S2 asserts).

**Owner names as queries:** when the name pattern equals an owner key at tier 0-1 and no callable matches, the methods
of that owner are returned (tier 2, `matched_on: owner`) so `HTTPTransport` or `IntoResponse` yields its methods or
trait impls instead of nothing. Type declarations themselves remain item 5.

### 5.6 `nav_find_symbol` (MCP) and `nav find-symbol` (CLI)

**MCP input:**

| Property | Type | Default | Limits |
|---|---|---|---|
| `query` | string | required | 1-200 bytes |
| `kind` | `function \| method \| any` | `any` | |
| `file` | string (glob) | none | <= 512 bytes |
| `owner` | string | none | <= 512 bytes |
| `language` | enum (12 labels) | none | |
| `role` | `production \| test \| any` | `any` | |
| `match` | `auto \| exact \| prefix \| fuzzy` | `auto` | |
| `max_results` | integer | 20 | 1-100 |
| `format`, `snippets` | existing enums | `canonical_json`, `none` | `snippets: symbol_header` shows the definition line, never a body |

Description (target <= 700 chars): "Finds callables by name: exact, prefix, camelCase/snake words, substring, bounded
fuzzy. Accepts qualified names (`Client.send`, `Owner::m`, `(*T).m`). Filters: kind, file glob, owner, language,
role. Use when you do not know the exact name or file, or a seed was ambiguous; do NOT use for callers/callees (use the
returned seed with nav_callers/nav_callees). Each item's seed is {kind:"loc", file, line, name}." (plus the snapshot
hedge).

**Result:** canonical `Evidence` (the house shape for lists; gets agent views, code roles and grouping for free), built
by `to_evidence(&SymbolSearchResult)`:

- one `EvidenceItem` per match: `symbol` = `SymbolRef::Function`; `location` = the callable span; `score` as above;
  `source: PrismCpg`; `fallback: false`; `snippet` per `snippets`;
- `why` uses three additive `Reason` variants, emitted only by this query (so no existing output changes):
  `Match { tier: u8, on: "name" | "owner" }`, `Owner { owner: String, trait_key: Option<String> }`,
  `CallerCount { exact: usize, name_only: usize }`;
- `query` = `find-symbol:<query>`; `truncated` and a `ResultTruncated` warning when matches exceed `max_results`;
  a `SkippedPath` warning when the `file` filter matches only skipped files.

The seed handle is derived from `symbol` (`file`, `start_line`, `name`), a rule stated in the tool description and
in `docs/MCP.md`; in `agent_json` views each item's `symbol_ref` already carries those fields, and a new
`NavigationViewKind::FindSymbol` adds up to 3 `next_queries` of the form `nav_callers {seed: <handle>}` for the top
items (`src/mcp/evidence_view.rs`, 1,510 weighted lines; about +40 lines, or a new `evidence_view_find.rs` if the
reviewer prefers no growth there).

**CLI:** `prism nav find-symbol --repo R --query Q [--kind K] [--file GLOB] [--owner O] [--language L] [--role R]
[--match M] [--max-results N] [--format text|json]`; text prints one line per match:
`score  file:start-end  Owner.name  [method, production]  callers 12 exact / 3 name-only  seed: file:line#name`.

**Errors:** `bad arguments` for an empty or over-long query, an unknown enum value, `max_results` outside 1-100.
Zero matches is a successful empty result with a hint warning ("no callable matches; try match: fuzzy or drop
filters"), not an error.

### 5.7 Determinism and confidence

- The catalog is built from `BTreeMap`-ordered inputs; matching is a pure function of the query and catalog; the sort
  key ends in `(file, start_line)`, which is unique per callable. No floats in the sort key.
- A match asserts that a callable with that name exists at that span: syntactic, so Exact by the static-binding
  definition in CLAUDE.md. Owner and trait keys come from the S3 owner index (syntactic). Matching tier is not
  confidence and is never shown as such. Caller counts are split Exact/NameOnly and inherit the precision floor:
  callers dropped as multi-owner collisions are not counted (the item's `CallerCount` says "resolved callers").

### 5.8 Language coverage

All 11 languages have callables in the catalog. Owners exist where `Language::method_owner` defines them: Rust
(impl/trait), Go (receiver), Python and JS/TS/TSX (class body), Java (class/enum), C++ (in-class and `Foo::bar`
out-of-line), Lua (`M.f`/`M:f`); C, Bash and Terraform have free functions only (source-verified:
`src/languages/mod.rs:1173-1295`). Not covered: trait/interface method declarations without bodies (not indexed,
measured on ripgrep), anonymous callables, Go promoted methods (aliases, not definitions; the defining method is
returned), TS overload signatures without bodies.

### 5.9 Lexical BM25 over identifiers and doc comments: not worth it now

- Identifier word matching already covers the reuse questions: one natural keyword per trial reuse question put
  gold-file helpers first among production results: httpx `merge` 6 of 6 production results in gold files
  (`_merge_headers`, `_merge_cookies`, `_merge_queryparams`, ...), ripgrep `escape` 9/9 and `unescape` 2/2, cobra
  `RangeArgs` 1/1, excalidraw `debounce` 2/2 and `throttle` 5/10 (measured over the inventory with the section 5.5
  tiers; keywords chosen by me after reading the questions, so this is an existence check, not a score).
- Agents issue keyword queries far more often than natural-language ones (4,813 vs 587 on CodeScaleBench,
  vendor-reported, from the archived research note).
- Doc comments are not in the index: extraction is per language (docstrings, `///`, Go leading comments, JSDoc), adds
  memory, and needs its own tests; reuse gold is unscored, so no lift could be shown.
- **Revisit trigger:** after S2 ships, if more than 20% of logged `nav_find_symbol` queries are multi-word phrases
  that return zero results, spec a doc-comment index then.

### 5.10 Reusable API for item 2

```rust
// src/navigation/symbol_catalog.rs (shared with item 6)
pub struct SymbolCatalog { /* entries in (file, start_line) order */ }
impl SymbolCatalog { pub fn get(s: &NavigationSession) -> &SymbolCatalog; }   // lazily built, cached per index
pub struct CatalogEntry { pub id: FunctionId, pub symbol: SymbolRef, pub owner: Option<String>,
    pub trait_keys: Vec<String>, pub kind: SymbolKind, pub language: &'static str, pub role: CodeRole,
    pub overload_stub: bool, pub callers_exact: usize, pub callers_name_only: usize,
    pub callers_exact_cross_file: usize, pub callers_name_only_cross_file: usize }  // cross-file: item 6 section 5.4

// src/navigation/symbol_search.rs
pub struct SymbolQuery { pub query: String, pub kind: KindFilter, pub file: Option<String>, pub owner: Option<String>,
    pub language: Option<String>, pub role: RoleFilter, pub mode: MatchMode, pub max_results: usize }
pub struct SymbolMatch { pub entry: CatalogEntry, pub tier: u8, pub qualifier_penalty: u8, pub matched_on: MatchedOn }
pub struct SymbolSearchResult { pub matches: Vec<SymbolMatch>, pub total_matches: usize, pub warnings: Vec<Warning> }
pub fn find_symbols(s: &NavigationSession, q: &SymbolQuery) -> SymbolSearchResult;
pub fn to_evidence(r: &SymbolSearchResult, query: &str) -> Evidence;
pub fn parse_qualified(query: &str) -> ParsedQuery;                            // section 5.3

// src/navigation/seed.rs
pub struct SeedHandle { pub file: String, pub line: usize, pub name: String }
impl SymbolMatch { pub fn seed(&self) -> SeedHandle; }
pub fn resolve_handle(s: &NavigationSession, h: &SeedHandle) -> Result<ResolvedFn, QueryError>;
```

Item 2's seed step: `find_symbols` with `max_results: 5`; if the top match is alone in its best tier, explore it; else
return the ranked candidates with their handles (the same contract as the `AmbiguousSymbol` details).
*Reconciled (00-README section 4):* item 2 calls `find_symbols` for its `query` seeds with `max_results: 20` (its
`candidates` section shows up to 20; `next_queries` carry the top 5), and calls `seed::resolve_fn` for symbol and
handle seeds, turning `AmbiguousSymbol.details` into `candidates`. `SymbolCatalog` is built in its own PR, N0,
before S1a and item 6's S1.

## 6. Data, cache and performance

- **Cache:** none. No `CACHE_VERSION` or nav call-edge sidecar change; owners, trait keys and callers are already in
  the cached call graph and call-edge index. Existing successful outputs are byte-identical; the changes are the
  `AmbiguousSymbol`/`SymbolNotFound` payloads (section 5.4) and the `tools/list` schema.
- **Catalog:** built lazily on first use from function nodes plus one pass over incoming edges, cached in a
  `OnceLock` on `NavigationIndex` (`src/navigation/mod.rs`, 1,149.5 weighted lines; +3 lines), so `refresh_index`
  and idle unload drop it with the index. Excalidraw has 8,606 inventory functions (measured); the largest stress
  corpora are judged at a few hundred thousand.
- **Budgets (pre-registered, measured in S1 with `PRISM_PROFILE`):** catalog build <= 300 ms and <= 200 bytes per
  callable on the largest stress corpus; `find_symbols` p95 <= 50 ms warm up to 10k callables and <= 200 ms on the
  largest corpus. If the memory bound fails, the fallback is no cache (rebuild per call), recorded in the PR.

## 7. Test plan (test-first variant fixtures)

Enumerate the variant space as fixtures before writing the matcher (internal lane notes (private): the test-first variant matrix rule):

- **Name classes:** exact; case variants; snake vs camel vs Pascal; prefix; word prefix; initials; substring; fuzzy
  inside and outside the 3x window; 1-2 character queries (no fuzzy); glob `*`/`?`; Unicode identifiers; Rust raw
  identifiers (`r#type`); Python dunders; JS inferred names (`const f = () => {}`), class-field arrow functions.
- **Identity classes:** same-file same-name methods in different classes (click `invoke`); Python `@overload` stubs
  plus implementation (click `Command.main`); C++ and Java overloads (same name, different lines); Rust trait impls
  (dual-keyed) and trait default methods; Go value and pointer receivers; Go promoted methods (not returned as
  definitions); anonymous callables and `<module>` (never returned); trait declarations without bodies (absent,
  documented).
- **Qualified spellings:** `Owner.m`, `Owner::m`, `(*Owner).m`, `module.f` (stem), `pkg.Owner.m` (last qualifier),
  unknown qualifier (q = 4, still listed), exact names containing a separator (exact match wins).
- **Filters:** each filter alone and combined; glob edge cases (`**`, trailing `/`, exact file, directory prefix);
  owner equal to a trait key; filter matching only skipped files (warning).
- **Ranking:** tier order; qualifier penalty; overload stubs after the implementation; production before test;
  caller weighting; path depth; final `(file, start_line)` tie-break.
- **Seed handle:** round-trip over every fixture function; `name` mismatch at the line (SymbolNotFound with hints);
  body line plus name (innermost); nested function on the outer function's start line; loc without `name` unchanged.
- **Errors:** candidates across files vs one file vs different owners (three sentences); more than 20 candidates
  (best 20 kept, "showing 20 of N" kept); `did_you_mean` ordering and its empty case.
- **Limits and negatives:** `max_results` 0/101; query empty, 201 bytes, only separators; unknown enum values.
- **Determinism:** two builds identical; `RAYON_NUM_THREADS=1` and `4`.
- **Golden transcripts** (re-blessed with the reason in each commit): id 3 (`tools/list`: seed `name` property in
  S1, new tool in S2) and id 12 in S2 (the unknown-tool error lists the available tools); add lines for a find-symbol call and
  a loc+name seed. S1 already changes id 3 once (the seed `name` property is a change to the existing seeded tools,
  so it ships with S1 even if S2 never does); S2 re-blesses id 3 again for the new tool. *Reconciled (00-README
  section 5):* the id-3 change and the new transcript steps (ids 30-34: a handle seed and a qualified symbol seed)
  belong to S1b, which also adds the `tools/list` size test (cap 17,500). S1b is the first golden merge
  (F1b → item 6 S2 → item 2 PR-4). Id 14 stays byte-identical: the shared `SeedInput` gains fields, not variants.
  S2 (F2, contingent) uses ids 50-54.

## 8. Evaluation

### 8.1 Sets

- **Set A:** every callable named in the 24 tracing-trial questions (texts in the private evidence store), spelled as the question spells it, with the
  question's file as the `file` filter when it names one. Pilot: 17 single-callable seeds on 4 corpora (`seeds.tsv`).
  The decision run adds the secbench-node-r1 seeds (J1 `darwin` in the `finders` object, J2 `find`, J4 the `darwin`
  finder) at the trial pin. R5 `Sink::matched` is reported as a set query (its trait declaration is not indexed);
  type-only names (`HTTPTransport`, `Renderer`, `Command`, `sinks::UTF8`) are reported as out of scope.
- **Set B:** the 11 runnable Tier-C Part-D tasks (`eval/tier_c/issues/structural.toml`): seed spelled with the task's
  receiver (e.g. `FastRegexMatcher.MatchString`), target = `def_site`; def sites that are interface or trait
  declarations (caddy `RequestMatcher`, hugo `Convert`, ruff `match_annotation`) are reported separately, since they
  are not callables.
- **Named failures from the gap analysis:** click `invoke` (`Context.invoke`, `Command.invoke`), click `Command.main`,
  axum `IntoResponse::into_response` (metric: all returned items are trait impls; count reported).

### 8.2 Metrics

- **M1 (S2):** `nav_find_symbol` first call puts the target at rank 1, alone in its best `t_name + q` tier.
- **M2:** handle round-trip `resolve(handle(f)) == f` for every named callable of every eval corpus. Must be 100%.
- **M3 (S1):** `nav_callers` with the question's spelling as a symbol seed (qualified seeds), plus `file` when the
  question names one, resolves to the target in one call.
- **No regression:** every seed that resolves today resolves to the same function after S1 (0 regressions allowed).

### 8.3 Pilot (measured baseline, simulated proposal)

| | Set A, 17 seeds |
|---|---:|
| today, bare name (measured) | 9/17 |
| today, best single call the question allows (measured) | 14/17 |
| proposed search, query only (simulated) | 15/17 (misses: `run`, `onChange`, bare common names) |
| proposed search, with the question's file filter (simulated) | 17/17 |
| proposed qualified seeds, M3 (simulated) | 17/17 |

The simulation parses owners with regexes, not with `method_owners`, so it is non-binding; it shows the rules are
sufficient on this set, not that the implementation will be. It also shows that **S1 alone (qualified seeds + file)
resolves the same seeds as the search**: the new tool's distinct value is discovery for partial or unknown names
(section 5.9) and ranked candidate lists for item 2.

### 8.4 Pre-registered decision thresholds

- **After S1 (CLI and seed fixes):** M2 = 100% and 0 regressions are hard requirements. If M3 >= 90% on Set A + B
  single seeds, seed friction is solved without a new tool.
- **S2 go/no-go (owner decision, recommendation attached):** build S2 if M1 >= 90% on Set A + B with file filters and
  >= 80% query-only, *and* the owner wants find-by-partial-name discovery on the MCP surface. If the owner prefers the
  smaller tool list, stop after S1: item 2 still calls `find_symbols` internally (section 5.10). Recommendation: build
  S2, because partial-name discovery is the one thing S1 cannot do and serena's empty `find_symbol` results from
  guessed name paths (7 of 22 in the trial; survey, via the gap analysis) are exactly that failure.
- *Reconciled (00-README X2):* the owner question is settled for now in favour of the smaller tool list. Item 2's
  `nav_explore` takes `{kind:"query"}` seeds through `find_symbols`, and **M1 (query-only >= 80%) gates those seeds**
  (if M1 fails, `query` seeds stay `bad arguments`). S2 becomes contingent (F2): built only on a later owner decision,
  for example if item 2's adoption gate T1 fails, and only within the program's `tools/list` cap.
- Report per corpus and per seed, never pooled only. Reuse discovery (section 5.9) is report-only; no threshold.

**Cost:** Set A minutes on the trial pins; Set B needs the Part-D corpora indexed at their SHAs (shared with item 6's
Set B run; run both in one host-quiet window). Harness: a Python script under `eval/` taking seed lists and repository
paths as arguments; no private data committed.

## 9. Slice plan

| PR | Content | Files | LOC (judgment) |
|---|---|---|---|
| N0 | *Reconciled (00-README X5):* `symbol_catalog.rs` (`CatalogEntry` of section 5.10 including the cross-file counts), `OnceLock` on `NavigationIndex`, fixtures; shared prerequisite of S1a and item 6's S1 | new `src/navigation/symbol_catalog.rs`, `src/navigation/mod.rs` (+3) | ~200 |
| S1a (F1a) | `symbol_search.rs` (parse, tiers, filters, ranking, `find_symbols`, `to_evidence`, the three `Reason` variants and their view/render arms), CLI `nav find-symbol`, fixtures, eval harness | new `src/navigation/symbol_search.rs`, `src/output/navigation.rs` (+30), `src/mcp/evidence_view.rs` (`view_reason` arms), `src/cli.rs` (+25), `src/main.rs` (+30), `eval/find_symbol/` | ~450 |
| S1b (F1b) | shared callable selection for both resolvers, `SeedHandle` and loc+name resolution (CLI, and the MCP seed `name` property on all six seed copies), qualified symbol seeds, `AmbiguousSymbol.details` and `SymbolNotFound.did_you_mean`, computed actionable sentence, `--location`+`--symbol`, `tools/list` size test (cap 17,500), golden id 3 + ids 30-34, `docs/MCP.md` Seeding paragraph, skill "Seed" step | `src/navigation/seed.rs` (+80), `src/reasoning/seeds.rs` (+20), `src/navigation/types.rs` (+25), `src/mcp/error.rs` (+50), `src/mcp/input.rs` (+15), `src/mcp/tools.rs` and `src/mcp/tools_reasoning.rs` seed schema (+6), `src/cli.rs`, `src/main.rs` | ~400 |
| gate | M2, regressions, M3; M1 (gates item 2's `query` seeds); Set B in the window shared with item 6 | - | - |
| S2 (F2, contingent) | MCP `nav_find_symbol`, its input parsing, `NavigationViewKind::FindSymbol` hints, golden re-bless (ids 3, 12) plus ids 50-54, `docs/MCP.md` row, skill table row ("I only know part of the name -> `nav_find_symbol`"); only on an owner decision within the cap | new `src/mcp/tools_search.rs`, `src/mcp/input.rs` (+50), `src/mcp/tools.rs` (+10 schema), `src/mcp/evidence_view.rs` (+40) | ~400 |

Cross-spec order (00-README section 6): N0 → S1a → S1b (parallel with item 6's S1 and item 4's S-1a) → gate. Item
4's S-1a merges after S1b, so `slice_at` is born with the unified grammar.

File-size rule (`scripts/file_size.py`, source-verified): `src/navigation/queries.rs` (2,534.5) is not touched;
`call_graph.rs` and `ast.rs` (over 2,000) are read-only here; `tools.rs` (1,322.5) and `evidence_view.rs` (1,510) get
only small additions and the S2 PR says why (the tool handler lives in `tools_search.rs`, like `tools_reasoning.rs`);
`seed.rs` (190) and `error.rs` (304.5) stay small. Mutation gate: tier 1 (`--since main --scope fn --authoritative`
plus lanes the PR adds). **Tier-2 trigger in S1 (S1b after the split):** the seed-resolution change alters which callable a seed resolves to
for qualified and loc+name seeds; add the lanes of `seed.rs` callers (`queries.rs` callers/callees/ego/symbol_spans,
`tools_reasoning.rs`) and name the trigger in the PR. Tier-A: not triggered (no call-resolution or CPG change).

## 10. Risks

- **Ranking surprises:** caller counts favour hubs that are not what the agent meant; mitigated by tier-first ordering
  and filters, and measured by M1.
- **Qualified-seed false confidence:** the last qualifier matching a file stem could pick a same-named function in a
  same-stem file elsewhere (e.g. two `utils.py`); such cases stay ambiguous because both candidates get `q = 1` and the
  seed path requires uniqueness. Tested.
- **Tool-list cost:** about +500 tokens per agent session for S2 (judgment from the char budget in section 4); the
  trial attributed Sonnet's +8-25% per-run cost to unused tool definitions. The size test caps it.
- **Memory:** the cached catalog on very large repositories; bounded in section 6 with a no-cache fallback.
- **Error payload change** may break a client that parses `AmbiguousSymbol` positionally; the change is additive in
  shape, order-changing in content, and documented.

## 11. Owner questions (each with a recommendation)

1. **New tool `nav_find_symbol` or fold into `nav_repo_map {query}`?** Recommend the new tool (section 4), gated by
   section 8.4. *Reconciled (00-README OQ1):* the program-level recommendation is to fold the search into
   `nav_explore`'s `query` seed and keep S2 contingent, because three new tools would exceed any cap near 16,500.
2. **Accept qualified names in existing symbol seeds (S1)?** Recommend yes: exact structural matching, never fuzzy,
   measured to remove the largest class of first-call failures.
3. **Return type declarations?** Recommend deferring to item 5's `SymbolRef::Type`; v1 returns an owner's methods when
   the query names an owner.
4. **BM25 over doc comments?** Recommend no (section 5.9), with the stated revisit trigger.
5. **Tests in results by default?** Recommend included and ranked after production (`role: any`), since several trial
   gold sets cite test files (P1, G1, G5).
6. **Reorder `AmbiguousSymbol` candidates by rank** (changes error content)? Recommend yes; keeping node-index order
   would keep the 20-candidate cap cutting the wrong 65 of axum's 85.
