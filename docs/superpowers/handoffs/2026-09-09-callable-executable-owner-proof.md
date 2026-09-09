# Handoff — bounded executable-owner proof design

**Superseded checkpoint:** PR295 merged as `4e88d33b`. The owner approved the
detached constructor, now implemented in the [current lane handoff](2026-09-09-detached-owner-constructor.md).
The remaining text is the historical PR295 closeout, not current PR/implementation
state. Its measured receipts retain their original HEADs. No production wiring is shipped.

**Written:** 2026-09-09 · **By:** /root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/slicing · docs/callable-executable-owner-proof · **Measured state:** `[MEASURED]` base7040ceb63c77142793ba08fed5946e6e54f29a86; design/fixtures committed and pushed as e1355b40, PR295 open. All seven local gates passed on that clean stable HEAD; closeout adds documentation/receipts only. No production authority changed.
**Predecessor:** merged S10 production-authority contract, PR293.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff > historical snapshots.
**Provenance:** written live; claims below distinguish measured evidence from planned acceptance.

## 0. Gating facts — settle these before starting anything below

(a) Lane ownership — RESOLVED: primary owns design and verification; no delegated work.
(b) Custody — RESOLVED: design/fixtures and compact evidence receipts in Git/PR295; previous stack branch retained unchanged. Raw evidence `/private/tmp/prism-owner-proof-xutDxX`, archived at `/private/tmp/prism-owner-proof-xutDxX-evidence.tgz`; hash in gate receipt. No private artifacts published.
(c) In flight — local verification complete; remote CI started for PR295, not claimed green. No production changes or further merges authorized by this slice.
(d) Authority — owner approved "bounded executable-owner proof design--one direct contextual TS/TSX receiver fixture plus refusal cases--before production wiring. Closure barriers and the unresolved react-scripts decision remain unchanged."

## 1. Resume order

1. Refresh PR295 and local status before further work; do not confuse this design PR with a shipped production consumer.
2. Read the [design](../specs/2026-09-09-callable-executable-owner-proof.md) and [readout](../../eval/receiver-closure/2026-09-09-executable-owner-design.md), including the exact admitted grammar and unimplemented acceptance requirements.
3. If separately approved, implement the detached constructor/new direct facts first, with per-predicate RED and genuine-epoch substitution tests. Production wiring, lifecycle/cache and public consumers remain separate gated work.

STOP on a required closure waiver, installation/private-repo mutation, unsupported executable-owner assumption, or production-wiring requirement. At review cap, classify findings before extending.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Base | done | origin/main7040ceb6, previous branch preserved |
| Navigation orientation | done | Prism resolve_call_site_full callers returned20/242 with StaleIndex; LSP tools unexposed; verify source directly |
| Positive candidate/control | characterized | compiler complete, absent imported Handler edge; inline/explicit controls resolve intended owners |
| Constructor design/refusals | complete at design boundary | eight required predicates, exact epoch ownership, index/Program census and cache bypass; no constructor implemented |
| Rust characterization | passed | one test, 108 TS/TSX full/subset cells; baseline controls, not new-route RED |
| Compiler/public characterization | passed | 54/54 baseline; future mode fails exactly four intended cells; earlier setup failures inadmissible |
| Full local gates | passed | clean e1355b40:591 observer,4018 Rust,4208 MCP,18 helpers,40 authority; one existing ignored per Rust run, doctests/fmt/diff pass |
| Publication | open PR295 | GitHub connector created PR; CI started for e1355b40. Publication is not CI success or merge authority |

## 3. Corrections to standing documents and memory

S10's separate owner design decision is now granted; production implementation is not. No memory edits. Historical source/CI receipts remain tied to their original HEADs. Prior sequence/merge stores now explicitly mark all PRs merged and point here. Direct-body props_class is absent: new provenance derivation is required. Compiler closure does not cover extra Prism-indexed source; the new route must bind both censuses.

## 4. Open work

| Work | State | Next action |
|---|---|---|
| Approved design slice | done | retain receipts and local evidence archive; no production implementation claim |
| Remote CI/review | pending externally | refresh exact PR head/checks; owner controls merge |
| Detached constructor | recommended, not implemented | obtain next bounded approval; require new direct derivation and epoch/negative tests before wiring |

## 5. Invariants and traps — do not do these

- No React/FC name authority, class guess in pre_resolved_target, closure waiver or react-scripts installation.
- Compiler type/member declarations and old props_class anchors are observations, not executable-owner proof.
- Baseline-passing refusals are controls, not newly captured behavioral RED; future intended positive must be absent at the public seam on base.
- Keep fresh/incremental/full/subset/navigation/CPG/cache obligations explicit; design must not pretend one helper ships the consumer.
- The binary is named prism, not slicing. LSP is unavailable; stale/truncated Prism navigation is orientation only.

## 6. Identifiers

Base7040ceb63c77142793ba08fed5946e6e54f29a86. Tested e1355b40bf0ba5aa30e9c9506bf504a8b588a2bf. Compiler `/private/tmp/prism-imported-alias-O4d6E1/package/lib/typescript.js`, expected SHA3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675. Evidence `/private/tmp/prism-owner-proof-xutDxX`. [PR295](https://github.com/shoedog/prism/pull/295); [gate/archive receipt](../../eval/receiver-closure/2026-09-09-executable-owner-design-gates.json).

## 7. Refutation verdict and owner questions

**§2c verdict:** SELF-PASS at round2/2 (NOT INDEPENDENT), no open WRONG, confidence95/100. Round1's contradictory future-test expectation was fixed and the final complete population rerun. Claim: "one bounded imported contextual receiver has a constructible future static-owner opportunity and explicit proof requirements", not an implemented proof. Evidence tier: compiler/public-CLI-backed characterization plus full Rust/MCP gates. Production construction/consumption/substitution/lifecycle correctness is not claimed.

**Questions the owner owes an answer to:** None within the design-only boundary.
