# Handoff — explicit live adoption pytest opt-in

**Written:** 2026-09-09 · **By:** /root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/slicing · fix/live-eval-explicit-opt-in
**Measured state:** `[MEASURED]` base4e30ca84; changed tests/docs/live-suite guard.
**Predecessor:** PR300 merged; owner approved next and clarified low Sonnet usage.
**Truth ordering:** measured live state > explicit owner authority > this handoff > history.
**Provenance:** written live; gate totals are not inherited.

## 0. Gating facts — settle these before starting anything below

(a) Ownership RESOLVED: primary owns lane; no delegates dispatched.
(b) Custody pending: implementation and RED captured, checkpoint next.
(c) In flight: focused GREEN complete; full suites pending.
(d) Authority: “merged proceed to next”, bounded explicit opt-in hardening.
No actual live model calls are needed or planned for this slice.

## 1. Resume order

1. `git status --short --branch`; read adjacent live-eval-opt-in plan.
2. Run focused tests then full suites; capture raw output below.
3. Review at two-round cap, checkpoint, publish and verify exact-head CI.

STOP on new external-model/config authority or open-class findings at the cap.

## 2. State ledger

| Item | State | Evidence |
|---|---|---|
| Merged base | done | fetched/logged4e30ca84, clean before branch |
| Safe behavioral RED | done | red.log9fail/2pass, import tripwire on unchanged suite bytes |
| Guard/docs | done | exact-value module guard; current README commands updated |
| Focused GREEN | done | green-corrected.log11pass; fake enabled path retains ten Sonnet trials |
| Review | done | two self-review rounds: import boundary and explicit enablement/collection controls |
| Full gates | next | stable implementation checkpoint |
| Publication | next | commit/push/PR after gates |

## 3. Corrections to standing documents and memory

PR300 is merged. Owner clarified previous Sonnet usage was about2% of five-hour
allowance and negligible weekly impact. Preserve factual historical execution
record, but do not describe follow-up as responding to material cost. No memory edits.

## 4. Open work

Full gates and publication. Project-boundary/acquisition
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

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "plain collection cannot enter this live suite
without exact opt-in; enabled test body remains available" · SELF-PASS (NOT
INDEPENDENT), TEST-BACKED by11 focused tests. Two rounds; no open WRONG/SMELL
findings within this guard contract. Full gates remain pending, not a done claim.

**Questions the owner owes an answer to:** None within this slice.
