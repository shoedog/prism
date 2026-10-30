# Receiver source self bindings

Sixth local increment, base `b3f6441e`, same future MR bundle. No publication.
Historical delivery: `7bdf9981`. The subsequent
[ownership audit](2026-09-11-callback-flow-ownership-audit.md) classifies the
previously unclassified gap below; it is not namespace-specific. Results here
remain the sixth-increment measurement, not current verification totals.

## Reproduced defects and repair

WRONG: `import * as ns from './origin'; obj.ns=function(){ns.item()}` treats the
property-derived callable display name `ns` as a lexical self binding. Both the
raw lexical guard and the separate receiver-provenance candidate collector did
this. The latter manufactured Materialized evidence even after the raw guard was
fixed. Identical differently named callbacks resolve the same imported target;
explicit `function ns` callbacks correctly refuse it. Typed outer/inner parameter
authority was also hidden by the inferred same-name candidate.

WRONG: `const client=new Client(); obj.client=function(){client.item=()=>0;};
obj.client(); client.item(value)` can retain an Exact edge to the original class
method because the inferred callback name hides the outer receiver mutation from
the closer-binding guard. Runtime controls show the anonymous mutation returns0,
while actual named-expression/parameter shadows leave the original method at42.

Both receiver checks now use `child_by_field_name("name")` for their existing
function/declaration kinds. Parameters, real named expressions, declarations,
locals/TDZ/var, catch/loop and enclosing shadows remain barriers. The specialized
declaration-write policy remains separate. CPG91/nav52 invalidate stale semantics.
This is source-binding repair, not dynamic receiver or new type-shape authority.

## TDD, value and custody

Initial25-test probe:13pass12fail. Five failures were inadmissible site-count
assumptions: generator property expressions have no eager call sites, and two
nested-scope cases have two owner observations. The corrected probe checks all
nested observations and tests generators at the raw guard seam only:18pass7fail.
With four write controls:21pass8fail. Lexical-only implementation:22pass7fail;
adding the provenance check:28pass1fail (the independent flow limitation below).

Initial expanded33tests:23passed/10failed on base, all33passing on candidate.
The final caller audit added three constructor-identifier controls because the
same provenance helper also gates `new Client()` inside a callback displayed as
`Client`. No further production change was required; full test verification was
refreshed after these additions, preserving the initial default run separately.

Final identical tests on detached base/candidate: **25passed/11failed → 36passed**.
Across185 matched synthetic call observations:39Exact restorations,6revocations,
140unchanged, no other target changes. Candidate has198rows;13 additional typed rows are not matched
because base tests stop at their first failing assertion. Do not count those as
matched gains. These are fixture/language/build observations, not real-site counts.
The receipt pins bytes and totals. Nine Node runtime controls pass.

Six-state source epochs verify full/incremental CPG call targets, exact app-only
changed sets, and repeated anonymous/named removal and restoration. Positive
cross-file parameter flow is NOT claimed. Negative states have no such flow.
Tests and source are frozen for full verification; source-checkpoint.tgz holds
the intermediate custody snapshot. All preceding bundle controls remain in scope.

## Explicit limits and next work

The original epoch test expected argument-use to parameter-token Def flow from a
property-assigned namespace callback. A differently named, already-resolved callback
produces no cross-file flow on both base and candidate, though its argument Use
variables exist. `base-epoch-probe.log` and `candidate-epoch-probe.log` capture the
same failing expectation. This predates the repair; its downstream mechanism has
not been classified. The test now accepts exact target transitions and logs positive
flow observations instead of claiming dataflow completion. This is a corrected
test assumption, not a re-baseline of a newly introduced failure.

Generator/async-generator property-expression raw self-name controls pass, but no
eager call-site recall gain is claimed for those forms. Runtime iteration controls
confirm their source binding semantics without expanding extraction.

Next recommendation: a bounded source-backed classification of the namespace
callback boundary-flow gap, with a plain named-function control and exact argument/
parameter endpoints, before choosing a repair. Static skipped CJS export keys
remain the next independent export-side item. Unknown-surface wildcard policy,
require-time forwarding, closure admission, React.FC and react-scripts are unchanged.

## Verification

Default Rust:4492passed; MCP:4685passed; MCP plus detached-owner-audit:4708passed.
Zero failures and one existing `resolution_test::slice_elem_variant_reserved`
ignore in each full configuration. Examples32passed. No prior totals inherited.
Node786, authority40 and Python940 passed; one explicit live-adoption Python skip.
Frozen CLI/MCP/example binary hashes were unchanged through the helper runs.
Formatting passed. All-feature Clippy completed; base/candidate sorted warning
lines match exactly (257 including summary lines), not a warning-free claim.
Fresh-release Tier-A matrix159/159; quick timed out after300142ms, code143, no
verdict or receipt-listed artifacts. It is incomplete, not passed.

Independent source review approved within two rounds: W0/S1 at final review,
with the sole note being final documentation totals/status refresh, addressed by
root during closeout. Round1's cache wording concern was clarified: the changed
false-mode receiver-write consumer is distinct from the prior true-mode repair.
No source changes after final review; no review extension.

Prism navigation returned a stale43-path warning. Current source and matched tests
supplied the fallback, not LSP or a real-repository census. Historical real-sites
fixture is still absent; three tests in `audit-imported-props-source.test.mjs`
remain explicitly excluded. Full multi-corpus is owner-triggered and not run.

Evidence: `/private/tmp/prism-receiver-self-6fQMsF`.
