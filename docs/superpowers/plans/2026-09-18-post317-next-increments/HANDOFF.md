# Handoff — post-317 planning custody

**Written:** 2026-09-19 · **By:** `/root/verification_setup` · **Provider:** codex
**Workspace:** `/private/tmp/prism-post316-slice1` · current planning/docs HEAD `60777a9b1e2adc2807dda9159450f97288e34085` · **Measured state:** `[MEASURED]` untracked custody paths only: `p2-split/`, `parked-p3/`, `CUSTODY-MAP.md`, and `HANDOFF.md`; probe `git status --short`.
**Predecessor:** `/root` controller
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker, folded from controller dispatch and byte-exact copied planning packets. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` `/root` owns commits and live dispatch; Sol owns the externally dispatched P2a implementation lane. — **RESOLVED by controller dispatch**
**(b) Custody exposure** — `[MEASURED]` this worktree has only the new untracked planning custody paths listed above; root must commit them if durable repository custody is wanted. — **OPEN until root commit**
**(c) In flight / irreversible** — `[INHERITED]` P2a is dispatched but core approval is pending; no public census, mutation, or production work has run. — **OPEN**
**(d) Authorization granted but not exercised** — "P2a dispatched/core pending, P2b deferred, P3 parked." No execution follows from this documentation copy.

## 1. Resume order

1. Treat `p2-split/P2A-SPEC.md` plus `p2-split/AMENDMENT.md` as the accepted P2a plan; preserve the explicit core-approval gate before public parsing or mutations.
2. Keep P2b deferred until a separate dispatch, budget, review cap, accepted P2a binding, and fresh native-readiness proof.
3. Keep `parked-p3/` parked; require a separate controller decision before execution.
4. Root may commit this docs-only custody batch after reviewing `CUSTODY-MAP.md`.

**STOP conditions:** Stop P2a before core approval; do not read the historical unsplit P2 packet as active authority; do not execute P3.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Exact caller reads source | done | `[INHERITED]` accepted source `14e86083c2c754ed412d440742bd5763be388e42`, tree `a0411db6d858fe34806376ce41d2621853848d8b`; docs closeout HEAD `60777a9b1e2adc2807dda9159450f97288e34085`. |
| Historical unsplit P2 | superseded | Existing `specs/02-fixed-source-parameter-census.md` and matching old prompts/review remain historical; accepted P2a split packet is `p2-split/`. |
| P2a syntax/integrity/frequency | pending | `[INHERITED]` accepted split artifact manifest `3b5e61b3f802ed961a46c2260f42ead3d387b7e0d86a1092119fd0cac323223a`; dispatched externally; core approval pending. |
| P2b native readiness | deferred | `p2-split/P2B-DEFERRED-SPEC.md`; no execution authorization. |
| P3 pilot | parked | `parked-p3/STATUS.md` and `parked-p3/README.md`; no execution authorization. |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| Historical unsplit P2 files under this directory | They describe the former full P2 contract as active. | `[MEASURED]` retain unchanged as historical; use `p2-split/AMENDMENT.md` and `p2-split/P2A-SPEC.md` for current P2a authority. |
| `parked-p3/STATUS.md` historical active-sequence wording | It predates the accepted P2a split state. | `[MEASURED]` `parked-p3/README.md` records P3's present PARKED state and directs current P2a state to this handoff. |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | P2a core review | pending | Review the externally frozen P2a artifact against `p2-split/P2A-SPEC.md`. | Core artifact/freeze | `prism.p2a-syntax-census.v1` |
| 2 | P2b native readiness | deferred | Do nothing until separate controller dispatch. | P2a acceptance and fresh authorization | `p2-split/P2B-DEFERRED-SPEC.md` |
| 3 | P3 pilot | parked | Do nothing until separate controller decision. | Controller decision | `parked-p3/STATUS.md` |
| 4 | Docs custody | pending | Root reviews and commits this docs-only batch. | Root commit | `CUSTODY-MAP.md` |

## 5. Invariants and traps — do not do these

- Never treat the presence of a copied prompt as execution authority — P2a still needs core approval; P2b and P3 are inactive.
- Never modify the historical unsplit P2 artifacts to make them resemble P2a — preserve their hashes and use the co-located split packet.
- Never infer native owner/slot/entry/call/Step5b conclusions from P2a syntax output — those claims are deferred to P2b.
- Keep P3 parked even if its historical brief names a next step — its durable copied status remains PARKED.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| accepted source | `14e86083c2c754ed412d440742bd5763be388e42` |
| accepted source tree | `a0411db6d858fe34806376ce41d2621853848d8b` |
| current docs closeout HEAD | `60777a9b1e2adc2807dda9159450f97288e34085` |
| P2a artifact manifest | `3b5e61b3f802ed961a46c2260f42ead3d387b7e0d86a1092119fd0cac323223a` |
| P2a normative spec | `6b681c35baa365260f9466d3479d2b04a5398add57b8a5229e4f72386a5e4d0c` |
| split acceptance | `a6da7de76d2f2d3f9c93b32e06dc97102f2befd3e0a6832c78813b9aa376304a` |
| old full P2 reference | `e6e1bb3e8a928943149e59cfa29598c1e15142d10468c32fe5704a70132c1f71` |
| P2 fixed input manifest | `f8ebbdd79ca01e5cdb675616b913f1fbb549d45378ab1047db84844a3bc8e696` |
| P3 preserved brief | `1839b042cd7d492b07b07dddcaec600cda4cf1979b94f39699788dc19aadab26` |
| P3 parked review | `30808433dd7cc2b96824acb73dd3bb03c23c6a3001bd6732cb74d79df9ca59c5` |

## 7. Refutation verdict and owner questions

**§2c verdict:** NOT APPLICABLE — no claim-bearing implementation changed; this is byte-exact planning custody. · claim: "The durable packet preserves current P2a/P2b/P3 controller state without changing historical planning bytes." · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: STATIC-ONLY · record: `CUSTODY-MAP.md` and `git status --short`.

**Questions the owner owes an answer to:** None.
