# Imported receivers: review and evidence

Base: `350cc89f686705e28745c9abeb7b76e1c58ee8fc` (merged #238).
Evidence: `/private/tmp/prism-imported-receivers-sDszwH`.
Three rounds declared before work; self-review only, NOT INDEPENDENT.

## Hypothesis / probe / result ledger

| Probe | Expected / alternative discriminator | Observed |
|---|---|---|
| Exported identity | Missing raw Class fact versus missing parsed declaration | `slice1-red.log`: empty export facts; source contains class under export wrapper; both export extraction and clean class index omit wrapper |
| Imported receiver | Same-file owner ceiling versus no reaching binding | `slice2-red.log`: positive imported recovery fails; corrected owner route and lexical import syntax pass `slice2-green2.log` |
| Python regular package | Parent initializer fence versus absent child class | `slice3-red.log`: inert initializer positive fails with indexed clean child; `slice3-green.log` passes positive/negative matrix |
| Live class writes | Class proof retains rebound name versus imported route bypass | `class-write-red.log`: same-file failure too; source count handles Python assignment only; `slice2-review1.log` enumerates sole initial matrix failure |
| Slot and barrel boundaries | Visible slot overrides / omitted class conflict versus duplicate method IDs | `review2-red.log`: five slot-shape cases ultimately checked; distinct-line methods avoid FunctionId collisions; barrel false Exact confirmed |
| Failed-import fallback | Wrong export identity versus later global fallback | `barrel-route-probe.log`: barrel absent from callable table but edge remains Exact `FreeSingle`; terminal failed-import route removes it |
| Parity | Stale unchanged importer versus valid refreshed proof | `parity-green.log`, final full suites: source cache reload, defining-file changes, initializer changes, full/incremental call-site parity and actual navigation incoming-edge sidecar roundtrip |

## WRONG findings, bounded fixes

1. WRONG — `class Client { m() {} }; Client = Other` followed by a typed or
   constructed receiver still gets Exact `Client.m`. Visible class-binding writes
   now revoke owner proof; writes to lexical shadows preserve the original owner.
2. WRONG — a same-name instance field, accessor, or constructor `this.m` assignment
   can replace the method slot while `x.m()` still targets the declaration Exact.
   Persist per-method slot exclusions and consult them with clean owner proof.
   Unknown computed members/writes are conservative barriers; unrelated and static
   fields preserve direct instance methods. Runtime reflection is not modeled.
3. WRONG — a barrel exporting a class and a function under the same name previously
   omitted class identity and selected the function. Keep class identity during
   conflict resolution, excluding it only from callable export projection.
4. WRONG — after an eligible unshadowed JS member import fails lookup, global
   `FreeSingle` can still select an unrelated function. That failed import is now
   terminal. Imported class names cannot become callable free-function authority.

Each has same-environment base RED in `base-final-red.log`. Base source custody
was compared against git objects (`control-custody.log`); only test modules were
transplanted. Base totals: 1,313 passed / 14 failed / 1 ignored across library,
integration and TypeScript targets. These failures include positive scope increments,
both cache pins, cache/served parity and the old TypeScript unsupported-import test
updated to the approved positive contract; they are not 14 independent defects.

The first custody shell probe reused zsh's special `path` variable and could not
find its commands; it was inadmissible, not a source/environment finding. The
corrected task-specific variable probe produced the matching hashes/bytes above.

## SMELL / limitations

- Constructor recovery reuses AST lexical walks; avoid scanning every import's
  mutation evidence for each constructor by filtering to its spelling first.
- Excalidraw `0642e72cfa2d9a71198200e52f37399384610ee3` and Django
  `e8cff2921ba169d806a15de18304f431f12700f4` bounded syntactic samples establish
  exclusions, not a measured coverage/recall gain. Do not generalize old metrics.
- The navigation skill's server snapshot warned of staleness. Its caller map was
  used only for orientation; current source and executable tests are authoritative.
- Python executable package initializers and JS reflection/arbitrary monkeypatching
  remain outside the bounded static proof. No runtime-soundness claim.

## Convergence

Round 1: raw/clean exported identity and imported positives; class-binding write
repair, complete negative enumeration before retry. Round 2: finite slot/barrel
findings above, then discriminating failed-import fallback probe and bounded fix.
Round 3: preservation controls, computed-slot negatives, Python ancestor chain,
same-environment final base and full-gate verification. No review restart or cap
extension. Final gate results are recorded in the living handoff.

Final gates: default 3,726/0/1; MCP 3,916/0/1; format/check/Clippy pass with
nonfatal warnings. Matrix 104/104, no regression/flip candidates. Quick completed
with oracle/SUT errors 0/0 and matrix 104/104; exit 2 solely for corpus SHA drift
against pinned `20c8490591a3`, with four stale adjudications. Reports preserved in
`/private/tmp/prism-imported-receivers-sDszwH/tier-a/`, not committed as a new baseline.
