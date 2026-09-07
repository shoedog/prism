# Handoff — bounded callable acquisition

**Written:** 2026-09-07 · **By:** /root · **Provider:** codex
**Workspace:** /private/tmp/prism-acquisition-next-NoX18k/links · feat/callable-canonical-links · **Measured state:** `[MEASURED]` eb5367a2; all three implementations committed, final verification closeout in progress.
**Predecessor:** PR266, confirmed merged at d4f06b58b11ed09f46a80f769a94764dafa5ddf0.
**Truth ordering:** measured live state > explicit owner/contract authority within its scope > this handoff for current operational state > earlier handoffs and non-authoritative summaries. A conflict between tiers stays OPEN in §0 — never resolved by document class alone.
**Provenance:** written live by the worker. `[MEASURED]` claims were probed by this writer; `[INHERITED]` claims were not.

## 0. Gating facts — settle these before starting anything below

(a) /root only; isolated from dirty original — RESOLVED.
(b) PR267/268 published; slice3 committed, publication next — OPEN publication.
(c) Observer109/109 and public replay validated; final-head Rust reruns completing;
PR267 CI test job pending — OPEN verification. No install or app writes.
(d) Owner: “merged. next 3 slices approved. merge when green or stack”.

## 1. Resume order

1. `git -C /private/tmp/prism-acquisition-next-NoX18k/links status --short --branch`.
2. Inspect task-root slice3-cargo-final.log and slice3-mcp-final.log for complete doctests and totals.
3. Publish slice3 on PR268. Merge only exact green heads; retarget dependents to
   main in order after parent merge and wait for their own CI (stacked-base PRs
   do not currently report checks). Owner explicitly permits leaving a stack.

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
| Canonical links/public replay | done | `[MEASURED]` 109 observer passed;249 links,853674175 regular-file bytes; valid reproduced unproven packet; canonical-links readout |
| Publication | pending | `[MEASURED]` PR267 on main; PR268 on267; third slice awaiting final gate closeout |

## 3. Corrections to standing documents and memory

PR266 is merged. Its statements that implementation needs approval are historical:
the owner now authorized these three bounded increments and green merge/stack.
No memory edits authorized. The old eager inventory is safe as an immutable buffer
source but prevents larger-tree compatibility; lazy-read tests specify new needs,
not an attribution that the old implementation served mutated bytes.
Acquisition admission now succeeds only under explicit installed/in-root options.
Earlier “missing React types” does not explain the installed-tree result: the
signature is anchored in installed declarations, but qualifier provenance stops
at ambiguous_declaration. No barrier may be removed on that evidence alone.

## 4. Open work

Finish final verification and publish/merge green heads or leave the approved
dependent stack. Evidence logs live in
/private/tmp/prism-acquisition-next-NoX18k outside the audited application.
Next recommendation, not a fourth implementation: source/compiler-backed audit of
the React qualifier's actual declaration population and ambient/global identity.

## 5. Invariants and traps — do not do these

- Never repeat the approved install: it already succeeded once.
- Never equate an unproven reproduced packet with closure or class authority.
- Large Buffer/object assertion diffs can stall RED reporting; assert the storage
  discriminator first. Initial stalled probe was stopped, not counted as evidence.
- Host gh auth-status failure is environment-local; escalated PR query succeeded.
- Full inventories plus verified reads are not an atomic filesystem snapshot.
- TextDecoder must preserve a BOM in link target filenames; captured WRONG fixed.
- Canonicalize the root in pinned-compiler comparisons: /var versus /private/var
  yields different symbols before any package-link conclusion can be drawn.
- RSS was not measured: time -l sysctl refusal is not memory evidence.

## 6. Identifiers

| Item | Verbatim |
|---|---|
| Task root | `/private/tmp/prism-acquisition-next-NoX18k` |
| Public acquired source | `/private/tmp/prism-acquire-w2FtSq/source` |
| Compiler | `/private/tmp/prism-imported-alias-O4d6E1/package/lib/typescript.js` |
| Profiles | `/private/tmp/prism-callable-authority-98TLLN/public/profiles` |
| Public packet | `/private/tmp/prism-acquisition-next-NoX18k/public-packet-final.json` |
| Packet SHA256 | `0c4acbb619db405af129588cd7fe992945de1c6d93ce03c5a423570752c52a04` |

## 7. Refutation verdict and owner questions

**§2c verdict:** SURVIVED · claim: "bounded canonical acquisition admits the unchanged public tree without granting closure or runtime authority" · pass: SELF-PASS (NOT INDEPENDENT) · evidence tier: TEST-BACKED · record: 2026-09-07-callable-canonical-links readout; two rounds, BOM WRONG corrected

**Questions the owner owes an answer to:** None.
