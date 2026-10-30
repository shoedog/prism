# Handoff — CommonJS export-object barriers

Superseded for current operational state by the [fourth-increment handoff](2026-09-11-cjs-terminal-proof.md).
This historical increment is commit032e1824, not whichever commit is currently HEAD.
Its deferred terminal defect is repaired in the follow-up; duplicate-set names now
survive as refusal-only markers rather than disappearing. Historical totals below
apply only to032e1824.

**Written:** 2026-09-11 · **By:** root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/slicing · feat/js-ts-module-binding-audit · **Measured state:** `[MEASURED]` tested base c3d110ef plus CJS changes; final source/test hashes in the baseline receipt. Delivery is the local commit containing this handoff, identified by `git log -1`.
**Predecessor:** esm-forwarding c3d110ef, local/unpushed.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live; evidence /private/tmp/prism-cjs-barrier-CBIIas.

## 0. Gating facts — settle these before starting anything below

(a) RESOLVED: root sole source writer; two rounds plus one disclosed bounded
hardening/verification extension; targeted final delta approved W0/S1.
(b) RESOLVED snapshot custody: source-checkpoint.tgz captures initial source;
final-source-checkpoint.tgz captures the pre-hardening state; the later
verified-source-checkpoint.tgz captures the final source/tests after hardening.
Implementation/tests/docs are saved in the local commit containing this handoff;
identify its SHA with `git log -1`. Three increments are bundled locally, unpushed.
(c) RESOLVED: no irreversible or remote operation in flight; local tests only.
(d) Owner: "ok, whats next? lets proceed to it amd include it in the PR too".
Local next increment authorized; bundled future PR, no publication performed.

## 1. Resume order

1. `git status --short --branch`; read adjacent cjs-export-barriers plan.
2. Matched base replay and final-source gates are complete with exclusions below.
3. Publication is held. Next capability proof: CJS terminal RHS identity and
initialization/capture timing before require-time snapshot/forwarding admission.

STOP: open-class findings, unrelated writes, promotion of CJS forwarding without
terminal and require-time snapshot proof, unavailable oracle called green.

## 2. State ledger

| Item | State | Evidence |
|---|---|---|
| Initial RED | done | base-barriers.log:8pass28fail on unchanged c3d110ef production |
| Initial GREEN | done | candidate-barriers.log:36pass |
| Final fixtures/epochs | done | base-final-cjs-v3.log44/45; candidate-final-cjs-v3.log89pass |
| Review/hardening | done | review-delta-red2fail; hardening-red-final8fail; final89pass; module controls62pass |
| Full gates | done | Rust4356/4549/4572; examples32; zero failures, one existing ignore per Rust config |
| Final helper/matrix gates | done | Node786, authority40, Python940+live skip, matrix159; binary hashes unchanged |
| Quick/exclusions | done | final quick300044ms no verdict; historical helpers3 unavailable; no multicorpus run |

## 3. Corrections to standing documents and memory

The proposed forwarding increment is superseded by a prerequisite producer-side
defect repair. No CJS Gap is promoted. The prior ESM increment remains complete.
No memory edits authorized.

## 4. Open work

No implementation work remains in this bounded increment. Publication is held.
Next, independently:
CJS Local can select a nested same-name decoy; terminal declaration/initialization
and capture-time proof remain missing. Consumer mutations, module cycles and
require-cache/snapshot ownership also remain open; do not claim CJS soundness.

## 5. Invariants and traps — do not do these

- Only temporary CJS facts are discarded; ESM provenance remains separate.
- Keep whole replacement/member mixtures refused, even apparently safe orderings.
- No new target authority, class/receiver expansion or closure-policy changes.
- Fresh CLI/MCP/example binaries must come from the same frozen source invocation.
- Prism navigation stale43paths; current source and executable tests are authority.

## 6. Identifiers

| Item | Value |
|---|---|
| Base | c3d110ef |
| Evidence | /private/tmp/prism-cjs-barrier-CBIIas |
| Prior evidence | /private/tmp/prism-esm-forwarding-1nk0WK |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED — independent targeted hardening-delta approval W0/S1,
TEST-BACKED; two source rounds plus one disclosed bounded extension. Claim: covered
producer mutation forms cannot leave authoritative CJS facts or poison independent
ESM facts. Final-source full gates complete with explicit quick/helper exclusions.

**Questions the owner owes an answer to:** None.
