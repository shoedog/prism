## Merged Spec Review: Prism Navigation Layer (Tier 1)

---

### BLOCKER 1 — Symbol identity: stable IDs at the API surface do not close the CPG-internal collision

**Both lenses; soundness is more precise.**

The spec proposes `FunctionIdRef { file, name, start_line, end_line, ordinal }` for collision-safe API IDs. But `assemble_graph` still inserts into `func_index` with key `(file, name)` (cpg.rs:566, 746) — last-writer-wins. Any Rust file with idiomatic `fn new()`, `fn default()`, or `fn fmt()` across `impl` blocks (including this repo) silently overwrites earlier entries. Every CPG-backed query (`callers`, `callees`, variable `Contains` edges, future `ego_graph` traversal) for an overwritten function returns wrong data or nothing, with no warning. Stable `FunctionIdRef` makes the collision observable at query time; it does not prevent it at build time.

**Resolution:** Either change `func_index` to key by `(file, name, start_line)` in CPG core, or detect collisions at index-build time, emit a `parse_quality` diagnostic per affected function, and refuse CPG-backed sub-queries for those symbols rather than returning wrong data. The spec must state explicitly which path is taken before the nav layer is implemented.

---

### BLOCKER 2 — JSON output contracts are not buildable

**Rigor only; soundness did not address (not yet implemented).**

The spec says navigation returns "compact JSON with source locations, snippets only when requested, and evidence fields" but gives no concrete schemas for `symbols`, `definition`, `references`, `callers`, `callees`, `nodes-at`, `ego-graph`, `dataflow`, or `chop`. The localization section has a partial candidate shape but score range, evidence variants, ordering, warnings, and error payloads are undefined. Without serde structs and golden examples per command — including empty-result, ambiguous-result, and error cases — implementation is unconstrained and LLM-caller contracts cannot be validated.

**Resolution:** Add concrete serde structs and at least one golden JSON example per navigation command before implementation begins.

---

### BLOCKER 3 — Whole-repo ingestion semantics are underspecified

**Rigor only.**

The spec says to walk the repo, filter by supported extension, exclude "obvious generated/vendor/build paths," parse, build, and cache. Unresolved: `.gitignore` and nested ignores, symlinks, hidden directories, invalid UTF-8, per-file read errors, maximum file size, path canonicalization, generated-file detection heuristics, and skip diagnostics visible to callers. Current CLI only parses diff-referenced files (main.rs:312–335); the whole-repo loader is a new behavioral surface and callers cannot predict what is included or skipped without an explicit contract.

**Resolution:** Define the traversal/skip contract in full before implementing `repo_loader.rs`. At minimum: gitignore honor, symlink policy, max-file-size limit, and `SkippedFile` reason codes.

---

### BLOCKER 4 — Cache identity is incomplete: namespace layout, skip-policy versioning, and grammar-version fingerprinting are all missing

**Both lenses; soundness identified the grammar-version gap in code.**

The spec lists cache keys (Prism version, cache-format version, repo-root identity, file set, file hashes, graph profile, skip-policy version, type-db presence/key, supported language set) but does not define namespace layout or how those keys are serialized. Concretely: `CpgCache` (cpg_cache.rs:47–61) stores `version`, `prism_version`, `file_hashes`, `has_type_db` — no grammar-crate version, no skip-policy version. After a `cargo update` that bumps a tree-sitter grammar, the cache returns stale `ParsedFile` trees without a miss. The skip-policy version field is named in the spec but not defined (what changes it?).

**Resolution:** Add a compile-time grammar-version fingerprint (built from grammar crate versions in `Cargo.lock`) to `CpgCache`. Define what constitutes a skip-policy version change. Specify the full cache key serialization before implementing the nav cache.

---

### MAJOR 1 — `function_at` is O(n_functions); must be addressed before MCP exposure

**Soundness only; code-grounded.**

`function_at` (cpg.rs:1489–1505) iterates the full `func_index` BTreeMap, filters by file string, then checks `[start_line, end_line]`. No file-partitioned sub-index, no interval structure. This is separate from `nodes_at` (which is O(1) via `location_index`). Any nav query requiring function context — resolving which function owns a line, scoping callers/callees, tagging returned nodes with their enclosing function — calls `function_at`. At MCP call rates (every editor keystroke or agent step), O(n_functions) per call is a cost cliff.

**Resolution:** Add a `line_range_index: BTreeMap<String, Vec<(usize, usize, NodeIndex)>>` sorted by `start_line` enabling binary-search per file. The spec should name this as a prerequisite before MCP exposure, not an optimization-later note.

---

### MAJOR 2 — Qualifier-aware caller resolution: `resolve_callers` ignores qualifiers; spec must commit to a path

**Both lenses.**

The spec lists `callers`/`callees`/`references`/`definition` as stable navigation operations and says to use qualifier-aware traversal. CPG construction already uses `resolve_callees_qualified` for call and data-flow edges (cpg.rs:820–869), but `resolve_callers` ignores `CallSite.qualifier` (call_graph.rs:801–809). If the navigation layer uses CPG edges for caller queries, qualify this explicitly. If it uses call-graph indexes, `resolve_callers` needs qualifier-aware logic with tests for imported/qualified calls before the layer can be called stable.

**Resolution:** State in the spec which code path `callers` uses (CPG edges vs call-graph index). If call-graph index, fix `resolve_callers` and add regression tests for qualified imports.

---

### MAJOR 3 — `ego-graph` is underspecified at both API and capability levels

**Both lenses; soundness is more precise on the capability gap.**

The spec proposes `ego_graph(session, seed, hops, edges)` but does not define edge selector grammar, traversal direction, inclusion of the seed node, BFS ordering, cycle handling, node/edge output shape, or multi-seed behavior. More critically: `Contains` edges exist only function→variable (cpg.rs:942); there are no function→statement containment edges. "Ego graph" conventionally implies a rich local neighborhood including internal structure, but what is actually reachable is a call-neighborhood plus variable nodes. If the reasoning layer's `FocusSet` seam assumes statement containment from `ego_graph`, that assumption is wrong at spec time.

**Resolution:** Add an explicit capability statement to the `ego_graph` API doc: what edge types are selectable, what is actually reachable with each, and that statement-level containment is not available until CPG core adds it. Confirm with the reasoning-layer design that `FocusSet` does not depend on statement containment before treating Phase 2 as fully specified.

---

### MAJOR 4 — Localization scoring is underspecified

**Rigor only.**

The pipeline is described as lexical/BM25-like plus CPG expansion and graph scoring, producing ranked advisory candidates. That is sufficient for intent, not implementation. Undefined: tokenizer, fields indexed, score components and weights, tie-breakers, maximum candidates returned, warning/error fields, and how structured hints from the open question interact with natural-language queries.

**Resolution:** Define scoring components and weights, maximum candidates, tie-breaker rules, and the output schema for warnings and errors before implementation.

---

### MAJOR 5 — Definition/reference precision backend boundaries lack provenance in output

**Rigor only.**

The spec recommends optional SCIP/Sourcegraph/Glean precision backends with local CPG fallback but does not define adapter contracts or provenance labeling in output. Callers cannot judge result confidence without knowing whether a hit came from Prism CPG, heuristic import resolution, or an external index, and whether fallback happened.

**Resolution:** `Evidence` results must carry a `source` field (e.g., `PrismCpg`, `HeuristicImport`, `ExternalIndex { name }`) and a `fallback: bool` flag. Define the adapter trait contract before implementing `ScipResolver`.

---

### MAJOR 6 — CLI compatibility requires a concrete matrix, not a prose guarantee; flatten hazard needs a design decision

**Both lenses.**

"Preserve existing behavior byte-for-byte" is not testable as written. The spec also leaves open the subcommand design, and if `ReviewArgs` is flattened via `#[command(flatten)]`, review-specific flags (`--diff`, `--algorithm`) are silently accepted without error when running `prism nav callers --diff foo.patch` — users get no feedback that the flags have no effect. The binary/clap name mismatch (`prism` vs `slicing`) is confirmed at Cargo.toml:8 and main.rs:38.

**Resolution:** Produce a concrete compatibility matrix: stdout, stderr, exit codes, help text, validation errors, `--list-algorithms`, and `--format` variants that must remain stable. Decide explicitly whether to gate flatten on subcommand presence or assert/warn in the nav dispatch branch when review-specific fields are non-default.

---

### MINOR 1 — Seed input forms, ambiguity errors, and tie rules are underspecified

**Rigor only.**

Navigation commands accept seeds like `symbol process_request` and `callers src/api.py:42` but the spec does not define accepted seed forms, ambiguity errors, candidate listing, location normalization, or deterministic tie rules. This partially overlaps with BLOCKER 1 (collision) but also covers the input parsing surface.

**Resolution:** Define a seed grammar and error protocol (e.g., `AmbiguousSymbol { candidates: Vec<FunctionIdRef> }`) before implementing `queries.rs`.

---

### MINOR 2 — Test plan should specify acceptance-level golden fixtures

**Rigor only.**

The roadmap asks for "focused integration tests" but the scenarios that matter for correctness are specific: duplicate symbols, static functions, qualified imports, unsupported files, cache invalidation, CLI legacy compatibility, empty results, and ambiguous seeds.

**Resolution:** Replace "focused integration tests" with a named golden-fixture list per scenario so implementation has a testable definition of done.

---

### MINOR 3 — `SymbolResolver` session coupling is avoidable

**Soundness only.**

The proposed trait signature `definition(session: &NavigationSession, sym: &FunctionIdRef)` couples `ScipResolver` — which reads a `.scip` file and has no use for the in-memory session graph — to the session lifetime unnecessarily, making the impl awkward and harder to test in isolation.

**Resolution:** Use a `ResolverContext<'_>` enum (`Session(&NavigationSession)` | `ExternalIndex(&Path)`) so the trait is honest about what each impl needs, or split into two traits composed at the call site.

---

**Verdict:** Not ready to plan. Four blockers must be resolved first: `func_index` CPG-internal collision path chosen and documented, JSON output schemas specified, whole-repo ingestion traversal contract written, and cache key fully defined (including grammar-version fingerprint). The design direction is sound and the repo has sufficient CPG/call/data-flow primitives; the gap is implementable contracts, not missing foundations.