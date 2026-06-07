# Design: Prism Navigation Layer (Tier 1 — whole-repo navigation/architecture)

**Status:** Folded design (clean-room codex + Claude spec, merged) — for spec review
**Date:** 2026-06-07
**Scope:** The whole-repo navigation/architecture layer only. The seeded
*reasoning* layer (taint/impact/data-flow from a seed) is the **next** initiative
and is explicitly out of scope here, but the seams must let it sit on top.

## 1. Provenance

This design merges two independent inputs: a firewalled clean-room codex design
(which read the repo read-only and never saw the Claude spec) and the Claude
working spec. Their **convergent spine** — an opt-in repo-wide library layer over
the existing CPG/call graph, a subcommand CLI that flattens the legacy review
args, a separate exact-hit nav cache, a resolver seam, deterministic explainable
output — is high-confidence. The codex pass additionally caught concrete
repo-grounded corrections (ownership lifetimes, stable IDs, graph-build
dependence on the DFG, incomplete containment, call-site evidence provenance,
language-limited imports) that this spec adopts.

## 2. Goals and non-goals

### Goals

- Answer whole-repo **navigation/architecture** queries: `nodes-at`,
  `callers`/`callees` (with call-site evidence), bounded `ego-graph`,
  `module-deps`/`repo-map`.
- Build a whole-repo graph index reusing the existing CPG/`CallGraph`.
- Deliver as a clean library API with a thin CLI adapter (MCP adapter follows
  once the library/CLI is stable).
- Preserve the existing diff-review behavior **byte-for-byte**.
- Produce structured, machine-comparable, **explainable** output (every result
  carries the edge/call/containment that justifies it).
- Be dogfoodable on this Rust repo.

### Non-goals

**Deferred (intended, later):**

- The seeded **reasoning layer** (taint/impact/data-flow from a seed; the
  `FocusSet` abstraction). Designed-for here, built next.
- **Cross-repo / org-scale** symbol resolution — via the SCIP/Glean resolver
  seam (§12).
- A persistent **always-fresh maintained index** (file watchers, background
  rebuilds). Whole-repo indexing *is* in scope (§9); only the
  always-fresh-under-churn operational layer is deferred.
- The **`CallStructureExperimental`** DFG-less build profile (§6) — needs
  CPG-core work first.
- **Incremental** nav cache (v1 is exact-hit only, §9).

**Rejected (not pursuing — evidence-gated):**

- Vector/embedding RAG as a primary retrieval path.
- Whole-repo long-context prompting as a substitute for retrieval.
- CGM-style learned-graph models.

## 3. Ownership model (no self-referential context)

`CpgContext` borrows `files` (`src/cpg.rs:56,60`), so any long-lived/owning
navigation state would be self-referential. Split ownership:

```rust
pub struct LoadedRepo {              // owns parsed files
    pub root: PathBuf,
    pub files: BTreeMap<String, ParsedFile>,   // ParsedFile already stores source (ast.rs:45)
    pub file_hashes: BTreeMap<String, String>,
    pub skipped: Vec<SkippedFile>,
    pub type_db: Option<TypeDatabase>,
}

pub struct NavigationIndex {         // owns the graph
    pub cpg: CodePropertyGraph,
    pub profile: GraphBuildProfile,
    pub parse_quality: BTreeMap<String, FileParseQuality>,
}

pub struct NavigationSession<'a> {   // borrows both, per-query/session facade
    pub repo: &'a LoadedRepo,
    pub index: &'a NavigationIndex,
}
```

Do **not** keep a second full-source copy (`ParsedFile.source` is authoritative);
hold `file_hashes` for cache identity only.

## 4. Stable identifiers

CPG `func_index` is keyed only by `(file, name)` (`src/cpg.rs:566,746`), so
overloads, duplicate names, and same-file duplicate symbols collide. Durable API
IDs must be collision-safe:

```rust
FunctionIdRef { file, name, start_line, end_line, ordinal }
StatementRef  { file, line, kind, ordinal }
VariableRef   { file, function, line, path, access, ordinal }
```

Raw `NodeIndex` may appear only as response-local debug metadata, never as a
durable key.

## 5. Repository loading

New `src/repo_loader.rs`: whole-repo discovery; supported-language filtering via
`Language::from_path` (`src/languages/mod.rs:42`); parse via `ParsedFile::parse`
(`src/ast.rs:63`); compute `file_hashes`; a skip policy (exclude `.git`,
`target/`, `node_modules/`, `vendor/`, `dist/`, `build/`; honor `.gitignore`
when present); record `SkippedFile` reasons. Produces a `LoadedRepo`.

## 6. Graph build profile

```rust
pub enum GraphBuildProfile { FullCpg, CallStructureExperimental }
```

**v1 defaults to `FullCpg`** — current CPG assembly and tests assume DFG-backed
variables and data flow. `assemble_graph` always expects a `DataFlowGraph`
(`src/cpg.rs:754`), and passing `DataFlowGraph::empty()` (`src/data_flow.rs:77`)
silently changes `nodes_at` by dropping variable nodes and `Contains` edges —
unacceptable as a default. For a repo of Prism's size a full build is seconds.

`CallStructureExperimental` (a cheaper call/structure-only graph) is **deferred**
and, when added, belongs **inside `src/cpg.rs`**, gated behind explicit
function→statement containment (§7) and tests proving `nodes_at`, callers,
callees, and ego graphs behave as documented.

## 7. Known CPG constraints the query layer must respect

- **`nodes_at` is exact-line only.** It returns nodes indexed at exactly
  `(file, line)` (`src/cpg.rs:1190`); function nodes are indexed only at
  `start_line` (`src/cpg.rs:747`). So `nodes-at` must *also* call `function_at`
  (`src/cpg.rs:1487`) and label exact vs enclosing evidence separately.
- **Containment is partial.** `Contains` edges exist function→variable
  (`src/cpg.rs:942`) but **not** function→statement (statements are created at
  `:950` but uncontained). Ego/module structure must not depend on statement
  containment until CPG core adds it.
- **Call-site lines live in `CallGraph`, not CPG edges.** CPG Function→Function
  `Call` edges drop call-site location (`src/cpg.rs:576`); the retained
  `CallGraph` is authoritative. Expose
  `CallSite { caller, callee_name, line, qualifier }` (`src/call_graph.rs:21`).
- **Use direct + qualifier-aware traversal.** The convenience helpers
  `callers_of_in_file`/`callees_of` return only `(FunctionId, depth)` and use
  unqualified resolution (`src/call_graph.rs:751,849`). Navigation traverses
  `CallGraph::calls`/`resolve_callers` directly and resolves with
  `resolve_callees_qualified` (`src/call_graph.rs:654`) to attach evidence.

## 8. Navigation query API + output model

New modules: `src/navigation/types.rs` (IDs, locations, edges, evidence,
errors), `src/navigation/resolver.rs` (resolver trait over call-graph
resolution), `src/navigation/queries.rs` (pure query execution),
`src/navigation/module_graph.rs` (import/call-derived file+dir graph),
`src/navigation/cache.rs` (nav cache), `src/output/navigation.rs` (JSON/text,
separate from review output). Each file < 600 lines.

```rust
fn nodes_at(session, file, line) -> Evidence;       // exact nodes + enclosing function
fn callers(session, seed, depth) -> Evidence;        // CallSite-evidenced
fn callees(session, seed, depth) -> Evidence;        // qualifier-aware
fn ego_graph(session, seed, hops, edges) -> Evidence;// BFS over CPG edges
fn module_deps(session, module) -> Evidence;         // heuristic, labeled (§10)
fn repo_map(session) -> Evidence;                    // heuristic, labeled (§10)
```

`Evidence` is uniform, serde-serializable: ranked items, each with a `Reason`
(`Dataflow`/`Call`/`CallerMissingHandler`/`Containment`/`Lexical`/
`UnresolvedImport`), `score`, optional snippet (off by default for token
budget), and `warnings` (parse-quality, resolution approximations).

## 9. Caching

A separate whole-repo nav cache namespace (distinct from the per-diff cache).
Metadata: Prism version, cache-format version, repo-root identity, file set,
file hashes, graph profile, skip-policy version, type-db presence/key, supported
language set. **v1 accepts exact hits only**; any changed/added/removed file,
profile change, type-db change, or version change is a Miss → full rebuild.
(`cpg_cache.rs` partial-hit requires an identical file set, `src/cpg_cache.rs:276`
— the constraint that makes incremental a deferred concern.)

## 10. Module map / repo map (heuristic in v1)

Imports are extracted only for Python, JS/TS/TSX, and Go (`src/ast.rs:295`), with
module paths kept as-is (not filesystem-resolved, `src/ast.rs:288`) and same-stem
resolver ambiguity (`src/call_graph.rs:651`). **Rust imports are not extracted.**
Therefore v1 `module_deps`/`repo_map` derive primarily from *resolved call*
file-to-file edges, plus optional raw import edges explicitly labeled
`unresolved_import`. On this Rust repo the call-derived map is the useful signal;
the import-derived map is best-effort and labeled.

## 11. Preserve diff-review (hard guarantee)

The bare invocation and all existing flags produce **byte-identical** output.
Enforced by extracting the current review path into `ReviewArgs` with **no
behavior change** and golden-output regression captured before the refactor.
Caution: the binary/test contract is `prism` (`tests/cli/output_test.rs:6`) while
clap's command name is `slicing` (`Cargo.toml:8`, `src/main.rs:38`) — do not
rename or reshape help while adding modes.

## 12. CLI seam

```rust
struct Cli {
    #[command(subcommand)] command: Option<Command>,
    #[command(flatten)]    review: ReviewArgs,   // today's flags
}
enum Command { Nav(NavArgs), Mcp(McpArgs) }
```

`main` routes `Some(Command::Nav)`/`Some(Command::Mcp)` before legacy validation;
`None` runs the extracted review path with `--repo`/`--diff` validation,
`--format callers`, cache behavior, parse warnings, and outputs unchanged.

```text
prism --repo . --diff changes.patch --algorithm review      # unchanged
prism nav nodes-at --repo . --location src/main.rs:498 --format json
prism nav callers  --repo . --symbol foo --file src/lib.rs --depth 2
prism nav repo-map --repo . --format json
```

## 13. MCP adapter (after CLI/library stable)

`src/bin/prism-mcp.rs`, a thin shell over `src/navigation`, exposing
`nodes_at`, `callers`, `callees`, `ego_graph`, `repo_map` as tools returning
`Evidence` JSON. Transport/SDK: official Rust MCP SDK (`rmcp`), validated by a
spike; fallback to a minimal stdio JSON-RPC server. Sequenced last so the
library/CLI behavior is fixed before a second consumer binds to it.

## 14. Resolver seam (SCIP later)

```rust
pub trait SymbolResolver {
    fn definition(&self, session: &NavigationSession, sym: &FunctionIdRef) -> Vec<Location>;
    fn references(&self, session: &NavigationSession, sym: &FunctionIdRef) -> Vec<Location>;
}
```

`HeuristicResolver` (current name+import logic) is the v1 default. A future
`ScipResolver` reads `.scip` index files for compiler-accurate defs/refs and the
cross-repo path; Prism consumes SCIP output, does not reimplement it.

## 15. Evaluation seam

`Evidence` is structured and machine-comparable so a later A/B can score
localization precision/recall and token cost against an agentic-search baseline,
per language. The harness is built once there are tools to measure.

## 16. Dogfooding on the Prism repo

`callers`/`callees`/`ego-graph`/`nodes-at` are call-graph-/CPG-backed and strong
on Rust → high immediate value. `module_deps`/`repo_map` are weaker on Rust
(no import extraction) → labeled best-effort; call-derived edges carry the
signal. Smoke test: `prism nav repo-map --repo .` and
`prism nav callers --symbol build_scoped` produce well-formed `Evidence`.

## 17. Build order

1. Extract the legacy review path into `ReviewArgs` — **no behavior change**
   (golden tests green).
2. `src/repo_loader.rs` + `LoadedRepo`.
3. `NavigationIndex` (FullCpg) + nav cache scaffolding (exact-hit).
4. `nodes-at` (exact + enclosing).
5. `callers`/`callees` with `CallSite` evidence (qualifier-aware).
6. Bounded CPG `ego-graph`.
7. `prism nav …` CLI + JSON output.
8. Exact-hit nav cache wired in.
9. `module_deps`/`repo_map` (labeled heuristics).
10. MCP adapter.
11. (Later) `CallStructureExperimental` after CPG-core containment + dup-symbol
    tests; then the seeded reasoning layer.

## 18. Module/file plan

New: `src/repo_loader.rs`; `src/navigation/{mod,types,resolver,queries,module_graph,cache}.rs`;
`src/output/navigation.rs`; `src/bin/prism-mcp.rs`.
Modified: `src/main.rs` (subcommand + `ReviewArgs` extraction); `src/lib.rs`
(`pub mod repo_loader; pub mod navigation;`); `Cargo.toml` (MCP bin + `rmcp`);
`tests/integration/coverage_test.rs` (if new language test files are added).

## 19. Resolved decisions (owner-approved)

- **Graph profile:** v1 = `FullCpg`. The DFG-less `CallStructureExperimental`
  profile is deferred and built on top later (needs CPG-core containment first).
- **Module map in v1:** kept, as labeled call-derived edges (raw imports labeled
  `unresolved_import`); not deferred.
- **MCP timing:** sequenced last (after the library/CLI behavior is stable).
- **Reasoning layer / `FocusSet`:** confirmed as the next initiative; not in this
  spec, but the seams here must let it sit on top.
