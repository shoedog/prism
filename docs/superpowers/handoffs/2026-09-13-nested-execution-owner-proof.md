# Handoff — nested-callable execution-owner proof and bounded-repair design

**Written:** 2026-09-14T14:48:19Z · **By:** /root Codex · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/slicing · feat/js-ts-module-binding-audit · **Measured state:** `[MEASURED]` HEAD 36aec5f7a16e9eb90002f01235365cdc4ab6395a · origin/main 9e40a376299a714ed11233a56a9e23554d40ca99 · Tree DIRTY · Probe `git fetch origin main && git status --short && git rev-parse origin/main` · Output `/private/tmp/prism-nested-owner-proof-Rjzipk/`
**Predecessor:** none — first in implementation lane
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — another session/agent alive in this lane? who owns it? `[MEASURED]` Proof implementation/review are complete; the controller exercised commit/publication authority on the isolated proof branch — **RESOLVED 2026-09-14T08:27:22Z**
**(b) Custody exposure** — unpushed commits, uncommitted work, single-copy/untracked artifacts: `[MEASURED]` proof source is merged through PR #315 at `9e40a376`; the original checkout remains dirty with preserved planning artifacts, and `/private/tmp/prism-nested-owner-proof-Rjzipk` remains host-local evidence — **OPEN only for off-host evidence retention**
**(c) In flight / irreversible** — running process, held lock, half-applied migration: `[MEASURED]` no verification or review process is active; the two-round cap completed with APPROVE — **RESOLVED 2026-09-14T07:03:28Z**
**(d) Authorization exercised** — controller approval produced and merged PR #315; this does not authorize the runtime repair.

## 1. Resume order

1. Start the bounded runtime repair only as a separate slice from merged main `9e40a376`, under explicit repair authority.
2. Keep the expected desired-owner test ignored until that repair slice captures its own unchanged-base RED and makes it green without changing the refusal controls.
3. Preserve `/private/tmp/prism-nested-owner-proof-Rjzipk` or copy it to approved off-host custody before host cleanup.

**STOP conditions:** production-body/cache/index/parameter/call-ownership changes; mismatch-stream drift between base and candidate; another Python or Tier-A quick retry; an open-class review finding at the two-round cap.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Pinned identity | done | `[MEASURED]` HEAD `36aec5f7…`, tree `2ac20fcb…`, equal to supplied pinned-main tree |
| Helper RED/GREEN | done | `[MEASURED]` `logs/helper-red.log`, `logs/helper-green.log` |
| Raw proof | done | `[MEASURED]` `logs/proof-audit-round2.log`: 7 passed, 1 named ignore |
| Desired RED/base attribution | done | `[MEASURED]` 387 rows on both checkouts, normalized SHA `b96b7274…`; exact eight-row positive oracle also passes |
| Full gates | done with exclusions | `[MEASURED]` default 4508/0/2; MCP 4701/0/2; owner 4701/23/2 with identical base NotPresent population; examples32; Node786; authority40; Python940+1skip; matrix159; quick INVALID timeout |
| Independent review | approved | `[MEASURED]` Round 1 `FIX-FIRST` (3 WRONG, 3 SMELL); Round 2/cap `APPROVE` (0 WRONG, 2 retained non-blocking SMELL); design verdict `READY_FOR_BOUNDED_REPAIR` |
| Durable readout | done | `[MEASURED]` `docs/eval/receiver-closure/2026-09-13-nested-execution-owner-proof.md` and adjacent baseline JSON |
| Publication | merged | `[MEASURED]` PR #315 merged into `main` at `9e40a376299a714ed11233a56a9e23554d40ca99` on 2026-09-14T14:42:29Z; merge parents are pinned base `c1dbc292` and reviewed head `53f9d782` |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| `docs/superpowers/plans/2026-09-13-nested-callable-owner-proof.md` checklist | Pre-implementation tasks were unchecked | `[MEASURED]` Tasks 0–5 are checked after two review rounds and final custody reconciliation |
| Memory | No current nested-owner proof result exists | `[MEASURED]` do not edit memory; user did not authorize a memory update |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Review fixes | done | Positive preservation, default RED and parked exact-call ownership were added; DFG/fallback claims narrowed; Rust gates rerun | None | same artifact |
| 2 | Final custody | merged | Exact proof source is in merged PR #315; final manifest/tar and reviewer records remain under the evidence root | off-host evidence retention | workstream 3 proof |
| 3 | Production repair | parked | Implement only in the successor slice; retain all exclusions and refusal controls | explicit repair authority | bounded repair contract |

## 5. Invariants and traps — do not do these

- Never edit production collector bodies, occurrence indexing, parameter admission, call/receiver ownership or caches — this slice is proof-only.
- Never interpret 387 source mismatches as 387 unique real sites or as wrong exact DFG/public results — they are consumer/language/multiplicity observations. The exact call-owner duplication is a separate parked WRONG.
- Never delete the conservative capture/fallback route — graph proof observed two `NameOnly(CfgIncomplete)` capture labels.
- JSON serialization of `CallGraph` fails on map keys → use the repository's bincode cache encoding.
- A synthetic inverted CPG span is an invalid negative → move `end_byte` outside the argument while preserving ordered endpoints.
- Owner-audit's 21 library + 2 integration failures are same-environment base-matched `NotPresent` compiler refusals; do not fix, skip or rebaseline them.
- Tier-A quick timed out at 300013ms with no report and is INVALID; do not retry or call it green.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Evidence root | `/private/tmp/prism-nested-owner-proof-Rjzipk` |
| Base tree | `2ac20fcbf743d54c508f914d685936285f30a549` |
| Desired rows/hash | `387` / `b96b7274a05149039d85672a3fac343ddc84838220fa74f1756c5cf17f83320b` |
| New ignored test | `ast::nested_execution_owner_tests::nested_execution_owner_desired_contract` |
| Cache pins | `CPG 92 / navigation 52` |
| Publication | `https://github.com/shoedog/prism/pull/315` · merged `9e40a376299a714ed11233a56a9e23554d40ca99` · reviewed head `53f9d78224794866299f70c4752aae1ef1dc5c41` · initial commit `92ace6875bcc3f37f8585a22e75076aaaece02f6` |
| Proof-freeze readout SHA-256 | `fa48bc8a3791d2eac55e18f0cd7cb9b88614e0e3139439fa3161d6a34749a8dc` |
| Post-merge custody readout SHA-256 | `2ffffbfa45e2a6cfb6f153a074d6fb314f6dad85af5bdf02689444a1c2906ca9` |
| Baseline JSON SHA-256 | `138f51aa0d0382722502aee12f1c90d6a52fbac5ee379a3e77baebabae71ec54` |

## 7. Refutation verdict and owner questions

**§2c verdict:** APPROVE at round 2/cap — 0 WRONG, 2 retained non-blocking SMELL; no closed-enumerable or open-class finding remains · claim: "the measured proof supports a bounded rvalue-specific scoped traversal without changing conservative capture authority" · production-design verdict: `READY_FOR_BOUNDED_REPAIR` · evidence tier: INDEPENDENTLY REVIEWED, TEST-BACKED · record: `/private/tmp/prism-nested-owner-proof-Rjzipk/review-round2.md`

**Questions the owner owes an answer to:** None.
