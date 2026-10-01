# Lane P owner questions

READ: D5 approved this planning lane, Exact is static binding, Option K preserves base on unproven positions, and numeric LOC caps are abolished. The answers below remain placeholders; recommendations are ASSUMPTION, not owner rulings. Prototype work is already authorized by the planning brief.

| ID | Question | Recommendation (ASSUMPTION) | Alternative / tradeoff | Owner answer |
|---|---|---|---|---|
| OQ1 | Approve P1’s finite scope and conservative cuts: Node10 configs, singleton paths substitutions, exact/single-wildcard keys, local single-parent extends, named/default ESM members; tied patterns, candidate competition, package boundary and unsupported syntax preserve the complete base row? | Yes. MEASURED: 3,121 correct X edges, 99.68% of the oracle’s 3,131 callable alias candidates; R/T 0. | Implement complete TypeScript resolution now: ordered substitutions, NodeNext, package exports and references add separate precedence/ownership proof surfaces with 0 measured public alias yield. | __OWNER_OQ1__ |
| OQ2 | For “nearest tsconfig that includes the file,” is membership the parsed root-file set (`files` plus `include` minus `exclude`, with files overriding exclude), rather than transitive program membership? | Root-file membership for P1, as explicitly demonstrated by TypeScript `getParsedCommandLineOfConfigFile().fileNames`; closer excluded configs allow the next including ancestor, while closer invalid/unsupported configs preserve base. | Program membership includes imported excluded files and potentially several projects with differing options. Requires project ownership disambiguation and a separate measurement; do not silently substitute it. | __OWNER_OQ2__ |
| OQ3 | Must fresh F aggregates be folded before P1 implementation dispatch? | Yes. Controller runs the provided script; fold only aggregates. Current F projection is READ from the brief and unmeasured. | Dispatch using public evidence first, explicitly leaving F acceptance open. Saves controller latency but cannot certify the private corpus. | __OWNER_OQ3__ |
| OQ4 | After P1, should P2 be parked unless F supplies material incremental yield? | Yes. Public MEASURED yield is 0 for bare baseUrl, non-relative .js→.ts, references and package/workspace routes. Re-measure each future mechanism before authorizing it. | Authorize a general resolver lane now for API completeness, with broader cost and review surface despite no public measured gain. | __OWNER_OQ4__ |

READ: no question asks to redefine Exact or to reopen Option K. READ: cache numbers on this bound base are CPG 103 and nav 59; prototype uses 104/60. If the implementation parent changes, bind that revision, use its actual versions +1 and re-run the controls and corpus comparisons. No stale approval transfers across that change.

## Controller interim dispositions (2026-10-01; the owner is asleep, so the owner answer slots above stay open)

The owner authorized overnight orchestration, including scope cuts and splits. The positions below let planning and review continue. Each one is **pending owner confirmation**, and none redefines Exact or Option K.
- **OQ1: proceed with the recommended finite P1 scope.** It is a scope cut that preserves base on everything outside it, so no new accepted cost.
- **OQ2: proceed with root-file membership,** as TypeScript defines it.
- **OQ3: yes.** The controller runs `CONTROLLER-paths.sh` and folds the F aggregates before the P1 implementation dispatch.
- **OQ4: P2 is parked** until a re-measurement shows yield. That is a scheduling position; the owner decides any later lane.
