# Test Coverage Tracking

Last updated: 2026-04-01

## Quick Reference

```bash
cargo test                                                    # Run all tests
cargo test -- test_algorithm_language_matrix --nocapture       # Print coverage matrix
cargo test -- test_language_coverage_minimum --nocapture       # Enforce minimum language coverage
cargo llvm-cov --text                                         # Line coverage by file
./coverage.sh --check 80                                      # Fail if line coverage < 80%
```

## Coverage Targets

| Metric | Target | How Measured |
|--------|--------|--------------|
| Overall line coverage | ≥ 80% | `cargo llvm-cov` / Codecov |
| Per-file line coverage | ≥ 80% (excl. `main.rs`) | `cargo llvm-cov --json` |
| Algorithm×Language matrix | ≥ 40% | `test_algorithm_language_matrix` |
| Minimum languages per algorithm | ≥ 2 | `test_language_coverage_minimum` |

## Measuring Coverage

### 1. Line Coverage (Codecov)

Uploaded automatically by CI on every push. View at:
<https://app.codecov.io/github/shoedog/prism>

Run locally:
```bash
cargo llvm-cov --text          # Summary per file
cargo llvm-cov --html          # HTML report → target/llvm-cov/html/
cargo llvm-cov --json \
  --output-path target/llvm-cov/coverage.json   # Machine-readable
```

### 2. Algorithm × Language Matrix

Shows which (algorithm, language) pairs have at least one dedicated test:

```
cargo test -- test_algorithm_language_matrix --nocapture
```

Output is a grid of `yes` / `-` for every combination. The matrix detects
tests by scanning `tests/integration_test.rs` for functions named
`test_<algorithm>_<language>`. Keep this naming convention for new tests.

**Languages tracked:** Python, JavaScript, TypeScript, Go, Java, C, C++, Rust, Lua

### 3. Minimum Language Coverage Enforcement

A CI-enforced test that fails if any algorithm has tests for fewer than the
required number of languages:

```
cargo test -- test_language_coverage_minimum
```

This prevents adding a new algorithm with only single-language tests.

## Current File Coverage

*Baseline recorded 2026-04-01 — update after each coverage push.*

| File | Lines | Coverage | Status |
|------|-------|----------|--------|
| `algorithms/original_diff.rs` | 28 | 100% | ✅ |
| `cpg.rs` | 104 | 100% | ✅ |
| `data_flow.rs` | 199 | 99.5% | ✅ |
| `access_path.rs` | 297 | 97.0% | ✅ |
| `slice.rs` | 132 | 97.0% | ✅ |
| `algorithms/absence_slice.rs` | 449 | 97.3% | ✅ |
| `algorithms/taint.rs` | 125 | 96.0% | ✅ |
| `algorithms/circular_slice.rs` | 70 | 95.7% | ✅ |
| `algorithms/membrane_slice.rs` | 160 | 95.6% | ✅ |
| `algorithms/echo_slice.rs` | 141 | 95.0% | ✅ |
| `call_graph.rs` | 370 | 94.9% | ✅ |
| `algorithms/gradient_slice.rs` | 117 | 94.0% | ✅ |
| `algorithms/relevant_slice.rs` | 88 | 93.2% | ✅ |
| `output.rs` | 119 | 92.4% | ✅ |
| `ast.rs` | 907 | 92.1% | ✅ |
| `algorithms/symmetry_slice.rs` | 200 | 92.0% | ✅ |
| `diff.rs` | 191 | 87.4% | ✅ |
| `algorithms/provenance_slice.rs` | 188 | 84.6% | ✅ |
| `algorithms/mod.rs` | 63 | 82.5% | ✅ |
| `algorithms/left_flow.rs` | 96 | 80.2% | ✅ |
| `algorithms/barrier_slice.rs` | 94 | 79.8% | ⚠️ |
| `algorithms/parent_function.rs` | 46 | 78.3% | ⚠️ |
| `algorithms/thin_slice.rs` | 53 | 75.5% | ⚠️ |
| `languages/mod.rs` | 335 | 73.7% | ⚠️ |
| `algorithms/quantum_slice.rs` | 348 | 72.4% | ⚠️ |
| `algorithms/phantom_slice.rs` | 204 | 69.1% | ⚠️ |
| `algorithms/full_flow.rs` | 122 | 67.2% | ⚠️ |
| `algorithms/vertical_slice.rs` | 114 | 65.8% | ⚠️ |
| `algorithms/resonance_slice.rs` | 118 | 64.4% | ⚠️ |
| `algorithms/horizontal_slice.rs` | 121 | 60.3% | ⚠️ |
| `algorithms/spiral_slice.rs` | 217 | 56.7% | ❌ |
| `algorithms/angle_slice.rs` | 142 | 55.6% | ❌ |
| `algorithms/conditioned_slice.rs` | 121 | 21.5% | ❌ |
| `algorithms/threed_slice.rs` | 123 | 0.0% | ❌ |
| `algorithms/delta_slice.rs` | 83 | 0.0% | ❌ |
| `algorithms/chop.rs` | 28 | 0.0% | ❌ |
| `main.rs` | 288 | 0.0% | — (excluded) |

Legend: ✅ ≥ 80% · ⚠️ 60–79% · ❌ < 60% · — excluded

## Algorithm × Language Matrix

Below is the target state. Rows = algorithms, columns = languages.
A cell marked ✓ means at least one `test_<algo>_<lang>` integration test exists.

**Priority rules for new tests:**
1. Every algorithm must be tested in ≥ 2 languages (enforced by CI).
2. The "core four" languages (Python, C, Go, JavaScript) should be covered first.
3. TypeScript, Java, C++, Rust, Lua round out breadth coverage.

## Adding Tests

Follow the naming convention so tests appear in the matrix:

```
test_<algorithm>_<language>[_optional_suffix]
```

Examples:
- `test_taint_python` → detected as Taint × Python
- `test_absence_slice_cpp_double_free` → detected as AbsenceSlice × C++
- `test_chop_go` → detected as Chop × Go

The detection logic scans for `fn test_` lines in `integration_test.rs` and
matches algorithm keywords and language keywords via substring matching.

**Language keywords used for matching:**

| Language | Keyword | Notes |
|----------|---------|-------|
| Python | `python` | |
| JavaScript | `javascript` or `_js_` or `_js` (end) | |
| TypeScript | `typescript` or `_ts_` or `_ts` (end) | |
| Go | `_go_` or `_go` (end) | |
| Java | `_java_` or `_java` (end) | Must not also match `javascript` |
| C | `_c_` or `_c` (end) | Must not also match `cpp`/`circular`/`conditioned` |
| C++ | `_cpp_` or `_cpp` (end) | |
| Rust | `_rust_` or `_rust` (end) | |
| Lua | `_lua_` or `_lua` (end) | |

## CI Integration

The GitHub Actions workflow (`.github/workflows/ci.yml`) runs:
1. `cargo test` — all tests including the matrix reporter
2. `cargo llvm-cov` — uploads LCOV to Codecov

To enforce the 80% threshold locally:
```bash
./coverage.sh --check 80
```
