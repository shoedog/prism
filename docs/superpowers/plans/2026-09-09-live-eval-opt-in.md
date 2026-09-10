# Explicit live adoption pytest opt-in

Base PR300 merge4e30ca84. Owner approved the next recommended hardening slice;
owner clarified prior Sonnet usage was negligible, so this is intentional-execution
control, not a cost incident response or restriction on useful Sonnet work.

## Contract

Only `PRISM_RUN_LIVE_EVALS=1` enables the existing live adoption pytest module.
Absent/empty/false-like/whitespace values skip at module level, before eval imports,
dummy-key mutation, source/golden loading, config creation and trial execution.
The documented DeepEval command retains existing models, metrics, goldens, K=5,
cache behavior and teardown. No new dependencies, hosted reporting or tracing.
Opted-in collect-only loads definitions but does not execute fixtures/trials.
No claim to sandbox arbitrary Python, stop pytest plugins, or guard independent
runner/2x2/Tier-C CLI workflows. No receiver/closure/budget policy changes.

## Plan and verification

1. Capture behavioral RED with real suite bytes copied into isolated pytest roots;
   dependency import tripwire prevents any real model execution on unguarded base.
2. Add smallest module-level guard and update current run instructions.
3. Test missing/invalid opt-in, explicit file and collect-only, actual unchanged
   opt-in body through fake dependencies (two probes, ten Sonnet trials), and
   opted-in collect-only with no config/trial/benchmark events.
4. Full deterministic Python collection, Rust default/MCP/audit and supporting
   observer/helper/authority suites; format. Tier-A not triggered: no resolution,
   navigation or CPG changes. Do not run actual live model evals for this guard.
5. Two self-review rounds maximum; commit, push and PR with exact gate totals.

## Hypothesis / probe / result

- Hypothesis: plain collection enters the live module without an opt-in barrier.
  Alternative: cached trajectories prevent model execution on one particular run;
  that does not protect a cache miss. Inspect imports/test body and use an import
  tripwire on unchanged source, independent of cache or account credentials.
- Expected RED: no opt-in reaches the tripwire; enabled fake controls still pass.
  Observed red.log:9 failures at recorded deepeval import,2 enabled controls pass.
  No production eval dependencies or model launch used by the probe.
- First GREEN attempt:10pass/1fail solely because collect-only prints a skip
  reason rather than the normal skipped-total summary. Import events were empty,
  separating this assertion mismatch from a missing guard; corrected assertion
  checks SKIPPED and explicit opt-in reason. Retain green.log as test-probe evidence.

## Completion

Checkpoint2811040 passes11 focused tests. Final test bytes rerun on merged base
still produce9fail/2pass safely. Full Python940pass/1 live-module skip; Rust
4037/4230/4253pass with one ignored each; observers694, helpers18, authority40.
Format/whitespace pass. No live model evaluation; Tier-A not triggered.
Two self-review rounds complete. See the [readout](../../eval/2026-09-09-live-eval-opt-in.md).
