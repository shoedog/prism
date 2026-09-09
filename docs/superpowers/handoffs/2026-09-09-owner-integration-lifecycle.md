# Handoff — disabled owner integration core

Current successor: PR297 merged as a120220f. Owner subsequently authorized the
[bounded opt-in activation lane](2026-09-09-owner-opt-in-activation.md). The states
below are historical PR297 receipts, not claims that activation remains unauthorized.

**Written:** 2026-09-09 · **By:** /root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/slicing · feat/owner-integration-lifecycle · **Measured state:** `[MEASURED]` matched controls at997b39d support review readiness; PR297 marked ready. Original quick remains INVALID as a baseline. This turn changes documentation/receipts only; prior full Rust totals below are inherited, not rerun. New evidence `/private/tmp/prism-tier-a-297-dSTyZl`.
**Predecessor:** PR296 detached constructor.
**Truth ordering:** measured state > owner authority within scope > current handoff > historical snapshots.
**Provenance:** written live; current source, test output and remote publication verified.

## 0. Gating facts — settle these before starting anything below

(a) Ownership RESOLVED: primary owns implementation/review; no delegates.
(b) Custody RESOLVED: previous branch preserved; new bounded lane and evidence directory.
(c) In flight: PR297 review and documentation-only closeout CI; no public activation or cache migration. CI passed at997b39d before this closeout.
(d) Authority: "merged, proceed to next" following opt-in integration with epoch/cache safeguards. Approved design permits disabled staging to avoid partial production wiring.

## 1. Resume order

1. `git status --short`; read current plan beside this handoff.
2. Implementation and two bounded self-review rounds complete; do not restart.
3. Read the matched Tier-A disposition and PR297 CI/review. Readiness is supported; a new accuracy baseline is not claimed. Do not silently waive/re-baseline or activate production.

STOP on closure expansion, install requirement, enabled partial wiring or open-class findings at cap.

## 2. State ledger

| Item | State | Evidence |
|---|---|---|
| Merged base / source seams | done | a892b67d; source inspection; stale/truncated Prism callers, LSP unavailable |
| Implementation / RED | implemented | seam RED NameOnly vs Exact; boundary RED 7 pass/2 fail; recomputation RED 0 pass/1 fail; archived sources beside logs |
| Full suites | passed on clean 2c033e8 | default4026, MCP4216, MCP+audit4233, observer694, helpers18, authority40; one ignored per Rust run; doctests/fmt/diff pass; clippy completes with warnings |
| Tier-A historical quick | INVALID baseline | preserved original corpus drift/oracle0.20; not relabeled passed |
| Matched disposition | done | 140 raw pairs identical,159 complete matrix records equal/ok; original six MCP no-item probes reproduce on base; both eval suites883 pass/two skips |
| Publication | ready for review | https://github.com/shoedog/prism/pull/297; no merge, new baseline or activation claim |

## 3. Corrections to standing documents and memory

PR296 is merged. Its detached scope remains historical; this lane stages integration,
not user-facing activation. No memory edits. No claims of current real-receiver gain.
The initial draft assumption was stronger than the approved harness spec: invalid
baseline prevents new accuracy anchoring, while review requires quick execution and
reported observations. Matched controls now support readiness without a rule change.

## 4. Open work

| Work | State | Next action |
|---|---|---|
| Disabled core | implemented | shared resolver, CPG and navigation consume one owned epoch |
| Lifecycle/cache | implemented | replace on refresh; no positive sidecar persistence; sticky graph and CPG refusal markers |
| Review readiness | done | matched disposition supports review; baseline validity remains separate |
| Public activation | parked | separate CLI/MCP publication checkpoint after core review |

## 5. Invariants and traps — do not do these

- No compiler subprocess in default CLI/MCP paths; no React.FC or react-scripts change.
- No proof in pre_resolved_target; no naked validated bool or public install setter.
- Clearing proof metadata alone does not clear CPG edges; retain sticky cache refusal.
- No CPG ReturnFlow gain claim for the no-result-assignment fixture.
- Tier-A is triggered by the resolver/navigation/CPG edits even though activation stays disabled.

## 6. Identifiers

Base a892b67ddec0883fc0ba7cfaae7fa33fdfb97eaf. Evidence `/private/tmp/prism-owner-integration-jYIuUv`.
Compiler `/private/tmp/prism-imported-alias-O4d6E1/package/lib/typescript.js`.
Plan `docs/superpowers/plans/2026-09-09-owner-integration-lifecycle.md`.
Tested 2c033e80c26df33f9fa403968a5c4d76ab10ef59. [PR297](https://github.com/shoedog/prism/pull/297).
Archive `/private/tmp/prism-owner-integration-jYIuUv-evidence.tgz`, SHA256 b3d0e6a056766a1a902e76bf2b3a772edfdbd5bb7ed984de84853b591c013e66.
[Readout](../../eval/receiver-closure/2026-09-09-owner-integration-lifecycle.md) and
[receipt](../../eval/receiver-closure/2026-09-09-owner-integration-lifecycle-gates.json).
New [disposition](../../eval/receiver-closure/2026-09-09-pr297-tier-a-disposition.md)
and [receipt](../../eval/receiver-closure/2026-09-09-pr297-tier-a-disposition.json).
New archive `/private/tmp/prism-tier-a-297-dSTyZl-evidence.tgz`, SHA256 6e3df1e3cae462a68c4c48aaadfda04ad97e5a8737fd92123bb9833605173b75.

## 7. Refutation verdict and owner questions

**§2c verdict:** BOUNDED SELF-PASS for disabled core (prior two implementation review rounds) and review readiness (two new diagnostic rounds). No demonstrated PR297 regression in matched controls. Same-environment base replay explains the missing oracle population; exact pinned sources restore both feature-gated controls. Historical quick remains invalid, not a new accuracy anchor. No independent-agent review or production activation claim.

**Questions the owner owes an answer to:** None for review readiness. Owner controls review/merge and separate authorization of activation or harness-policy work.
