# JS/TS/TSX exact parameter-token argument binding

Step 5b now selects only an existing, supported parameter-token Def for JS/TS/TSX.
A default/optional slot without such a Def stays unbound; a same-name assignment
or alias-resolved declaration in the body can no longer substitute for it.
The shared parallel/serial selector retains original slot positions and validates
the graph node's complete owner, path, access and UTF-8 token endpoints. Missing
or ambiguous AST owners and missing/substituted nodes refuse. CPG cache 80→81;
skip-policy and navigation call-edge versions remain unchanged.

No parameter occurrence syntax is added. Non-JS normalization/body lookup,
caller field/base supplementation and confidence propagation are preserved.
No React.FC, closure admission or unresolved react-scripts decision changes.

## RED-first proof and alternatives

The final pre-repair nine-test run had 3 passes / 6 failures; controls already
passing are not RED regressions. Cache pin and native-observer contract each
failed separately. A supplementary frozen-base duplicate-owner fixture produces
two erroneous edges (captured assertion failure); the candidate produces none.
The final focused suite, including that same-name/same-line owner regression,
passes 10 across JS/TS/TSX; cache pin 1 and all 6 observer tests pass.
The old observer test characterized the defective fallback and is explicitly
changed to demand its absence; observer production code is unchanged.

| Hypothesis / alternative | Discriminating evidence |
|---|---|
| Step 5b selects a body Def; alternative: wrong callee or colliding caller occurrence | Resolved call fixtures retain the actual callee/body definitions, with calls on separate lines; exact graph endpoints are body tokens before repair and absent afterward. Required parameter controls retain exact token endpoints. |
| Entry-line lookup alone is sufficient; alternative: a different token shares the key | Same-line default/body assignment and each graph-node field substitution fail before repair. Exact-byte/owner checks reject all substitutions afterward, including a genuine other-owner parameter node. |
| Unsupported slots may be removed from the vector | A supported last parameter following a default remains bound to the original third argument; the middle body overwrite receives no edge. |
| Missing entry Def permits body fallback | Removing only the entry index leaves a later same-name body Def; the old binder selects it and the repaired binder refuses. |

Fixtures cover JS/TS/TSX defaults, overwrites, var redeclarations, alias-resolved
locals, same-line targets, multiline Unicode, duplicates, recovery/destructuring,
TS/TSX optionals and parallel/serial parity. Malformed refusal fixtures are not
claimed compiler-valid. Existing Go/Python normalization control passes both
before and after repair. Parameter producers and line-anchored Uses are unchanged.

## Fixed-population value checkpoint

The unchanged production observer/comparator was replayed on the same immutable
source copies. The prior loop-repair candidate's production is byte-identical
to merged base b2b141cd; only the observer's test contract changes here. File
hashes, loader skips, parse status, function inventory and parameter/slot shapes
match. A separate multiset check proves every removed edge is exactly one of
the previously source-classified non-loop body-target observations.

| Observation | Public Excalidraw before → after | Authorized private frontend before → after |
|---|---:|---:|
| Loaded files / skips | 628 / 584 unchanged | 1,122 / 140 unchanged |
| Functions | 8,614 unchanged | 12,713 unchanged |
| Parameter-token DFG and CPG Defs | 3,892 unchanged | 1,676 unchanged |
| Slot-token-matched Use→Def flows | 6,016 unchanged | 2,420 unchanged |
| Unmatched Use→Def flows | 3 → 0 | 61 → 0 |
| All observed Use→Def flows | 6,019 → 6,016 | 2,481 → 2,420 |
| Added / removed flows | 0 / 3 | 0 / 61 |

The removed flows target local declarations (3 public / 2 private) or later
assignments (59 private), not parameter tokens. No legitimate body Def is removed;
the repair removes only incorrect argument edges to those nodes. Raw private
source, identities and graph rows stay local; only aggregates/custody metadata
are published. These are graph observations, not unique calls, runtime recall,
compiler-verified accuracy or new receiver/executable authority. Zero unmatched
flows does not prove every retained slot mapping is semantically correct or
every real call/parameter is represented.

## Verification and custody

Full Rust/Node gates started and ended on clean, unchanged checkpoint
`103677308b46332cdab13ce292e0086460aaa657`. The implementation is cc289999;
the later checkpoint adds only one regression and handoff updates. Subsequent
closeout changes are documentation only.

| Gate | Result |
|---|---|
| Rust default / MCP / MCP+detached-owner-audit | 4,142 / 4,335 / 4,358 passed; one existing ignore each |
| Observers / receiver helpers / authority controls | 726 / 18 / 40 passed |
| Formatting / diff | passed |

Rust's existing ignore is `resolution_test::slice_elem_variant_reserved`.

Independent implementation review, round 1 of cap 2 at 10367730, returned APPROVE,
0 WRONG / 0 SMELL. Full Rust/Node gates were pending during that review and
are recorded separately above. Round 2 independently approved documentation
closeout at `77e004b89ecdc87a6b7415a25b64c3dc9971818b`, with 0 WRONG / 0 SMELL.
The subsequent review/publication-status record is self-checked, not a third round.

Python passed 940 with one intentional live-adoption skip; all 32 native example
tests and 10 comparator tests passed. Clippy completed with warnings; this is not a
warning-free claim or an attribution of every warning to the unchanged base.
Those additional checks used cc289999 before the supplemental test-only commit.
The separately inventoried grammar-bootstrap and asserted-member policy tests
passed 10 on 10367730 without skips. Bootstrap negatives reused already-pinned
local archives; no dependency download or install was performed.

All 159 Tier-A matrix cases passed after fresh builds, including after the final test
addition. Base and candidate quick runs are both INVALID: corpus SHA drift from
pinned 20c8490591a3, C-method/C-name/U-method each 4/6 successful probes, and oracle
error rate 0.20 above 0.10. SUT error rate 0 on both. Candidate ran on clean cc289999;
base b9c8c59d had one untracked test source. The final owner regression was added
after candidate quick; production code is unchanged. Pending rows 64→76 are not
a matched accuracy comparison or evidence of a regression. No baseline or
adjudication was changed.

Both quick runs have the same pinned outcomes: target-c-method flip candidate;
module-deps-feature-gated and load-repo-feature-gated missing expected oracle-miss
sites; ambiguous-symbol-contract ok. The verification receipt retains compact
pinned differences and hashes of the raw local reports. No full multi-corpus,
live-model or compiler-Program accuracy evaluation was run.

Prism navigation traced the affected Step 5b consumers but reported 41 stale paths;
current source established the shared endpoint-selection scope. No LSP-resolved
proof is claimed. Private raw evidence remains local.

## Next recommendation

With the demonstrated body-target defect repaired, assess the bounded value and
proof requirements for optional/default identifier parameter occurrences before
adding syntax support. Keep positional argument binding separate from default
initializer value flow (an omitted argument is not an observed explicit argument).
The unchanged syntax inventory contains 194 optional/249 default identifiers
publicly and 0 optional/259 default identifiers privately. These are syntax
entries, not yet a count of recoverable argument edges or real receiver sites.
Use real-site classification and refusal fixtures; do not infer recall from the
absence of unmatched graph endpoints. Exact/member read provenance, named-arrow
coverage and same-line occurrence identity remain separate work.

See the [verification receipt](2026-09-10-js-ts-exact-parameter-binding-verification.json).
Local verification archive: `/private/tmp/prism-exact-param-10367730-evidence.tgz`,
32,891,223 bytes, mode0600; SHA256
`12f4c872c7132d4b2997b743ee2f3af15e936b97827aecf001ee1e75201eb944`.
Gzip integrity checked. Raw archive is private and must not be published.
Publication is blocked by command policy: the authorized `git push -u origin
fix/js-ts-exact-parameter-binding` was rejected because approval is required but
the session is set to Never. No alternate publication route was attempted. The
branch and final closeout commits remain local; owner-run push is needed before
PR creation. No automatic merge.
