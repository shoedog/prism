# Handoff — nested-callable execution-owner proof and bounded-repair design

**Written:** 2026-09-14T08:27:22Z · **By:** /root Codex · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/slicing · feat/js-ts-module-binding-audit · **Measured state:** `[MEASURED]` HEAD 36aec5f7a16e9eb90002f01235365cdc4ab6395a · Tree DIRTY · Probe `git status --short && git rev-parse HEAD HEAD^{tree}` · Output `/private/tmp/prism-nested-owner-proof-Rjzipk/`
**Predecessor:** none — first in implementation lane
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — another session/agent alive in this lane? who owns it? `[MEASURED]` Proof implementation/review are complete; the controller exercised commit/publication authority on the isolated proof branch — **RESOLVED 2026-09-14T08:27:22Z**
**(b) Custody exposure** — unpushed commits, uncommitted work, single-copy/untracked artifacts: `[MEASURED]` proof source is committed and pushed in PR #315; the original checkout remains dirty with preserved planning artifacts, and `/private/tmp/prism-nested-owner-proof-Rjzipk` remains host-local evidence — **OPEN only for PR merge and off-host evidence retention**
**(c) In flight / irreversible** — running process, held lock, half-applied migration: `[MEASURED]` no verification or review process is active; the two-round cap completed with APPROVE — **RESOLVED 2026-09-14T07:03:28Z**
**(d) Authorization exercised** — controller approval produced branch `feat/nested-callable-owner-proof` and PR #315; this does not authorize merge or the runtime repair.

## 1. Resume order

1. Review PR #315 at `feat/nested-callable-owner-proof`; its initial proof commit is `92ace687` and its base is `c1dbc292`.
2. Merge only under separate controller authority; keep the expected desired-owner test ignored in this proof PR.
3. Start the bounded runtime repair as a separate slice after merge or explicit stacked-branch authority; do not treat proof approval as repair approval.

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
| Publication | open | `[MEASURED]` branch `feat/nested-callable-owner-proof` pushed; PR #315 open/non-draft/mergeable against `c1dbc292` |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| `docs/superpowers/plans/2026-09-13-nested-callable-owner-proof.md` checklist | Pre-implementation tasks were unchecked | `[MEASURED]` Tasks 0–5 are checked after two review rounds and final custody reconciliation |
| Memory | No current nested-owner proof result exists | `[MEASURED]` do not edit memory; user did not authorize a memory update |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Review fixes | done | Positive preservation, default RED and parked exact-call ownership were added; DFG/fallback claims narrowed; Rust gates rerun | None | same artifact |
| 2 | Final custody | published | Exact proof source is in PR #315; final manifest/tar and reviewer records remain under the evidence root | PR merge | workstream 3 proof |
| 3 | Production repair | parked | Implement only in the successor slice; retain all exclusions and refusal controls | merge or explicit stacked-branch authority | bounded repair contract |

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
| Publication | `https://github.com/shoedog/prism/pull/315` · branch `feat/nested-callable-owner-proof` · initial commit `92ace6875bcc3f37f8585a22e75076aaaece02f6` |
| Readout SHA-256 | `fa48bc8a3791d2eac55e18f0cd7cb9b88614e0e3139439fa3161d6a34749a8dc` |
| Baseline JSON SHA-256 | `138f51aa0d0382722502aee12f1c90d6a52fbac5ee379a3e77baebabae71ec54` |

## 7. Refutation verdict and owner questions

**§2c verdict:** APPROVE at round 2/cap — 0 WRONG, 2 retained non-blocking SMELL; no closed-enumerable or open-class finding remains · claim: "the measured proof supports a bounded rvalue-specific scoped traversal without changing conservative capture authority" · production-design verdict: `READY_FOR_BOUNDED_REPAIR` · evidence tier: INDEPENDENTLY REVIEWED, TEST-BACKED · record: `/private/tmp/prism-nested-owner-proof-Rjzipk/review-round2.md`

**Questions the owner owes an answer to:** None.
