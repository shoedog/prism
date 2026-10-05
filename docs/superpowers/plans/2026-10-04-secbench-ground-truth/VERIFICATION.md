> Historical R0 record. R1 changes ground truth, attribution and conversions; use [R1.md](R1.md) for the current result and verification. Original evidence remains immutable.

# MEAS-A verification

## Tested state

Branch `plan/secbench-ground-truth`, HEAD
`4e592daa7858a195eb3a9eb77c83dfbc763b49fa`. Dirty scope is only
`eval/pyproject.toml`, `eval/secbench/`, this packet and root VERIFICATION.md. No product source changes.
Release SUT SHA256 `d603e7cd7f372137c254f4d7cf98ea8c33f92ac22dc510bea0e1b00e919aa455`.
Final ledger has exactly 600 distinct class+entry rows. Input population/hash
checks and raw stdout hash validation passed; invocation errors are preserved.

## Test results

| Check | Result | Evidence under /Users/wesleyjinks/prism-evidence/meas/secbench/ |
|---|---|---|
| `CARGO_NET_OFFLINE=true cargo build --release` | passed; original clean base binary retained | run/binding.json; custody.json |
| `cargo test --offline` | 4,946 passed, 0 failed, 1 ignored; 29 groups (rerun at verification gate) | verification-gate-cargo.log |
| Full eval pytest suite | 965 passed, 3 skipped; 10 subtests passed (environment-limited) | verification-gate-eval.log |
| `node --test eval/secbench/inspect.test.mjs` | 22 passed, 0 failed | verification-gate-node.log |
| `python3 -m unittest eval.secbench.test_run eval.secbench.test_replay eval.secbench.test_io eval.secbench.test_cli` | 27 passed, 0 failed | verification-gate-harness.log |
| Module CLI `--help` | parser/options execute | cli-help.log |
| Same-environment pre-run harness control | 3 Node and 2 Python behavioral assertions fail on preserved older code, current tests pass | control-tests.log; snapshots/pre-run.tar.gz |
| Fresh corrected Prism controls | aaptjs and thenify function-only; hangersteak traced; all match replay | live-controls/results.json |
| Repeat full replay | entries.jsonl, summary.json, summary.md, binding.json byte-identical | determinism.json |
| `git diff --check` | passed | final custody record/transcript |
| Product diff (src/Cargo/build/vendor) | empty | custody.json and final transcript |

Full eval command used the preinstalled read-only environment, with auto-loading
plugins disabled after pytest-rerunfailures tried to bind a socket before collection:

```sh
cd /Users/wesleyjinks/code/prism-secbench/eval
PYTHONDONTWRITEBYTECODE=1 PYTEST_DISABLE_PLUGIN_AUTOLOAD=1 \
DEEPEVAL_TELEMETRY_OPT_OUT=YES \
/Users/wesleyjinks/code/slicing/eval/.venv/bin/python -m pytest -q -rs
```

The initial pre-collection socket refusal was inadmissible, not a test or product
failure. Disabling unrelated plugin autoload makes the local suite runnable.

## Exact exclusions and unverified claims

- Rust ignored `resolution_test::slice_elem_variant_reserved`: reserved SliceElem
  classifier, intentionally not implemented in the product's existing suite.
- Eval `adoption/tests/test_prism_adoption.py`: live adoption eval requires explicit
  `PRISM_RUN_LIVE_EVALS=1`; not enabled in this offline measurement lane.
- Eval `test_resolve_matched_binaries_against_real_binaries` and
  `test_warm_gate_real_cache_distinguishes_cold_from_warm`: no built prism-mcp
  release binary / matched `PRISM_BIN` and `PRISM_MCP_BIN` pair. No MCP build added.
- Console-script installation/wheel execution is not tested; no dependencies were
  installed. The direct module command and registered configuration are checked.
- The final one-command full direct run was not repeated after bounded classifier
  changes. All original full observations are authenticated and replayed by the
  corrected classifier; three decisive live controls match. Replay determinism is
  established, fresh full-corpus Prism determinism is not.
- GT remains unavailable for 391 acquired entries. Full population path-syntax
  point prevalence, root-specific break attribution for every entry, independent
  adjudication accuracy and actual workstream conversion counts are unverified.
- No exploit/runtime/CVE validation, external dependency bodies, regex complexity
  analysis, field-selective API source soundness, or live LSP/Tier-A quick/matrix
  run. Tier-A product-change requirement is not triggered because src is unchanged.

## Observation error population

The original 2,123 invocations retain 31 dfg-stats errors, 18 callees errors,
17 callers errors, 18 ego errors, 26 frontier errors, 33 witness errors and one
classic-taint error; zero chop errors. These include initial GT candidates later
excluded. There are 28 final eligible decisive-query error outcomes. Comparison
channel errors are not silently converted into taint misses. For example
SymbolNotFound, UnsupportedFile and LocationOutOfRange remain in the raw JSON.

## Custody and integration

Raw observations/bindings, final/input inspections, logs, harness/packet and the
original release binary are retained in snapshots/final.tar.gz. artifact-index.json
contains its SHA256 and final file hashes. No Git writes: controller commits.
Suggested commits:

1. `eval: add pinned offline SecBench source-to-sink benchmark`
   — eval/secbench/ and eval/pyproject.toml.
2. `docs: record SecBench tracing measurements and bounded workstream ranking`
   — this packet, including summary.json and observation-binding.json.

Original run classifications are superseded, not discarded; run/SUPERSEDED.md
points to final/. Handoff is reconciled with the current counts and limitations.

## Verification hook completion

The authoritative current test command/totals/audit record is repo-root
VERIFICATION.md, with the required `## Verified` and `## Not verified` sections.
**environment-limited; Excluded:** live adoption and the two named real-MCP checks
above, plus the existing reserved SliceElem Rust ignore. No unexpected test failures
surfaced; no baselines or product source were changed.
