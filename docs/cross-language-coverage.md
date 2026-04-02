# Cross-Language Coverage Measurement

**Date:** 2026-04-02
**Status:** Design — ready for implementation
**Goal:** Measure and display which languages have coverage for which algorithms and language features, surface it in PRs, README, and CI

---

## Current State

Prism supports 9 languages but the README only lists 5. There's no systematic way to know that Go multi-return works but Go range iteration doesn't, or that JS destructuring is handled but for-of destructuring isn't. The `test_language_coverage_minimum` test enforces that each algorithm has tests in ≥2 languages, but doesn't track which features are covered per language.

**Test distribution today** (545+ tests):
```
Python:      65 tests     JavaScript:  37 tests
Go:          46 tests     TypeScript:  17 tests
C/C++:       49 tests     Rust:        18 tests
Java:        21 tests     Lua:         16 tests
```

**Algorithm coverage varies wildly:**
- taint: 8 languages, 27 tests
- absence_slice: 8 languages, 22 tests
- phantom_slice: 2 languages, 2 tests
- resonance_slice: 2 languages, 2 tests

---

## Architecture

Three components, each independently useful:

```
┌──────────────────────────────┐
│  1. Coverage Matrix (JSON)   │  Source of truth
│  What features each language │  Checked into repo
│  has, whether Prism handles  │  Updated by PRs
│  them, and which test proves │
│  it                          │
└──────────┬───────────────────┘
           │ read by
┌──────────▼───────────────────┐
│  2. Coverage Test (Rust)     │  CI enforcement
│  Validates matrix claims     │  Fails on regression
│  against actual test names   │
│  + generates report JSON     │
└──────────┬───────────────────┘
           │ generates
┌──────────▼───────────────────┐
│  3. Badge Generator (script) │  README + PR display
│  Reads report JSON           │  shields.io badges
│  Outputs markdown table +    │  Per-language %
│  badge URLs                  │
└──────────────────────────────┘
```

---

## 1. Coverage Matrix Definition

A JSON file at `coverage/matrix.json` declares three layers:

### Layer 1: Language Features

What language-specific patterns does each language have, and does Prism handle them?

```json
{
  "language_features": {
    "variable_binding": {
      "simple_assignment": {
        "languages": ["python", "javascript", "typescript", "go", "java", "c", "cpp", "rust", "lua"],
        "status": "handled",
        "tests": ["test_*_assignment_*"]
      },
      "multi_target_assign": {
        "languages": ["python", "go"],
        "status": "handled",
        "tests": ["test_go_multi_return_*", "test_python_tuple_unpack_*"]
      },
      "object_destructuring": {
        "languages": ["javascript", "typescript"],
        "status": "handled",
        "tests": ["test_destructuring_*"]
      },
      "for_of_destructuring": {
        "languages": ["javascript", "typescript"],
        "status": "gap",
        "gap_id": 3,
        "tracking": "docs/language-analysis-gaps.md"
      },
      "with_as_binding": {
        "languages": ["python"],
        "status": "gap",
        "gap_id": 4
      }
    },
    "field_access": {
      "dot_access": {
        "languages": ["python", "javascript", "typescript", "go", "java", "rust", "lua"],
        "status": "handled"
      },
      "arrow_access": {
        "languages": ["c", "cpp"],
        "status": "handled"
      },
      "optional_chaining": {
        "languages": ["javascript", "typescript"],
        "status": "handled",
        "tests": ["test_optional_chaining_*", "test_js_optional_chaining_*"]
      },
      "colon_method": {
        "languages": ["lua"],
        "status": "handled",
        "tests": ["test_lua_colon_*"]
      }
    },
    "control_flow": {
      "if_else": {
        "languages": ["python", "javascript", "typescript", "go", "java", "c", "cpp", "rust", "lua"],
        "status": "handled"
      },
      "try_catch": {
        "languages": ["python", "javascript", "typescript", "java"],
        "status": "handled",
        "tests": ["test_python_try_*", "test_js_try_*"]
      },
      "goto": {
        "languages": ["c", "cpp", "go"],
        "status": "handled"
      },
      "defer": {
        "languages": ["go"],
        "status": "handled"
      },
      "match_arms": {
        "languages": ["rust"],
        "status": "handled"
      },
      "for_else": {
        "languages": ["python"],
        "status": "handled"
      }
    }
  }
}
```

### Layer 2: Algorithm × Language

Which algorithms have been tested with which languages?

```json
{
  "algorithm_coverage": {
    "taint": {
      "python": { "tests": 6, "status": "full" },
      "go": { "tests": 3, "status": "full" },
      "c": { "tests": 8, "status": "full" },
      "javascript": { "tests": 3, "status": "full" },
      "typescript": { "tests": 1, "status": "basic" },
      "rust": { "tests": 3, "status": "full" },
      "lua": { "tests": 3, "status": "full" },
      "java": { "tests": 0, "status": "none" }
    }
  }
}
```

Status levels: `full` (≥3 tests including edge cases), `basic` (1-2 tests), `none` (0 tests).

### Layer 3: Infrastructure Features

Which DFG/CPG/CFG capabilities work for which languages?

```json
{
  "infrastructure": {
    "data_flow_graph": {
      "languages": ["python", "javascript", "typescript", "go", "java", "c", "cpp", "rust", "lua"],
      "status": "handled"
    },
    "field_sensitive_dfg": {
      "languages": ["python", "javascript", "typescript", "go", "java", "c", "cpp", "rust", "lua"],
      "status": "handled"
    },
    "alias_tracking": {
      "languages": ["python", "javascript", "typescript", "go", "c", "cpp", "rust"],
      "status": "handled",
      "notes": "Java and Lua not tested"
    },
    "cfg_edges": {
      "languages": ["python", "javascript", "typescript", "go", "java", "c", "cpp", "rust"],
      "status": "handled",
      "notes": "Lua pcall/xpcall not modeled"
    },
    "type_enrichment": {
      "languages": ["c", "cpp"],
      "status": "handled",
      "notes": "clang + tree-sitter fallback"
    }
  }
}
```

---

## 2. Coverage Validation Test

A Rust integration test that reads `coverage/matrix.json`, verifies claimed coverage against actual test names, and generates a report.

```rust
#[test]
fn test_cross_language_coverage() {
    let matrix: CoverageMatrix = load_matrix("coverage/matrix.json");
    let test_source = fs::read_to_string("tests/integration_test.rs").unwrap();
    let test_names: Vec<&str> = extract_test_names(&test_source);

    let mut report = CoverageReport::new();

    for (feature, spec) in &matrix.language_features {
        for lang in &spec.languages {
            if spec.status == "handled" {
                // Verify at least one test name matches the pattern
                let has_test = spec.tests.iter().any(|pattern| {
                    test_names.iter().any(|t| matches_glob(t, pattern))
                });
                report.record(feature, lang, has_test);

                if !has_test {
                    report.warn(format!(
                        "Feature '{}' claims handled for {} but no matching test found",
                        feature, lang
                    ));
                }
            }
        }
    }

    // Write report JSON
    let json = serde_json::to_string_pretty(&report).unwrap();
    fs::write("coverage/report.json", &json).unwrap();

    // Fail if coverage dropped
    assert!(
        report.overall_percentage() >= 85,
        "Cross-language coverage dropped below 85%: {}%",
        report.overall_percentage()
    );
}
```

### What this catches

- PR claims a gap is fixed but doesn't add a test → matrix says "handled" but validation fails
- PR accidentally removes a test → coverage drops, test fails
- PR adds a new language feature but doesn't update the matrix → matrix is stale (caught by review, not by test)

---

## 3. Badge Generation

### Per-language badges

Generated from the coverage report:

```markdown
![Python](https://img.shields.io/badge/Python-83%25-yellow?logo=python)
![JavaScript](https://img.shields.io/badge/JavaScript-93%25-green?logo=javascript)
![TypeScript](https://img.shields.io/badge/TypeScript-93%25-green?logo=typescript)
![Go](https://img.shields.io/badge/Go-93%25-green?logo=go)
![Java](https://img.shields.io/badge/Java-100%25-brightgreen?logo=openjdk)
![C](https://img.shields.io/badge/C-100%25-brightgreen?logo=c)
![C++](https://img.shields.io/badge/C%2B%2B-100%25-brightgreen?logo=cplusplus)
![Rust](https://img.shields.io/badge/Rust-91%25-green?logo=rust)
![Lua](https://img.shields.io/badge/Lua-100%25-brightgreen?logo=lua)
```

Color thresholds: `brightgreen` ≥95%, `green` ≥85%, `yellow` ≥70%, `orange` ≥50%, `red` <50%.

### Summary badge

```markdown
![Language Coverage](https://img.shields.io/badge/language_coverage-9_languages_%7C_94%25-green)
```

### Algorithm coverage table (for README)

```markdown
## Algorithm × Language Coverage

| Algorithm | Py | JS | TS | Go | Ja | C | C++ | Rs | Lua |
|-----------|----|----|----|----|----|----|-----|----|----|
| taint     | ✅ | ✅ | 🟡 | ✅ | ❌ | ✅ | ✅  | ✅ | ✅ |
| chop      | ✅ | ✅ | ❌ | ✅ | ✅ | ❌ | ❌  | ❌ | ❌ |
| membrane  | ✅ | ✅ | 🟡 | ✅ | ✅ | ✅ | ✅  | 🟡 | 🟡 |
| ...       |    |    |    |    |    |    |     |    |    |

✅ = full (3+ tests)  🟡 = basic (1-2 tests)  ❌ = none
```

---

## 4. CI Integration

### GitHub Actions workflow

```yaml
name: Coverage Matrix
on: [push, pull_request]

jobs:
  coverage-matrix:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Run coverage matrix test
        run: cargo test test_cross_language_coverage -- --nocapture

      - name: Generate badges
        run: python3 scripts/generate_coverage_badges.py

      - name: Comment PR with coverage
        if: github.event_name == 'pull_request'
        uses: actions/github-script@v7
        with:
          script: |
            const fs = require('fs');
            const report = JSON.parse(fs.readFileSync('coverage/report.json'));
            const body = fs.readFileSync('coverage/pr_comment.md', 'utf8');
            github.rest.issues.createComment({
              issue_number: context.issue.number,
              owner: context.repo.owner,
              repo: context.repo.repo,
              body: body
            });
```

### PR comment format

When a PR changes analysis code, the CI posts a comment like:

```markdown
### Language Coverage Report

| Language | Features | Coverage | Δ |
|----------|----------|----------|---|
| Python   | 15/18    | 83%      | — |
| JS       | 16/16    | 100%     | +1 ✅ (for-of destructuring) |
| Go       | 14/15    | 93%      | — |
| Rust     | 11/12    | 91%      | — |

**Overall: 94% (+0.5%)**

<details>
<summary>Remaining gaps (5)</summary>

- Python: `with...as` binding, `for k,v in dict.items()`, comprehension taint
- JS/TS: (none remaining)
- Go: `for k, v := range m`
- Rust: `?` operator early-return
</details>
```

---

## 5. Implementation Plan

### Phase 1: Coverage matrix + test (1 day)

1. Create `coverage/matrix.json` with current state
2. Write `test_cross_language_coverage` in integration tests
3. Write `scripts/generate_coverage_badges.py` to output badge markdown
4. Update README with badges and algorithm × language table

### Phase 2: CI integration (0.5 day)

1. Add GitHub Actions workflow step
2. Add PR comment generation
3. Add coverage threshold enforcement (fail if <85%)

### Phase 3: Delta reporting (0.5 day)

1. Compare current report against baseline (committed `coverage/baseline.json`)
2. Generate delta for PR comments (what changed)
3. Update baseline on merge to main

---

## Design Decisions

### Why not code coverage (llvm-cov/tarpaulin)?

Code coverage measures "which lines of Prism's Rust code were executed." That's useful for finding untested code paths but doesn't answer "does Prism handle Python walrus operators?" A test might execute the walrus code path without actually testing walrus behavior (e.g., a test that happens to parse a file with `:=` but doesn't verify the DFG edge).

Language feature coverage is a **semantic** metric, not a **syntactic** one. The matrix approach measures intent: "we claim this feature works for this language, and here's the test that proves it."

Code coverage (codecov) and language feature coverage are complementary. Codecov is already set up and running.

### Why JSON not TOML/YAML?

The matrix needs to be machine-readable for badge generation and CI. JSON is natively parseable in both Rust (serde_json, already a dependency) and Python (stdlib). TOML would work too but adds no value over JSON for this use case.

### Why test-name matching not runtime probing?

Runtime probing (running each feature through the pipeline and checking results) is more rigorous but much slower — it would add 100+ test cases that each build a CPG. Test-name matching is a lightweight proxy: if a test named `test_python_walrus_operator_def` exists and passes, we're confident walrus is handled. The matrix's `tests` field makes the mapping explicit and auditable.

### Badge hosting for private repo

shields.io can't read files from private repos. Options:

1. **Static badges in README** — generated by CI script, committed on merge. Simple, no external deps. Badges are stale between merges but that's fine for a development tool.
2. **GitHub Actions badge** — the existing CI badge pattern. Coverage test passes/fails, badge reflects status.
3. **GitHub Pages** — serve `coverage/report.json` via GitHub Pages, use shields.io dynamic badge endpoint. Requires enabling Pages on the repo.

**Recommendation:** Option 1 for v1. The badge generation script writes badge markdown to a file, CI commits it on merge to main. Zero external dependencies.

---

## Appendix: Current Coverage Snapshot

### Language Feature Coverage

| Language   | Handled | Total | Coverage |
|------------|---------|-------|----------|
| Python     | 15      | 18    | 83%      |
| JavaScript | 15      | 16    | 93%      |
| TypeScript | 15      | 16    | 93%      |
| Go         | 14      | 15    | 93%      |
| Java       | 12      | 12    | 100%     |
| C          | 12      | 12    | 100%     |
| C++        | 14      | 14    | 100%     |
| Rust       | 11      | 12    | 91%      |
| Lua        | 10      | 10    | 100%     |

### Known Gaps (5 remaining)

| # | Feature | Languages | Severity | Status |
|---|---------|-----------|----------|--------|
| 3 | for-of destructuring | JS/TS | High | PR E |
| 4 | `with...as` binding | Python | High | PR E |
| 5 | for-range multi-target | Go, Python | Medium | PR E |
| 6 | Spread field provenance | JS/TS | Low | Deferred |
| 7 | Comprehension taint | Python | Low | Deferred |
| — | `?` operator CFG | Rust | Low | Deferred |
