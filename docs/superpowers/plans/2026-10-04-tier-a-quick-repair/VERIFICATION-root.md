# MEAS-B verification

Checkout: `/Users/wesleyjinks/code/prism-tiera`, branch `plan/tier-a-quick-repair`,
base HEAD `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`. Changes are uncommitted;
controller owns Git writes. Evidence root: `/Users/wesleyjinks/prism-evidence/meas/tiera`.

Full suites were attempted, including unchanged Rust and Node tests. Verification is
**environment-limited**: their explicit compiler/input gates prevent successful
execution of the cases named under `## Not verified`. These failures are recorded,
not re-baselined or fixed outside this lane. The runnable eval suite is green.

| Check | Observed totals | Log in evidence root |
|---|---|---|
| Full eval suite, final source | 983 passed, 1 skipped | verification-eval.log |
| Repair regressions, candidate | 43 passed | verification-regressions.log |
| Same regressions, bound original harness | 42 failed, 1 positive control passed, 1 original reader warning | verification-red.log |
| Matrix | 178/178 ok; regressions [], flip_candidates [] | verification-matrix.log and final quick JSONs |
| Full Rust all features/all targets including doctests | 5139 passed, 23 failed, 1 ignored; 31 target results | rust-tests-all.log |
| Full Node script suite, 44 selected files | 121 passed, 31 failed, 1 skipped; 153 reported tests | hook-node-tests.log, hook-node-launch.log |
| Script Python suite | 57 passed, 82 subtests passed | hook-mutgate-tests.log |
| Full quick | exit 0, 3/3 VALID; 162.24s observed wall, 162.071s summed corpus timers | validated-quick.log, final-validated/ |
| Whitespace check | passed | git diff --check |

## Exact commands and results

All commands below were run locally in this session. No network acquisition or
Git writes were performed. `PYTHONPATH` binds the borrowed editable Python
installation to this checkout; offline/no-sync avoids package acquisition.

Full eval suite, cwd `/Users/wesleyjinks/code/prism-tiera/eval`:

```bash
PYTHONPATH=/Users/wesleyjinks/code/prism-tiera/eval PYTHONDONTWRITEBYTECODE=1 UV_CACHE_DIR=/Users/wesleyjinks/.local/share/prism/uv-cache UV_PROJECT_ENVIRONMENT=/Users/wesleyjinks/code/slicing/eval/.venv PRISM_BIN=/Users/wesleyjinks/code/prism-tiera/target/release/prism PRISM_MCP_BIN=/Users/wesleyjinks/code/prism-tiera/target/release/prism-mcp uv run --offline --no-sync pytest -q -p no:rerunfailures > /Users/wesleyjinks/prism-evidence/meas/tiera/verification-eval.log 2>&1
```

Result: **983 passed, 1 skipped**, 21.44s; the only skipped module requires
explicit live-model opt-in. Both real binary paths use matching release builds.

Repair regression suite, same cwd:

```bash
PYTHONPATH=/Users/wesleyjinks/code/prism-tiera/eval PYTHONDONTWRITEBYTECODE=1 /Users/wesleyjinks/code/slicing/eval/.venv/bin/python -B -m pytest -q -p no:rerunfailures tests/test_quick_repair.py > /Users/wesleyjinks/prism-evidence/meas/tiera/verification-regressions.log 2>&1
```

Result: **43 passed**, 2.17s.

Pre-change control, cwd
`/Users/wesleyjinks/prism-evidence/meas/tiera/base-control/eval`:

```bash
PYTHONPATH=/Users/wesleyjinks/prism-evidence/meas/tiera/base-control/eval PYTHONDONTWRITEBYTECODE=1 /Users/wesleyjinks/code/slicing/eval/.venv/bin/python -B -m pytest -q -p no:rerunfailures /Users/wesleyjinks/code/prism-tiera/eval/tests/test_quick_repair.py > /Users/wesleyjinks/prism-evidence/meas/tiera/verification-red.log 2>&1
```

Result: **42 failed, 1 passed, 1 warning**, 16.04s. Original eval sources and
tracked skill resources were exported from the bound HEAD without Git writes.
The same Python executable, test bytes and host are used. The one passing case
is the unchanged-source positive control. Missing newly added APIs/modules are
part of the RED capability checks; behavioral failures also cover blocked writes,
dead/malformed servers, bookend drift, empty inventories and CLI validation.
The warning comes from the original reader's unhandled malformed-header ValueError;
the candidate handles it and promptly wakes the pending request.

Full Rust suite, cwd `/Users/wesleyjinks/code/prism-tiera`:

```bash
CARGO_NET_OFFLINE=true CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --all-features --no-fail-fast > /Users/wesleyjinks/prism-evidence/meas/tiera/rust-tests-all.log 2>&1
```

Result: **5139 passed, 23 failed, 1 ignored** across 31 reported target results.
All 23 failures report absent explicit pinned compiler inputs (`NotPresent`).
Rust source/tests and Cargo bytes match the base, so this is also execution of
that unchanged artifact in this environment. No regression attribution is made.
This full run preceded the hook's additional Python tests; only the repair test
file changed after the valid quick measurement.

Full Node suite, same cwd:

```bash
python3 - <<'PY' > /Users/wesleyjinks/prism-evidence/meas/tiera/hook-node-launch.log 2>&1
import subprocess
from pathlib import Path
files=subprocess.check_output(['rg','--files','scripts','-g','*.test.mjs'],text=True).splitlines()
print('Selected files:',len(files),flush=True)
with Path('/Users/wesleyjinks/prism-evidence/meas/tiera/hook-node-tests.log').open('w') as log:
    result=subprocess.run(['node','--test',*files],stdout=log,stderr=subprocess.STDOUT,timeout=180)
print('Return code:',result.returncode)
raise SystemExit(result.returncode)
PY
```

Result: selected **44 files**, **121 passed, 31 failed, 1 skipped**, 153 reported
tests, approximately 2.99s. The 31 failed modules refuse missing explicit compiler,
profile or example inputs. Node script bytes are unchanged from the base.

Script Python suite, same cwd (the only script Python test file found):

```bash
PYTHONDONTWRITEBYTECODE=1 /Users/wesleyjinks/code/slicing/eval/.venv/bin/python -B -m pytest -q -p no:rerunfailures scripts/mutgate/test_mutgate.py > /Users/wesleyjinks/prism-evidence/meas/tiera/hook-mutgate-tests.log 2>&1
```

Result: **57 passed, 82 subtests passed**, 14.79s.

Release rebuild, cwd `/Users/wesleyjinks/code/prism-tiera`:

```bash
CARGO_NET_OFFLINE=true cargo build --release
CARGO_NET_OFFLINE=true cargo build --release --features mcp --bin prism-mcp
```

Both builds passed. The ordinary release SUT was rebuilt immediately before
`--allow-stale-sut` quick verification. The separate MCP binary build supplies
the matching real-binary pytest integration tests.

Matrix, cwd `/Users/wesleyjinks/code/prism-tiera/eval`:

```bash
CARGO_NET_OFFLINE=true PYTHONPATH=/Users/wesleyjinks/code/prism-tiera/eval PYTHONDONTWRITEBYTECODE=1 UV_CACHE_DIR=/Users/wesleyjinks/.local/share/prism/uv-cache UV_PROJECT_ENVIRONMENT=/Users/wesleyjinks/code/slicing/eval/.venv uv run --offline --no-sync tier-a --matrix-only --allow-stale-sut > /Users/wesleyjinks/prism-evidence/meas/tiera/verification-matrix.log 2>&1
```

Result: **178/178 ok**. Final quick independently embeds 178 ok results with
empty regression and matrix flip-candidate lists.

Bounded quick, same cwd, immediately preceded by an ordinary release rebuild:

```bash
CARGO_NET_OFFLINE=true PYTHONPATH=/Users/wesleyjinks/code/prism-tiera/eval PYTHONDONTWRITEBYTECODE=1 UV_CACHE_DIR=/Users/wesleyjinks/.local/share/prism/uv-cache UV_PROJECT_ENVIRONMENT=/Users/wesleyjinks/code/slicing/eval/.venv timeout --kill-after=5 1800 uv run --offline --no-sync tier-a --quick --allow-stale-sut --date meas-b-final --out-dir /Users/wesleyjinks/prism-evidence/meas/tiera/final-validated > /Users/wesleyjinks/prism-evidence/meas/tiera/validated-quick.log 2>&1
```

Result: exit **0**, Rust/TS/JS all **VALID**, zero oracle/SUT error rates, complete
input manifests match before and after. Observed wall **162.2439s**; monotonic
corpus timer sum **162.071s**. Only tests and documentation changed afterward;
implementation/source input hashes retain this run's binding.

Whitespace, cwd `/Users/wesleyjinks/code/prism-tiera`:

```bash
git diff --check
```

Result: passed. No baseline/adjudication changes.

## Verified

Every added/fixed behavior has regression coverage in
`eval/tests/test_quick_repair.py`; each behavior group includes a negative or edge
case. Candidate and original-harness controls use the exact same test file.

| Behavior group | Regression checks and edge cases |
|---|---|
| Bounded LSP lifecycle | Session budget cannot reset; a nonreading child fills the pipe; EOF and malformed framing wake pending requests; shutdown reaps the child |
| Timeout accounting and progress | Oracle vs server/null failure, capability/startup timeout, query timeout records; partial inventories never publish; capsys observes progress/errors; pinned timeout remains explicit |
| Bounded retries | ContentModified succeeds on retry and fails after bounded attempts; remaining query budget shrinks and retry records remain visible |
| Bounded SUT and cache | Subprocess timeout is classified; expired corpus budget launches no command; writable cache is used and matrix bypasses it |
| Immutable corpus validity | Changed content, altered/missing/malformed manifest, symlink, missing file and added configuration are rejected; post-cleanup drift invalidates; unchanged-source positive control passes |
| Sampling and selection | Indexed matcher preserves one-use/tie matching without scanning unrelated files; JS/TS extensions and nested strata; explicit quick corpus/lang filters and empty filter edge |
| CLI/report validity | Zero, negative, NaN and infinite limits rejected; out-dir honored; empty successful inventory is invalid; timeout/configuration rendered; aggregate counts and zero denominators are JSON-safe |
| Native JS/TS oracle | Newline request/Content-Length response roundtrip, server error, late response isolation; incoming/outgoing and exclusive multiline spans; external targets excluded; scalar/alias inventory excluded; definition column conversion and empty definitions |
| Native error distinctions | Allowed null configure/open differs from forbidden null query, valid empty calls, timeout and server failure |
| Rust feature configuration | Configured mcp options are sent; absent configuration sends none; feature-only live control at unchanged source coordinates distinguishes inactive from enabled symbol |

The packet's diagnosis preserves expected/falsifying observations and controls.
The final JS/TS raw call-site measurements are TS callers P/R .6429/.5294,
TS callees .55/.9167, JS callers 1/.625 and JS callees 1/.6. These are sampled
local call-resolution observations; precision/recall do not certify member
DFG, async semantics or whole-project type completeness. Historical baselines
remain intact; disclosed contextual feature flips are in the packet readout.

## Not verified

**environment-limited**. **Excluded:** successfully reaching the intended semantic
assertions of the 23 Rust tests and 31 Node modules named below. They were selected
by the full commands and failed their explicit input gates; the pinned compiler,
profiles and example inputs are absent. Acquisition/packaging is outside this lane.

Additional exclusions:

- Eval `adoption/tests/test_prism_adoption.py`: live adoption requires explicit
  `PRISM_RUN_LIVE_EVALS=1`; no live model calls were enabled.
- Rust `resolution_test::slice_elem_variant_reserved`: explicitly ignored because
  SliceElem is reserved by the spec and the classifier returns None.
- Node `valid inputs cannot authenticate tampered shipped generated bytes`:
  requires already acquired pinned archives through `PRISM_GRAMMAR_ARCHIVES`.
- A human-triggered all-corpus LSP run, installed Excalidraw dependencies,
  alternate server/OS versions and live Go/Python oracles were not checked.
- No independent reviewer agent was requested. Verification is a test-backed
  self-pass. Controller commits and baseline adoption remain pending.

The named outside-scope failures are preserved below and in the raw logs. There
is no claim that the entire project suite is green. Existing tests/expectations
were not re-baselined, disabled or silently repaired.

### Named outside-scope input failures

The full all-feature Rust command ran all targets: 5139 passed, 23 failed, 1 ignored. These 23 named tests require an explicit pinned compiler (`NotPresent`); it was not configured. Rust source and test bytes are unchanged from the bound base. This is not an attribution of a regression.

- `api::owner::tests::public_owner_session_resolves_contextual_member`
- `executable_owner::audit_tests::all_anchor_fields_from_two_genuine_inputs_cannot_be_spliced`
- `executable_owner::audit_tests::detached_constructor_proves_genuine_ts_tsx_and_rejects_other_epochs`
- `executable_owner::audit_tests::every_design_refusal_reaches_the_actual_constructor_in_ts_and_tsx`
- `executable_owner::audit_tests::parser_mapping_rejects_line_id_collisions_and_accepts_unicode_original_bytes`
- `executable_owner::audit_tests::real_acquisition_rejects_extra_missing_changed_and_excluded_prism_inputs`
- `executable_owner::audit_tests::same_root_positive_changed_and_unproven_epochs_do_not_reuse_authority`
- `executable_owner::integration::tests::cpg_uses_owned_inputs_refuses_cross_source_and_does_not_cache_edges`
- `executable_owner::integration::tests::direct_recomputation_cannot_keep_a_previous_epoch`
- `executable_owner::integration::tests::exact_site_fields_and_two_genuine_epoch_proofs_cannot_be_substituted`
- `executable_owner::integration::tests::full_subset_clone_serde_and_generic_mutation_have_explicit_authority`
- `executable_owner::integration::tests::fresh_sessions_replace_positive_different_unproven_failure_and_restore`
- `executable_owner::integration::tests::navigation_discards_even_a_supplied_persistent_sidecar_store`
- `executable_owner::integration::tests::generic_incremental_entry_drops_opt_in_and_rebuilds_without_stale_edges`
- `executable_owner::integration::tests::ordinary_cached_context_does_not_adopt_ephemeral_edges`
- `executable_owner::integration::tests::reconstruction_preserves_sticky_refusal_after_sidecar_clear`
- `executable_owner::integration::tests::staged_owner_reaches_the_real_shared_resolver`
- `executable_owner::integration::tests::tsx_positive_and_existing_annotation_routes_remain_distinct`
- `mcp::transport::owner_tests::owner_admission_wire_reports_fresh_inputs_and_compiler_phase_without_stale_edges`
- `mcp::transport::owner_tests::owner_wire_closure_refusal_and_dependency_restore_are_fresh`
- `mcp::transport::owner_tests::owner_wire_reacquires_a_b_unproven_and_config_failure_without_stale_edges`
- `owner_activation_test::owner_cli_and_api_agree_and_default_has_no_new_exact_edge`
- `owner_activation_test::owner_mcp_binary_handshake_and_default_concise_wire_gain`

The Node command selected 44 script test files and reported 121 passed, 31 failed, 1 skipped (153 reported tests). The failed modules require explicit compiler/example inputs such as `PRISM_TYPESCRIPT` and `PRISM_NATIVE_PARAMETER_EXAMPLE`. Node scripts are unchanged from the bound base. No compiler packaging/acquisition was performed.

- `scripts/callable-observations/closure-classification.test.mjs`
- `scripts/callable-observations/closure-policy-proof.test.mjs`
- `scripts/callable-observations/config-provenance-integration.test.mjs`
- `scripts/callable-observations/config-provenance.test.mjs`
- `scripts/callable-observations/direct-owner.test.mjs`
- `scripts/callable-observations/entries.test.mjs`
- `scripts/callable-observations/entry-obligations-integration.test.mjs`
- `scripts/callable-observations/exact-ambient.test.mjs`
- `scripts/callable-observations/identity-domains.test.mjs`
- `scripts/callable-observations/index.test.mjs`
- `scripts/callable-observations/lib-search-cases.test.mjs`
- `scripts/callable-observations/lib-search.test.mjs`
- `scripts/callable-observations/lookup.test.mjs`
- `scripts/callable-observations/membership.test.mjs`
- `scripts/callable-observations/merged-wildcard-proof.test.mjs`
- `scripts/callable-observations/merged-wildcard.test.mjs`
- `scripts/callable-observations/module-search-cases.test.mjs`
- `scripts/callable-observations/module-search.test.mjs`
- `scripts/callable-observations/nested.test.mjs`
- `scripts/callable-observations/props-class.test.mjs`
- `scripts/callable-observations/required-paths.test.mjs`
- `scripts/callable-observations/semantic-closure-integration.test.mjs`
- `scripts/callable-observations/semantic-closure-v2-integration.test.mjs`
- `scripts/callable-observations/semantic-closure-v3-integration.test.mjs`
- `scripts/callable-observations/type-lib.test.mjs`
- `scripts/callable-observations/type-search-cases.test.mjs`
- `scripts/callable-observations/type-search-identity.test.mjs`
- `scripts/callable-observations/umd-qualifier.test.mjs`
- `scripts/callable-observations/wildcard.test.mjs`
- `scripts/parameter-frequency/index.test.mjs`
- `scripts/parameter-slot-characterization/request.test.mjs`

Live model adoption was intentionally not enabled. A human-triggered all-corpus LSP run, alternate OS/server versions, installed Excalidraw dependencies, and live Go/Python oracle runs were not verified. Compiler-worker packaging/freshness/acquisition remain outside this lane.
