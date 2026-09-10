# Live adoption pytest opt-in

Published as [PR301](https://github.com/shoedog/prism/pull/301).

Base: PR300 merge `4e30ca84`. Guard/test checkpoint: `2811040`.

The live adoption pytest module now skips unless `PRISM_RUN_LIVE_EVALS` is exactly
`1`. This is an intentional-execution safeguard, not a restriction on useful
Sonnet work: the owner clarified that prior usage was negligible.

The guard runs before this module's DeepEval/adoption imports, dummy-key mutation,
skill/golden loading, configuration creation or trials. Ordinary pytest can run
the deterministic suites without entering this live module. Selecting the live
file alone is not opt-in and leaves no runnable tests (pytest exit5); the skip
reason names the required variable.

For an intentional live evaluation, retain the existing DeepEval workflow:

```sh
cd eval
PRISM_RUN_LIVE_EVALS=1 ADOPT_ID=round-N uv run deepeval test run \
  adoption/tests/test_prism_adoption.py \
  --identifier prism-adoption-round-N -n 5 -i -s
```

Keep the opt-in on the individual command rather than exporting it shell-wide.
No changes to Sonnet selection, K=5, metrics, goldens, thresholds, caches or teardown.
The body from `K = 5` through teardown is byte-identical to base. Direct runner
calls, 2x2 scripts, independent Tier-C CLI commands and pytest plugin startup are
outside this module-level guard. No claim to sandbox arbitrary Python.

## Evidence

Tests copy the actual suite bytes to an isolated pytest root. An import tripwire
refuses real evaluation dependencies before they can run, including on the
unguarded base. Positive controls supply fake dependency interfaces, not models.

- Final tests against merged base: **9 failed, 2 passed**. Every negative failure
  records a forbidden DeepEval import; the two enabled controls pass.
- Guarded candidate: **11 passed**. Absent/empty/false-like/whitespace values,
  explicit-file selection and collect-only cannot opt in. Exact1 executes two
  fake probes, ten Sonnet trials, five scores and one benchmark. Opted-in
  collect-only imports definitions but creates no config, trial or benchmark.
- Full default Python collection with opt-in cleared: **940 passed, 1 skipped**.
  The sole skip is the live adoption module; both local real-binary controls ran.
- Full Rust default/MCP/audit: **4,037 / 4,230 / 4,253 passed**, with one ignored
  (`resolution_test::slice_elem_variant_reserved`) per run. Observers **694**,
  helpers **18**, authority profiles **40** passed; format and whitespace checks pass.
  See the [machine receipt](2026-09-09-live-eval-opt-in.json) for clean-HEAD evidence.
- No actual live-model evaluation was run for this slice. The real DeepEval CLI's
  external model execution is intentionally not an end-to-end verification claim.
- Tier-A is not triggered: no call resolution, navigation or CPG changes. No
  receiver, closure, cache, budget, React.FC or react-scripts policy changes.

The initial candidate run had10pass/1fail because the test expected a skipped-total
summary during collect-only. Its event log already showed no forbidden imports;
the assertion was corrected to pytest's skip-reason output, not the implementation.
Both initial and corrected logs are retained. Two self-review rounds, SELF-PASS
(not independent); no open correctness findings within the stated guard contract.

## Next

Return to the separately scoped, source-backed complete project-boundary and
acquisition-feasibility decision. Do not raise the512-file limit alone, prune
sources to force admission or bypass unresolved closure. Keep react-scripts
unresolved and React.FC expansion separate.
