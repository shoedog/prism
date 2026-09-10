# Handoff — explicit live adoption pytest opt-in

**Written:** 2026-09-09 · **By:** /root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/slicing · fix/live-eval-explicit-opt-in
**Measured state:** `[MEASURED]` guard/test checkpoint2811040 on base4e30ca84;
full clean-HEAD gate runner passed. Subsequent edits are verification docs only.
**Predecessor:** PR300 merged; owner approved next and clarified low Sonnet usage.
**Truth ordering:** measured live state > explicit owner authority > this handoff > history.
**Provenance:** written live; gate totals are not inherited.

## 0. Gating facts — settle these before starting anything below

(a) Ownership RESOLVED: primary owns lane; no delegates dispatched.
(b) Custody RESOLVED: guard/tests2811040 and verification docs0f431fb pushed;
PR301 opened: https://github.com/shoedog/prism/pull/301. Evidence archived below.
(c) In flight: local verification complete; exact-head remote CI pending, not green.
(d) Authority: “merged proceed to next”, bounded explicit opt-in hardening.
No actual live model calls are needed or planned for this slice.

## 1. Resume order

1. `git status --short --branch`; read adjacent live-eval-opt-in plan.
2. Read the [completed readout](../../eval/2026-09-09-live-eval-opt-in.md) and receipt.
3. Verify PR301 exact-head CI; this follow-up is publication metadata only.
   No automatic merge.

STOP on new external-model/config authority or open-class findings at the cap.

## 2. State ledger

| Item | State | Evidence |
|---|---|---|
| Merged base | done | fetched/logged4e30ca84, clean before branch |
| Safe behavioral RED | done | red.log9fail/2pass, import tripwire on unchanged suite bytes |
| Guard/docs | done | exact-value module guard; current README commands updated |
| Focused GREEN | done | green-corrected.log11pass; fake enabled path retains ten Sonnet trials |
| Review | done | two self-review rounds: import boundary and explicit enablement/collection controls |
| Final-tests base replay | done | final-tests-base-red.log9fail/2pass; same test bytes as candidate |
| Full gates | done | Python940pass/1skip; Rust4037/4230/4253pass, one ignored each; observer694/helper18/authority40 |
| Publication | done | PR301, pushed0f431fb plus publication-only follow-up |

## 3. Corrections to standing documents and memory

PR300 is merged. Owner clarified previous Sonnet usage was about2% of five-hour
allowance and negligible weekly impact. Preserve factual historical execution
record, but do not describe follow-up as responding to material cost. No memory edits.

## 4. Open work

Owner review/merge and remote CI. Project-boundary/acquisition
feasibility remains the separate next design slice after this guard.

## 5. Invariants and traps — do not do these

- Never run live models to prove the guard: fake dependency subprocesses suffice.
- Exact1 only; explicit file selection, cached results and credentials are not opt-in.
- Guard must precede eval imports/config/data loading; plugin startup is outside scope.
- Keep Sonnet/K/goldens/metrics/cache logic unchanged; no automatic threshold tuning.
- Full verification must clear any inherited live opt-in before broad collection.

## 6. Identifiers

Evidence `/private/tmp/prism-live-opt-in-GTQE8v`; RED `red.log`.
Test `eval/tests/test_live_eval_opt_in.py`.
Guard `eval/adoption/tests/test_prism_adoption.py`.
Variable `PRISM_RUN_LIVE_EVALS`, required value `1`.
Gate summary `gate-logs-2026-09-10T01-13-03-669Z-75134/summary.json`.
Archive `/private/tmp/prism-live-opt-in-GTQE8v-evidence.tgz`, mode0600,
SHA256 `0cdcfe9544f690f5f260f8c5f408bda1d7236de7de2a516361525e6bc51d8612`.
Immutable verification checkpoint before publication metadata; no credentials.

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "plain collection cannot enter this live suite
without exact opt-in; enabled test body remains available" · SELF-PASS (NOT
INDEPENDENT), TEST-BACKED by11 focused tests. Two rounds; no open WRONG/SMELL
findings within this guard contract. Full deterministic gates passed; no live
model run claimed, and Tier-A was not triggered by this test-only boundary change.

**Questions the owner owes an answer to:** None within this slice.
