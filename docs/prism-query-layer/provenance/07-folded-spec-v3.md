# Design: Prism Navigation Layer (Tier 1) — v3 (planning-ready)

**Status:** Planning-ready — closes both spec-review rounds
**Date:** 2026-06-07
**Scope:** Whole-repo navigation/architecture layer only. The seeded *reasoning*
layer (`FocusSet`, taint/impact/data-flow) is the next initiative; seams here let
it sit on top.

## 0. Review disposition

| Finding | Resolution | §︎ |
|---|---|---|
| R1-B1 func_index collision | **Option C:** nav-local `line_range_index` + `name_index` (from existing `Function` nodes); core untouched. Core re-key is a tracked follow-up | §3, §19 |
| R1-B2 JSON schemas | serde structs + per-command/per-warning goldens | §8 |
| R1-B3 ingestion contract | full traversal/skip contract | §5 |
| R1-B4 grammar version | `build.rs` compile-time grammar fingerprint in cache key | §9 |
| R1-M1 function_at O(n) | nav-local `line_range_index` (binary search); core `function_at` never called by nav | §3, §7 |
| R1-M2 resolve_callers qualifier | nav does its own qualifier-aware caller resolution from `CallSite`; core fix is the follow-up | §7, §19 |
| R1-M3 ego-graph spec | full capability + "no statement containment" | §8 |
| R1-M4 localization | out of scope (reasoning layer) | §2 |
| R1-M5 provenance | `source` + `fallback` on `Evidence` | §8, §14 |
| R1-M6 CLI flatten/matrix | flatten gated; compatibility matrix | §11, §12 |
| R2-B2 parse-failure enforcement | nav excludes `ParseFailed`; review stays lenient; excluded-file query → `SkippedPath` + empty | §5, §8 |
| R2-M1 score formula | `score = 1/(1+hop)`, ties `(file,start_line,ordinal)` | §8 |
| R2-M2 TypeRegistry/live_types | added to `NavigationIndex` | §3 |
| R2-M3 line_range_index semantics | innermost enclosing (smallest range containing the line) | §3, §7 |
| R2-M4 goldens incomplete | goldens for callees/ego/module_deps/repo_map + each `WarningKind` | §8 |
| R2-M5 compat fixture list | enumerated flag set + CI gate; **no sanctioned delta** (review untouched) | §12 |
| R2-m1 coverage arrays | **two** `all_test_files` arrays, not three (+ CLAUDE.md fix) | §16 |
| R2-m2 grammar fingerprint mechanism | `build.rs` constant, not runtime `Cargo.lock` read | §9 |
| R2-m3 resolver object-safety | `ResolverContext::Session(Arc<NavigationSession>)` | §14 |
| R2-m4 resolve_callers scope | one-line core forward; tracked separately | §19 |
| R2-m5 prism/slicing name | documented deliberate no-change | §11 |

## 1. Provenance

Clean-room codex design (repo-grounded) ⊗ Claude spec, hardened across two
codex-rigor + claude-soundness review rounds. Convergent spine: opt-in repo-wide
library over the existing CPG/call graph; subcommand CLI flattening legacy review
args (gated); separate exact-hit nav cache; resolver seam; explainable output.

## 2. Goals and non-goals

**Goals.** Whole-repo navigation/architecture queries (`nodes-at`,
`callers`/`callees` with call-site evidence, bounded `ego-graph`,
`module-deps`/`repo-map`) over a whole-repo graph reusing the CPG/`CallGraph`;
clean library + thin CLI (MCP after stable); **diff-review byte-for-byte with no
re-baseline** (v1 makes zero CPG-core edits, §11); structured, explainable,
deterministic output; dogfoodable on this Rust repo.

**Deferred (intended, later) — §19 tracks the core ones:** the reasoning layer
(`FocusSet`); the CPG-core `func_index` re-key + qualifier-aware `resolve_callers`
(their own goldens); cross-repo via the SCIP/Glean resolver seam; an always-fresh
maintained index (only the churn-daemon — whole-repo indexing is in scope, §9);
the DFG-less `CallStructureExperimental` profile (§6); an incremental nav cache.
**Rejected (evidence-gated):** vector RAG, whole-repo long-context, CGM-style
models. **Out of scope (reasoning layer):** NL localization/scoring;
`chop`/`dataflow`/`symbols`/`definition`/`references` as first-class commands.

## 3. Ownership model + nav-local indexes (Option C)

`CpgContext` borrows `files` (`cpg.rs:56,60`); split ownership for long-lived
state:

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
    // Nav-local, derived from CpgNode::Function {start_line,end_line} (cpg.rs:394-399):
    line_range_index: BTreeMap<String, Vec<(usize,usize,NodeIndex)>>,  // per file, sorted by start
    name_index:       BTreeMap<(String,String), Vec<NodeIndex>>,       // (file,name) -> all defs
}

struct NavigationSession<'a> { repo: &'a LoadedRepo, index: &'a NavigationIndex }
```

**Option C — core is untouched.** `func_index` keeps its `(file,name)` key. The
collision risk (same-named `impl`-block fns overwrite, e.g. Rust `fn new`) is
neutralized *for navigation* by deriving two nav-local indexes from the
already-present `Function` node data:

- `name_index` maps `(file,name) → Vec<NodeIndex>` of **all** definitions, so a
  `symbol:` seed can enumerate candidates and return `AmbiguousSymbol{candidates}`
  instead of silently picking one.
- `line_range_index` maps each line to the **innermost enclosing** function
  (smallest `[start,end]` containing the line — correct for closures/lambdas,
  R2-M3), giving O(log n) location→function without the core's O(n) `function_at`
  (R1-M1) and without ever calling it.

`types`/`live_types`/`cpg` are extracted (owned) from a whole-repo
`CpgContext::build`. Durable API IDs are collision-safe, never raw `NodeIndex`:
`FunctionIdRef{file,name,start_line,end_line,ordinal}`,
`StatementRef{file,line,kind,ordinal}`,
`VariableRef{file,function,line,path,access,ordinal}` (`ordinal` orders multiple
same-key defs deterministically by `start_line`).

## 4. (folded into §3)

## 5. Repository loading — traversal/skip + parse-failure contract

`src/repo_loader.rs` → `LoadedRepo`:

- **Discovery:** recursive walk from `root`; only `Language::from_path`-supported
  extensions (`languages/mod.rs:42`).
- **.gitignore** honored (root + nested) + built-in skips (`.git/`, `target/`,
  `node_modules/`, `vendor/`, `dist/`, `build/`).
- **Symlinks** not followed (`Symlink`); **hidden dirs** skipped (`Hidden`);
  **max size** 2 MiB (`TooLarge{bytes}`); **read error / non-UTF-8**
  (`Unreadable`/`NotUtf8`); keys repo-relative, lexically normalized, `/`-sep.
- **Parse-failure enforcement (R2-B2):** files over the severe parse-error
  threshold (`check_parse_quality`, `algorithms/mod.rs:63-101`) are excluded from
  the **nav** graph as `SkippedFile{reason: ParseFailed}`. The **diff-review path
  keeps its current lenient behavior unchanged** (builds from its own unfiltered
  set) — this exclusion is a `repo_loader`/nav concern only (build Step 3), not a
  CPG-core change, so review output is unaffected. A nav query referencing a
  symbol in an excluded/skipped file returns **empty `items` + a `SkippedPath`
  warning**, never a hard error.

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
  (`:747`) → nav uses its `line_range_index` for the enclosing function; labels
  exact vs enclosing.
- `Contains` exists function→variable (`cpg.rs:942`), **not** function→statement
  (`:950`) — ego/module structure must not rely on statement containment (§8).
- Call-site lines live in `CallGraph` (`cpg.rs:576`); expose
  `CallSite{caller,callee_name,line,qualifier}` (`call_graph.rs:21`).
- **`callers`/`callees` do their own qualifier-aware resolution in the nav layer**
  from `CallSite` + `resolve_callees_qualified` (`call_graph.rs:654`); they do
  **not** modify core `resolve_callers` (which ignores `qualifier`,
  `call_graph.rs:801`). The core fix is a tracked follow-up (§19), keeping v1
  review output byte-identical.

## 8. Navigation query API + output model

Modules: `src/navigation/{types,resolver,queries,module_graph,cache}.rs`,
`src/output/navigation.rs` (each < 600 lines).

**Seed grammar:** `symbol:<name>[@<file>]` | `loc:<file>:<line>`. A `symbol:` seed
resolving to >1 def via `name_index` → `AmbiguousSymbol{candidates}`. Locations
normalize to repo-relative keys.

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
hops; the seed/direct hit = 1.0); `items` ordered by `score` desc then
`(file, start_line, ordinal)`; `truncated=true` when `--max-results` clips. No
NL/BM25 scoring in this layer.

**Operations:** `nodes_at(s,&Location)`, `callers(s,seed,depth)`,
`callees(s,seed,depth)`, `ego_graph(s,seed,hops,EgoEdges)`,
`module_deps(s,module)`, `repo_map(s)`.

**`ego_graph` capability:** `EgoEdges ⊆ {Call,Return,DataFlow,ContainsVariable}`
(no `ContainsStatement` — unavailable); `direction ∈ {Out,In,Both}`; seed
included; breadth-ordered, deduped, cycle-guarded; output
`{nodes:[SymbolRef+Location], edges:[{from,to,kind,reason}]}`. Doc states
statement-level neighborhood is unreachable until CPG core adds containment.

**Goldens (representative; the complete per-command/per-warning set lives with
the fixtures, §16):**

```jsonc
// callers symbol:build_scoped — success (hop 1)
{ "query":"callers:build_scoped@src/cpg.rs",
  "items":[{"symbol":{"Function":{"file":"src/algorithms/mod.rs","name":"run_slicing_compat",
            "start_line":210,"end_line":240,"ordinal":0}},
           "location":{"file":"src/algorithms/mod.rs","start_line":210,"end_line":240},
           "score":1.0,"source":"PrismCpg","fallback":false,
           "why":[{"CalledBy":{"caller":"run_slicing_compat","call_site_line":223}}],"snippet":null}],
  "truncated":false,"warnings":[] }

// callees symbol:run_slicing — qualifier-aware
{ "query":"callees:run_slicing@src/algorithms/mod.rs",
  "items":[{"symbol":{"Function":{"file":"src/algorithms/taint.rs","name":"slice",
            "start_line":1240,"end_line":1320,"ordinal":0}},
           "location":{"file":"src/algorithms/taint.rs","start_line":1240,"end_line":1320},
           "score":1.0,"source":"PrismCpg","fallback":false,
           "why":[{"Calls":{"callee":"slice","call_site_line":151,"qualifier":"taint"}}],"snippet":null}],
  "truncated":false,"warnings":[] }

// ego-graph loc:src/cpg.rs:760 hops=1 edges=Call,ContainsVariable
{ "query":"ego:src/cpg.rs:760",
  "items":[],"truncated":false,"warnings":[],
  "graph":{"nodes":[{"symbol":{"Function":{"file":"src/cpg.rs","name":"assemble_graph",
                     "start_line":726,"end_line":1050,"ordinal":0}},
                     "location":{"file":"src/cpg.rs","start_line":726,"end_line":1050}}],
           "edges":[{"from":0,"to":1,"kind":"ContainsVariable",
                     "reason":{"Containment":{"parent":{"Function":{"file":"src/cpg.rs",
                       "name":"assemble_graph","start_line":726,"end_line":1050,"ordinal":0}}}}}]} }

// module_deps src/cpg.rs — call-derived + labeled unresolved import
{ "query":"module-deps:src/cpg.rs",
  "items":[{"symbol":null,"location":{"file":"src/call_graph.rs","start_line":1,"end_line":1},
            "score":1.0,"source":"PrismCpg","fallback":false,
            "why":[{"Calls":{"callee":"CallGraph::build","call_site_line":662,"qualifier":"call_graph"}}],"snippet":null}],
  "truncated":false,
  "warnings":[{"kind":"UnresolvedModule","message":"import 'crate::data_flow' not filesystem-resolved","location":{"file":"src/cpg.rs","start_line":18,"end_line":18}}] }

// empty result + warning
{ "query":"callers:nonexistent","items":[],"truncated":false,
  "warnings":[{"kind":"AmbiguousSymbol","message":"no function named 'nonexistent'","location":null}] }

// skipped-path warning (ParseFailed file)
{ "query":"nodes-at:vendor/broken.js:10","items":[],"truncated":false,
  "warnings":[{"kind":"SkippedPath","message":"file excluded: ParseFailed","location":{"file":"vendor/broken.js","start_line":10,"end_line":10}}] }

// ambiguous seed → error envelope (exit 3)
{ "error":{"AmbiguousSymbol":{"candidates":[
    {"Function":{"file":"src/cpg.rs","name":"build","start_line":76,"end_line":98,"ordinal":0}},
    {"Function":{"file":"src/cpg.rs","name":"build","start_line":608,"end_line":620,"ordinal":1}}]}} }
```

## 9. Caching

Separate nav cache namespace. Key (serialized stable):

```rust
struct NavCacheKey { prism_version: String, cache_format_version: u32,
  grammar_fingerprint: String,   // build.rs compile-time const over tree-sitter-* crate versions
  repo_root_id: String, file_set_hash: String, graph_profile: GraphBuildProfile,
  skip_policy_version: u32, type_db_key: Option<String>, supported_languages: Vec<String> }
```

The **grammar fingerprint** is a `build.rs`-generated compile-time constant
(matching the `env!("CARGO_PKG_VERSION")` pattern at `cpg_cache.rs:181,249`), not
a runtime `Cargo.lock` read (R2-m2) — closes the stale-tree-after-`cargo update`
bug. **v1 exact-hit only**: any change is a Miss → full rebuild.
`skip_policy_version` bumps when `SkipReason`/skip defaults change.

## 10. Module/repo map (heuristic, labeled)

Imports extracted only for Python/JS/TS/TSX/Go (`ast.rs:295`), as-is
(`ast.rs:288`), same-stem ambiguity (`call_graph.rs:651`); **Rust imports not
extracted**. v1 derives the map from *resolved call* file→file edges
(`source:PrismCpg`) plus optional raw imports labeled `UnresolvedImport`
(`source:HeuristicImport`). On this Rust repo the call-derived map carries the
signal.

## 11. Preserve diff-review (byte-for-byte, no re-baseline)

Because v1 makes **zero CPG-core edits** (Option C, §3), the diff-review path is
unchanged and its goldens are **byte-identical with no re-baseline**. The work is
purely: extract the review path into `ReviewArgs` (mechanical, no behavior
change) and add the additive nav layer. The binary is `prism` while clap's app
name is `slicing` (`Cargo.toml:9`, `main.rs:38`) — **left unchanged deliberately**
(renaming would reshape `--help` and perturb the compat matrix; the eventual
rename is a separate cosmetic item, R2-m5).

## 12. CLI seam + compatibility matrix

```rust
struct Cli { #[command(subcommand)] command: Option<Command>,
             #[command(flatten)] review: ReviewArgs }
enum Command { Nav(NavArgs), Mcp(McpArgs) }
```

**Flatten hazard gated (R1-M6):** with a subcommand present, dispatch asserts the
flattened review-only fields are at defaults; a non-default review flag under
`nav`/`mcp` (e.g. `prism nav callers --diff x`) is a hard usage error, not a
silent no-op. `None` runs the extracted review path unchanged.

**Compatibility matrix (R2-M5), regression-locked, NO sanctioned delta:** golden
captures of stdout bytes, stderr bytes, exit code, `--help` text, and
validation-error text for: the bare `--repo/--diff` invocation; each
`--algorithm` in the `review` preset + `leftflow`,`taint`,`chop`,`thin`;
`--format {text,json,paper,review}`; and `--list-algorithms`. A dedicated CI step
diffs these and fails on any deviation (there is no approved delta in v1).

```text
prism --repo . --diff changes.patch --algorithm review        # byte-identical
prism nav callers  --repo . --symbol build_scoped --depth 2 --format json
prism nav nodes-at --repo . --location src/main.rs:498 --format json
prism nav repo-map --repo . --format json
```

## 13. MCP adapter (after CLI/library stable)

`src/bin/prism-mcp.rs`, thin over `src/navigation`, exposing `nodes_at`,
`callers`, `callees`, `ego_graph`, `repo_map` (→ `Evidence` JSON). SDK `rmcp`,
validated by a spike; fallback to a minimal stdio JSON-RPC server. Sequenced after
the nav-local indexes and library/CLI behavior are fixed.

## 14. Resolver seam

```rust
enum ResolverContext { Session(Arc<NavigationSession<'static>>), ExternalIndex(PathBuf) }
trait SymbolResolver {
  fn definition(&self, cx: &ResolverContext, sym: &FunctionIdRef) -> Vec<(Location, Source)>;
  fn references(&self, cx: &ResolverContext, sym: &FunctionIdRef) -> Vec<(Location, Source)>;
}
```

`ResolverContext` owns via `Arc` so the trait is object-safe (no method-arg
lifetime / HRTB, R2-m3). `HeuristicResolver` (default) uses the session; a future
`ScipResolver` reads a `.scip` path. Results carry `Source`; `fallback=true` marks
a heuristic fallback after an external miss.

## 15. Evaluation seam

`Evidence` is structured/comparable for a later A/B vs an agentic-search baseline
(localization precision/recall, token cost), per language. Built when there are
tools to measure.

## 16. Testing — named golden fixtures

Per-scenario fixtures (small multi-file repo + expected `Evidence`/error JSON):
**duplicate same-name functions** (incl. a Rust `impl`-block `fn new` →
`AmbiguousSymbol`/candidate enumeration via `name_index`), **static/free
functions**, **qualified/imported calls** (callers + callees), **closures/lambdas**
(innermost-enclosing `line_range_index`), **unsupported & skipped files** (each
`SkipReason`, incl. `ParseFailed`→`SkippedPath`), **cache invalidation** (content
change, file add/remove, grammar-fingerprint bump), **CLI legacy compatibility**
(the §12 matrix), **empty results**, **nodes-at exact+enclosing**, **ego-graph**
shape, **module/repo map** labeling, and a **dogfood smoke** (`repo-map` +
`callers build_scoped` on this repo). Update **both** `all_test_files` arrays in
`tests/integration/coverage_test.rs` (`:106`, `:325`) — there are **two**, not
three (R2-m1; CLAUDE.md's "3 copies" note is stale and should be corrected).

## 17. Build order

1. Extract the review path into `ReviewArgs` — **no behavior change**; §12 compat
   matrix green. (No CPG-core edits.)
2. `repo_loader.rs` + `LoadedRepo` (full §5 contract incl. ParseFailed exclusion).
3. `NavigationIndex` (FullCpg) + nav-local `line_range_index`/`name_index` +
   `types`/`live_types`; nav cache key/scaffolding (§9, exact-hit).
4. `navigation/types.rs` (serde §8) + `resolver.rs` (§14).
5. `nodes-at` (exact + enclosing).
6. `callers`/`callees` (nav-local qualifier-aware, CallSite evidence).
7. bounded `ego-graph` (§8).
8. `prism nav …` CLI + `output/navigation.rs` JSON.
9. exact-hit nav cache wired in.
10. `module_deps`/`repo_map` (labeled, §10).
11. MCP adapter.
12. (Follow-ups, §19) core `func_index` re-key + qualifier-aware `resolve_callers`
    (own goldens); then `CallStructureExperimental`; then the reasoning layer.

## 18. Module/file plan

New: `src/repo_loader.rs`;
`src/navigation/{mod,types,resolver,queries,module_graph,cache}.rs`;
`src/output/navigation.rs`; `src/bin/prism-mcp.rs`; `build.rs` (grammar
fingerprint).
Modified (**additive only — no core logic edits**): `src/main.rs` (subcommand +
`ReviewArgs` extraction); `src/lib.rs` (`pub mod repo_loader; pub mod navigation;`);
`Cargo.toml` (MCP bin + `rmcp` + `build.rs`); `tests/integration/coverage_test.rs`
(both arrays); CLAUDE.md (the "two arrays" correction).

## 19. Tracked follow-ups (separate slices, own goldens)

1. **CPG-core `func_index` re-key** to `(file,name,start_line)` so the *review*
   path is also collision-safe — invasive (cascades `cpg.rs`, `cpg_cache.rs`
   incl. `reconstruct_cpg`, `data_flow.rs`, `call_graph.rs`, algorithms), needs a
   reviewed golden re-baseline. Decoupled from navigation (which is already safe
   via §3).
2. **Qualifier-aware core `resolve_callers`** (`call_graph.rs:801` one-line
   forward to `resolve_callees_qualified`) — may shift call-graph-using algorithm
   output; gets its own regression review.
3. `CallStructureExperimental` profile (needs function→statement containment).
4. The reasoning layer (`FocusSet`, taint/impact/data-flow).
