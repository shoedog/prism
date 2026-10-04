# S2 decisions and controller follow-through

## Owner decision S2-O7 (2026-10-04)

**“Static-binding contract.”** Exact grades static binding. [CLAUDE.md](../../../../CLAUDE.md#navigation), lines 233–239, excludes runtime module mutation for every rung. S2-O7 **supersedes S2-O6's runtime-mutation scope**. Runtime re-acquisition F3, namespace enumeration F4, evaluated/reflected codegen F7, including the R1b [].filter.constructor WRONG under the former contract, are out of model by owner decision and disclosed. This is a scope change, not a runtime-safety proof or a mechanism-based downgrade of the old finding.

Visible static aliases/member writes remain in model and keep base under the original whitelist. Selected R2 candidate: c35719e1 + F1/F2/F5/F6/corrected F8, no F3/F4/F7 global cuts. Combined yield132/132/0/0 and final worker gates complete; see REPAIR-R2/HANDOFF/VERIFICATION for current measurements. Old R1b STOPs are historical option costs, not present authorization gates. R2 stops only if combined X <122 or a correctness/input gate fails.

## Superseded owner decision S2-O6 (2026-10-03)

“Ship X gain, fail-closed.” Its expanded runtime-closure scope is superseded. The conservative lexical whitelist is retained. S2-W1 static Alias=C; Alias.sm=replacement remains an in-model refusal; tests preserve it.

| ID | Current answer / remaining work |
|---|---|
| S2-O1 | Controller interim: captured separate S2 relative envelope; pending adoption confirmation; lane-P unchanged. |
| S2-O2 | Controller interim: two hops per leg, up to four composed; pending adoption confirmation. |
| S2-O3 | T stays base; ownership increment separately authorized. |
| S2-O4 | Inherited controller F base aggregates: 974 low /32 callable-low; 26 instance +six default function members; zero static-class rows; 1,271 UNJOINABLE. Not reverified by planner. Private head wrapper remains controller-only. |
| S2-O5 | Positional mutation survivor disclosed as coverage SMELL; authoritative registry remains controller obligation. |

No private F read, Git writes, independent review dispatch, publication/merge or adoption by this worker. R1 review used; cumulative review cap two, R2 next. Tier-A quick skipped/unverified per repair-r2-brief, not a green result.
