# CLAUDE.md — Instructions for AI Assistants Working on This Codebase

## Project Overview

Rust implementation of 26 code slicing algorithms for defect-focused automated
code review. Based on arXiv:2505.17928 plus the established program slicing
taxonomy and novel theoretical extensions.

## Build & Test

```bash
cargo build          # Build the project
cargo test           # Run all 43 tests (unit + integration)
cargo run -- --help  # Show CLI usage
cargo run -- --list-algorithms  # List all 26 algorithms
```

## Code Organization

- `src/algorithms/` — All 26 slicing algorithms. Each is self-contained.
- `src/ast.rs` — Tree-sitter AST wrapper. All tree-sitter interaction goes through here.
- `src/call_graph.rs` — Cross-file call graph with forward/reverse edges and cycle detection.
- `src/data_flow.rs` — Def-use chains, reachability queries, chopping, taint propagation.
- `src/languages/mod.rs` — Language-specific node type mappings. Add new languages here.
- `src/diff.rs` — Diff parsing and the `DiffBlock` output type.
- `src/slice.rs` — Config, 26-variant algorithm enum, and result types.
- `src/output.rs` — Output formatters (text, JSON, paper-compatible).
- `tests/integration_test.rs` — Integration tests with fixtures for all 5 languages.

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

## Key Design Decisions

1. **Tree-sitter for multi-language AST parsing.** The original paper used
   cppcheck (C++ only). We use tree-sitter to support 5 languages.

2. **Name-based variable tracking** instead of cppcheck's `varId` system.

3. **BTreeMap/BTreeSet everywhere** for deterministic, sorted output.

4. **Shared infrastructure:** `call_graph.rs` and `data_flow.rs` are reused
   across multiple algorithms. Build them once, pass to algorithms.

5. **Algorithm-specific configs** live in each algorithm's module (e.g.,
   `BarrierConfig`, `TaintConfig`, `SpiralConfig`), not in the central
   `SliceConfig`, to keep the core config lean.

## Adding a New Language

1. Add the tree-sitter grammar crate to `Cargo.toml`
2. Add a variant to `Language` enum in `src/languages/mod.rs`
3. Implement all the node type methods for the new language
4. Add a test fixture in `tests/integration_test.rs`

## Adding a New Slicing Algorithm

1. Create `src/algorithms/your_algo.rs` with a `pub fn slice(...)` function
2. Add variant to `SlicingAlgorithm` in `src/slice.rs` (both enum and `from_str`/`name`/`all`)
3. Add `pub mod your_algo;` in `src/algorithms/mod.rs`
4. Wire it up in the `run_slicing` dispatcher in `src/algorithms/mod.rs`
5. Add CLI flags in `src/main.rs` if it needs algorithm-specific config
6. Add tests in `tests/integration_test.rs`

## Common Patterns

- **Line numbers are 1-indexed** throughout. Tree-sitter uses 0-indexed rows;
  conversion happens in `ast.rs`.
- **`DiffBlock.file_line_map`** maps `filename → (line_number → is_diff_line)`.
- **Cross-file references**: Many algorithms include lines from multiple files.
  These appear as additional entries in `file_line_map`.
- **Algorithms that need call graph or data flow** build them at the start of
  their `slice()` function. If performance becomes an issue, these could be
  pre-built and passed in.

## Dependencies

- `tree-sitter` + 5 language grammars for AST parsing
- `clap` for CLI
- `serde`/`serde_json` for serialization
- `anyhow`/`thiserror` for error handling
