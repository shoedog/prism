# Parameter syntax-frequency core acceptance — final round 2 of 2

**CORE APPROVE — 0 WRONG, 0 remaining SMELL.** All four round1 WRONGs and S1 are RESOLVED. No review-cap extension or source-scope expansion occurred. Public measurement and final gates remain pending; this is not final implementation/publication acceptance.

## Exact binding

- Candidate `f79bb95485940bcb1418a2510cd9ec73a65ca37f`, tree `c5c48a54f2b4ba989c9c3ad7d2a4b4f36a5aca29`.
- Specification `a44bf6002b129862a9eb906fab9e9a31074aa017ae62ca5d198a663a9c31d13f`.
- Implementation SHA `d1c77914d4db177af010e52efc2e308eb5bf6a9269ea9c50753dca5b69f66856`; tests `2d930123b847815f18aaf233c2250b5fb1b3afd398af3427803f89164d3cdc56`; unchanged README `43e8374a8c60ecf60e72f01372a03824378ffbcd935abcbc5e3cd0bd9b3d8d13`.
- Each file independently extracted from frozen git objects and authenticated in `authentication.json`. Active executable size429+400=829, within600/600/1200. Shared worktree clean; diff check clean. No repo edits or public source parsing by reviewer.

## Closed findings

| Finding | Status and mechanism-level closure |
|---|---|
| W1 expression bindings leak into outer flags | RESOLVED: restricted recursion follows pattern elements and BindingElement.name only; observes initializer presence without descending initializer/property expression. Permanent complete outer/inner rows and frequency tuples plus saved fixtures pass; genuine nested binding positives remain green. |
| W2 Unicode ordering | RESOLVED: comparison iterates full Unicode code points, including prefix ordering; applied to discovery, file rows and frequency sorting. Saved U+E000/U+10000 reverse-order input and permanent repeat control pass. |
| W3 declaration strata | RESOLVED: declaration flag must equal validated `.d.ts` suffix before parsing; both mismatch directions refuse, valid declaration/nondeclaration controls pass. |
| W4 foreign output deleted on timeout | RESOLVED: child only writes unique run staging; parent publishes by new-only link after successful completion, then cleans staging. Timeout never deletes requested output. Saved timeout race and independent successful-child publication conflict both preserve foreign bytes. |
| S1 selected mutation assertions | RESOLVED: both fixed mutations now deep-compare complete expected packets with only declared identity/span/pattern/count transforms; cold/repeat and exact restore retained. |

## Independent behavioral evidence

- `candidate.log`: **27 passed,0 failed** = complete own21 + unchanged saved reviewer adversarial/custody6. Same pinned TypeScript5.9.3 compiler and synthetic fixtures only.
- `publication-edge.log`: **1 passed,0 failed**. Independent writer creates sentinel after initial output check; worker completes; parent link returns EEXIST (`publish failed`), sentinel and creation marker survive, and own staging is removed. This exercises actual publication conflict rather than only early refusal/timeout.
- `predecessor-control.log`: exact prior `5ff898308947d1c90d683b31c92da704d03cb2a4` implementation with final permanent tests, same environment: **21 tests,16 passed,5 reported failures**. Four concrete repaired behaviors fail; the fifth is their declaration-test parent container, not a fifth defect. No structural/zero-selection/setup failure. This authenticates regression tests for all four repairs.
- Prior observer-feature baseline and mutation evidence remain historical, separately labeled; no native predecessor regression assertion is made. All probe expectations/alternatives/results are in `hypotheses.md`; raw sources/logs retained and hashed in `evidence.sha256`.

## Remaining gates and limits

Core approval permits the authorized exact414-file public cold/repeat census next. Final acceptance still requires independent full output/custody/count reconciliation, complete active Node/default Rust and required CI gates, explicit exclusions and durable readout/receipt. Those gates were not run in this core review. Syntax-frequency observation conveys no native parameter support, Program/type-check validity, runtime readiness, accuracy or semantic demand claim. No Tier-A trigger is introduced by this script-only change.
