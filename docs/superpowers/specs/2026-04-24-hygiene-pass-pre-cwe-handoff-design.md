# Hygiene Pass + Pre-Handoff Architectural Baseline — Design

**Date:** 2026-04-24
**Status:** Approved — pending implementation plan
**Driver:** Close out three uncommitted algorithms (peer_consistency, callback_dispatcher, primitive) before ACKing the eval team's CWE coverage handoff (`~/code/agent-eval/analysis/prism-cwe-coverage-handoff.md`)
**Scope:** Hygiene + lightweight design-prep. No new algorithms, no architecture changes.

---

## 1. Purpose

The working tree contains three new slicing algorithms (peer_consistency_slice, callback_dispatcher_slice, primitive_slice) wired into modified files (`src/slice.rs`, `src/algorithms/mod.rs`, `src/main.rs`, `src/ast.rs`, `tests/integration/{core,coverage}_test.rs`, `coverage/matrix.json`, `README.md`) but uncommitted, with regressed coverage badges (algorithm coverage 100% → 90% across most languages) reflecting absent test coverage.

The eval team has filed a substantial CWE coverage handoff requesting a category-aware sanitizer registry, framework-detection layer, and per-language taint sink taxonomy across six CWE families. Acceptance criterion #5 of that handoff is "no regression on Tier 1" — Tier 1 explicitly includes T1-002 (peer/callback) and T1-005 (primitive). A clean, tested baseline of those algorithms is a prerequisite for handoff work.

This design covers two coupled goals:
1. Land the three uncommitted algorithms with adequate test coverage and committed wire-up.
2. Capture an inventory of current source/sink/sanitizer infrastructure and tentative answers to the handoff's open architectural questions, so the upcoming ACK doc can be written from a baked baseline rather than cold.

---

## 2. Approach

Three commits, in order. Each leaves `cargo test && cargo fmt --check` green.

| # | Commit | Approx scope |
|---|---|---|
| 1 | Land three new algorithms with tests + matrix update + cleanups | 32 new tests, 4 new test files, ~6 modified files |
| 2 | Wire `tests/fixtures/hapi-4552.diff` as review-suite regression smoke test | 1 new test file, 1 trimmed source fixture |
| 3 | Refresh `Plan.md` with T1 status updates + pre-handoff architectural baseline section | Plan.md edit only |

The three-commit shape gives reviewable units, clean revert points, and a deliberate synthesis pass at the end (commit 3 benefits from observations made in commits 1-2).

---

## 3. Commit 1 — Algorithms with tests

### 3.1 New test files

| File | Cargo test target | Language scope | Test count | Depth |
|---|---|---|---|---|
| `tests/algo/novel/peer_consistency_test.rs` | `algo_novel_peer_consistency` | C, C++ | 7 | ✅ full |
| `tests/algo/novel/callback_dispatcher_test.rs` | `algo_novel_callback_dispatcher` | C, C++ | 7 | ✅ full |
| `tests/algo/novel/primitive_test.rs` | `algo_novel_primitive` | Python, C | 15 | ✅ full |
| `tests/algo/novel/primitive_lang_test.rs` | `algo_novel_primitive_lang` | JS, Go | 3 | 🟡 basic |

All test files follow the existing `tests/algo/novel/contract_test.rs` shape:
```rust
#[path = "../../common/mod.rs"]
mod common;
use common::*;
use prism::slice::SliceResult;

fn run_<algo>(source: &str, path: &str, lang: Language, diff_lines: BTreeSet<usize>) -> SliceResult { ... }

#[test]
fn test_<algo>_<scenario>_<lang>() { ... }
```

Test-name prefixes match what `tests/integration/coverage_test.rs:74-90` and `:670-674` already declare for the matrix scanner.

### 3.2 peer_consistency_test.rs (7 tests)

C tests (4):
- `test_peer_consistency_uniform_unguarded_cluster_c` — 3+ siblings sharing `struct vty *vty` first param, all dereference, none guard. Expect 1 `concern` finding, category `peer_guard_divergence`.
- `test_peer_consistency_divergent_cluster_c` — 4 siblings, 3 guard, 1 unguarded. Expect 1 `warning` finding naming the divergent function.
- `test_peer_consistency_all_guarded_no_finding_c` — 4 siblings all guarded. No finding.
- `test_peer_consistency_cluster_too_small_no_finding_c` — 2 siblings only. No finding (threshold is 3).

C++ tests (3):
- `test_peer_consistency_uniform_unguarded_cluster_cpp` — class-pointer first param.
- `test_peer_consistency_divergent_cluster_cpp`.
- `test_peer_consistency_only_fires_on_diff_touched_param_cpp` — verifies the `touched_params` filter (cluster is silent unless ≥1 member is in the diff).

### 3.3 callback_dispatcher_test.rs (7 tests)

C tests (4):
- `test_callback_dispatcher_designated_init_to_null_dispatch_c` — `.show_opaque_info = my_func` registration + `tab->show_opaque_info(NULL, lsa)` dispatch in another file. Expect `concern`, category `callback_null_arg_dispatch`, related_files contains the dispatch-site file.
- `test_callback_dispatcher_assignment_field_clean_dispatch_c` — `obj->cb = my_func` + `obj->cb(arg, lsa)` (no NULL). Expect `info`, category `callback_dispatcher_chain`.
- `test_callback_dispatcher_registrar_call_arg_c` — `register_handler(my_func)` style. Expect `warning`, category `callback_registrar_call`.
- `test_callback_dispatcher_no_invocations_no_finding_c` — registration without matching invocations. No finding.

C++ tests (3):
- `test_callback_dispatcher_designated_init_cpp`.
- `test_callback_dispatcher_g_signal_connect_cpp` — GLib registrar pattern (aligns with the T1-006 GLib callback work in `eebafb6`).
- `test_callback_dispatcher_unrelated_function_no_finding_cpp`.

### 3.4 primitive_test.rs (15 tests)

Python tests (10):
- `test_primitive_hash_trunc_direct_below_128_python` — `h.hexdigest()[:12]` on a diff line. Rule `HASH_TRUNCATED_BELOW_128_BITS`, severity `concern`.
- `test_primitive_hash_trunc_at_threshold_no_finding_python` — `[:32]` (exactly 128 bits). No finding.
- `test_primitive_hash_trunc_raw_below_threshold_python` — `h.digest()[:8]` (raw bytes, threshold 16). Description includes `bytes` unit.
- `test_primitive_hash_trunc_via_call_python` — `def get_str_hash(s, length): return h.hexdigest()[:length]` defined + `get_str_hash(s, 12)` called. 2-pass `HASH_TRUNCATION_VIA_CALL`; `related_files` and `related_lines` point to callee def.
- `test_primitive_weak_hash_for_identity_python` — `cache_key = hashlib.md5(b).hexdigest()`.
- `test_primitive_weak_hash_for_checksum_no_finding_python` — `checksum = hashlib.md5(...)` (non-identity name). No finding.
- `test_primitive_shell_true_with_fstring_python` — `subprocess.run(f"cp {x}", shell=True)`.
- `test_primitive_shell_true_no_interp_no_finding_python` — `subprocess.run("ls", shell=True)`. No finding.
- `test_primitive_cert_validation_disabled_python` — `requests.get(url, verify=False)`.
- `test_primitive_hardcoded_secret_python` — `API_KEY = "sk-real-1234"` fires; same test asserts `TOKEN = "changeme"` (placeholder) does not.

C tests (5):
- `test_primitive_cert_validation_verifypeer_zero_c` — `CURLOPT_SSL_VERIFYPEER, 0L` on diff line. Severity `concern`.
- `test_primitive_cert_validation_verifyhost_zero_c` — `CURLOPT_SSL_VERIFYHOST, 0`. Different marker, exercises the marker list.
- `test_primitive_cert_validation_in_dirty_function_concern_c` — marker on a non-diff line of a function containing another diff line. Severity `concern` via dirty-function classification.
- `test_primitive_cert_validation_outside_dirty_function_suggestion_c` — marker in an unrelated function in a diff-touched file. Severity `suggestion` (lowest tier).
- `test_primitive_no_finding_proper_curl_validation_c` — `CURLOPT_SSL_VERIFYPEER, 1L`. No finding (negative).

The C tests collectively exercise (a) the multi-marker cert-validation rule and (b) the three-tier severity classification (`concern` on diff line → `concern` in dirty function → `suggestion` elsewhere) — shared logic across all primitive rules.

### 3.5 primitive_lang_test.rs (3 tests, 🟡 basic)

Limitations confirmed by reading the algorithm:
- `HARDCODED_SECRET` only matches single-identifier LHS shapes (bare `NAME = "literal"` or `obj.field = "literal"`). `const`/`let`/`var` (JS) and `static const char *` (C) bypass the LHS-identifier check; Go's `:=` short-form does match (line 663 strips the trailing colon), so only keyword-prefixed forms miss.
- `CERT_VALIDATION_DISABLED` is pure substring match — works in any language as long as the marker text appears.

JS tests (2):
- `test_primitive_cert_validation_disabled_reject_unauthorized_js` — `https.request({ rejectUnauthorized: false })`.
- `test_primitive_hardcoded_secret_object_field_js` — `this.apiKey = "sk-real-1234"` (uses the cross-language `obj.field` rsplit path).

Go tests (1):
- `test_primitive_cert_validation_disabled_insecure_skip_verify_go` — `tls.Config{InsecureSkipVerify: true}`.

### 3.6 Source cleanups

- Remove `arg_snippet` dead-code placeholder in `src/algorithms/callback_dispatcher_slice.rs:303` (`let _ = invocations.iter().map(|i| &i.arg_snippet).count();`).
- Add trailing newline to `coverage/matrix.json`.

### 3.7 Coverage matrix update

In `coverage/matrix.json`, transition cells for `primitive_slice` from `none` → `basic` for `javascript` and `go` to reflect the new tests in `primitive_lang_test.rs`. Other cells remain unchanged — peer/callback are intentionally C/C++-only by algorithm design.

### 3.8 Cargo.toml additions

```toml
[[test]]
name = "algo_novel_peer_consistency"
path = "tests/algo/novel/peer_consistency_test.rs"

[[test]]
name = "algo_novel_callback_dispatcher"
path = "tests/algo/novel/callback_dispatcher_test.rs"

[[test]]
name = "algo_novel_primitive"
path = "tests/algo/novel/primitive_test.rs"

[[test]]
name = "algo_novel_primitive_lang"
path = "tests/algo/novel/primitive_lang_test.rs"
```

### 3.9 Already-staged changes that ride along in commit 1

These are already in the working tree and commit alongside the test work:
- `src/slice.rs` — `SlicingAlgorithm` enum entries + `from_str` + `name` + `all` + review suite.
- `src/algorithms/mod.rs` — module declarations + dispatch.
- `src/main.rs` — `--list-algorithms` text.
- `src/ast.rs` — Python `typed_parameter` / `default_parameter` / splat name extraction (supports primitive's 2-pass detection).
- `src/algorithms/provenance_slice.rs` — formatter-only changes.
- `tests/integration/{core,coverage}_test.rs` — count assertion (27 → 30) and matrix scanner registrations.
- `tests/algo/taxonomy/taint_cve_test.rs` — formatter-only.
- `tests/cli/output_test.rs` — formatter-only.
- `README.md` — 27 → 30 algorithm count, three new algorithm rows in the novel-extensions table, badge regression to honest 90% reflecting the new gaps before tests close them.

---

## 4. Commit 2 — Hapi-4552 regression smoke test

### 4.1 Files

- Keep `tests/fixtures/hapi-4552.diff` as input (already untracked, this commit tracks it).
- Replace `tests/fixtures/hapi-4552-output.txt` (6,621-line snapshot) with a generation script `scripts/regenerate_hapi_snapshot.sh` that any reviewer can run to inspect current `--algorithm review --format text` output. The captured snapshot is too brittle for byte-equal comparison.
- Keep `tests/fixtures/hapi-4552-output.json` (4 lines) as inspection reference; do not assert against it.
- Add trimmed real source: `tests/fixtures/hapi-4552-source/lib/transmit.js` — ~300-line trim of the relevant function bodies from the real hapijs repo, sized to match the diff line numbers (266, 364, 373, etc.). MIT-licensed; include a header comment noting the trim and original source.

### 4.2 New test file

`tests/integration/hapi_regression_test.rs`, registered as `integration_hapi_regression`:

```rust
#[test]
fn test_hapi_4552_review_suite_smoke() {
    // 1. Load tests/fixtures/hapi-4552.diff.
    // 2. Load tests/fixtures/hapi-4552-source/lib/transmit.js.
    // 3. Run algorithms::run_slicing_compat with the review preset.
    // 4. Structural assertions:
    //    - At least one result has a left_flow finding (default algorithm fires).
    //    - The diff lines (269 stream.on('close', ...), 368 from.on('close', ...), 384 internals.destroyPipe def) appear in at least one block's file_line_map.
    //    - parent_function output includes a function spanning the new internals.destroyPipe definition line.
}
```

Loose smoke test, not a snapshot. Catches "review preset stops firing on this diff" or "parent_function stops finding the new function" — real regressions. Does not catch wording changes — correct granularity.

If during implementation the review preset's output on this fixture is observed to be poor (flooded with FPs, missing obvious findings), the test will *expose* the poor behavior with a `// FIXME: ...` comment rather than codify it as the desired baseline.

### 4.3 Cargo.toml addition

```toml
[[test]]
name = "integration_hapi_regression"
path = "tests/integration/hapi_regression_test.rs"
```

---

## 5. Commit 3 — Plan.md refresh

### 5.1 Edit-in-place updates

**Header line:**
```diff
-Last updated: 2026-04-02 (Phase 6 complete, CPG improvements plan)
+Last updated: 2026-04-24 (T1-006 handoff, three new algorithms, pre-handoff baseline added)
```

**`### P1 — Important Fixes (Partial)` table** — add rows for:
- T1-006 work (committed `eebafb6`): IPC taint sources, CFG multiline call edge fix, GLib callbacks.
- `e3fa16d`: slice_text JSON output fix + settings_t provenance source.
- `37ef823`: Spiral added to review suite, taint sinks expanded, provenance import-suppression FP fix.

**New subsection: `### Algorithms — Tier 1 (T1) Capability Expansion`**, listing:
- T1-002: PeerConsistencySlice + CallbackDispatcherSlice (FRR CVE-2025-61102 driver, C/C++ only).
- T1-005: PrimitiveSlice (Python primary; basic C/JS/Go for cert + secret rules).
- 32 tests added across `algo/novel/{peer_consistency,callback_dispatcher,primitive,primitive_lang}_test.rs` + 1 hapi regression smoke test.

### 5.2 New section: `## Pre-handoff Architectural Baseline (D5: CWE Coverage)`

Inserted between existing `## Remaining Work` and `## Architecture Notes`. ~300-400 words. Subsections:

**Inventory of current source/sink/sanitizer infrastructure** — pointers to `taint.rs` sink arrays, `provenance_slice.rs` origin classification + `WEB_FRAMEWORK_MODULES` + `PROVENANCE_OVERLAP_KEYWORDS`. Flag explicitly: no sanitizer recognition exists in algorithm logic today; no framework detection layer beyond import-suppression.

**Tentative answers to handoff §10 open questions:**
- **Q1 Config vs code:** Rust modules with declarative const arrays (existing pattern). CLI `--taint-source-extra` passthrough for the eval team's mid-run debugging value.
- **Q2 Per-framework module structure:** per-framework modules under `src/frameworks/`, mirroring `src/languages/` shape, registered through a small `FrameworkRegistry`.
- **Q3 Sanitizer granularity:** boolean cleansed/uncleansed per category. Confidence is unwarranted complexity.
- **Q4 Phasing:** agree with eval team — Go (Phase 1) → Python (Phase 2) → JS (Phase 3) → Java stretch.
- **Q5 Unknown-framework default:** quiet mode (eval team's preference).

**Phasing recommendation:** Phase 0 (this hygiene pass) → Phase 1 Go (1-2w) → Phase 2 Python (2-3w) → Phase 3 JS (1-2w) → Phase 4 Java stretch.

**Known cross-language gap notes (from this hygiene pass):**
- `primitive_slice::detect_hardcoded_secret` only matches single-identifier LHS forms (bare `NAME = "literal"` and `obj.field = "literal"`). `const`/`let`/`var` (JS) and `static const char *` (C) bypass the LHS-identifier check; Go's `:=` short-form does match (line 663 strips the trailing colon), so only keyword-prefixed forms miss. Deferred rather than patched — the handoff's category-aware sanitizer/source registry will likely subsume this rule.

### 5.3 What is NOT touched in Plan.md

- Architecture notes section.
- CPG phase summaries (Phases 1-6).
- Reference list.

### 5.4 What is NOT touched in this hygiene pass

- `TEST_GAPS.md`, `TEST_COVERAGE.md` — separate update cadence; unrelated to handoff prep.

---

## 6. Verification & acceptance

Each commit must satisfy, in order:

1. `cargo build` succeeds.
2. `cargo test` passes — including all newly added tests, including `integration_core_test::test_all_algorithms_listed` (asserts 30) and the matrix scanner in `integration_coverage_test`.
3. `cargo fmt --check` passes.
4. Manual: run `cargo run -- --list-algorithms` and visually verify the three new algorithms are listed with expected descriptions.

Commit-specific acceptance:
- **Commit 1:** Running `python3 scripts/generate_coverage_badges.py` produces an updated badge set; the regenerated badges are committed alongside the tests. New cells (peer/callback C+C++, primitive Python+C+JS+Go) read accurately as `basic` or `full`; intentionally-restricted cells (peer/callback for non-C/C++ languages) remain marked as `none` with a justification note in the matrix or in Plan.md. The `arg_snippet` dead-code line in `src/algorithms/callback_dispatcher_slice.rs:303` is removed. `coverage/matrix.json` ends with newline.
- **Commit 2:** `cargo test --test integration_hapi_regression` passes. The smoke test asserts on JSON-shaped result fields, not text formatting.
- **Commit 3:** `Plan.md` parses as valid markdown; the new "Pre-handoff Architectural Baseline" section is present between `## Remaining Work` and `## Architecture Notes`; the §10 question answers map 1:1 to the handoff doc's open questions.

---

## 7. Out of scope

Explicitly deferred to the handoff Phase 1+ work:

- Extending `primitive_slice` with new rule patterns beyond what the algorithm already supports.
- Building the category-aware sanitizer registry.
- Building the framework detection layer.
- Modifying `taint.rs` or `provenance_slice.rs` algorithm logic.
- Patching `HARDCODED_SECRET` to handle keyword-prefixed LHS shapes — `const`/`let`/`var` (JS) and `static const char *` (C). (Go's `:=` already matches; only the keyword-prefixed forms miss.) Deferred — likely subsumed by handoff registry redesign.
- Updating `TEST_GAPS.md` or `TEST_COVERAGE.md`.
- Touching `docs/` plans other than `Plan.md`.
- Anything in the agent-eval roadmap beyond reading the supporting docs (`prism-assessment.md`, `prism-algorithm-matrix.md`) for context to inform the Plan.md baseline section.

---

## 8. Dependencies & risks

**Risks:**
- **Hapi-4552 review-suite output may be poor.** Mitigation: the smoke test exposes poor behavior with a FIXME comment rather than codifies it.
- **Trimmed hapi source may not align with diff line numbers exactly.** Mitigation: the trim is sized intentionally to match; if mismatch is found, regenerate the trim with corrected sizing.
- **The audit pass may reveal that the existing taint/provenance infrastructure is more (or less) developed than assumed.** Mitigation: the Plan.md baseline section is descriptive — whatever the audit finds is what gets recorded.

**No external dependencies.** All work is local: tree-sitter grammars, petgraph, existing `prism` crate.

---

## 9. Estimate

~1.5 days end-to-end:
- Commit 1: ~6 hours (29 algo tests + 3 cross-language tests = 32 tests, source cleanups, matrix update, badge regeneration, verify cargo passes)
- Commit 2: ~2 hours (trim hapi source to ~300 lines, write smoke test, verify)
- Commit 3: ~2 hours (audit pass on `taint.rs` + `provenance_slice.rs`, write baseline section, edit-in-place table updates)
- Buffer: ~2 hours for cargo test failures, fmt drift, unexpected gotchas in hapi fixture

Once committed, the ACK doc (separate task #3) is ~1 hour of synthesis on top of the already-written Plan.md baseline section.

---

*End of design.*
