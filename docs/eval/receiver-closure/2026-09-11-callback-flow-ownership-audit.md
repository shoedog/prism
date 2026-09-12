# Callback argument-flow ownership: classification and repair requirements

Seventh local increment, production base `7bdf9981`, same future MR bundle.
Historical delivery `0b345b15`. The subsequent
[contained-capture repair](2026-09-12-contained-rvalue-captures.md) supersedes its
next-work/ignored-test guidance; observations and receipt here remain base evidence.
Only a test-module registration, audit tests and documentation change. No production
repair, cache bump, new call admission or real-repository value claim.

## Findings and causal discrimination

**WRONG — false parameter Use and missing boundary flow.** In
`import * as ns from './origin'; const obj={ns:null};obj.ns=function(value){return ns.item(value);};`,
with `export function item(input) { return input; }`, the call resolves exactly but
has no argument-to-parameter edge. The indexed `value` Use is the declaration token
at68..73; the actual argument is90..95. The exact callee parameter Def exists at21..26.
This reproduces on unchanged production, not a regression attributed to this audit.

The chain is source-backed and tested at each seam:

1. `src/ast.rs::rvalue_identifier_spans_on_lines` queries the file root with a
   callable byte range. An overlapping ancestor assignment capture survives the
   line and assignment-kind gates. Recursive RHS collection visits the signature.
2. `src/data_flow.rs` retains both the false declaration Use and true argument Use.
   The AST span extractor sorts them by line/start/end before DFG ingestion, putting
   the false occurrence first on the same line.
3. `src/cpg/build.rs` indexes the first occurrence per file/function/start-line/
   line/path/access, selecting the false parameter token.
4. `argument_var_node_in_span` correctly rejects it as outside the call argument.
   Neither source argument lookup nor exact callee parameter binding is missing.

A test-only graph/index clone selects the already-retained true DFG occurrence;
existing Step5b then emits the exact input edge. An out-of-span substitution still
refuses. This discriminates index selection from absent DFG evidence. It does not
prove a production extraction patch, general occurrence policy or ownership theorem.

**WRONG — separate nested-owner observations.** Within `outer`, a nested function
return is collected as an outer rvalue by the span extractor. Assignment-expression
callbacks additionally expose nested signature Uses and nested RHS paths. The
initializer control has no signature phantom; the span return collector still
crosses its nested callable boundary. These tests establish wrong extractor
ownership, not a newly measured false exact cross-file edge.

**SMELL — broader collision/ownership design.** A genuine earlier same-line
`sink(value)` also prevents later `ns.item(value)` argument selection, without a
phantom token or escaping capture. A different-line control flows. The present
first-per-line model cannot represent both occurrences through this index; the
existing containment refusal is safe. Whether to expand occurrence identity is a
separate design decision. Defaults, destructuring and nested execution scopes need
explicit proof requirements before changing their extraction semantics.

## Corrected scope and executable baseline

This is neither namespace-specific nor callback-wide. Named-import callbacks fail
too; named declarations and multiline calls flow. A multiline callback may retain
a false Use on its header line without blocking the later argument line. Moving
the assignment head to the prior line prevents that ancestor capture's line gate.

Initial hypotheses also included simple const function, const arrow and const
object-pair initializers. The complete probe falsified all nine language/style
expectations: these controls flow, with no escaping accepted assignment or phantom
parameter Use. Mechanism-level exclusion: the query captures `variable_declarator`,
but JS/TS/TSX `Language::is_assignment_node` accepts only assignment expressions and
augmented assignment expressions. Do not inherit the broader initial hypothesis.

The committed test module is `src/cpg/namespace_flow_audit_tests.rs`. It contains:

| Population | Current observations |
|---|---|
|14forms × JS/TS/TSX, full builds |42:24Supported,9ParameterTokenUse,3GenuineSameLineUse,3RefusedTarget,3RefusedParameter |
|5source epochs per language |15full +12incremental; changed source set is exactly app |
|2nested styles ×3languages |6extractor ownership observations with genuine RHS/return controls |
|Desired-behavior regression |15rows;9flow controls,6missing edges; captured RED and explicitly ignored |

Normal tests assert the classified current behavior, not a repair. The ignored
`namespace_flow_compact_callback_requires_argument_edge` is the desired positive
contract; run it explicitly with `cargo test --offline --lib
namespace_flow_compact_callback_requires_argument_edge -- --ignored --nocapture`.
It intentionally fails until the bounded repair. Explicit self-binding refusal,
unsupported parameter refusal, byte containment and real earlier-use collisions
remain independent controls. Production stays CPG91/nav52.

## Next bounded implementation and acceptance requirements

Recommend **caller-contained rvalue query captures first**, separately from nested
callable execution-scope repair and genuine same-line occurrence indexing.

- Capture matching RED for affected assignment forms, including the assigned-arrow
  form not yet covered here. Assert no signature-token rvalue at raw/DFG seams and
  restored exact input edges at the CPG seam, in full and incremental builds.
- Define containment consistently for the path/span siblings and inspect their
  actual consumers. Preserve true in-callable assignment, augmented-assignment,
  call-argument and return reads. Include non-JS controls if shared code changes.
- Keep const initializer controls, line placement, explicit self shadows,
  unsupported/default/destructured parameter refusals and out-of-span substitution
  negatives. Do not treat the current simple fixture set as complete scope proof.
- Preserve first-per-line indexing and argument-byte containment. The genuine
  same-line collision remains refused; no namespace spelling special case.
- Audit actual affected cache consumers and update versions if semantics change.
  Capture full-suite totals, helpers and fresh Tier-A before review.

Nested callable ownership needs a subsequent explicit execution-scope design:
body, signature/default evaluation, closure captures and path/span asymmetry are
not all solved by containing an ancestor query capture. Static skipped CJS export
names remain another independent item. Unknown-CJS surfaces, require-time snapshot
forwarding, closure admission, unresolved react-scripts and React.FC are unchanged.

## Verification and custody

Focused audit:3passed/0failed/1known-gap ignore; explicit desired-behavior run:
0passed/1failed with all six missing rows enumerated. Initial incorrect classification
expectations are retained in the evidence logs, not silently presented as successes.
Round2 review found two closed audit-test defects: the desired-behavior assertion
accepted a count without exact endpoints/confidence, and the nested initializer
path refusal was logged but not asserted. Both were corrected on the same artifact
at the declared two-round cap. The desired test now reuses strict `observe` and
requires Supported; nested path presence equals the expected disposition for both
styles. Focused GREEN and desired RED were refreshed; no third independent approval
is claimed. Earlier logs/snapshots remain evidence, not final-source verification.

Post-review Node786, authority40 and Python940 pass; Python1intentional live-adoption skip.
Three tests in `audit-imported-props-source.test.mjs` are excluded because required
historical `/private/tmp/prism-imported-alias-O4d6E1/real-sites.jsonl` is absent.
Post-review fresh-release Tier-A matrix159passes. Quick timed out after300024ms without a
verdict or report; it is not passed. Its generated oracle snapshot was preserved
under the evidence root rather than committed. Full Rust verification was
refreshed after the two test-only corrections. Post-review release CLI hashes
differed, so helpers/matrix/bounded quick were repeated against the exact new
binaries. Both rounds of evidence are preserved. No full multi-corpus/live adoption run.

Final Rust totals: **4495default /4688MCP /4711MCP+detached-owner-audit passed**,
zero failures, two ignored per configuration. Ignores are the pre-existing
`resolution_test::slice_elem_variant_reserved` and the explicitly RED desired-flow
regression. Examples32pass/0fail/0ignore. Formatting and Clippy complete; Clippy
reports257warning lines, none mentioning the new audit file. All final source/test
hashes match the receipt. Delivered in the seventh local bundle commit after
`7bdf9981`, without push or PR publication.

Evidence: `/private/tmp/prism-namespace-flow-MhHdDf`. The adjacent baseline receipt
pins final test/source hashes and observation rows. No new real-repository census
or compiler-valid TS-program claim: these are parsed synthetic fixture observations.
Prism navigation returned43stale paths and a truncated12/24callee set; current
source and executable probes provide the mechanism evidence, not navigation completeness.
