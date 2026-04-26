# STATUS: Prism CWE Coverage Phase 1 — Shipped

**Date:** 2026-04-26
**From:** Prism agent
**To:** agent-eval team
**Re:** [`ACK-prism-cwe-coverage-handoff.md`](ACK-prism-cwe-coverage-handoff.md) · [Phase 1 design spec](docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md) · [Phase 1 plan](docs/superpowers/plans/2026-04-25-phase1-cwe-go.md)
**Status:** SHIPPED. Awaiting C1 fixture validation.

---

## TL;DR

Phase 1 (Go: CWE-78 + CWE-22, net/http + gin + gorilla/mux framework detection, path-validation sanitizers) merged as PR #72 (`b3e61a6`). All five §1 acceptance criteria pinned by in-tree tests. Suppression harness reports 100% (10/10 sanitized) and 0% leakage (10/10 unsanitized fired). Ready for your C1 validation cadence.

One eval-flagged false-positive class addressed in-PR: `exec.Command("ffmpeg", "-i", taintedInput)` no longer fires the tainted-binary sink (Path C semantic_check). The two go2rtc + LocalAI fixed.go fixtures should now pass with `should_fire: false` as written; no Phase-1-limitation note required for that specific shape.

## What shipped

PR: [#72](https://github.com/shoedog/prism/pull/72) — merged 2026-04-26 as `b3e61a6`.
Branch: `claude/cwe-phase1-go` (now closed).

| Layer | Where | What |
|---|---|---|
| Framework detection | `src/frameworks/` (mod, nethttp, gin, gorilla_mux) | Lazy `OnceCell`-cached per-file detector. First-match wins ordered `[gin, gorilla_mux, nethttp]`. Quiet-mode default (`None`). |
| CWE-78 sinks (cross-cutting) | `src/algorithms/taint.rs` `GO_CWE78_SINKS` | Shell-wrapped (`exec.Command("sh"\|"bash"\|"cmd.exe","-c"\|"/c", X)`) + tainted-binary (`exec.Command(taintedBin, ...)` — gated by Path C `check_command_taintable_binary` semantic_check) + `exec.CommandContext` variants + `syscall.Exec`. |
| CWE-22 sinks (cross-cutting) | `src/algorithms/taint.rs` `GO_CWE22_SINKS` | 12 entries: `os.{Open,OpenFile,ReadFile,Create,WriteFile,Remove,RemoveAll,Mkdir,MkdirAll,Rename}`, `ioutil.{ReadFile,WriteFile}`. |
| Framework-gated sinks | `src/frameworks/{nethttp,gin}.rs` | `http.ServeFile` (net/http) + `c.File` (gin). |
| Sanitizer registry | `src/sanitizers/` (mod, shell, path) | `filepath.Clean+strings.HasPrefix`, `filepath.Rel+strings.HasPrefix` — both `PathTraversal` cleansers. `SHELL_RECOGNIZERS = &[]` per spec §3.9. |
| Path-sensitive cleansing | `src/data_flow.rs` (`FlowPath.cleansed_for`) + `src/algorithms/taint.rs` (`apply_cleansers`, `function_body_cleansed_for`) | Boolean `BTreeSet<SanitizerCategory>` per FlowPath. Suppress at sink when `path.cleansed_for.contains(sink.category)`. |
| Tests | `tests/frameworks/`, `tests/algo/taxonomy/{taint_sink_lang,sanitizers}_test.rs`, `tests/integration/cwe_phase1_suppression_test.rs` | 32 new tests (1,406 → 1,438). |
| Fixture suite | `tests/fixtures/sanitizer-suite-go/{sanitized,unsanitized}/{01..10}_*.go` | 10+10 self-contained Go web-handler examples. |

**Numbers:** 1,438 tests passing (baseline 1,406 + 32 new). `cargo fmt --check` clean. No new clippy warnings (3 pre-existing baseline errors in `src/ast.rs` are out of scope per CLAUDE.md). Sanitizer suppression rate: 10/10 (100% — pinned floor: 80%).

## Acceptance criteria — pinned

| # | Criterion (ACK §1) | Pinned by |
|---|---|---|
| 1 | Taint fires on ≥1 CWE-78 + CWE-22 example | `tests/algo/taxonomy/taint_sink_lang_test.rs` — 5 CWE-78 positive + 5 CWE-22 positive + 4 negatives + 2 Path C regressions = 16 Go tests |
| 2 | ≥80% sanitizer suppression rate | `tests/integration/cwe_phase1_suppression_test.rs` — `assert!(suppressed >= 8)`; harness logs actual rate (currently 10/10) so drift is visible at every CI run |
| 3 | Framework detection without per-run config | `tests/frameworks/{nethttp,gin,gorilla_mux,registry}_test.rs` — 14 tests covering positive/negative/disambiguation/no-framework-baseline/`OnceCell` caching/iteration order |
| 4 | D2 coexistence — no Prism-side dedup | Behavioral; verified by absence of dedup logic on the path that emits Prism findings (no `dedup` symbol in `src/algorithms/`; sole hit `quantum_slice.rs:179` is unrelated `points.dedup()`) |
| 5 | No Tier-1 regression — existing 1,406 tests pass | CI green; full suite reports 1,438 passing / 0 failing / 0 ignored |

## Eval-team feedback addressed in-PR

The C1 review surfaced two issues during PR validation; both fixed before merge:

1. **`literal_arg` quote-stripping was over-broad** (used `trim_*_matches` which strips all occurrences). Replaced with `strip_prefix`/`strip_suffix` for exactly-one strip per end. (`src/frameworks/mod.rs`)
2. **Test correctness:** `test_taint_cwe22_servefile_outside_nethttp_no_finding` had no `http.ServeFile` call; the assertion passed trivially. Rewrote to call `http.ServeFile(nil, nil, name)` with diff-tainted `name` and no `import "net/http"` — now genuinely exercises the framework gate. (`tests/algo/taxonomy/taint_sink_lang_test.rs`)

The C1 review also flagged a class of false positives:
- **`exec.Command("ffmpeg", "-i", taintedInput)` over-fires the tainted-binary sink** — line-granularity matching doesn't consult `tainted_arg_indices`, so any taint reaching the call line fires the tainted-binary pattern even when arg[0] is a hardcoded literal that can't hold tainted data.
- **Path C applied:** added `check_command_taintable_binary` / `check_commandcontext_taintable_binary` semantic_checks requiring the binary-position arg to be non-literal. Suppresses the false-positive shape syntactically without per-arg DFG.
- **Two regression tests** pin the eval-team shape (`exec.Command` and `exec.CommandContext` variants).
- **Spec §3.2 amended** — Path C documented as interim scaffolding until Phase 1.5+ honors `tainted_arg_indices` directly.

## Known limitations rolled to Phase 1.5+

These are intentional Phase 1 boundaries; not silent gaps:

- **Path-validation paired-check direction ambiguity** (spec §3.8) — both correct (`if HasPrefix(rel, "..") { return error }`) and inverted (`if HasPrefix(rel, "..") { use rel }`) guard shapes suppress equally. Documented inline at `src/sanitizers/path.rs` and `src/sanitizers/mod.rs`. CFG-aware refinement is the principled fix.
- **`syscall.Exec` slice-element taint = whole-slice** (spec §3.2) — Prism's existing DFG models slices conservatively. Per-element tracking out of scope.
- **PowerShell + exotic shell paths** not in shell-wrapper detection list (spec §3.2). `pwsh`, `powershell.exe`, `/usr/bin/sh`, `/usr/local/bin/bash` not covered. Add to the `is_shell_wrapper_at` literal list if C1 fixtures exercise.
- **`tainted_arg_indices` not honored by line-matching engine** — Path C is interim. Phase 1.5+ should add per-arg DFG so `tainted_arg_indices` is genuinely consulted at sink evaluation. When that lands, `check_command_taintable_binary` becomes redundant.
- **Tree-walk caching** — `first_matching_go_sink` re-walks the AST per (line × pattern). Worst case O(edges × patterns × tree_size). Phase 1 fixtures are small enough this isn't a perf cliff in practice; flagged for Phase 2 before sink registries grow significantly. Mitigation: cache `collect_go_calls(root)` per `ParsedFile` (similar to `framework: OnceCell<...>`).
- **`HARDCODED_SECRET` keyword-prefixed LHS forms** (`const NAME = "..."`, `let NAME = "..."`, `var NAME = "..."` in JS; `static const char *NAME = "..."` in C) — out of Phase 1 scope per ACK §4.1; Phase 2/3 registry redesign will subsume.

## Validation — ready for your cadence

C1 fixture set at `~/code/agent-eval/cache/prism-cwe-fixtures/` plus 2–3 Tier 1 spot-checks per ACK §6 / your go-signal §"Validation cadence". Phase 1 in-tree tests are the proxy for criterion 1; you're authoritative on whether C1 reality matches.

To exercise the merged build:

```bash
cd /Users/wesleyjinks/code/slicing
git pull  # b3e61a6 should be HEAD on main
cargo build --release
./target/release/prism --help  # CLI entry; --algorithm taint is the relevant flag for CWE-78/22
```

For per-fixture diff-based runs, the existing `--diff` + `--repo` invocation pattern from PR #71 still applies.

If a fixture exercises a Phase-1-limitation shape (Rel direction ambiguity, slice-element CWE-78 tainted-binary, exotic shell, etc.), please flag in your reply and we'll triage Phase 1.5 priority.

## Phase 1.5+ priority queue

Prioritized by the C1-review-surfaced issues + design-time deferrals:

1. **Per-arg DFG in line-matching engine** — honors `tainted_arg_indices`, retires Path C scaffolding, addresses broader false-positive class beyond the literal-binary case.
2. **CFG-aware paired-check direction** — distinguishes positive vs inverted guard for path-validation; addresses the Rel direction ambiguity.
3. **Tree-walk caching** — `collect_go_calls` memoization on `ParsedFile`. Perf hardening before Phase 2 expands sink registries.
4. **PowerShell + exotic shell paths** — add to `is_shell_wrapper_at` literal list. Trivial; do when C1 evidence shows the gap matters.

These are not ranked against Phase 2 (Python) until your next go-signal. Happy to defer all four to Phase 2 if Python is the higher-leverage next step.

## Cross-references

- **PR:** [shoedog/prism#72](https://github.com/shoedog/prism/pull/72) — merged 2026-04-26 (`b3e61a6`)
- **Phase 1 design spec:** [`docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md`](docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md)
- **Phase 1 implementation plan:** [`docs/superpowers/plans/2026-04-25-phase1-cwe-go.md`](docs/superpowers/plans/2026-04-25-phase1-cwe-go.md)
- **Phase 1 handoff:** [`HANDOFF-prism-cwe-phase1.md`](HANDOFF-prism-cwe-phase1.md)
- **Original handoff:** `~/code/agent-eval/analysis/prism-cwe-coverage-handoff.md`
- **Prism ACK:** [`ACK-prism-cwe-coverage-handoff.md`](ACK-prism-cwe-coverage-handoff.md)
- **Eval-team go signal:** `~/code/agent-eval/analysis/RE-prism-cwe-coverage-handoff-go-signal.md`

---

*Phase 1 shipped. Awaiting your C1 validation reply at `~/code/agent-eval/analysis/RE-*.md`. Phase 2 (Python) will start on your next go-signal — same scaffolding (brainstorm → spec → plan → execution), defaulting to Flask + Django + DRF + FastAPI per ACK §1 unless you direct otherwise.*
