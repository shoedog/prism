# Design: Prism Navigation Layer (Tier 1) — v2 (review-hardened)

**Status:** Hardened design — closes the spec-review blockers/majors; for re-review or planning
**Date:** 2026-06-07
**Scope:** Whole-repo navigation/architecture layer only. The seeded *reasoning*
layer (`FocusSet`, taint/impact/data-flow) is the next initiative; seams here must
let it sit on top.

## 0. Review disposition (closes the joint spec-review)

| Finding | Resolution | Section |
|---|---|---|
| B1 func_index collision | Re-key `func_index` to `(file,name,start_line)` in CPG core **+** build-time collision detect → diagnostic + `AmbiguousSymbol`; re-baseline review goldens | §3, §4, §11 |
| B2 no JSON schemas | Concrete serde structs + per-command golden examples (incl. empty/ambiguous/error) | §8 |
| B3 ingestion underspecified | Full traversal/skip contract (gitignore, symlinks, max-size, UTF-8, reason codes) | §5 |
| B4 cache identity / grammar version | Add grammar-version fingerprint from `Cargo.lock`; define skip-policy version; full key serialization | §9 |
| M1 `function_at` O(n) | Add per-file `line_range_index` (binary search); prerequisite before MCP | §3, §7 |
| M2 `resolve_callers` ignores qualifier | `callers` uses the call-graph index; make `resolve_callers` qualifier-aware + tests | §7 |
| M3 ego-graph underspecified | Full capability spec; explicit "no statement-level containment" | §8 |
| M4 localization scoring | **Out of scope** — reasoning-layer concern, not Tier-1 (review saw the analysis doc) | §2 |
| M5 backend provenance | `Evidence` carries `source` + `fallback`; resolver adapter contract | §8, §14 |
| M6 CLI flatten hazard / matrix | Gate flatten on subcommand; compatibility matrix | §11, §12 |
| m1 seed grammar | Seed grammar + `AmbiguousSymbol` protocol | §8 |
| m2 golden fixtures | Named fixture list per scenario | §16 |
| m3 resolver session coupling | `ResolverContext` enum, not `&NavigationSession` | §14 |

## 1. Provenance

Merges a firewalled clean-room codex design (repo-grounded, never saw the Claude
spec) with the Claude spec; hardened against a codex-rigor + claude-soundness
review. Convergent spine (opt-in repo-wide library over the existing CPG/call
graph; subcommand CLI flattening legacy review args; separate exact-hit nav
cache; resolver seam; explainable output) is high-confidence.

## 2. Goals and non-goals

**Goals.** Whole-repo navigation/architecture queries (`nodes-at`,
`callers`/`callees` with call-site evidence, bounded `ego-graph`,
`module-deps`/`repo-map`) over a whole-repo graph reusing the CPG/`CallGraph`;
clean library API + thin CLI (MCP after stable); diff-review preserved (§11);
structured, explainable, deterministic output; dogfoodable on this Rust repo.

**Deferred (intended, later):** the reasoning layer (`FocusSet`); cross-repo via
the SCIP/Glean resolver seam; an always-fresh maintained index (only the
churn-daemon is deferred — whole-repo indexing is in scope, §9); the DFG-less
`CallStructureExperimental` profile (§6); an incremental nav cache (v1 is
exact-hit). **Rejected (evidence-gated):** vector RAG, whole-repo long-context,
CGM-style learned models. **Out of scope (not this layer):** NL localization and
scoring (M4), and `chop`/`dataflow`/`symbols`/`definition`/`references` as
first-class commands — those belong to the reasoning layer.

## 3. Ownership model + CPG-core changes

`CpgContext` borrows `files` (`cpg.rs:56,60`), so split ownership to avoid a
self-referential long-lived context:

```rust
struct LoadedRepo { root: PathBuf, files: BTreeMap<String,ParsedFile>,
                    file_hashes: BTreeMap<String,String>, skipped: Vec<SkippedFile>,
                    type_db: Option<TypeDatabase> }     // ParsedFile owns source (ast.rs:45)
struct NavigationIndex { cpg: CodePropertyGraph, profile: GraphBuildProfile,
                         parse_quality: BTreeMap<String,FileParseQuality> }
struct NavigationSession<'a> { repo: &'a LoadedRepo, index: &'a NavigationIndex }
```

**Two CPG-core changes (shared, behind the review re-baseline, §11):**

1. **Re-key `func_index`** from `(file,name)` to `(file,name,start_line)`
   (`cpg.rs:566,746`) so same-named functions across `impl` blocks no longer
   overwrite. Build-time collision detection remains: if two functions still map
   to one key (e.g., macro-generated same-line defs), record a `Collision`
   `parse_quality` diagnostic and have CPG-backed sub-queries return
   `AmbiguousSymbol` rather than wrong data.
2. **Add `line_range_index: BTreeMap<String, Vec<(usize,usize,NodeIndex)>>`**
   (per file, sorted by `start_line`) so `function_at` is binary search, not the
   O(n_functions) scan at `cpg.rs:1489`. Prerequisite before MCP exposure (M1).

Durable API IDs are collision-safe and never raw `NodeIndex`:
`FunctionIdRef{file,name,start_line,end_line,ordinal}`,
`StatementRef{file,line,kind,ordinal}`,
`VariableRef{file,function,line,path,access,ordinal}`.

## 4. (folded into §3)

## 5. Repository loading — traversal/skip contract (B3)

`src/repo_loader.rs` produces `LoadedRepo`. Contract:

- **Discovery:** recursive walk from `root`; include only `Language::from_path`
  -supported extensions (`languages/mod.rs:42`).
- **.gitignore:** honored when present (root + nested), plus a built-in skip set
  (`.git/`, `target/`, `node_modules/`, `vendor/`, `dist/`, `build/`).
- **Symlinks:** not followed (skip with reason `Symlink`).
- **Hidden dirs:** skipped except explicitly supported configs; `Hidden` reason.
- **Max file size:** default 2 MiB; larger → skip, reason `TooLarge{bytes}`.
- **Invalid UTF-8 / read error:** skip, reason `Unreadable{io}` / `NotUtf8`.
- **Path canonicalization:** store repo-relative, lexically normalized,
  `/`-separated keys (matches existing `file` keys).
- **Parse:** `ParsedFile::parse` (`ast.rs:63`); parse-quality recorded
  (`check_parse_quality`); files exceeding the severe-error threshold →
  `SkippedFile{ reason: ParseFailed }` and excluded from the graph.

```rust
struct SkippedFile { path: String, reason: SkipReason }
enum SkipReason { Unsupported, Ignored, Symlink, Hidden, TooLarge{bytes:u64},
                  Unreadable, NotUtf8, ParseFailed }
```

`skipped` is returned to callers (and surfaced in `Evidence.warnings` when a
query references a skipped path), so inclusion is predictable.

## 6. Graph build profile

`enum GraphBuildProfile { FullCpg, CallStructureExperimental }`. **v1 =
`FullCpg`** (assembly assumes a DFG; `DataFlowGraph::empty()` silently drops
variable nodes + `Contains` edges, `data_flow.rs:77` / `cpg.rs:754`). Seconds on
this repo. `CallStructureExperimental` is deferred and, when added, lives inside
`cpg.rs` behind explicit function→statement containment + tests.

## 7. CPG constraints the query layer respects

- `nodes_at` is exact-line only (`cpg.rs:1190`; functions indexed at `start_line`
  only, `:747`) → also call `function_at` via the new `line_range_index`; label
  exact vs enclosing.
- `Contains` exists function→variable (`cpg.rs:942`) but **not**
  function→statement (`:950` uncontained) — ego/module structure must not rely on
  statement containment (§8, M3).
- Call-site lines live in `CallGraph`, not CPG `Call` edges (`cpg.rs:576`);
  expose `CallSite{caller,callee_name,line,qualifier}` (`call_graph.rs:21`).
- `callers` traverses the **call-graph index**: make `resolve_callers`
  qualifier-aware (today it ignores `CallSite.qualifier`, `call_graph.rs:801`),
  with regression tests for imported/qualified calls (M2). `callees` uses
  `resolve_callees_qualified` (`call_graph.rs:654`), not the unqualified
  convenience helper.

## 8. Navigation query API + output model (B2, M3, M5, m1)

Modules: `src/navigation/{types,resolver,queries,module_graph,cache}.rs`,
`src/output/navigation.rs` (each < 600 lines).

**Seed grammar (m1):** `symbol:<name>[@<file>]` | `loc:<file>:<line>`. Resolution
that maps to >1 `FunctionIdRef` returns `AmbiguousSymbol{candidates}` (never a
silent pick). Locations are normalized to repo-relative keys.

**Serde contract (B2):**

```rust
struct Location { file: String, start_line: usize, end_line: usize }
enum SymbolRef { Function{..}, Statement{..}, Variable{..} }   // the *Ref types, §3
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
                      snippet: Option<String> }      // snippet only with --snippets
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

**Determinism:** `items` ordered by `score` desc, then `(file, start_line,
ordinal)`. `score ∈ [0,1]`; for structural queries it encodes proximity
(1.0 = direct edge, decaying per hop) — no NL/BM25 scoring in this layer.
`truncated=true` when `--max-results` clips.

**Operations:**

```rust
fn nodes_at(s,&Location)            // exact nodes + EnclosingFunction
fn callers(s,seed,depth)            // call-graph traversal, CallSite evidence
fn callees(s,seed,depth)            // resolve_callees_qualified
fn ego_graph(s,seed,hops,EgoEdges)  // BFS over selected CPG edges
fn module_deps(s,module)            // §10
fn repo_map(s)                      // §10
```

**`ego_graph` capability (M3):** `EgoEdges` selects from `{Call, Return,
DataFlow, ContainsVariable}` (no `ContainsStatement` — unavailable); `direction
∈ {Out,In,Both}`; the seed node is included; BFS is breadth-ordered and
deduplicated; cycles are visited-guarded; output is `{nodes:[SymbolRef+Location],
edges:[{from,to,kind,reason}]}`. The doc states explicitly that statement-level
neighborhood is not reachable until CPG core adds containment.

**Golden examples (one per command; full set lives beside the fixtures):**

```jsonc
// callers seed=symbol:build_scoped — success
{ "query":"callers:build_scoped@src/cpg.rs",
  "items":[{ "symbol":{"Function":{"file":"src/algorithms/mod.rs","name":"run_slicing_compat",
             "start_line":210,"end_line":240,"ordinal":0}},
           "location":{"file":"src/algorithms/mod.rs","start_line":210,"end_line":240},
           "score":1.0,"source":"PrismCpg","fallback":false,
           "why":[{"CalledBy":{"caller":"run_slicing_compat","call_site_line":223}}],
           "snippet":null }],
  "truncated":false,"warnings":[] }

// nodes-at — ambiguous-free location with enclosing function
{ "query":"nodes-at:src/cpg.rs:760","items":[ /* variable/def nodes */
  { "symbol":{"Variable":{"file":"src/cpg.rs","function":"assemble_graph","line":760,
              "path":"idx","access":"Def","ordinal":0}},
    "location":{"file":"src/cpg.rs","start_line":760,"end_line":760},
    "score":1.0,"source":"PrismCpg","fallback":false,
    "why":[{"EnclosingFunction":{"function":{"Function":{"file":"src/cpg.rs",
            "name":"assemble_graph","start_line":726,"end_line":1050,"ordinal":0}}}}],
    "snippet":null }],"truncated":false,"warnings":[] }

// empty result
{ "query":"callers:nonexistent","items":[],"truncated":false,
  "warnings":[{"kind":"SymbolNotFound","message":"no function named 'nonexistent'","location":null}] }

// ambiguous seed → error envelope (exit code 3)
{ "error":{"AmbiguousSymbol":{"candidates":[
    {"Function":{"file":"src/cpg.rs","name":"build","start_line":76,"end_line":98,"ordinal":0}},
    {"Function":{"file":"src/cpg.rs","name":"build","start_line":608,"end_line":620,"ordinal":1}}]}} }
```

`module_deps`/`repo_map` reuse `Evidence` with `Reason::ResolvedImport` /
`UnresolvedImport` and `source` labeling (§10).

## 9. Caching (B4)

Separate nav cache namespace, distinct dir from the per-diff cache. Key
(all serialized, order-stable):

```rust
struct NavCacheKey {
  prism_version: String, cache_format_version: u32,
  grammar_fingerprint: String,   // hash of tree-sitter-* crate versions from Cargo.lock
  repo_root_id: String,          // canonical root path hash
  file_set_hash: String,         // hash over sorted (path,file_hash)
  graph_profile: GraphBuildProfile,
  skip_policy_version: u32,       // bumped when SkipReason set / defaults change
  type_db_key: Option<String>, supported_languages: Vec<String>,
}
```

The **grammar fingerprint** closes the stale-tree bug (a `cargo update` bumping
`tree-sitter-rust` ⇒ different key ⇒ Miss). **v1 is exact-hit only**: any change
(file content/set, profile, type-db, skip-policy, grammar, version) is a Miss →
full rebuild. Incremental is deferred (the existing `cpg_cache.rs:276` partial-hit
needs an identical file set anyway).

## 10. Module/repo map (heuristic, labeled)

Imports are extracted only for Python/JS/TS/TSX/Go (`ast.rs:295`), as-is/not
filesystem-resolved (`ast.rs:288`), with same-stem ambiguity (`call_graph.rs:651`),
and **Rust imports are not extracted**. So v1 derives the map primarily from
*resolved call* file→file edges (`Reason::Calls`/`CalledBy`, `source:PrismCpg`),
plus optional raw import edges labeled `Reason::UnresolvedImport`
(`source:HeuristicImport`). On this Rust repo the call-derived map carries the
signal; the import map is best-effort and labeled. Output is `Evidence` over
file/dir nodes.

## 11. Preserve diff-review (refined guarantee)

The review path is extracted into `ReviewArgs` with **no behavior change of its
own**. The *only* permitted output change is the documented `func_index`
re-key (§3), which can legitimately stop collapsing same-named functions. That
change is captured by a **one-time, reviewed golden re-baseline** committed in the
same slice, with a written diff rationale; everything else (stdout/stderr, exit
codes, help, `--list-algorithms`, `--format` variants) is byte-identical. The
binary/test contract is `prism` while clap's command name is `slicing`
(`Cargo.toml:8`, `main.rs:38`) — left unchanged.

## 12. CLI seam + compatibility matrix (M6)

```rust
struct Cli { #[command(subcommand)] command: Option<Command>,
             #[command(flatten)] review: ReviewArgs }
enum Command { Nav(NavArgs), Mcp(McpArgs) }
```

**Flatten hazard gated:** when a subcommand is present, the dispatch asserts the
flattened review-only fields are at defaults; a non-default review flag under
`nav`/`mcp` (e.g. `prism nav callers --diff x`) is a hard usage error, not a
silent no-op. `None` runs the extracted review path unchanged.

**Compatibility matrix (regression-locked):** for the bare invocation and each
existing flag combo — stdout bytes, stderr bytes, exit code, `--help` text,
validation-error text, `--list-algorithms`, every `--format` variant — captured
as goldens. The func_index re-baseline (§11) is the sole sanctioned delta.

```text
prism --repo . --diff changes.patch --algorithm review        # unchanged
prism nav nodes-at --repo . --location src/main.rs:498 --format json
prism nav callers  --repo . --symbol build_scoped --depth 2 --format json
prism nav repo-map --repo . --format json
```

## 13. MCP adapter (after CLI/library stable)

`src/bin/prism-mcp.rs`, thin over `src/navigation`, exposing `nodes_at`,
`callers`, `callees`, `ego_graph`, `repo_map` (returning `Evidence` JSON). SDK:
`rmcp`, validated by a spike; fallback to a minimal stdio JSON-RPC server.
Sequenced after the `line_range_index` (M1) and library/CLI behavior are fixed.

## 14. Resolver seam (m3, M5)

```rust
enum ResolverContext<'a> { Session(&'a NavigationSession<'a>), ExternalIndex(&'a Path) }
trait SymbolResolver {
  fn definition(&self, cx: ResolverContext, sym: &FunctionIdRef) -> Vec<(Location, Source)>;
  fn references(&self, cx: ResolverContext, sym: &FunctionIdRef) -> Vec<(Location, Source)>;
}
```

`HeuristicResolver` (default) uses the session; a future `ScipResolver` reads a
`.scip` path — the enum keeps each impl honest and independently testable. Every
result carries its `Source`; `Evidence.items[].fallback` marks a heuristic
fallback after an external miss.

## 15. Evaluation seam

`Evidence` is structured/comparable for a later A/B vs an agentic-search baseline
(localization precision/recall, token cost), per language. Built when there are
tools to measure.

## 16. Testing — named golden fixtures (m2)

Per-scenario fixtures (each a small multi-file repo + expected `Evidence`/error
JSON): **duplicate same-name functions** (the func_index case, incl. a Rust
`impl`-block `fn new`), **static/free functions**, **qualified/imported calls**
(callers + callees), **unsupported & skipped files** (each `SkipReason`),
**cache invalidation** (content change, file add/remove, grammar-fingerprint
bump), **CLI legacy compatibility** (the §12 matrix), **empty results**,
**ambiguous seed** (`AmbiguousSymbol`), **nodes-at exact+enclosing**, and a
**dogfood smoke** (`repo-map` + `callers build_scoped` on this repo). Plus the
review golden re-baseline (§11). Update the three `coverage_test.rs`
`all_test_files` copies for any new language test files.

## 17. Build order

1. CPG core: `func_index` re-key + collision diagnostic + `line_range_index`;
   re-baseline review goldens (§11) — **no other review behavior change**.
2. Extract review path into `ReviewArgs`; §12 compatibility matrix green.
3. `repo_loader.rs` + `LoadedRepo` (full §5 contract).
4. `NavigationIndex` (FullCpg) + nav cache key/scaffolding (§9, exact-hit).
5. `navigation/types.rs` (serde contract §8) + `resolver.rs` (§14).
6. `nodes-at` (exact + enclosing).
7. `callers`/`callees` (CallSite evidence, qualifier-aware `resolve_callers`).
8. bounded `ego-graph` (§8 capability).
9. `prism nav …` CLI + `output/navigation.rs` JSON.
10. exact-hit nav cache wired in.
11. `module_deps`/`repo_map` (labeled, §10).
12. MCP adapter.
13. (Later) `CallStructureExperimental`; then the reasoning layer.

## 18. Module/file plan

New: `src/repo_loader.rs`; `src/navigation/{mod,types,resolver,queries,module_graph,cache}.rs`;
`src/output/navigation.rs`; `src/bin/prism-mcp.rs`.
Modified: `src/cpg.rs` (func_index key, collision diagnostic, `line_range_index`,
`function_at`); `src/call_graph.rs` (qualifier-aware `resolve_callers`);
`src/main.rs` (subcommand + `ReviewArgs`); `src/lib.rs`; `Cargo.toml` (MCP bin +
`rmcp`); `tests/integration/coverage_test.rs`.
