# Handoff — bounded callable acquisition

**Written:** 2026-09-06 · **By:** /root · **Provider:** codex
**Workspace:** /private/tmp/prism-acquisition-next-NoX18k/profile · feat/callable-installed-profile · **Measured state:** `[MEASURED]` parent0ea180d4; explicit-profile implementation in progress.
**Predecessor:** PR266, confirmed merged at d4f06b58b11ed09f46a80f769a94764dafa5ddf0.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

(a) /root only; isolated from dirty original — RESOLVED.
(b) Implementation committed0ea180d4; publication next — OPEN publication.
(c) Full observer/default/MCP gates green; no install — RESOLVED verification.
(d) Owner: “merged. next 3 slices approved. merge when green or stack”.

## 1. Resume order

1. `git -C /private/tmp/prism-acquisition-next-NoX18k/profile status --short --branch`.
2. Inspect task-root slice2-node-final.log, slice2-cargo.log and slice2-mcp.log; all complete and green.
3. Publish slice1, then explicit profile, then links/public replay per the spec.

**STOP conditions:** no new installs/private acquisition; no quiet default-limit
increase or runtime authority; two self-review rounds per slice, classify at cap.

## 2. State ledger

| Item | State | Evidence / correction |
|---|---|---|
| Merge/isolation | done | `[MEASURED]` PR266 merged; original dirty files preserved |
| Sequence/spec | done | callable-bounded-acquisition spec; three increments, no runtime changes |
| Streamed inventory RED | done | `[MEASURED]` slice1-red-fixed.log: 5 tests, 2 pass, 3 fail on extracted eager implementation |
| Streaming and verified lazy read | done | `[MEASURED]` 90 observer,4017 default Rust,4207 MCP passed;0 failed; Rust1 ignored each; readout records controls |
| Larger explicit profile | done | `[MEASURED]` RED4 fail; final96 observer,4017 default Rust,4207 MCP passed;0 failed; Rust1 ignored each; defaults unchanged |
| Canonical links/public replay | next | acquired source retained at prior task root; no install needed |

## 3. Corrections to standing documents and memory

PR266 is merged. Its statements that implementation needs approval are historical:
the owner now authorized these three bounded increments and green merge/stack.
No memory edits authorized. The old eager inventory is safe as an immutable buffer
source but prevents larger-tree compatibility; lazy-read tests specify new needs,
not an attribution that the old implementation served mutated bytes.

## 4. Open work

Finish all three increments and publish/merge green heads. Evidence logs live in
/private/tmp/prism-acquisition-next-NoX18k outside the audited application.

## 5. Invariants and traps — do not do these

- Never repeat the approved install: it already succeeded once.
- Never equate an unproven reproduced packet with closure or class authority.
- Large Buffer/object assertion diffs can stall RED reporting; assert the storage
  discriminator first. Initial stalled probe was stopped, not counted as evidence.
- Host gh auth-status failure is environment-local; escalated PR query succeeded.
- Full inventories plus verified reads are not an atomic filesystem snapshot.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Task root | `/private/tmp/prism-acquisition-next-NoX18k` |
| Public acquired source | `/private/tmp/prism-acquire-w2FtSq/source` |
| Compiler | `/private/tmp/prism-imported-alias-O4d6E1/package/lib/typescript.js` |
| Profiles | `/private/tmp/prism-callable-authority-98TLLN/public/profiles` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "larger profile selection is explicit and independently validated" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: slice2-red.log, slice2-node-final.log; two rounds

**Questions the owner owes an answer to:** None.
