# Design: Prism Navigation Layer (Tier 1) — v4 (planning-ready)

**Status:** Planning-ready — closes three spec-review rounds
**Date:** 2026-06-07
**Scope:** Whole-repo navigation/architecture layer only. The seeded *reasoning*
layer (`FocusSet`, taint/impact/data-flow) is the next initiative; seams here let
it sit on top.

## 0. Review disposition

| Finding | Resolution | §︎ |
|---|---|---|
| R1-B1 func_index collision | **Option C:** nav-local `line_range_index` + `name_index`; core untouched | §3, §19 |
| R1-B2 JSON schemas | serde structs + per-command/per-warning goldens | §8 |
| R1-B3 ingestion contract | full traversal/skip contract | §5 |
| R1-B4 grammar version | `build.rs` compile-time grammar fingerprint | §9 |
| R1-M1 function_at O(n) | nav-local `line_range_index`; core `function_at` never called by nav | §3, §7 |
| R1-M2 resolve_callers qualifier | nav does own qualifier-aware caller resolution; core fix is follow-up | §7, §19 |
| R1-M3 ego-graph spec | full capability + "no statement containment" | §8 |
| R1-M4 localization | out of scope (reasoning layer) | §2 |
| R1-M5 provenance | `source` + `fallback` on `Evidence` | §8, §14 |
| R1-M6 CLI flatten/matrix | flatten gated; compat matrix locks the dispatch-error text | §11, §12 |
| R2-B2 parse-failure enforcement | nav excludes `ParseFailed`; review lenient; excluded query → `SkippedPath`+empty | §5, §8 |
| R2-M1 score formula | `score = 1/(1+hop)`, ties `(file,start_line,ordinal)` | §8 |
| R2-M2 TypeRegistry/live_types | added to `NavigationIndex` | §3 |
| R2-M3 line_range_index semantics | innermost enclosing | §3, §7 |
| R2-M4 goldens incomplete | goldens for callees/ego/module_deps/repo_map + each `WarningKind` (illustrative) | §8 |
| R2-M5 compat fixture list | enumerated flag set + CI gate; no sanctioned delta | §12 |
| R2-m1 coverage arrays | **3 arrays in 3 functions** (`all_test_files` :106/:325, `test_files` :472); CLAUDE.md correct | §16 |
| R2-m2 grammar fingerprint | `build.rs` constant | §9 |
| R2-m3 resolver object-safety | session owns via `Arc`; `ResolverContext::Session(Arc<NavigationSession>)` | §3, §14 |
| R3-B1 session lifetime contradiction | `NavigationSession` owns `Arc<LoadedRepo>`/`Arc<NavigationIndex>` (no `'a`) | §3, §14 |
| R3-M1 golden line numbers | §8 goldens are **illustrative**; canonical fixtures generated from the live tool | §8, §17 |
| R3-M2 skip precedence / propagation | total skip precedence; per-file error never aborts; `SkippedPath` in `Evidence.warnings` | §5 |
| R3-M3 node-model coverage | `nodes-at` returns Function+Variable only; class/struct/module deferred | §8 |
| R3-M4 language precision tiers | per-language precision table | §10 |
| R3-M5 build constructor | nav index uses `CpgContext::build` (`scope=None`), not `build_scoped` | §17 |
| R3-m1 complexity label | `O(log f + k)`, not `O(log n)` | §3, §7 |
| R3-m2 flatten error artifact | lock the dispatch-error stderr text in the compat matrix | §12 |
| R3-m3 coverage count meta-fix | three arrays; CLAUDE.md "3 copies" is correct (not stale) | §16 |

## 1. Provenance

Clean-room codex design ⊗ Claude spec, hardened across three codex-rigor +
claude-soundness rounds. Convergent spine: opt-in repo-wide library over the
existing CPG/call graph; subcommand CLI flattening legacy review args (gated);
separate exact-hit nav cache; resolver seam; explainable output.

## 2. Goals and non-goals

**Goals.** Whole-repo navigation/architecture queries (`nodes-at`,
`callers`/`callees` with call-site evidence, bounded `ego-graph`,
`module-deps`/`repo-map`) over a whole-repo graph reusing the CPG/`CallGraph`;
clean library + thin CLI (MCP after stable); **diff-review byte-for-byte with no
re-baseline** (v1 makes zero CPG-core edits, §11); structured, explainable,
deterministic output; dogfoodable on this Rust repo.

**Deferred (intended, later) — §19 tracks the core ones:** the reasoning layer
(`FocusSet`); the CPG-core `func_index` re-key + qualifier-aware `resolve_callers`;
cross-repo via the SCIP/Glean resolver seam; an always-fresh maintained index
(only the churn-daemon — whole-repo indexing is in scope, §9); the DFG-less
`CallStructureExperimental` profile (§6); an incremental nav cache. **Rejected
(evidence-gated):** vector RAG, whole-repo long-context, CGM-style models. **Out
of scope (reasoning layer):** NL localization/scoring; class/module structural
nodes; `chop`/`dataflow`/`symbols`/`definition`/`references` as first-class
commands.

## 3. Ownership model + nav-local indexes (Option C)

`CpgContext` borrows `files` (`cpg.rs:56,60`); split ownership for long-lived
state, owning everything via `Arc` so no borrowed lifetime escapes into the
resolver seam (R3-B1):

```rust
struct LoadedRepo { root: PathBuf, files: BTreeMap<String,ParsedFile>,
                    file_hashes: BTreeMap<String,String>, skipped: Vec<SkippedFile>,
                    type_db: Option<TypeDatabase> }      // ParsedFile owns source (ast.rs:45)

struct NavigationIndex {
    cpg: CodePropertyGraph,
    types: TypeRegistry,                  // R2-M2: callee resolution for non-C/C++ langs
    live_types: BTreeSet<String>,         // R2-M2: RTA dispatch pruning
    profile: GraphBuildProfile,
    parse_quality: BTreeMap<String, FileParseQuality>,
    line_range_index: BTreeMap<String, Vec<(usize,usize,NodeIndex)>>,  // per file, sorted by start
    name_index:       BTreeMap<(String,String), Vec<NodeIndex>>,       // (file,name) -> all defs
}

struct NavigationSession { repo: Arc<LoadedRepo>, index: Arc<NavigationIndex> }  // owns, no 'a
```

**Option C — core is untouched.** `func_index` keeps its `(file,name)` key. The
collision risk (same-named `impl`-block fns overwrite, e.g. Rust `fn new`) is
neutralized *for navigation* by deriving two nav-local indexes from the
already-present `CpgNode::Function {start_line,end_line}` data (`cpg.rs:394-399`):

- `name_index` maps `(file,name) → Vec<NodeIndex>` of **all** defs, so a `symbol:`
  seed enumerates candidates → `AmbiguousSymbol{candidates}` (never a silent pick).
- `line_range_index` resolves a line to the **innermost enclosing** function
  (smallest `[start,end]` containing it — correct for closures/lambdas, R2-M3) in
  **O(log f + k)** (f = files, k = nesting depth ≤ ~5; R3-m1), without ever calling
  the core's O(n) `function_at` (`cpg.rs:1490`, R1-M1).

`cpg`/`types`/`live_types` are extracted (owned) from a whole-repo
`CpgContext::build` (§17). Durable API IDs are collision-safe, never raw
`NodeIndex`: `FunctionIdRef{file,name,start_line,end_line,ordinal}`,
`StatementRef{file,line,kind,ordinal}`,
`VariableRef{file,function,line,path,access,ordinal}` (`ordinal` orders same-key
defs by `start_line`).

## 4. (folded into §3)

## 5. Repository loading — traversal/skip + parse-failure contract

`src/repo_loader.rs` → `LoadedRepo`:

- **Discovery:** recursive walk; only `Language::from_path`-supported extensions
  (`languages/mod.rs:42`).
- **Skip precedence (R3-M2), highest first:** explicit `.gitignore` (root+nested)
  > built-in patterns (`.git/`,`target/`,`node_modules/`,`vendor/`,`dist/`,
  `build/`) > `Symlink` (not followed) / `Hidden` > `TooLarge` (>2 MiB) >
  `Unreadable`/`NotUtf8` > `Unsupported` extension > `ParseFailed`. The first
  matching rule sets the `SkipReason`.
- **Parse-failure enforcement (R2-B2):** files over the severe parse-error
  threshold (`check_parse_quality`, `algorithms/mod.rs:63-101`) are excluded from
  the **nav** graph (`ParseFailed`). The **diff-review path keeps its current
  lenient behavior unchanged** (this is a `repo_loader`/nav concern, build Step 3,
  not a CPG-core change).
- **Failure propagation (R3-M2):** a per-file read/parse error **never aborts** a
  load or a query; it is recorded as a `SkippedFile`. Keys repo-relative,
  lexically normalized, `/`-sep.
- **Query interaction:** a nav query whose seed names a skipped/excluded file
  returns **empty `items` + a `SkippedPath` warning in `Evidence.warnings`** (with
  the path's `SkipReason`), never a hard error.

```rust
struct SkippedFile { path: String, reason: SkipReason }
enum SkipReason { Unsupported, Ignored, Symlink, Hidden, TooLarge{bytes:u64},
                  Unreadable, NotUtf8, ParseFailed }
```

## 6. Graph build profile

`enum GraphBuildProfile { FullCpg, CallStructureExperimental }`. **v1 =
`FullCpg`** (assembly assumes a DFG; `DataFlowGraph::empty()` drops variable nodes
+ `Contains` edges). `CallStructureExperimental` deferred (needs CPG-core
function→statement containment + tests).

## 7. CPG constraints the query layer respects

- `nodes_at` exact-line only (`cpg.rs:1190`); functions indexed at `start_line`
  (`:747`) → nav uses `line_range_index` (O(log f + k)) for the enclosing
  function; labels exact vs enclosing.
- `Contains` exists function→variable (`cpg.rs:942`), **not** function→statement
  (`:950`).
- Call-site lines live in `CallGraph` (`cpg.rs:576`);
  `CallSite{caller,callee_name,line,qualifier}` (`call_graph.rs:21`).
- **`callers`/`callees` do their own qualifier-aware resolution in the nav layer**
  from `CallSite` + `resolve_callees_qualified` (`call_graph.rs:654`); they do
  **not** modify core `resolve_callers` (`call_graph.rs:801`). Core fix is a
  tracked follow-up (§19); v1 review output stays byte-identical.

## 8. Navigation query API + output model

Modules: `src/navigation/{types,resolver,queries,module_graph,cache}.rs`,
`src/output/navigation.rs` (each < 600 lines).

**Seed grammar:** `symbol:<name>[@<file>]` | `loc:<file>:<line>`. `symbol:`
resolving to >1 def via `name_index` → `AmbiguousSymbol{candidates}`.

**Node-model scope (R3-M3):** v1 `nodes-at` returns **`Function` and `Variable`
nodes** (and the enclosing function); class/struct/module are not first-class CPG
nodes and will not appear — deferred.

**Serde contract:**

```rust
struct Location { file: String, start_line: usize, end_line: usize }
enum SymbolRef { Function{..}, Statement{..}, Variable{..} }     // §3 *Ref types
enum Source { PrismCpg, HeuristicImport, ExternalIndex{ name: String } }
enum Reason {
  Calls{ callee: String, call_site_line: usize, qualifier: Option<String> },
  CalledBy{ caller: String, call_site_line: usize },
  EnclosingFunction{ function: SymbolRef },
  Containment{ parent: SymbolRef },
  ResolvedImport{ module: String, target_file: String },
  UnresolvedImport{ module: String },
}
struct EvidenceItem { symbol: Option<SymbolRef>, location: Location, score: f32,
                      source: Source, fallback: bool, why: Vec<Reason>,
                      snippet: Option<String> }              // snippet only with --snippets
struct Evidence { query: String, items: Vec<EvidenceItem>, truncated: bool,
                  warnings: Vec<Warning> }
struct Warning { kind: WarningKind, message: String, location: Option<Location> }
enum WarningKind { ParseQuality, AmbiguousSymbol, IndirectCallApprox,
                   UnresolvedModule, Collision, SkippedPath }
enum QueryError { AmbiguousSymbol{ candidates: Vec<SymbolRef> },
                  SymbolNotFound{ seed: String },
                  LocationOutOfRange{ file: String, line: usize },
                  UnsupportedFile{ file: String } }
```

**Determinism & scoring (R2-M1):** `score = 1.0 / (1 + hop_distance)` (integer
hops; seed/direct = 1.0); `items` ordered by `score` desc then
`(file, start_line, ordinal)`; `truncated=true` on `--max-results` clip. No
NL/BM25 scoring here.

**`ego_graph` capability:** `EgoEdges ⊆ {Call,Return,DataFlow,ContainsVariable}`
(no `ContainsStatement`); `direction ∈ {Out,In,Both}`; seed included;
breadth-ordered, deduped, cycle-guarded; output `{nodes:[SymbolRef+Location],
edges:[{from,to,kind,reason}]}`.

**Goldens — ILLUSTRATIVE ONLY (R3-M1).** The JSON below shows *shape*; line
numbers are not authoritative. **Canonical golden fixtures are generated by
running the tool against the live repo** (§16, §17 Step 6) — never hand-copied
from this spec. One representative per command/warning (full set lives with the
fixtures):

```jsonc
{ "query":"callers:build_scoped@src/cpg.rs",                 // callers (shape)
  "items":[{"symbol":{"Function":{"file":"src/algorithms/mod.rs","name":"run_slicing_compat",
            "start_line":0,"end_line":0,"ordinal":0}},"location":{"file":"src/algorithms/mod.rs",
            "start_line":0,"end_line":0},"score":1.0,"source":"PrismCpg","fallback":false,
           "why":[{"CalledBy":{"caller":"run_slicing_compat","call_site_line":0}}],"snippet":null}],
  "truncated":false,"warnings":[] }
{ "query":"callees:run_slicing@src/algorithms/mod.rs",        // callees, qualifier-aware
  "items":[{"symbol":{"Function":{"file":"src/algorithms/taint.rs","name":"slice",
            "start_line":0,"end_line":0,"ordinal":0}},"location":{"file":"src/algorithms/taint.rs",
            "start_line":0,"end_line":0},"score":1.0,"source":"PrismCpg","fallback":false,
           "why":[{"Calls":{"callee":"slice","call_site_line":0,"qualifier":"taint"}}],"snippet":null}],
  "truncated":false,"warnings":[] }
{ "query":"ego:src/cpg.rs:LINE","items":[],"truncated":false,"warnings":[],   // ego-graph
  "graph":{"nodes":[/* Function/Variable nodes */],
           "edges":[{"from":0,"to":1,"kind":"ContainsVariable","reason":{"Containment":{"parent":/*fn*/}}}]} }
{ "query":"module-deps:src/cpg.rs",                            // module map (call-derived + labeled)
  "items":[{"symbol":null,"location":{"file":"src/call_graph.rs","start_line":1,"end_line":1},
            "score":1.0,"source":"PrismCpg","fallback":false,
            "why":[{"Calls":{"callee":"CallGraph::build","call_site_line":0,"qualifier":"call_graph"}}],"snippet":null}],
  "truncated":false,
  "warnings":[{"kind":"UnresolvedModule","message":"import not filesystem-resolved","location":null}] }
{ "query":"callers:nonexistent","items":[],"truncated":false,  // empty
  "warnings":[{"kind":"AmbiguousSymbol","message":"no function named 'nonexistent'","location":null}] }
{ "query":"nodes-at:vendor/broken.js:10","items":[],"truncated":false,        // skipped path
  "warnings":[{"kind":"SkippedPath","message":"file excluded: ParseFailed","location":{"file":"vendor/broken.js","start_line":10,"end_line":10}}] }
{ "error":{"AmbiguousSymbol":{"candidates":[/* >1 FunctionIdRef */]}} }       // ambiguous (exit 3)
```

## 9. Caching

Separate nav cache namespace. Key (serialized stable): `prism_version`,
`cache_format_version`, `grammar_fingerprint`, `repo_root_id`, `file_set_hash`,
`graph_profile`, `skip_policy_version`, `type_db_key`, `supported_languages`. The
**grammar fingerprint** is a `build.rs`-generated compile-time constant over
`tree-sitter-*` crate versions (matching `env!("CARGO_PKG_VERSION")` at
`cpg_cache.rs:181,249`; R2-m2), **not** a runtime `Cargo.lock` read — closes the
stale-tree-after-`cargo update` bug. **v1 exact-hit only**: any change → Miss →
full rebuild.

## 10. Module/repo map + language precision tiers

Imports extracted only for Python/JS/TS/TSX/Go (`ast.rs:295`), as-is
(`ast.rs:288`), same-stem ambiguity (`call_graph.rs:651`). v1 derives the map from
*resolved call* file→file edges (`source:PrismCpg`) + optional raw imports labeled
`UnresolvedImport` (`source:HeuristicImport`).

**Per-language v1 precision (R3-M4):**

| Tier | Languages | Behavior |
|---|---|---|
| Exact (imports + calls) | Python, JS, TS/TSX, Go | resolved imports + call edges |
| Call-derived only | **Rust**, Java, C, C++ | call edges; imports surface as `UnresolvedImport` |
| Calls only, no imports | Lua, Terraform, Bash | call edges where present; no import map |

Nav integration tests must cover ≥1 call-derived-only language (Rust, this repo)
so the expected `UnresolvedImport`/degraded behavior is encoded, not silent.

## 11. Preserve diff-review (byte-for-byte, no re-baseline)

v1 makes **zero CPG-core edits** (Option C, §3) → diff-review goldens are
byte-identical, no re-baseline. Work = extract the review path into `ReviewArgs`
(mechanical) + the additive nav layer. Binary is `prism`, clap app name is
`slicing` (`Cargo.toml:9`, `main.rs:38`) — **left unchanged deliberately** (R2-m5;
renaming reshapes `--help`/the compat matrix; eventual rename is a separate item).

## 12. CLI seam + compatibility matrix

```rust
struct Cli { #[command(subcommand)] command: Option<Command>,
             #[command(flatten)] review: ReviewArgs }
enum Command { Nav(NavArgs), Mcp(McpArgs) }
```

**Flatten hazard gated (R1-M6, R3-m2):** with a subcommand present, dispatch
asserts the flattened review-only fields are at defaults; a non-default review
flag under `nav`/`mcp` (e.g. `prism nav callers --diff x`) is a **hard usage
error** at dispatch, not a silent no-op. This is an application-level error (clap
parses it first), so its **exact stderr text is captured in the compat matrix**
so future wording changes are intentional, not silent regressions. `None` runs
the extracted review path unchanged.

**Compatibility matrix (R2-M5), regression-locked, NO sanctioned delta:** golden
captures of stdout bytes, stderr bytes, exit code, `--help` text, validation-error
text, and the nav dispatch-error text for: the bare `--repo/--diff` invocation;
`--algorithm` in `review` + `leftflow`,`taint`,`chop`,`thin`; `--format
{text,json,paper,review}`; `--list-algorithms`. A CI step diffs these and fails on
any deviation.

```text
prism --repo . --diff changes.patch --algorithm review        # byte-identical
prism nav callers  --repo . --symbol build_scoped --depth 2 --format json
prism nav nodes-at --repo . --location src/main.rs:498 --format json
prism nav repo-map --repo . --format json
```

## 13. MCP adapter (after CLI/library stable)

`src/bin/prism-mcp.rs`, thin over `src/navigation`, exposing `nodes_at`,
`callers`, `callees`, `ego_graph`, `repo_map` (→ `Evidence` JSON). SDK `rmcp`,
validated by a spike; fallback to minimal stdio JSON-RPC. After the nav-local
indexes + library/CLI are fixed.

## 14. Resolver seam

```rust
enum ResolverContext { Session(Arc<NavigationSession>), ExternalIndex(PathBuf) }
trait SymbolResolver {
  fn definition(&self, cx: &ResolverContext, sym: &FunctionIdRef) -> Vec<(Location, Source)>;
  fn references(&self, cx: &ResolverContext, sym: &FunctionIdRef) -> Vec<(Location, Source)>;
}
```

Since `NavigationSession` now owns via `Arc` (no lifetime, §3, R3-B1),
`ResolverContext` is `'static` and the trait is object-safe. `HeuristicResolver`
(default) uses the session; a future `ScipResolver` reads a `.scip` path. Results
carry `Source`; `fallback=true` marks a heuristic fallback after an external miss.

## 15. Evaluation seam

`Evidence` is structured/comparable for a later A/B vs an agentic-search baseline
(localization precision/recall, token cost), per language. Built when there are
tools to measure.

## 16. Testing — named golden fixtures

Per-scenario fixtures (small multi-file repo + expected `Evidence`/error JSON,
**generated from the tool, not hand-copied from §8**, R3-M1): **duplicate
same-name functions** (Rust `impl`-block `fn new` → `AmbiguousSymbol`/candidate
enumeration), **static/free functions**, **qualified/imported calls**,
**closures/lambdas** (innermost-enclosing), **skipped files** (each `SkipReason`
incl. `ParseFailed`→`SkippedPath`, exercising the §5 precedence), **cache
invalidation** (content, file add/remove, grammar-fingerprint bump), **CLI legacy
compatibility** (§12 matrix incl. the nav dispatch-error text), **empty results**,
**nodes-at exact+enclosing** (Function/Variable only), **ego-graph** shape,
**module/repo map** labeling on a **call-derived-only language (Rust)**, and a
**dogfood smoke** (`repo-map` + `callers build_scoped` on this repo). When adding
test files, update **all three** arrays in `tests/integration/coverage_test.rs`:
`all_test_files` (`:106`, `:325`) and `test_files` (`:472`) — CLAUDE.md's "3
copies" instruction is correct (R3-m3).

## 17. Build order

1. Extract review path into `ReviewArgs` — **no behavior change**; §12 compat
   matrix green. (No CPG-core edits.)
2. `repo_loader.rs` + `LoadedRepo` (full §5 contract incl. precedence + ParseFailed).
3. `NavigationIndex` via **`CpgContext::build(files, type_db)` — `scope` must be
   `None` (whole-repo); never `build_scoped` (R3-M5)**; derive `line_range_index`
   /`name_index`; capture `types`/`live_types`; nav cache key (§9, exact-hit).
4. `navigation/types.rs` (serde §8) + `resolver.rs` (§14).
5. `nodes-at` (exact + enclosing; Function/Variable only).
6. `callers`/`callees` (nav-local qualifier-aware, CallSite evidence); **generate
   the canonical goldens from the live tool** here.
7. bounded `ego-graph` (§8).
8. `prism nav …` CLI + `output/navigation.rs` JSON.
9. exact-hit nav cache wired in.
10. `module_deps`/`repo_map` (labeled, §10).
11. MCP adapter.
12. (Follow-ups, §19.)

## 18. Module/file plan

New: `src/repo_loader.rs`;
`src/navigation/{mod,types,resolver,queries,module_graph,cache}.rs`;
`src/output/navigation.rs`; `src/bin/prism-mcp.rs`; `build.rs`.
Modified (**additive only — no core logic edits**): `src/main.rs` (subcommand +
`ReviewArgs`); `src/lib.rs`; `Cargo.toml` (MCP bin + `rmcp` + `build.rs`);
`tests/integration/coverage_test.rs` (all three arrays).

## 19. Tracked follow-ups (separate slices, own goldens)

1. **CPG-core `func_index` re-key** to `(file,name,start_line)` so the *review*
   path is also collision-safe — invasive (`cpg.rs`, `cpg_cache.rs` incl.
   `reconstruct_cpg`, `data_flow.rs`, `call_graph.rs`, algorithms), needs a
   reviewed golden re-baseline.
2. **Qualifier-aware core `resolve_callers`** (`call_graph.rs:801` → forward to
   `resolve_callees_qualified`) — may shift call-graph-using algorithm output; own
   regression review.
3. `CallStructureExperimental` profile (needs function→statement containment).
4. Class/struct/module CPG nodes (enables richer `nodes-at`/`ego-graph`).
5. The reasoning layer (`FocusSet`, taint/impact/data-flow).
