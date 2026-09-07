# Bounded callable declaration/alias provenance

Implemented locally on feat/callable-alias-provenance, base PR260 merge
8e7744e27573ab55d0ef70dee22a6dd9eae5c935. Commit/publication is controller-owned
under the current AGENTS operation notes; no new commit, push or PR is claimed.

## Outcome and authority boundary

The opt-in configured observer now retains immediate import/re-export aliases,
qualified namespace bindings, defining-source type-alias bodies and generic
argument/parameter anchors. A bounded chain ends at a callable type or singleton
non-inherited callable interface, or retains an explicit partial/unproven reason.
Schema /1 and producer0.2.0 replace /0 and0.1.0. Old packets reject pre-I/O;
the producer digest covers the new module and validation recomputes every field.

This is source provenance, not generic substitution, instantiated class identity,
runtime write proof or Exact authority. Both authority flags remain false. No
runtime resolver, navigation, CPG, cache, Rust source or default dependency changed.
The lsp-nav workflow used pinned compiler source/API fixtures because LSP tools
were unavailable. getImmediateAliasedSymbol preserves intermediate alias evidence;
getAliasedSymbol's collapsed result is not treated as a complete chain.

## Review and captured behavioral controls

Three SELF-PASS rounds, NOT INDEPENDENT; no cap extension or restart.
Base red.log contains three behavioral failures on unchanged predecessor tooling:
missing named/barrel provenance, missing scoped namespace/generic provenance,
and failure to reject the newly obsolete schema before caller-root access. These
establish the new contract's RED, not a retroactive claim that PR260 promised it.

Closed WRONG cases found in the new implementation:

| Input / state | Incorrect result | Mechanism / bounded correction |
|---|---|---|
| Namespace/default import of a local export-equals namespace | Supported React namespace chain stopped before its declaration | Immediate alias target is ExportAssignment, not yet the namespace; anchor and allow only a direct unique local namespace gateway |
| Duplicate local type aliases | Chosen compiler symbol appeared unique and chain reported traced | Same-environment compiler probe found two syntactic declarations with distinct symbols but one selected declaration; inventory same-scope peers |
| Imported export-equals gateway | Named import jumped to a remote definition and reported traced | Record module export/binding evidence; imported gateways remain unsupported |
| Duplicate explicit re-export names | Error recovery selected one export and chain reported traced | Inventory export-name peers separately from local bindings |
| UMD export-as-namespace directive beside local namespace | First duplicate guard falsely refused the valid local binding | Directive is not a second local declaration; enumerate actual binding forms |

Round1 captured the namespace failure. Round2 captured duplicate/gateway failures,
enumerated the full ten-case negative population before retry, then corrected the
binding inventory and passed34/34. The null-terminal mutation error during that
intermediate run was an inadmissible tamper probe, not evidence of validation;
the final fixture explicitly checks its traced precondition. Round3 added four
passing adversarial controls for mixed direct/re-export collisions, local exports
versus duplicate imports, nested namespace limits, and producer digest coverage.
Despite its provisional filename, round3-red.log is GREEN, not RED evidence.

Defining-file and binder anchors distinguish same-spelled local aliases without
substituting generic arguments. Explicit-any remains independent. Controls retain
star/merged/missing/cyclic/inherited/conditional/intersection/noncallable barriers,
per-chain limits, unsafe and forged nested anchors, full recomputation and barrel
A→B→missing→A replacement. Existing acquisition and assertion negatives remain.

## Final verification

- Configured producer/validator:38 passed,0 failed,0 skipped; prior23 retained.
- Pinned React18/React19 compiler fixtures:40 passed; helper tests:4 passed.
- Full cargo test:4017 passed,0 failed,1 existing ignored,28 summary groups.
- Full cargo test --features mcp:4207 passed,0 failed,1 existing ignored,30 groups.
- Both Rust totals include2 doctests. Existing ignored case is
  resolution_test::slice_elem_variant_reserved. Existing test-code warnings remain.
- cargo fmt --check and git diff --check passed. No fresh warning-clean/Clippy claim.

Rust source/tests/Cargo inputs are unchanged; Node refinements do not alter the
tested Rust artifact. Tier-A is not triggered by standalone observer/docs changes.
Prior quick remains INHERITED baseline-invalid, not freshly green. No full
multicorpus, baseline rewrite, application tests or installed application closure
is claimed. The private read-only replay reproduced an unproven packet; private
source, names, paths and detailed measurements remain in separate local custody.

SMELL / explicit limits: trusted quiescent snapshots are not atomic filesystem
proof; namespace merges, arbitrary type evaluation, nested binding ownership and
runtime class/write proof remain excluded. Traced chain status does not upgrade
an incomplete Program's closure. No public real-receiver Exact gain is claimed.

## Custody and next boundary

Evidence root: /private/tmp/prism-alias-provenance-D0uxox/public. It contains RED,
intermediate and final logs and source checkpoints. The final public archive is
/private/tmp/prism-callable-alias-provenance-public-evidence.tgz; its external
SHA256 sidecar binds the archive without a self-referential source digest. Only
public evidence is included. The controller handoff names the scoped source files.

Recommended separately approved successor: source-backed nested callback binding
ownership observations inside supported annotated implementations, with shadow,
write and duplicate negatives first. Instantiated Props/class identity and any
runtime Exact consumer remain later, separately justified boundaries.
