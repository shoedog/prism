# Handoff — nested-callable execution-owner bounded repair

**Written:** 2026-09-15T01:24:15Z · **By:** /root Codex · **Provider:** codex
**Workspace:** /private/tmp/prism-nested-owner-proof-pr-dYl0Qn · feat/nested-callable-owner-repair · **Measured state:** `[MEASURED]` implementation commit 0858c23e pushed and PR #316 open against main before this docs-only custody reconciliation · Probe `git status --short && git show --stat --oneline --decorate HEAD` · Output `/private/tmp/prism-nested-owner-repair-TfBvIW/`
**Predecessor:** `/root` proof lane; merged PR #315 handoff `docs/superpowers/handoffs/2026-09-13-nested-execution-owner-proof.md`
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

**(a) Lane ownership** — another session/agent alive in this lane? who owns it? `[MEASURED]` `/root` completed implementation; independent round 1 returned APPROVE with 0 WRONG and one retained SMELL, and no round 2 is required — **RESOLVED 2026-09-15T00:18:05Z**
**(b) Custody exposure** — unpushed commits, uncommitted work, single-copy/untracked artifacts: `[MEASURED]` implementation commit `0858c23e` is pushed; PR #316 is open; evidence remains host-local at `/private/tmp/prism-nested-owner-repair-TfBvIW`; this handoff is the docs-only publication reconciliation — **RESOLVED for source and remote custody; retain host evidence**
**(c) In flight / irreversible** — running process, held lock, half-applied migration: `[MEASURED]` no verification process is active and no full multi-corpus run occurred; the authorized commit, push and PR creation completed — **RESOLVED 2026-09-15T01:24:15Z**
**(d) Authorization granted and exercised** — “Anything else for this slice? if not create a PR” authorized `.git` writes and PR publication. Commit `0858c23e`, branch push and PR #316 are complete. Merge, live adoption and a full multi-corpus run remain separately unauthorized.

## 1. Resume order

1. Review PR #316 and preserve the round-1 APPROVE plus retained non-blocking SMELL.
2. Treat Tier-A matrix and quick as INVALID and preserve every recorded exclusion; do not rebaseline.
3. Merge, run live adoption or trigger a full multi-corpus evaluation only under separate authority.

**STOP conditions:** open-class review findings at round 2; any request to change occurrence indexing, parameter admission, call/receiver ownership or navigation caches; a second environmental retry for Node, Python, Tier-A matrix or quick; any full multi-corpus request without human trigger.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Runtime repair | done | `[MEASURED]` private exact callable scope plus capture and recursive barriers in `src/ast.rs`; CPG cache 93 |
| Fail-first controls | done | `[MEASURED]` 387-row aggregate RED; nested-default, unindexed-generator and example RED logs under custody root |
| Focused behavior | done | `[MEASURED]` focused 9/0/0; desired 1/0/0; namespace 5/0/0; contained 3/0/0 |
| Full Rust | done with inherited baseline | `[MEASURED]` default 4510/0/1; MCP 4703/0/1; owner 4703/23/1 with exact base failure-set hash; examples 32/0/0 |
| Cross-language | done with exclusions | `[MEASURED]` Node 786, authority 40, Python coverage 940 with one live-adoption exclusion |
| Tier-A | invalid | `[MEASURED]` matrix dependency setup failed after one retry; quick timed out at 300009 ms with no report |
| Independent review | done | `[MEASURED]` round 1 APPROVE, 0 WRONG/1 SMELL; no round 2 required; `review-round1.md` |
| Publication | done | `[MEASURED]` implementation commit `0858c23e` pushed; PR #316 open against `main`; merge not performed |

## 3. Corrections to standing documents and memory

| Location | Stale or false assertion | Correction |
|---|---|---|
| 2026-09-13 proof readout | Production repair remains the following slice | `[MEASURED]` historically true for that proof; successor repair is now implemented in this uncommitted checkout, so do not rewrite proof history |
| 2026-09-13 proof baseline | CPG cache 92 and desired test ignored | `[MEASURED]` historical proof receipt remains immutable; repair candidate is cache 93 and desired test enabled |
| Memory | Proof-only boundary | `[INHERITED]` memory is historical and was not edited; this handoff/readout carry current repair state |

## 4. Open work

| # | Work | State | Exact next action | Blocked by | Identifiers |
|---:|---|---|---|---|---|
| 1 | Independent review | done | Preserve round-1 APPROVE and retained non-blocking SMELL | None | 0 WRONG / 1 SMELL |
| 2 | Controller custody | done | Preserve PR #316 and host evidence; do not merge without separate authority | None | `0858c23e`, PR #316 |
| 3 | Deferred semantics | parked | Separate plans for class phases, same-line indexing and broadened admission | explicit owner scope | workstream 3 follow-ups |

## 5. Invariants and traps — do not do these

- Never loosen `compute_param_def_nodes` or `argument_var_node_in_span` — the repair is upstream rvalue ownership only.
- Never convert `NameOnly(CfgIncomplete)` capture edges to Exact by deleting their warning source — capture doubt is preserved.
- Do not remove duplicate call-owner rows in epochs 1–3 — call ownership is separately parked.
- Do not claim the unchanged FullFlow vector as a public taint fix — source/DFG correction and public output are distinct.
- The examples gate initially exposed a stale nested-edge assertion; its candidate green and unchanged-base RED are both retained.
- Tier-A matrix setup failures and quick timeout are INVALID, never green; retry caps are exhausted.
- The Node source-custody module remains an explicit three-test exclusion; never run it without all five authenticated inputs.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Base/main | `9e40a376299a714ed11233a56a9e23554d40ca99` / tree `21e9fc389518e9c7928869baf854bfc6ad1a4054` |
| Checkout HEAD | `53f9d78224794866299f70c4752aae1ef1dc5c41` / same tree before edits |
| Implementation commit | `0858c23e` on `feat/nested-callable-owner-repair` |
| Pull request | `https://github.com/shoedog/prism/pull/316` (open against `main` at publication) |
| Evidence | `/private/tmp/prism-nested-owner-repair-TfBvIW` |
| Proof evidence | `/private/tmp/prism-nested-owner-proof-Rjzipk` |
| Aggregate base RED | `387` rows / `b96b7274a05149039d85672a3fac343ddc84838220fa74f1756c5cf17f83320b` |
| Owner baseline failures | `23` names / `95694aa34ec43f56b0561c3af299f0b70d26aee3d73de1ac124957d10fa245cc` |
| Cache pins | `CPG 93 / navigation 52` |
| Readout | `docs/eval/receiver-closure/2026-09-14-nested-execution-owner-repair.md` |
| Receipt | `docs/eval/receiver-closure/2026-09-14-nested-execution-owner-repair-baseline.json` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "the bounded repair excludes nested execution regions without broadening admission or corrupting preserved ownership/capture paths" · pass: INDEPENDENT · evidence tier: TEST-BACKED · record: `/private/tmp/prism-nested-owner-repair-TfBvIW/review-round1.md`

**Questions the owner owes an answer to:** None for this slice. Merge, live adoption and full multi-corpus authority remain separate.
