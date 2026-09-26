# Spec round 3 (sol, final; FIX 2 WRONG / 1 SMELL): at-cap fold record

Round 3 was the owner-approved final spec round. Its findings were bounded and converging (sol's own convergence
view). The owner decided the dispositions (SPEC §0 D10–D11), and the folds below are the **disclosed extension at the
cap**. There is no round 4: implementation starts directly, and sol reviews the implementation.

| # | Finding (sol r3) | Disposition | Where in the packet | Evidence |
|---|---|---|---|---|
| W1 | The span and JSX gates protect only R4c `import_member`. R3 `ImportQualified` (namespace `<Lib.Island/>` → 2× Exact including a nested decoy) and R4 `LocalDef` (producer-local non-JSX `Island({})` → Exact) bypass them. Pre-existing on base | **Deferred to S1b (owner D10).** S1's contract is narrowed to the new R4c `import_member` route. S1b is widened to "span-verify all JS/TS export routes": the D4 list/default `Local` false Exact, the R3 namespace decoy, and producer-local R4 for wrapped targets. Sol's two inputs are S1b's first RED cases, recorded as controls C62 and C63. No R3 or R4 code in S1 | SPEC §0 D4/D10, §2, §3.2, §3.4 (Coverage), §4, §12 | C62 and C63 are byte-identical on base and S1 (P31) |
| W2 | R6-P4 misses module-scope `var` declarations nested in top-level control flow (`if (flag) { var memo = fake; }` → a false Exact under Branch P) | **Folded (owner D11).** R6-P4 is one recursive walk from the root: it counts top-level and exported declarations plus `var` declarators and `for (var …)` heads hoisted out of blocks, loops, `switch` and `try`; it stops at nested functions, classes, and non-top-level `let`/`const`. Adds T-R6-P4b (C57–C59, one `callee_provenance` each), positive twins (C60, C61, C67), T-R6-P4 as controls (C64–C66), and mutant M14 (root-only walk) | SPEC §3.1 P4, §4, §7 (T-R6-P4, T-R6-P4b, M14), §9 | P30–P34: all 67 controls as expected; M14 kills on C57 and C59; 107/4 unchanged; Tier-A 159/159; 4,559 / 0 / 1 |
| SMELL | The RED rule included T-J4, a base-green preservation control | **Folded (owner D11).** Mandatory JSX-gate RED evidence covers T-J1–T-J3 only. T-J4 is classified as a preservation control. The RED rule now also states that MB* tests are characterizations, not refusal RED | SPEC §7 (RED rule, T-J4) | – |

## Budget consequence (disclosed)

- **First W2 fold breached the cap.** Implemented as a second, separate walk, it measured **357 src** (P30), above the
  350 cap before the cache bumps.
- **Replaced, not compressed.** It became a single shared walk (sol's "share one walk" alternative, and the simplest
  correct one): **328 src**, with identical results on all 67 controls and on the corpora. The forecast with the cache
  bumps is about 332 against 350.
- **Tests are the tightest bucket.** The forecast is about 585 against 600; the IMPLEMENTOR checkpoint at 540 applies.

## Prior-round closure, as sol recorded it, and after the fold

- r1 W1, r2 W1, r2 W2, r2 W3: OUT OF MODEL (MB1–MB3). Unchanged.
- r2 SMELL 1 and 2: CLOSED. Unchanged.
- r1 JSX-only condition: sol marked it NOT CLOSED globally. It is now **CLOSED as scoped**: the contract is explicitly
  the R4c route (D10). R3 and R4 are recorded S1b scope.
