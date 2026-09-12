# Enumerable CJS refusal and source-self write proof

Historical fifth increment, delivered in `b3f6441e`. The deferred receiver defect
is addressed in [receiver source self bindings](2026-09-11-receiver-source-self.md).

Fifth local increment, base `88af6511`, for the same future PR bundle. No publication.

## Reproduced defects and implementation

WRONG: `module.exports.item=origin; const alias=module.exports;` lost its raw
`item` claim when the producer barrier rejected the file. A barrel exporting
that module and a second callable `item` could incorrectly choose the second
target. Tri-state resolution cannot block a name erased before it reaches the
resolver. Keep already-extracted named/conflicted keys as UnprovenLocal refusal
markers for every rejected CJS producer, not only duplicates. Independently
claimed ESM names remain untouched. No name or callable authority is invented.

WRONG: `function origin(){origin=other}` was treated as writing a closer self
binding rather than the module binding, so ESM forwarded function authority could
survive replacement. Also, `let origin=function(){origin=other}` inherited a
synthetic self binding from its display name, including in the prior CJS proof.
The write-proof mode now uses actual source names only for named expressions,
and module-value-written uses that mode. Parameters, locals, catches, genuinely
named expressions and nested declaration scopes remain barriers. Seven Node
controls distinguish these mechanisms, including ESM live-binding replacement.

This hardens shared module write proof used by ESM/import/class/ambient consumers;
it is not a CJS-only change. General receiver lexical lookup is unchanged. CJS
post-capture rebinding stays supported. Cache versions: CPG90/nav51.

## TDD and value

Initial valid baseline: 10 passed / 23 failed; first implementation: 33 passed.
An initial fixture compilation error was inadmissible and corrected before RED.
The first combined base command stopped after the failing library target; the
complete replay uses `--no-fail-fast`, retaining both outputs.

Final matching tests: 37 integration tests (base 10 passed / 27 failed) and one
unit test (base failed), all passing on the candidate. The CJS selection passes
188 tests including 151 prior controls. The write unit covers 17 scope/write forms
in JS/TS/TSX. Three epoch tests check actual producer-only changed sets and exact
argument-use to parameter-token Def flows in full/incremental builds, across two
five-state sequences: rejected barrel claims and ESM self-write authority. Raw
fact serialization and independent ESM-name custody are tested too.

Across 234 matched synthetic call observations: 138 Exact revocations, 96 unchanged,
no restorations or other target changes. Repeated labels are counted by occurrence.
These include conservative refusals; they are not 138 unique runtime defects or
a real-corpus recall gain. The adjacent receipt pins source/test/log bytes.

Two older raw-fact assertions now accept only absence or a refusal-only marker,
not callable targets. Direct-resolution controls are unchanged; this is the new
raw-fact contract, not re-baselining an unexpected behavioral failure.

## Scope audit and remaining decisions

The independent design review kept this slice to already-enumerated names. A
wildcard unknown-surface flag is NOT adopted. It would affect arbitrary queried
names, including otherwise-valid mixed-module ESM `export *` fallback, and needs
an explicit owner policy. Mere `this`, `arguments` or `eval` in ESM must not be
treated as proof of CommonJS presence.

Residual observation probes (not acceptance tests) cover five still-unenumerated
shapes: computed-only writes, alias-only writes, skipped static RHS expressions,
wrapper-only writes and whole-object spreads. Static names hidden by a skipped
RHS/spread can be considered separately from arbitrary-name wildcard policy.
The general receiver audit also compares an anonymous `obj.ns=function(){ns.item()}`
with a genuinely named `function ns(){...}` and a differently named property.
Matched base/candidate residual replays produced 48 unchanged observations using
byte-identical external probe locks: 30 unenumerated-CJS rows still select the
other star branch, six anonymous-property receiver rows have no Exact edge, six
explicit-self-name rows correctly refuse, and six differently named property
controls resolve. These are observations, not 30 demonstrated runtime bugs;
some uncertainty fixtures contain unresolved names. The general lexical predicate
was deliberately not changed in this refusal slice.

WRONG, deferred: an anonymous `obj.ns=function(){ns.item()}` does not bind `ns`,
yet its inferred display name hides the imported namespace from receiver lookup.
Three additional Node ESM runtime controls return 42 for this case and the
differently named property, and throw TypeError for the explicit `function ns`
self-binding control. Source mechanism plus matched runtime/analysis controls
establish the recall defect independently of this increment.

Recommended next implementation: bounded source-only self bindings in general
receiver lookup, with explicit named-expression and lexical-shadow negatives.
Then handle skipped static CJS names independently of unknown-name wildcard policy.
No require-time snapshots, forwarding/alias expansion, closure admission, React.FC
or react-scripts disposition changes.

## Verification

Default Rust: 4,456 passed; MCP: 4,649 passed; MCP plus detached-owner-audit:
4,672 passed. Zero failures; one existing `slice_elem_variant_reserved` ignore
in each full configuration. Examples: 32 passed. Node: 786 passed;
authority: 40 passed; Python: 940 passed, one explicit live-adoption skip.
Frozen release CLI/MCP/example binary hashes were unchanged across helper gates.
Formatting passed. CI Clippy completed; candidate/base sorted warning lines are
identical (256 each, including summary lines), not a warning-free claim.

Fresh-release Tier-A matrix: 159/159. Quick was attempted before final review but
timed out after 300,046 ms (code143), with no verdict or receipt-listed artifact;
it is incomplete, not passed. Three tests in
`docs/eval/receiver-closure/audit-imported-props-source.test.mjs` were excluded
because their historical `real-sites.jsonl` fixture is absent. Full multi-corpus
and a new real-repository census were not run.

Independent final source review: APPROVE, WRONG0/SMELL0, within the two-round cap.
Prism navigation returned stale43-path evidence; current source and tests supplied
the fallback, not LSP authority or a full real-site census.

Evidence: `/private/tmp/prism-cjs-refusal-amrJB2`.
