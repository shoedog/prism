# Testing Guide

## Running Tests

```bash
cargo test                        # Run all tests (unit + integration)
cargo test -- --nocapture         # Show println! output from tests
cargo test test_taint             # Run tests whose name contains "test_taint"
```

There are currently 66+ tests split across unit tests (inline in `src/`) and
integration tests (`tests/integration_test.rs`).

## Local Coverage Reports

First, install the tooling (one-time setup):

```bash
cargo install cargo-llvm-cov
rustup component add llvm-tools-preview
```

Then use the `coverage.sh` script or the `Makefile` targets:

```bash
# Print a text summary to stdout
./coverage.sh
make coverage

# Open the HTML report in your browser (macOS)
make coverage-html

# Write machine-readable JSON to target/llvm-cov/coverage.json
make coverage-json

# Fail if overall line coverage is below a threshold
./coverage.sh --check 80
```

The HTML report is written to `target/llvm-cov/html/index.html`.
Coverage output files are in `target/llvm-cov/` (git-ignored).

## CI Pipeline

Every push and pull request targeting `main` runs four jobs:

| Job | What it checks |
|-----|---------------|
| **Test Suite** | `cargo build` + `cargo test` |
| **Clippy Lint** | `cargo clippy --all-targets -- -W clippy::all` |
| **Format Check** | `cargo fmt --all -- --check` |
| **Coverage** | `cargo llvm-cov` — text + JSON reports uploaded as artifacts; summary posted to the job summary |

The Coverage job runs after Test Suite passes (`needs: test`). Coverage
artifacts (`coverage-summary.txt`, `coverage.json`) are retained for 90 days
and accessible from the Actions run page.

## Algorithm × Language Coverage Matrix

A special test prints which algorithm × language pairs have integration tests:

```bash
cargo test -- test_algorithm_language_matrix --nocapture
```

This always passes — it is a documentation/reporting tool that makes coverage
gaps visible at a glance.

## Adding New Tests

Follow the patterns in `tests/integration_test.rs`:

1. **Helper fixture** — if you need a new language/scenario, add a
   `make_<language>_test()` helper that returns `(files, sources, diff)`.

2. **Test function** — name it `test_<algorithm>_<language>` (e.g.
   `test_taint_python`) so it appears correctly in the coverage matrix.

3. **Run the algorithm** — call `algorithms::run_slicing(...)` with the
   appropriate `SlicingAlgorithm` variant and `SliceConfig`.

4. **Assert** — check that `result.blocks` is non-empty, or verify specific
   line numbers/files appear in `file_line_map`.

Example skeleton:

```rust
#[test]
fn test_taint_go() {
    let (files, sources, diff) = make_go_test();
    let result = algorithms::run_slicing(
        &files,
        &sources,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}
```

For algorithms that need algorithm-specific config (e.g. `BarrierConfig`,
`TaintConfig`), set it on the `SliceConfig` before passing it in — see existing
tests for examples.
