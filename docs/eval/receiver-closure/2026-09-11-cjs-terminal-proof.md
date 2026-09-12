# Direct CommonJS terminal identity and capture proof

Fourth local increment, base `032e1824`, bundled with the preceding module-binding
audit, ESM forwarding and CJS producer barriers. No publication performed.

## Diagnosis and bounded repair

WRONG: `function holder(){function origin(){}} exports.item=origin` could
resolve a consumer to the nested function although no module binding exists.
Likewise, a callable initializer after export or a pre-export reassignment could
leave an Exact edge to the wrong capture-time value. Identical consumer controls
and raw/state tests separate producer authority from require extraction.

Each direct identifier RHS now proves one top-level function/generator declaration
with a body, or a direct arrow/function-expression variable initializer completed
before capture. The actual eager callable name and byte span must match uniquely.
Hoisted declarations, async/generators, anonymous/same-named expressions, comments
and direct arrows remain supported. Differently self-named expressions, wrappers,
aliases, imports, competing type/value declarations and nested decoys refuse.

Visible pre-capture and non-root writes revoke proof. Only a plain identifier
assignment in a later root expression statement can be ignored. This preserves
`exports.item=origin; origin=other`, while a later `exports.later=origin` refuses.
Parameters and genuine expression self-bindings remain closer shadows; a function
declaration's self-name is an outer binding. Seven Node wrapper controls confirm
these timing/scope distinctions; they do not model require/cache ownership.

UnprovenLocal records a refusal-only name claim, not callable authority. Tri-state
resolution carries retained unproven/conflicted/rejected-forwarding claims through
named/star paths, preventing a good sibling from being chosen after a bad claim
vanishes. Duplicate CJS writes still revoke the whole CJS callable set, but its
names survive as blocked markers unless independently owned by ESM. Valid callable
siblings of an ordinary noncallable property remain usable. Cache versions: CPG89/nav50.

## TDD and value

Initial valid baseline: 24 passed / 25 failed; first implementation: 49 passed. One invalid JS
type-alias fixture was corrected to explicit TS/TSX before counting results.
Self-binding hardening captured 1 pass / 2 failures before its specialized fix.

Final byte-matched 62 new tests: base **26 passed / 36 failed**, candidate all pass.
The full CJS selection is **151 passed**, including 89 prior controls. Module-binding
controls: 62 passed; ESM-forwarding controls: 46 passed before final review; final full
suites rerun them. Three source-epoch tests cover ten producer-only states through
full/incremental CPG argument-use to exact parameter-token Def flows. Serde replay,
duplicate ordering, per-occurrence captures and barrel refusals are also tested.

Across 294 logged synthetic fixture/language/build observations, 150 Exact targets are
revoked and 144 unchanged; no restorations or other target changes. Conservative
refusals are included: this is neither 150 demonstrated runtime bugs nor a real-site
recall measurement. The adjacent baseline receipt pins test/log/snapshot hashes.

Two independent review rounds found blocked-claim erasure first in resolution,
then in the producer duplicate-set merge guard. At the cap, the latter was closed
and enumerable: three captured RED regressions, localized refusal-marker repair,
one disclosed repair/verification extension. Targeted delta approved W0/S0.

## Verification

Final-source Rust: default **4,418**, MCP **4,611**, MCP + detached-owner-audit
**4,634 passed**, zero failures. Each run has one existing reserved-spec ignore
(`resolution_test::slice_elem_variant_reserved`). Examples: **32 passed**.
Node: 786 passed, no skips; authority: 40 passed; Python: 940 passed, one explicit live
adoption skip. Frozen CLI/MCP/example hashes were unchanged across helper runs.
Fresh-release Tier-A matrix: 159/159. Tier-A quick reached its five-minute cap
(300,022ms) without a verdict: incomplete, not green. Its snapshot/logs are retained.
Three historical imported-props helper tests remain excluded because their pinned
real-sites.jsonl input is absent. No full multi-corpus run or real-repo recall
measurement was requested or performed.
Formatting and CI-style Clippy completed successfully. Clippy warning-category
counts match a same-environment run of the base with the same final fixtures;
no existing warnings were silently fixed or suppressed.
Evidence: `/private/tmp/prism-cjs-terminal-iVeqxl`.

## Remaining boundaries and next recommendation

Next: a bounded TDD audit of rejection/absence propagation for the still-discarded
nonduplicate unsafe CJS producer forms, and the declaration-self distinction at
other existing scope consumers, before require-time forwarding admission. The
current blocked-name guarantee only covers claims retained by extraction; other
unsafe shapes and missing/depth/cycle closure policy are not silently repaired.

Require-time snapshots, shared export mutation, cache replacement, cycles/partial
initialization, forwarding/alias expansion, class/receiver authority and React.FC
remain separate. No closure admission or react-scripts decision change. Prism
navigation was stale/truncated; current source and executable tests supplied the
fallback, not an LSP or complete real-corpus oracle.
