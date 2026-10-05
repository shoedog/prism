# Verification — MEAS-A SecBench harness

Verified checkout: `/Users/wesleyjinks/code/prism-secbench`, branch
`plan/secbench-ground-truth`, HEAD `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`.
This record satisfies the verification hook's repository-root requirement.
The implementation changes are the new offline eval harness, eval registration,
and measurement/verification documents; Rust product source is unchanged.

## Commands and totals

Run from the repository root unless an explicit `cd` is shown. All log paths
below are under `/Users/wesleyjinks/prism-evidence/meas/secbench/`.

```sh
CARGO_NET_OFFLINE=true cargo test --offline > /Users/wesleyjinks/prism-evidence/meas/secbench/verification-gate-cargo.log 2>&1
```

Full default-feature Rust suite: **4,946 passed, 0 failed, 1 ignored**, across
29 test groups, including integration tests and doctests.

```sh
cd /Users/wesleyjinks/code/prism-secbench/eval
PYTHONDONTWRITEBYTECODE=1 PYTEST_DISABLE_PLUGIN_AUTOLOAD=1 DEEPEVAL_TELEMETRY_OPT_OUT=YES /Users/wesleyjinks/code/slicing/eval/.venv/bin/python -m pytest -q -rs > /Users/wesleyjinks/prism-evidence/meas/secbench/verification-gate-eval.log 2>&1
```

Largest configured offline eval subset, **environment-limited**:
**965 passed, 0 failed, 3 skipped, 10 subtests passed**.
Plugin autoload is disabled because the initial default invocation's unrelated
pytest-rerunfailures plugin tried a sandbox-denied socket bind before collection.
That refusal was inadmissible as test/product evidence, not a regression.

```sh
cd /Users/wesleyjinks/code/prism-secbench
node --test eval/secbench/inspect.test.mjs > /Users/wesleyjinks/prism-evidence/meas/secbench/verification-gate-node.log 2>&1
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest eval.secbench.test_run eval.secbench.test_replay eval.secbench.test_io eval.secbench.test_cli > /Users/wesleyjinks/prism-evidence/meas/secbench/verification-gate-harness.log 2>&1
```

Node parsing tests: **22 passed, 0 failed**. Python harness tests: **27 passed,
0 failed**; these Python cases are included in the eval total, not additional
independent cases to add to 965.

```sh
git diff --check
git diff --exit-code -- src Cargo.toml Cargo.lock build.rs vendor
```

Both checks passed; the product diff is empty. No commits or Git writes were made.

## Verified

- All project-default Rust tests and all locally configured eval tests were run,
  beyond the tests added by this task. No unexpected suite failures occurred,
  including no failures outside the task's scope. No baseline was changed.
- Every implemented behavior family has acceptance tests and negative/edge cases,
  listed below. The hook audit added attribution branch/default/rest checks, invalid-UTF8, sink recovery/admission,
  compiler-pin, invocation-output/error/timeout, input-drift and CLI edge coverage.
- Complete corpus observations, corrected classifications, input pins and replay
  checks remain unchanged: 600 entry rows, 192 eligible, 84 traced, 391 unavailable
  GT, 17 acquisition exclusions. The final ledger SHA256 remains
  `84d58d1d1c3fc1bf987a6be9ad6882d58acfe29516e2d0f6fb57537c353c023a`.
- Prior repeat replay authenticated inputs/raw stdout and produced identical
  JSONL, JSON, Markdown and binding files. Three fresh corrected Prism controls
  matched replay. These observations remain bound to the original release SUT.

### Acceptance and regression test audit

| Behavior | Tests and negative/edge cases |
|---|---|
| Single offline module command | test_cli.py: expected help/options; unknown option rejected before measurement |
| Export/require/API derivation | inspect.test.mjs: direct/member/destructured aliases; unresolved or multiple exports, malformed exploit, callback-only input |
| Payload parameter selection | marker/initializer versus benign target; HTTP setup versus forwarded request; returned factory API rejected; multiple unmarked parameters unavailable |
| Parameter forms | destructured/rest/default and constructor/object-spread cases; supplied non-payload argument omitted |
| Sink binding/admission | exact coordinates and unique basename repair; ambiguous/missing/unsafe paths; invalid/out-of-range coordinates; stream options omitted and regex receiver included |
| Source/sink identity | test_run.py + parser tests: parameter byte span, UTF8 BOM, member path, unrelated callback, wrong occurrence, synthetic unique use versus collision |
| Syntax and path evidence | actual constructs versus strings/comments; unrelated package syntax and parser gaps stay unknown; test-fed rest entry is present regardless of tracing |
| Five outcome handling | reached parameter, function span only, boundary exit, no continuation; invalid schema/decisive invocation error stays prism_error |
| Attribution/mapping | unrelated and reached features omitted; source binding/subtype and declared workstream mapping; exclusions never become opportunities |
| CLI output protocol | test_io.py: JSON/JSONL success and retained byte hashes; nonzero structured error, malformed JSON, timeout; no plain stdout/stderr scratch files left |
| Acquisition byte custody | stable input pins; source/exploit/metadata drift, added file and symlink all rejected |
| Comparison versus decisive queries | measure error wrapping; comparison-only failure retains reached witness; synthetic diff has only source declaration line |
| Replay custody | successful reuse; changed raw error bytes and new source/sink lines rejected; unavailable GT stays separate |
| Adjudication | expected-outcome drift rejected; resolved/unresolved categories and agreement kept distinct |

### Same-environment pre-change controls

The actual Git base has no `eval/secbench/` package. An exact read-only
`git archive HEAD eval` export was made in `/private/tmp/secbench-base-rvungh0n`.
The standard-library CLI tests were run against that export with the same Python
interpreter/environment and produced **2 expected failures**, then passed against
current code. This is feature-availability evidence: no expected benchmark help,
and stderr explicitly says `No module named eval.secbench`. It is not algorithm
accuracy evidence and is not a failed collection presented as a regression.
Exact invocation:

```sh
PRISM_SECBENCH_TEST_ROOT=/private/tmp/secbench-base-rvungh0n PYTHONDONTWRITEBYTECODE=1 python3 /Users/wesleyjinks/code/prism-secbench/eval/secbench/test_cli.py
```

For behavioral corrections after the harness's first stable point, current tests
were copied beside preserved pre-run inspect.mjs/run.py in
`/private/tmp/secbench-legacy-3kb07vri` and run in that directory:

```sh
node --test inspect.test.mjs
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest test_run
```

The older parser produces **14 passes, 8 expected assertion failures**: incorrect
payload parameter selection, HTTP/factory setup sources, stream-option sink values,
BOM byte offsets, missing member identity and ambiguous payload handling. Before the final extra call-chain test, the older
classifier produces **6 passes, 2 expected behavioral assertion failures and 3
errors**. The failures are incorrect traced credit for a wrong sink occurrence and
colliding synthetic use. One error reproduces the null-callee crash; two are absent
new summary fields and are labeled feature additions, not legacy behavioral
regressions. Current parser/classifier tests all pass.
`verification-gate-base-control.log` and `verification-gate-legacy-control.log`
retain actual assertion/output evidence. Expected RED controls are separate from
unexpected full-suite failures. No failure is attributed to the Rust product.

## Not verified

This run is environment-limited.

Excluded:

- `adoption/tests/test_prism_adoption.py`: live adoption evaluation requires
  explicit `PRISM_RUN_LIVE_EVALS=1`, which is not enabled in this offline/no-network
  measurement lane.
- `tests/test_tc_harness_hardening.py::test_resolve_matched_binaries_against_real_binaries`:
  no built `prism-mcp` release binary and no configured matched
  `PRISM_BIN`/`PRISM_MCP_BIN` pair; this optional MCP build was not added.
- `tests/test_tc_harness_hardening.py::test_warm_gate_real_cache_distinguishes_cold_from_warm`:
  requires the same absent real MCP binary pair for its eager-server/cache check.
- `resolution_test::slice_elem_variant_reserved`: existing intentional Rust ignore;
  SliceElem is reserved and the classifier is not implemented. This is an existing
  product scope exclusion, not an environment failure or a re-baseline.

Also unverified: feature-expanded Cargo/MCP builds, console-script wheel installation,
a second fresh full-corpus Prism run after classifier corrections, independent
adjudication accuracy, root-specific attribution for every unreached path, actual
workstream conversions, full-population payload-path syntax prevalence, field-selective
API input soundness and runtime exploit validity. No exploit was executed or product
capability changed to close those measurement/design limitations.

All 192 eligible inputs were also checked: selected parameter declarations and
function start use the same line. No multiline source-seed mismatch affects this
measurement. This is a bound check, not a generalized multiline support claim.

No test failure outside task scope surfaced. Known prior pre-collection environment
refusal and deliberately failing regression controls are documented separately above.

## Custody and controller integration

The packet VERIFICATION.md and HANDOFF.md are reconciled with these final totals.
The root record, expanded tests, logs and stable measurement artifacts are retained
in `/Users/wesleyjinks/prism-evidence/meas/secbench/snapshots/final.tar.gz`; its digest
and per-file hashes are in artifact-index.json. The controller commits the harness/
eval registration and the packet plus this root VERIFICATION.md; the worker performs
no Git writes.
