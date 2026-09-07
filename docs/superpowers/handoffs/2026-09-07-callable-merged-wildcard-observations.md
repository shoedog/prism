# Handoff — bounded merged wildcard observations

**Written:** 2026-09-07 · **By:** /root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/slicing · feat/callable-merged-wildcard-observations · **Measured state:** `[MEASURED]` implementation f719b2b, RED edad861 and verification7c42aea pushed on base e45eaef0; PR276 open; all final local gates/replay and two self-review rounds complete. This is the final documentation-only publication note.
**Predecessor:** PR275, confirmed merged e45eaef07e24764597bfbd904aeca83771f97cbb.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

(a) /root only, no agents; dedicated clean-main branch — RESOLVED.
(b) RED/implementation/verification committed and pushed; PR276 open — RESOLVED. Remote CI/review is separate from completed local gates.
(c) All processes complete; final gate totals and raw archive reconciled — RESOLVED.
(d) Owner: "merged, proceed to next" after PR275 and bounded observation recommendation.

## 1. Resume order

1. `git status --short --branch` in /Users/wesleyjinks/code/slicing.
2. Read same-date merged-wildcard-observations spec and task-root red.log.
3. Check PR276 remote CI/review; do not merge without owner direction.

**STOP conditions:** two SELF-PASS rounds, no closure/asset/runtime/class expansion,
React.FC, installs, application/config edits, or unrelated recovery changes.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Base | done | `[MEASURED]` fetch/PR view confirms merged e45eaef0; detached control in task root |
| Design / RED | done | `[MEASURED]` red.log0pass/33fail, every failure is missing new observation field; production unchanged |
| Implementation | done | `[MEASURED]` final43 new tests pass within observer-full-final.log264/0/0; schema10/producer0.11.0 |
| Ordering compatibility | done | `[MEASURED]` order-red.log1pass/1fail: mixed repeated imports changed [7,41] to [41,7]; comparator now preserves complete old evidence before new-field tie-break |
| Historical audit compatibility | done | `[MEASURED]` same environment base accepts schema9 audit; intermediate schema bump invalid_packet; pinned-digest historical reader restores it |
| Final public replay | done | `[MEASURED]` replay-final-evidence.json84 observed, selected Vite empty block84; complete old projection equal; validation-final.json true/unproven; source-before equals source-final |
| Full gates | done | `[MEASURED]` observer264/0/0; helpers7/7; authority40/failures=[]; Rust default4017/0/1,MCP4207/0/1 including doctests; adjacent gates JSON |
| Exact-base control | done | `[MEASURED]` base-final-red.log41 missing-field failures and2 passing ordering controls |
| Self-review | done | Two SELF-PASS rounds, NOT INDEPENDENT; no additional WRONG/SMELL, zero open findings, no cap extension |
| Publication | done | `[MEASURED]` https://github.com/shoedog/prism/pull/276 open against main; branch pushed |

## 3. Corrections to standing documents and memory

PR275 merged; predecessor/roadmap and executable-tool README reconciled. Historical
audit uses its fixed digest independently of schema10. No memory edits authorized.

## 4. Open work

PR276 awaits remote CI/review and owner merge. Recommended next: source-backed closure-policy proof
requirements, not admission; keep14 source gaps/264 refused probes and outside
lookups explicit. No asset, typed/value merge or React.FC expansion.

## 5. Invariants and traps — do not do these

- Read selected `valueDeclaration` from the same checker symbol, never infer it from order.
- Empty/shorthand positive evidence is side-effect-only; `duplicate_provider` stays unchanged.
- Asset presence is not an eligibility input and cannot supply Program closure.
- Original-source census behind redirects is essential to augmentation barriers.
- LSP-navigation skill fallback: no LSP tools exposed; pinned compiler/source checks.
- Test anchors passed to Array.map need a unary wrapper; numeric index is not a SourceFile. Initial defensive-fixture failures were inadmissible, then corrected.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Task root | /private/tmp/prism-merged-wildcard-obs-97JKM5 |
| Control worktree | /private/tmp/prism-merged-wildcard-obs-97JKM5/base |
| Compiler | /private/tmp/prism-imported-alias-O4d6E1/package/lib/typescript.js |
| Public source | /private/tmp/prism-acquire-w2FtSq/source |
| Recovery, untouched | archive/dirty-callable-20260907 / a86bbeb |
| Raw archive | /private/tmp/prism-merged-wildcard-obs-97JKM5/merged-wildcard-observations-evidence.tgz |
| Archive SHA256 | fa6c49f0d8d3b4de914c43f39b21c6d0e782b74fa44e68e2f6689db59cec0857 |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "bounded source-pair observations preserve old lookup and closure policy" · pass: two SELF-PASS rounds (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: observer-full-final.log, base-final-red.log, replay-final-evidence.json, validation-final.json; no open findings

**Questions the owner owes an answer to:** None.
