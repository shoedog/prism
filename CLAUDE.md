# CLAUDE.md — Instructions for AI Assistants Working on This Codebase

## Project Overview

Rust implementation of 26 code slicing algorithms for defect-focused automated
code review. Based on arXiv:2505.17928 plus the established program slicing
taxonomy and novel theoretical extensions.

## Build & Test

```bash
cargo build          # Build the project
cargo test           # Run all tests (unit + integration)
cargo run -- --help  # Show CLI usage
cargo run -- --list-algorithms  # List all 26 algorithms
```

Run specific test suites:
```bash
cargo test --test algo_paper          # Paper algorithm tests
cargo test --test algo_taint_cve      # Taint CVE tests
cargo test --test lang_c_algo         # C language-specific tests
cargo test --test cli_validation      # CLI validation tests
cargo test --test integration_core    # Core integration tests
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
- `slice.rs` — `SlicingAlgorithm` enum (26 variants), `SliceConfig`, and `SliceResult`.
- `output.rs` — Output formatters (text, JSON, paper-compatible, review).
- `main.rs` — CLI entry point using clap with 22+ algorithm-specific flags.
- `algorithms/` — All 26 slicing algorithms. Each is self-contained.

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

Cargo.toml defines named test targets (e.g., `cargo test --test algo_paper`).
Shared test helpers in `tests/common/mod.rs` provide fixture generators like
`make_python_test()`, `make_javascript_test()`, etc.

### Language Coverage Matrix

`tests/integration/coverage_test.rs` contains a hardcoded list of test file
paths (`all_test_files`) that it scans for `fn test_*` names to build an
algorithm × language coverage matrix. **This list appears 3 times** — once in
each of `test_algorithm_language_matrix`, `test_language_coverage_minimum`, and
`test_coverage_matrix_validation`. When adding or renaming test files, all 3
copies must be updated or the matrix will under-report coverage. Run
`cargo test --test integration_coverage` to verify.

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

## Architecture

### CpgContext (Code Property Graph)

The modern architecture centers on `CpgContext`, which bundles:
- `cpg: CodePropertyGraph` — unified graph merging AST, DFG, call graph, and CFG
- `files: &BTreeMap<String, ParsedFile>` — parsed ASTs
- `type_db: Option<&TypeDatabase>` — optional C/C++ type enrichment

Algorithms fall into two categories:
1. **Simple** (use `ctx.files` only): `original_diff`, `parent_function`, `left_flow`, `full_flow`, `thin_slice`, `relevant_slice`, `quantum_slice`, `horizontal_slice`, `angle_slice`, `absence_slice`, `symmetry_slice`
2. **Graph-based** (use `ctx.cpg` or full context): `barrier_slice`, `taint`, `spiral_slice`, `circular_slice`, `vertical_slice`, `threed_slice`, `delta_slice`, `conditioned_slice`, `gradient_slice`, `provenance_slice`, `phantom_slice`, `resonance_slice`, `membrane_slice`, `echo_slice`

### Algorithm Dispatch

`src/algorithms/mod.rs` contains:
- `run_slicing(ctx: &CpgContext, diff: &DiffInput, config: &SliceConfig)` — main dispatcher
- `run_slicing_compat(...)` — backward-compatible wrapper that builds `CpgContext` automatically
- `check_parse_warnings(...)` — reports tree-sitter parse errors (>10% warn, >30% skip)

## Key Design Decisions

1. **Tree-sitter for multi-language AST parsing.** The original paper used
   cppcheck (C++ only). We use tree-sitter to support 9 languages.

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
   `advanced_test.rs`, `lang_test.rs`) and register each as a separate
   `[[test]]` target in `Cargo.toml`.

## Supported Languages

9 languages with dedicated tree-sitter grammars:
Python, JavaScript, TypeScript, Go, Java, C, C++, Rust, Lua.

## CLI Usage

```bash
# Single algorithm
cargo run -- --repo /path/to/repo --diff diff.patch --algorithm leftflow

# Multiple algorithms
cargo run -- --repo /path/to/repo --diff diff.patch --algorithm "leftflow,fullflow,taint"

# Preset suites
cargo run -- --repo /path/to/repo --diff diff.patch --algorithm review  # review suite
cargo run -- --repo /path/to/repo --diff diff.patch --algorithm all     # all 26

# Output formats: text (default), json, paper, review
cargo run -- --repo /path/to/repo --diff diff.patch --format json
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
5. Add language-specific tests in `tests/lang/`
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
6. Add tests in `tests/algo/` (appropriate subcategory)
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

## Dependencies

- `tree-sitter` + 9 language grammars for AST parsing
- `petgraph` for graph data structures (CFG, CPG)
- `clap` for CLI
- `serde`/`serde_json` for serialization
- `anyhow`/`thiserror` for error handling
- `tempfile`, `assert_cmd`, `predicates` (dev-dependencies for testing)
