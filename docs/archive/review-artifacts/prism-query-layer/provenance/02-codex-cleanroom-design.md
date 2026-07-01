I’ll verify the draft against the current repo read-only, focusing on the cited seams that could have drifted: `CpgContext`, call graph data, CLI shape, cache semantics, and output modules. Then I’ll stop and write the second pass with corrections tied to actual lines.The worktree is clean and the draft’s file set exists. I’m reading the exact implementations now so the second pass can separate “confirmed” from “assumed” instead of preserving draft claims that are too broad.A correction is emerging: the first draft treated “sources” as a natural whole-repo loader output, but `ParsedFile` already retains source text, and the existing CLI only keeps a separate source map for cache hashing. I’m checking parser and cache details before locking that boundary down.The current CLI is strictly flat, not command-based, so adding `nav` would require a real clap enum split rather than just branching on an optional command field. I’m also checking whether module/import data is rich enough for a module map or whether that remains a v1 heuristic.Another important nuance: there are already `CodePropertyGraph` convenience traversal methods, but they return only functions/depth and lose the originating call-site. For navigation, those are useful for compatibility checks, while explainable output still needs direct access to `CallGraph::calls` and `CallGraph::resolve_callers`.One more mismatch with the draft: the binary is named `prism`, while clap currently advertises the command name as `slicing`. That affects the compatibility contract and the exact shape of new subcommands. I’m checking tests around the CLI so the refined plan names the right regression boundary.**GAPS / UNCERTAINTIES REGISTER**

1. The draft’s `NavigationContext { repo, ctx }` shape is risky if it ever owns the repo. `CpgContext` borrows `files` (`src/cpg.rs:56`, `src/cpg.rs:60`), so an owning persistent MCP/session state would become self-referential. Refine into separate owned `LoadedRepo`, owned `NavigationIndex`, and borrowed per-query/session facade.

2. `NodeRef` cannot rely on CPG’s `(file, function_name)` index alone. `func_index` is keyed only by `(String, String)` (`src/cpg.rs:566`) and insertion uses that key (`src/cpg.rs:746`), so overloads, duplicate methods, generated duplicate names, or same-file duplicate symbols can overwrite lookup entries. Stable IDs need line range and possibly ordinal.

3. `nodes_at` is narrower than “all semantic context at this location.” It returns only nodes indexed at exactly `(file, line)` (`src/cpg.rs:1190`). Function nodes are indexed only at `start_line` (`src/cpg.rs:747`), so `nodes-at` must also call `function_at` for enclosing function context (`src/cpg.rs:1487`).

4. The draft overstated containment. `CpgEdge::Contains` is documented as function-to-statement-or-variable (`src/cpg.rs:472`), but current assembly adds Contains only from function to variable nodes (`src/cpg.rs:942`). Statement nodes are created (`src/cpg.rs:950`) but not contained by functions. Ego/module structure must not depend on statement containment until CPG core adds it.

5. `CallAndStructure` is underspecified. `assemble_graph` always expects a `DataFlowGraph`, creates variables from DFG (`src/cpg.rs:754`), then adds statements and CFG (`src/cpg.rs:950`, `src/cpg.rs:966`). Passing `DataFlowGraph::empty()` is possible (`src/data_flow.rs:77`) but changes `nodes_at` by removing variable nodes and Contains edges. This should not be the v1 default.

6. Existing call traversal helpers are not enough for explainable navigation. `CallGraph::callers_of_in_file` returns only `(FunctionId, depth)` and tracks visited by function name (`src/call_graph.rs:751`), while `callees_of` uses unqualified `resolve_callees` (`src/call_graph.rs:849`) instead of qualifier-aware resolution. Navigation needs direct traversal over `CallSite` plus `resolve_callees_qualified` (`src/call_graph.rs:654`).

7. Call-site evidence must come from retained `CallGraph`, not CPG edges. The CPG explicitly retains `CallGraph` because Function-to-Function Call edges do not capture call-site locations (`src/cpg.rs:576`). The output model should expose `CallSite { caller, callee_name, line, qualifier }` (`src/call_graph.rs:21`).

8. Module maps are heuristic in v1. Imports are extracted only for Python, JS/TS/TSX, and Go (`src/ast.rs:295`), and module paths are “as-is,” not filesystem-resolved (`src/ast.rs:288`). Resolver matching also has same-stem ambiguity (`src/call_graph.rs:651`). Module dependency output must label unresolved/import-derived evidence explicitly.

9. The draft’s `LoadedRepo { sources }` duplicates data. `ParsedFile` already stores `source` (`src/ast.rs:45`). The current CLI keeps a separate `sources` map for cache hashing/output (`src/main.rs:313`), but whole-repo navigation should prefer `ParsedFile.source` plus precomputed `file_hashes`, not a second full source copy.

10. CLI growth is more invasive than the draft implied. There is no current `cli.command`; the CLI is a flat `Parser` with `repo` and `diff` required unless `--list-algorithms` (`src/main.rs:42`, `src/main.rs:52`). Adding `nav` requires an optional clap subcommand split while preserving legacy top-level args.

11. Cache reuse must be separated. Existing cache metadata is for one CPG with file hashes and type-db presence (`src/cpg_cache.rs:45`) and the module states it covers files referenced in the current diff (`src/cpg_cache.rs:10`). Partial hit only works for identical file sets (`src/cpg_cache.rs:276`). Whole-repo nav needs its own metadata and exact-hit policy first.

12. The binary/test contract is `prism`, even though clap’s command name is currently `"slicing"` (`Cargo.toml:8`, `src/main.rs:38`). Regression tests invoke `Command::cargo_bin("prism")` (`tests/cli/output_test.rs:6`). Do not rename or reshape help casually while adding modes.

**REFINED DESIGN**

Keep navigation as an opt-in, repo-wide library layer over the existing CPG and call graph. Do not fold it into slicing algorithms and do not make repo-wide indexing part of the diff-review path.

Core ownership seam:

```rust
pub struct LoadedRepo {
    pub root: PathBuf,
    pub files: BTreeMap<String, ParsedFile>,
    pub file_hashes: BTreeMap<String, String>,
    pub skipped: Vec<SkippedFile>,
    pub type_db: Option<TypeDatabase>,
}

pub struct NavigationIndex {
    pub cpg: CodePropertyGraph,
    pub profile: GraphBuildProfile,
    pub parse_quality: BTreeMap<String, FileParseQuality>,
}

pub struct NavigationSession<'a> {
    pub repo: &'a LoadedRepo,
    pub index: &'a NavigationIndex,
}
```

This avoids a self-referential context. `LoadedRepo` owns parsed files. `NavigationIndex` owns the graph. `NavigationSession` borrows both for queries.

Add modules:

- `src/repo_loader.rs`: whole-repo discovery, supported-language filtering via `Language::from_path` (`src/languages/mod.rs:42`), parsing via `ParsedFile::parse` (`src/ast.rs:63`), file hashes, skip policy.
- `src/navigation/types.rs`: stable serializable IDs, locations, edges, evidence, errors.
- `src/navigation/resolver.rs`: resolver trait over current call graph resolution.
- `src/navigation/queries.rs`: pure query execution.
- `src/navigation/module_graph.rs`: call/import-derived file and directory graph.
- `src/navigation/cache.rs`: separate whole-repo nav cache.
- `src/output/navigation.rs`: JSON/text formatting, separate from review output.

Use stable IDs like:

```rust
FunctionIdRef { file, name, start_line, end_line, ordinal }
StatementRef { file, line, kind, ordinal }
VariableRef { file, function, line, path, access, ordinal }
```

Raw `NodeIndex` may appear only as response-local debug metadata, never as the durable API key.

Query behavior:

- `nodes_at(file, line)`: return exact CPG nodes from `nodes_at`, plus enclosing function from `function_at`. Mark exact vs enclosing evidence separately.
- `callers(seed, depth)`: resolve seed to one or more `FunctionId`s, then traverse `CallGraph::callers` and filter each `CallSite` through `resolve_callers` / qualified resolution as needed. Return the call line.
- `callees(seed, depth)`: traverse `CallGraph::calls`; resolve each `CallSite` with `resolve_callees_qualified`, not the convenience `callees_of`.
- `ego_graph(seed, hops, edge_filter)`: BFS over CPG edges for structural neighborhoods. For Call/Return edges, attach recovered `CallSite` evidence when possible.
- `dataflow(...)`: require `FullCpg`.
- `module_deps` / `repo_map`: v1 derives resolved call file-to-file edges, plus optional raw import edges labeled `unresolved_import`.

Build profiles:

```rust
pub enum GraphBuildProfile {
    FullCpg,
    CallStructureExperimental,
}
```

V1 should default to `FullCpg` for correctness because current CPG behavior and tests assume DFG-backed variables and data-flow. Add `CallStructureExperimental` later inside `src/cpg.rs`, not in navigation, after adding explicit function-to-statement containment and tests proving `nodes_at`, callers, callees, and ego graphs behave as documented.

CLI seam:

Refactor current flat CLI into `ReviewArgs`, but preserve legacy invocation:

```text
prism --repo . --diff changes.patch --algorithm review
prism nav nodes-at --repo . --location src/main.rs:498 --format json
prism nav callers --repo . --symbol foo --file src/lib.rs --depth 2
```

Implementation shape:

```rust
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    review: ReviewArgs,
}

enum Command {
    Nav(NavArgs),
    Mcp(McpArgs),
}
```

In `main`, route `Some(Command::Nav)` before legacy validation. If no subcommand, run the extracted current review path and keep `--repo`/`--diff` validation, `--format callers`, cache behavior, parse warnings, and outputs unchanged.

Cache policy:

Navigation cache gets its own namespace and metadata: Prism version, cache format version, repo root identity, file set, file hashes, graph profile, skip policy version, type-db presence/key, and supported-language set. V1 accepts exact hits only. Changed files, added/removed files, profile changes, type-db changes, or Prism version changes are misses.

Delivery order:

1. Extract legacy review path with no behavior change.
2. Add `repo_loader` and `LoadedRepo`.
3. Add `NavigationIndex` using `FullCpg`.
4. Ship `nodes-at`.
5. Ship callers/callees with call-site evidence.
6. Ship bounded CPG ego graph.
7. Add `prism nav ...` JSON output.
8. Add exact-hit navigation cache.
9. Add module/repo map.
10. Add MCP adapter after CLI/library behavior is stable.
11. Add optimized `CallStructureExperimental` only after containment and duplicate-symbol tests.