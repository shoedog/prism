# Handoff — bounded required-path completeness repair

**Written:** 2026-09-07 · **By:** /root · **Provider:** codex
**Workspace:** /Users/wesleyjinks/code/slicing · fix/callable-required-path-completeness · **Measured state:** `[MEASURED]` HEAD8534f4af; dirty bounded implementation/tests/spec. Probe git status and exact-base red-base.log in task root.
**Predecessor:** PR277 merged8534f4af7bafd069728faa4ebbe4d5fc062e3edd, freshly checked this turn.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

(a) Lane ownership: /root only, dedicated fresh branch; no agents dispatched — RESOLVED.
(b) Custody exposure: `[MEASURED]` bounded changes uncommitted; raw RED in task root — OPEN until checkpoint.
(c) In flight: full observer rerun and default Rust gates; no application/install changes — RESOLVED.
(d) Owner: "Proceed to next. Recommend where we fix the two issues the audit found in the next slice or later. authorized to fix in next slice if that is the recommendation". Recommendation: path repair now, type/lib observations and react-scripts disposition separately. Commit/push/PR workflow; no auto-merge.

## 1. Resume order

1. `git status --short --branch` in /Users/wesleyjinks/code/slicing; inspect task-root logs.
2. Inspect observer-final.log/rust-default.log in task root; after default finishes run cargo test --features mcp. Targeted final24/24 already green.
3. Finish full gates, public historical controls, two self-review passes and publication.

**STOP conditions:** no closure admission, runtime/class authority, React.FC expansion,
application/install changes, unrelated custody branches, or review-cap extension without classification.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Exact base | done | `[MEASURED]` detached task-root/base8534f4af |
| RED | done | `[MEASURED]` red-base-final.log24 tests17 failed7 passed; includes reporting/version failures, not17 newly demonstrated false-closure cases |
| Repair | done | `[MEASURED]` green-required-final.log24/24 plus initial combined32/32; former KNOWN WRONG now rejecting regression |
| Public replay | done | `[MEASURED]` packet differs from predecessor only in producer; path-proof.json78/78 actual inclusion proofs; validation.json true/unproven; source unchanged |
| Full gates | pending | `[MEASURED]` helpers7/7, authority40/failures=[]; first observer295/296: stale digest-test list fixed after base1/1 control; full rerun/default in progress, MCP next |
| react-scripts | parked | `[INHERITED]` PR277 source/cache evidence; next type/lib slice, no app edit |

## 3. Corrections to standing documents and memory

PR277 is merged; its known-WRONG characterization is replaced with a rejecting
regression. Historical readout remains base evidence. README/roadmap and predecessor
handoff have successor links; final gate/publication state pending. No memory edits authorized.

## 4. Open work

Finish full gates and historical audit controls, two SELF-PASS rounds, durable
evidence and publication. Targeted repair and public replay are green at this checkpoint.

## 5. Invariants and traps — do not do these

- Do not blanket-refuse failed_lookups; it includes harmless candidate probes.
- Do not substitute lexical presence or incidental Program membership for source/index inclusion.
- Use original redirect bytes, not inherited AST references.
- Parser acceptance of historical schema10 packets is not current validation or closure authority.
- LSP skill fallback uses pinned compiler/source; no LSP tools exposed.
- Cleanup custody and archive/dirty-callable branches stay separate from implementation.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Task root | /private/tmp/prism-required-path-o41JkY |
| Base | 8534f4af7bafd069728faa4ebbe4d5fc062e3edd |
| Compiler | /private/tmp/prism-imported-alias-O4d6E1/package/lib/typescript.js |
| Public source | /private/tmp/prism-acquire-w2FtSq/source |
| Spec | docs/superpowers/specs/2026-09-07-callable-required-path-completeness.md |

## 7. Refutation verdict and owner questions

**§2c verdict:** NOT RUN — implementation checkpoint · claim: "required path failures cannot falsely establish complete closure" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: task-root/red-base.log; two review passes pending.

**Questions the owner owes an answer to:** None.
