# Handoff — post-317 planning custody

**Written:** 2026-09-19 · **By:** `/root/verification_setup` · **Provider:** codex
**Workspace:** `/private/tmp/prism-post316-slice1` · docs HEAD before this update `1f6b65c70ca7df97cc7af86909eef2b8fe9c424a` · **Measured state:** `[MEASURED]` this docs-only P2a stop receipt and handoff refresh are pending root commit; probe `git status --short`.
**Predecessor:** `/root` controller
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker, folded from controller dispatch and byte-exact copied planning packets. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — `[INHERITED]` `/root` owns commits and future dispatch; the external P2a extraction stopped before compile. — **RESOLVED by controller PARK-DESIGN classification**
**(b) Custody exposure** — `[MEASURED]` compact stop receipt `p2-split/P2A-BUDGET-STOP.md` binds external snapshot and receipt hashes; root must commit this docs-only refresh for repository custody. — **OPEN until root commit**
**(c) In flight / irreversible** — `[INHERITED]` P2a has no authorized in-flight execution; it stopped before compile/public parse. — **RESOLVED by external stop receipt**
**(d) Authorization granted but not exercised** — "P2a extraction again exceeded cap ... PARK-DESIGN; NO further split/ceiling increase/execution." P2b remains deferred; P3 remains parked.

## 1. Resume order

1. Treat `p2-split/P2A-BUDGET-STOP.md` as the current P2a execution status; preserve the accepted spec and amendment as historical planning authority.
2. Do not split, raise a ceiling, compile, parse public members, census, mutate, or review P2a without a new controller design decision and bounded contract.
3. Keep P2b deferred and `parked-p3/` parked.
4. Root may commit this docs-only custody refresh after reviewing the external hash binding.

**STOP conditions:** Stop all P2a execution under the present contract; do not read the historical unsplit P2 packet as active authority; do not execute P2b or P3.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Exact caller reads source | done | `[INHERITED]` accepted source `14e86083c2c754ed412d440742bd5763be388e42`, tree `a0411db6d858fe34806376ce41d2621853848d8b`; docs closeout HEAD `60777a9b1e2adc2807dda9159450f97288e34085`. |
| Historical unsplit P2 | superseded | Existing `specs/02-fixed-source-parameter-census.md` and matching old prompts/review remain historical; accepted P2a split packet is `p2-split/`. |
| P2a syntax/integrity/frequency | parked-design | `[INHERITED]` accepted split manifest/spec remain historical; external extraction stopped precompile at 1,257/900 formatted Rust lines. `p2-split/P2A-BUDGET-STOP.md` binds snapshot `main.rs` SHA `596d3540d39c5959127fb1ba6b7624b957aa35b7dda8fa62cc776a688a6ef192`. |
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
| 1 | P2a redesign | parked-design | Do nothing until controller selects and accepts a new bounded contract. | Controller design decision | `p2-split/P2A-BUDGET-STOP.md` |
| 2 | P2b native readiness | deferred | Do nothing until separate controller dispatch. | P2a acceptance and fresh authorization | `p2-split/P2B-DEFERRED-SPEC.md` |
| 3 | P3 pilot | parked | Do nothing until separate controller decision. | Controller decision | `parked-p3/STATUS.md` |
| 4 | Docs custody | pending | Root reviews and commits this docs-only batch. | Root commit | `CUSTODY-MAP.md` |

## 5. Invariants and traps — do not do these

- Never resume P2a by splitting again, raising a ceiling, compacting formatted code, compiling, or public parsing — controller classified the repeated size defect PARK-DESIGN.
- Never modify the historical unsplit P2 artifacts to make them resemble P2a — preserve their hashes and use the co-located split packet.
- Never infer native owner/slot/entry/call/Step5b conclusions from P2a syntax output — those claims are deferred to P2b.
- Keep P3 parked even if its historical brief names a next step — its durable copied status remains PARKED.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| accepted source | `14e86083c2c754ed412d440742bd5763be388e42` |
| accepted source tree | `a0411db6d858fe34806376ce41d2621853848d8b` |
| docs HEAD before P2a stop update | `1f6b65c70ca7df97cc7af86909eef2b8fe9c424a` |
| P2a artifact manifest | `3b5e61b3f802ed961a46c2260f42ead3d387b7e0d86a1092119fd0cac323223a` |
| P2a normative spec | `6b681c35baa365260f9466d3479d2b04a5398add57b8a5229e4f72386a5e4d0c` |
| split acceptance | `a6da7de76d2f2d3f9c93b32e06dc97102f2befd3e0a6832c78813b9aa376304a` |
| old full P2 reference | `e6e1bb3e8a928943149e59cfa29598c1e15142d10468c32fe5704a70132c1f71` |
| P2 fixed input manifest | `f8ebbdd79ca01e5cdb675616b913f1fbb549d45378ab1047db84844a3bc8e696` |
| P2a external stop receipt | `cd05343f60a53c3975de585d9e9b6ec51b3f6d58778ccc873cc34dead7a380d2` |
| P2a stopped source snapshot | `596d3540d39c5959127fb1ba6b7624b957aa35b7dda8fa62cc776a688a6ef192` |
| P2a snapshot manifest | `472e3b49c498fcfc8203605beaa0ede4596d7e2c4ef8a3029232a7b432eda1d7` |
| P2a terminal handoff | `ba21572b1bcce78e52272975dcf786f9224ca431f079a9e59946402862f501c1` |
| P2a terminal audit | `0701897076b62149c9126630c52dd86eed38a424b4392dfadd6ee2ee39e7ffb0` |
| P3 preserved brief | `1839b042cd7d492b07b07dddcaec600cda4cf1979b94f39699788dc19aadab26` |
| P3 parked review | `30808433dd7cc2b96824acb73dd3bb03c23c6a3001bd6732cb74d79df9ca59c5` |

## 7. Refutation verdict and owner questions

**§2c verdict:** NOT APPLICABLE — no claim-bearing implementation changed; this is status custody. · claim: "P2a stopped precompile and has no P2a census or behavioral result under the present contract." · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: STATIC-ONLY · record: `p2-split/P2A-BUDGET-STOP.md` and external receipt hash.

**Questions the owner owes an answer to:** None.
