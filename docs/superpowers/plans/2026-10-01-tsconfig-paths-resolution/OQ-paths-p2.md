# Owner questions — P2

| ID | Question | Evidence and disposition | State |
|---|---|---|---|
| P2-OQ1 | Is the realized yield material? | INHERITED: controller supplied +32 certified rows, native relative ceiling 33. P2 as built is not material; the earlier opportunity-based decision is superseded. | RESOLVED: not material |
| P2-OQ2 | What does this prototype actually recover? | INHERITED: 32 changes, all CORRECT_STATIC_BINDING and JS_EXPORT_HOP; 24 member-written; no key changes. | RESOLVED by controller aggregate |
| P2-OQ3 | Where does the prototype live? | MEASURED: this clone on proto/tsconfig-paths-p2 at e80fbf54; plan predecessor supplied as 1cb46b80. No worker Git writes. | RESOLVED |
| P2-OQ4 | What blocks the remainder? | P2-GAP-DIAGNOSIS and CONTROLLER-p2-gap.sh classify 749 minus 32 = 717 unrecovered rows by first prism gate; per-class private results are not available to the planner. | OPEN: controller-only run |
| P2-E5 | Should member-written terminals bind? | MEASURED: same-environment unchanged P1 local/relative Call and JSX probes pass in both grammars. Both scoped and module write scans collect binding names and ignore member/subscript targets. Bind them with the existing proof; binding writes stay base. Supplied ceilings: top749 with457 /292 without; including43 gives792 with475 /317 without. | RESOLVED: consistent with existing E5 semantics; no new ruling needed |

READ: S6/program-graph ownership and accepted tolerant ambient-scan cost stay settled. No scope expansion to force the43. Current authorization is diagnosis only; any broader production proposal requires the controller partition. Diagnostic correction cap2; independent review not dispatched.
