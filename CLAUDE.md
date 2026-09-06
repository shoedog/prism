# CLAUDE.md — Instructions for AI Assistants Working on This Codebase

## Project Overview

Rust implementation of 30 code slicing algorithms for defect-focused automated
code review. Based on arXiv:2505.17928 plus the established program slicing
taxonomy and novel theoretical extensions.

## Build & Test

```bash
cargo build          # Build the project
cargo test           # Run all tests (unit + integration)
cargo fmt --check    # Check formatting (must pass before PR)
cargo run -- --help  # Show CLI usage
cargo run -- --list-algorithms  # List all 30 algorithms
cargo build --bin prism-mcp --features mcp  # Build the MCP stdio server
cargo test --features mcp                   # Run tests with MCP enabled
```

## Accuracy Harness (Tier-A)

When a change touches call resolution, navigation queries, or CPG construction
(`src/call_graph.rs`, `src/navigation/`, `src/cpg/`, `src/ast.rs`):

```bash
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut   # seconds, no LSP — run before committing
cd eval && uv run tier-a --quick --allow-stale-sut         # minutes, needs rust-analyzer — before review
```

Use `--allow-stale-sut` only with an immediate preceding rebuild in the same
worktree; it is for the normal dirty pre-commit state, not stale binaries.
Paste regressions/flip-candidates into the PR description rather than re-baselining.
Full multi-corpus runs (`uv run tier-a --corpus all`) are human-triggered; see
`eval/README.md`. The committed baseline lives in `docs/eval/tier-a/`.

### Before Creating a PR

Always run these checks before committing/pushing:
```bash
cargo fmt            # Fix formatting
cargo test           # All tests must pass
```

Run specific test suites (one umbrella binary per tests/ subdirectory; filter by
file-derived module name):
```bash
cargo test --test algo_paper                       # Paper algorithm tests
cargo test --test algo_taxonomy taint_cve_test::   # Taint CVE tests
cargo test --test lang_c algo_test::               # C language-specific tests
cargo test --test cli validation_test::            # CLI validation tests
cargo test --test integration core_test::          # Core integration tests
cargo test --test integration coverage_test::      # Coverage matrix
```

## Code Organization

### Core Modules (`src/`)

- `lib.rs` — Public API; re-exports all modules.
- `ast.rs` — Tree-sitter AST wrapper (`ParsedFile`). All tree-sitter interaction goes through here.
- `cfg.rs` — Control Flow Graph construction from tree-sitter AST (intraprocedural CFG edges).
- `cpg.rs` — Code Property Graph (`CpgContext`): unified graph merging AST, DFG, call graph, and CFG. This is the main interface that algorithms use.
- `call_graph.rs` — Cross-file call graph with forward/reverse edges and cycle detection.
- `data_flow.rs` — Def-use chains, reachability queries, chopping, taint propagation.
- `access_path.rs` — Structured variable access paths for field-sensitive data flow analysis (e.g., `x`, `dev->name`, `self.config.timeout`).
- `type_db.rs` — Optional C/C++ type enrichment from `compile_commands.json` + clang. Provides struct definitions, field types, typedefs, and class hierarchy.
- `languages/mod.rs` — Language-specific node type mappings. Add new languages here.
- `diff.rs` — Diff parsing and `DiffInput`/`DiffInfo` types.
- `slice.rs` — `SlicingAlgorithm` enum (30 variants), `SliceConfig`, and `SliceResult`.
- `output/` — Output formatters: `mod.rs`, `navigation.rs`, `review.rs`, `review_compact.rs`, `mermaid.rs`, `sarif.rs` (SARIF 2.1 serializer), `sarif_rules.rs` (rule descriptions + pure mapping helpers), `sarif_model.rs` (serde model structs).
- `cli.rs` — The clap `Cli` derive tree (`Cli`, `ReviewArgs`, `Command`, `NavArgs`, `NavQuery`, `TargetsArgs`); `pub mod cli` in `lib.rs` so `tests/cli/readme_test.rs` can parse-only via `Cli::try_parse_from` without executing the binary.
- `main.rs` — CLI entry point: `main()`, `run_review`/`run_nav`/`run_targets`, and the format-arm dispatch; the clap grammar itself lives in `cli.rs`.
- `algorithms/` — All 30 slicing algorithms. Each is self-contained.
- `reasoning/` — Tier-2 reasoning layer: `taint_trace` consumer, `SeedSet`, output shaper.
- `api/` — `prism::api`: the stable, `#[non_exhaustive]` facade for embedding prism as a library (`build_info`, `review`, `run`, `nav`). See its module doc for the compatibility promise; `main.rs` is its first consumer.
- `targets/` — `prism targets`: projects findings into the instrumentation-targets contract (`mod.rs`: `project`/`TargetsMeta`/id/dedupe/warnings; `model.rs`: schema DTOs; `mapping.rs`: category table, hint parsers, language lowering). Schema at `docs/contracts/targets.schema.json`.
- `finding_confidence.rs` — Single source for per-finding confidence/tier/parse-quality classification (`classify`, `evidence_files`, `ParseQuality::min_over`), consumed by both `output/sarif.rs` and `targets/`.

### Test Structure (`tests/`)

```
tests/
├── algo/              # Algorithm-specific tests
│   ├── novel/         # 8 test files (absence, echo, membrane, provenance + lang variants)
│   ├── paper/         # 1 test file (paper algorithms 6-9)
│   ├── taxonomy/      # 5 test files (taint CVE/lang/sink, misc, misc_lang)
│   └── theoretical/   # 4 test files (angle, quantum, spiral, vertical)
├── ast/               # AST infrastructure tests (access_path, binding, cpg, dfg, field)
├── cli/               # CLI tests (algo, output, validation)
├── common/            # Shared test helpers and fixture generators
│   └── mod.rs         # Re-exports core types + fixture generators per language
├── fixtures/          # Test data files (bash, c, python, terraform)
├── integration/       # Integration tests (call_graph, core, coverage)
└── lang/              # Language-specific tests
    ├── c/             # algo, complex, cve
    ├── cpp/
    ├── go/            # algo, advanced, lang
    ├── javascript/    # algo, destructuring, lang
    ├── lua/
    ├── rust/
    └── typescript/    # typescript, lang
```

Cargo.toml defines one umbrella test target per `tests/` subdirectory (e.g.,
`cargo test --test algo_paper`).
Shared test helpers in `tests/common/mod.rs` provide fixture generators like
`make_python_test()`, `make_javascript_test()`, etc.

### Language Coverage Matrix

`tests/integration/coverage_test.rs` contains a hardcoded list of test file
paths (`all_test_files`) that it scans for `fn test_*` names to build an
algorithm × language coverage matrix. **This list appears 3 times** — once in
each of `test_algorithm_language_matrix`, `test_language_coverage_minimum`, and
`test_coverage_matrix_validation`. When adding or renaming test files, all 3
copies must be updated or the matrix will under-report coverage. Run
`cargo test --test integration coverage_test::` to verify.

## Algorithm Implementation Map

### Paper (arXiv:2505.17928)
- `original_diff.rs` → Algorithm 6 (AnalysisOnlydiff)
- `parent_function.rs` → Algorithm 7 (AnalysisFunction)
- `left_flow.rs` → Algorithm 8 (AnalysisRelevantCode)
- `full_flow.rs` → Algorithm 9 (AnalysisRelevantCodeRHS)

### Established Taxonomy (SLICING_METHODS.md §4)
- `thin_slice.rs` → Data deps only, no control flow
- `barrier_slice.rs` → Depth-limited interprocedural (uses `call_graph.rs`)
- `chop.rs` → Source-to-sink paths (uses `data_flow.rs`)
- `taint.rs` → Forward taint propagation (uses `data_flow.rs`)
- `relevant_slice.rs` → LeftFlow + alternate branch paths
- `conditioned_slice.rs` → LeftFlow pruned by value predicate
- `delta_slice.rs` → Two-version data-flow diff

### Theoretical Extensions (SLICING_METHODS.md §5)
- `spiral_slice.rs` → Adaptive-depth rings (composes other algorithms)
- `circular_slice.rs` → Cross-function cycle detection (uses `call_graph.rs` + `data_flow.rs`)
- `quantum_slice.rs` → Async state enumeration (language-specific pattern detection)
- `horizontal_slice.rs` → Peer pattern consistency (decorator/name/class matching)
- `vertical_slice.rs` → End-to-end feature path (uses `call_graph.rs` + layer heuristics)
- `angle_slice.rs` → Cross-cutting concern trace (keyword pattern matching)
- `threed_slice.rs` → Temporal-structural risk (uses `call_graph.rs` + git)

### Novel Extensions
- `absence_slice.rs` → Missing counterpart detection (open/close, lock/unlock)
- `resonance_slice.rs` → Git co-change coupling (requires git history)
- `symmetry_slice.rs` → Broken symmetry detection (serialize/deserialize, encode/decode)
- `gradient_slice.rs` → Continuous relevance scoring (decaying scores)
- `provenance_slice.rs` → Data origin tracing (user_input, config, database, etc.)
- `phantom_slice.rs` → Recently deleted code surfacing (requires git history)
- `membrane_slice.rs` → Module boundary impact (cross-file callers of changed APIs)
- `echo_slice.rs` → Ripple effect modeling (callers missing error handling)
- `contract_slice.rs` → Implicit behavioral contract extraction and violation detection (guard clauses, postconditions)
- `peer_consistency_slice.rs` → Peer-signature guard divergence — sibling functions sharing a first-parameter name where some/all lack a NULL guard
- `callback_dispatcher_slice.rs` → Resolve function-pointer-in-struct registrations to their dispatcher invocation sites; flags NULL argument passing
- `primitive_slice.rs` → Deterministic security-primitive fingerprint sweep (hash truncation, weak-hash-for-identity, shell=True interpolation, disabled cert validation, hardcoded secrets)

## Architecture

### CpgContext (Code Property Graph)

The modern architecture centers on `CpgContext`, which bundles:
- `cpg: CodePropertyGraph` — unified graph merging AST, DFG, call graph, and CFG
- `files: &BTreeMap<String, ParsedFile>` — parsed ASTs
- `type_db: Option<&TypeDatabase>` — optional C/C++ type enrichment

Whole-repo `prism nav` indexes use a prism-owned per-repo cache at
`dirs::cache_dir()/prism/nav/<hash(canonical repo root)>/`, with `--no-cache`
and `--cache-dir` gating that navigation store specifically.
Navigation queries include `nodes-at`, `symbol-spans`, `callers`, `callees`, `ego`,
`module-deps`, `repo-map`, `dfg-stats`, and the CLI-only `onboard` report.
`module-deps`/`repo-map` live in `src/navigation/module_graph.rs`; onboarding report
construction lives in `src/navigation/onboarding.rs` and deterministic Markdown/JSON
rendering in `src/output/onboarding.rs`. `onboard --out` is explicit create-new-only
output and must never overwrite an existing file.
Navigation and CPG call resolution share one ordered ladder (S3),
`CallGraph::resolve_call_site` in `src/resolution.rs`, which returns each callee
with a `ResolutionConfidence` (`Exact` | `NameOnly`) and a `ResolutionKind`. The
rungs (R1–R7): qualified `T::m`/`mod::T::m` and `Self::`/`self.`/receiver-var
calls bind via a per-`(owner, name)` method index (Rust trait impls are dual-keyed
under the trait, demoted when >1 impl); import-qualified `pkg.f()` narrows by file
stem or Go package directory with no permissive fall-through; `Class.m()` binds
when the qualifier is itself an owner key; unqualified calls prefer a local free
definition, then Java/C++ implicit-`this`, then cross-file free functions
(methods excluded — a method needs a receiver); unknown-receiver `x.m()` uses
**P6-lite** syntactic receiver typing behind a swappable `ReceiverClassifier` seam
(`resolution.rs`, `legacy ↔ expanded`): Rust/Go typed params + constructor locals,
plus Phase-IP PR-2's Go type-assertion (`x.(Module).M()`) and `var`-declared
(`var r Runner`) receivers, with a std-wrapper peel list and shadow-bail; recovered
receivers route through the existing `owner_lookup → interface_impls → drop` ladder
(recover-and-route — recovery is syntactic, routing decides interface-vs-concrete).
It otherwise demotes a single in-repo owner / drops a multi-owner collision (the
precision floor); module-qualified free functions fall back to file-stem matching.
Navigation maps confidence to `score` (Exact 1.0 / NameOnly 0.6) with a
`Reason::Resolution` and a `Collision` warning when same-name receiver sites are
dropped. Phase-IP type-confirmed dispatch has **shipped** Go embedding promotion
(#95) and Go interface satisfaction (#96); remaining gaps (spec §2.4): Python
inheritance, field/return-typed receivers (the S3.1 struct-field-index candidate),
cross-package concrete-asserted keys, and package-level `var` receivers.
`prism nav call-stats --repo <dir>` reports the resolution-kind histogram and drop
classification; `prism nav interface-manifest --repo <dir>` emits the PR-2 in-scope
interface-dispatch manifest (the §8a denominator for the precision gate report).

`prism nav dfg-stats --repo <dir>` reports `dfg_label_exact`,
`dfg_label_loop_carried`, five mutually exclusive `dfg_label_nameonly_*` doubt counters, and the
two reaching-definition availability counters. Loop-carried is a subset of Exact, so labeled-edge
count is Exact plus the five NameOnly counters. `--edges` emits sorted JSONL with the doubt
vocabulary `killed`, `sameline`, `cfg_incomplete`, `alias_unstable`, and `call_nameonly`;
`kill_line` is present only for `killed`. The same counters are nested under `call-stats.dfg_labels`.

Finding confidence is `exact | nameonly | unlabeled`; tier is `asserted` only for Exact evidence
whose evidence-bearing files all parse cleanly, otherwise `candidate`. `asserted` grades the
evidence path, not the heuristic's truth. `--resolution nominal` is the default and reports
CPG-derived findings as `unlabeled/candidate`; `--resolution scoped` reports retained evidence
labels. `--min-confidence exact` keeps only Exact findings. Its default, `nameonly`, retains all
three confidence values, including ungraded Unlabeled findings. The filter is supported only by
finding-bearing `json`, `review`, `sarif`, and `targets`; the CLI rejects it for `text`, `paper`,
`mermaid`, and `callers`.

CPG cache v73 persists DataFlow labels and per-file RD statistics across cold, full-hit, and
partial-hit builds. The B8 rule that capture reads in deferred or nested callables become
`NameOnly(CfgIncomplete)` is **PROVISIONAL** pending callable-timing work.

### MCP Adapter

> **User-facing install/usage guide:** [`docs/MCP.md`](docs/MCP.md) — `claude mcp add` / Codex / Kiro
> wiring, cache warming, the gotchas, and the bundled agent **skills** in [`skills/`](skills/)
> (`prism-code-navigation`, `prism-code-slicing`). This section is the architecture notes.

`prism-mcp` is a local stdio MCP server behind the cargo `mcp` feature:

```bash
cargo run --bin prism-mcp --features mcp -- --repo /path/to/repo
```

The server exposes nine tools: seven read-only navigation tools (the six graph/evidence
queries return Prism `Evidence` JSON; `nav_symbol_spans` returns a dedicated coordinate
result), one read-only reasoning tool `taint_reaches` (also Evidence), and one
non-destructive local-state-changing tool `refresh_index` (returns a refresh summary).

- `nav_nodes_at` — evidence for a repository file and 1-indexed line.
- `nav_symbol_spans` — exact read-only callable coordinates without source text.
- `nav_callers` — incoming callers for a symbol or location seed.
- `nav_callees` — outgoing callees for a symbol or location seed.
- `nav_ego_graph` — local graph around a symbol or location seed.
- `nav_module_deps` — outbound module dependencies for one file.
- `nav_repo_map` — whole-repository module dependency graph.
- `taint_reaches` — forward taint reachability from a seed.
- `refresh_index` — re-indexes the repo snapshot for this server session.

Build, test, or lint MCP code with `--features mcp`; the default build keeps
the adapter disabled.

`Evidence` also has an additive optional `reasoning` field. It is omitted when
absent so existing navigation and diff-review output remains byte-compatible.

Algorithms fall into two categories:
1. **Simple/AST-only** (use `ctx.files` only; `phantom_slice`/`resonance_slice` also read git history): `original_diff`, `parent_function`, `thin_slice`, `quantum_slice`, `horizontal_slice`, `angle_slice`, `absence_slice`, `symmetry_slice`, `phantom_slice`, `resonance_slice`, `contract_slice`, `peer_consistency_slice`, `callback_dispatcher_slice`, `primitive_slice`
2. **Graph-based** (require the CPG; source of truth `SlicingAlgorithm::needs_cpg()`, src/slice.rs:210): `left_flow`, `full_flow`, `relevant_slice`, `conditioned_slice`, `barrier_slice`, `chop`, `taint`, `delta_slice`, `spiral_slice`, `circular_slice`, `vertical_slice`, `threed_slice`, `gradient_slice`, `provenance_slice`, `membrane_slice`, `echo_slice`

### Algorithm Dispatch

`src/algorithms/mod.rs` contains:
- `run_slicing(ctx: &CpgContext, diff: &DiffInput, config: &SliceConfig)` — main dispatcher
- `run_slicing_compat(...)` — backward-compatible wrapper that builds `CpgContext` automatically
- `check_parse_warnings(...)` — reports tree-sitter parse errors (>10% warn, >30% skip)

## Key Design Decisions

1. **Tree-sitter for multi-language AST parsing.** The original paper used
   cppcheck (C++ only). We use tree-sitter to support 11 languages.

2. **Name-based variable tracking** instead of cppcheck's `varId` system.

3. **BTreeMap/BTreeSet everywhere** for deterministic, sorted output.

4. **Shared infrastructure:** `call_graph.rs`, `data_flow.rs`, `cfg.rs`, and
   `cpg.rs` are reused across multiple algorithms. Build them once via
   `CpgContext`, pass to algorithms.

5. **Algorithm-specific configs** live in each algorithm's module (e.g.,
   `BarrierConfig`, `TaintConfig`, `SpiralConfig`), not in the central
   `SliceConfig`, to keep the core config lean.

6. **Field-sensitive analysis** via `access_path.rs` tracks structured paths
   (e.g., `self.config.timeout`) rather than just variable names.

7. **Keep files under 600 lines.** Split test files and source modules when
   they approach this limit. For tests, group by category (e.g., `algo_test.rs`,
   `advanced_test.rs`, `lang_test.rs`) and register **one umbrella `[[test]]`
   target per `tests/` subdirectory** (its `main.rs` declares the files as
   modules); individual files stay under 600 lines.

## Supported Languages

11 languages (12 tree-sitter grammar variants — TSX is parsed separately from
TypeScript):
Python, JavaScript, TypeScript, Go, Java, C, C++, Rust, Lua, Terraform/HCL, Bash.

## CLI Usage

```bash
# Single algorithm
cargo run -- --repo /path/to/repo --diff diff.patch --algorithm leftflow

# Multiple algorithms
cargo run -- --repo /path/to/repo --diff diff.patch --algorithm "leftflow,fullflow,taint"

# Preset suites
cargo run -- --repo /path/to/repo --diff diff.patch --algorithm review  # review suite
cargo run -- --repo /path/to/repo --diff diff.patch --algorithm all     # all 30

# Output formats: text (default), json, paper, review, callers, mermaid, sarif
cargo run -- --repo /path/to/repo --diff diff.patch --format json

# Navigation cache controls: these gate the nav store, not review CPG caching
cargo run -- nav --no-cache callers --repo /path/to/repo --symbol run
cargo run -- nav --cache-dir /tmp/prism-nav callers --repo /path/to/repo --symbol run
cargo run -- nav module-deps --repo /path/to/repo --file src/main.rs --format json
cargo run -- nav repo-map --repo /path/to/repo --format json
cargo run -- nav dfg-stats --repo /path/to/repo
cargo run -- nav dfg-stats --repo /path/to/repo --edges
```

Key algorithm-specific flags:
- `--barrier-depth`, `--barrier-symbols` (BarrierSlice)
- `--chop-source`, `--chop-sink` (Chop)
- `--taint-source` (Taint, repeatable)
- `--condition` (ConditionedSlice)
- `--old-repo` (DeltaSlice)
- `--spiral-max-ring` (SpiralSlice)
- `--quantum-var` (QuantumSlice)
- `--peer-pattern` (HorizontalSlice)
- `--layers` (VerticalSlice)
- `--concern` (AngleSlice)
- `--temporal-days` (ThreeDSlice, ResonanceSlice)
- `--compile-commands` (C/C++ type enrichment)

## Adding a New Language

1. Add the tree-sitter grammar crate to `Cargo.toml`
2. Add a variant to `Language` enum in `src/languages/mod.rs`
3. Implement all the node type methods for the new language
4. Add a fixture generator in `tests/common/mod.rs`
5. Add language-specific tests in `tests/lang/` and add a `mod <stem>;` line to
   that directory's `main.rs`
6. Add the new test file paths to the `all_test_files` arrays in
   `tests/integration/coverage_test.rs` (there are 3 copies — one each in
   `test_algorithm_language_matrix`, `test_language_coverage_minimum`, and
   `test_coverage_matrix_validation`)

## Adding a New Slicing Algorithm

1. Create `src/algorithms/your_algo.rs` with a `pub fn slice(...)` function
2. Add variant to `SlicingAlgorithm` in `src/slice.rs` (both enum and `from_str`/`name`/`all`)
3. Add `pub mod your_algo;` in `src/algorithms/mod.rs`
4. Wire it up in the `run_slicing` dispatcher in `src/algorithms/mod.rs`
5. Add CLI flags in `src/main.rs` if it needs algorithm-specific config
6. Add tests in `tests/algo/` (appropriate subcategory) and add a `mod <stem>;`
   line to that directory's `main.rs`
7. Add the algorithm to the `algorithms` list in `test_algorithm_language_matrix`
   and `test_language_coverage_minimum` in `tests/integration/coverage_test.rs`

## Common Patterns

- **Line numbers are 1-indexed** throughout. Tree-sitter uses 0-indexed rows;
  conversion happens in `ast.rs`.
- **`DiffBlock.file_line_map`** maps `filename → (line_number → is_diff_line)`.
- **Cross-file references**: Many algorithms include lines from multiple files.
  These appear as additional entries in `file_line_map`.
- **Algorithms that need call graph or data flow** receive them via `CpgContext`.
  The graph is built once and shared across algorithm invocations.
- **Finding severity has four values**: `info`, `suggestion`, `warning`, `concern`
  (`SliceFinding.severity`, `src/slice.rs`; ranked in that order by
  `output::severity_rank`). `--format review`'s default floor is `warning`;
  SARIF maps unknown severities to `error` (louder, never quieter); targets
  maps unknown severities to `concern` and records a warning.

## Dependencies

- `tree-sitter` + 11 language grammar crates (12 parsed language variants) for AST parsing
- `petgraph` for graph data structures (CFG, CPG)
- `clap` for CLI
- `dirs` for prism-owned cache directory discovery
- rayon for parallel file parsing/extraction in the CPG build
- `serde`/`serde_json` for serialization
- `anyhow`/`thiserror` for error handling
- `build.rs` emits `GRAMMAR_FINGERPRINT` from `Cargo.lock` tree-sitter grammar versions for cache invalidation
- `tempfile`, `assert_cmd`, `predicates` (dev-dependencies for testing)
