# Type-only / explicit-relative receiver review

Authority: merged #239 (`862166d`); owner requested continuation. Three-round
self-review cap declared before implementation; no independent-review claim.
Evidence directory: `/private/tmp/prism-type-relative-EpssQC`.

## Findings (WRONG before SMELL)

1. WRONG — inherited type-name collision loses its blocker. On parseable TS with
   `import type {Client as Alias}` and a value import of `Other as Alias`, a typed
   `x: Alias; x.m()` gets Exact `Other.m`. With a same-file `class Alias`, it gets
   Exact that class instead. The discarded type import makes the existing resolver
   see an uncontested name. This is false authority on conflicting input, not a
   claim that the conflicting program passes the TypeScript compiler. Fixed by
   retaining separate type-import facts and making failed type proof terminal.
   Same-environment base fails both scenarios; candidate passes.
2. WRONG — round-two candidate skipped a module namespace collision. For named
   type import plus `namespace Alias {}` or `namespace Alias.Inner {}`, it emitted
   Exact imported `Client.m` despite the contract's conflicting-type fence.
   Parser evidence shows `expression_statement -> internal_module`, with compound
   name for the dotted form. Fixed the wrapper traversal and root-name extraction.
   Exported namespace, class/interface/ambient collisions already passed. This is
   candidate-introduced, not attributed to merged #239.

SMELL — the bounded Excalidraw/Django syntax sample is not a corpus recall measure.
Excalidraw's typed Scene arrow-function shape has a minimized executable regression;
Django's sampled models use is a static field, not evidence of a gained method edge.
Default/namespace type imports, reexports, structural TS types, executable Python
initializers and namespace-only relative anchors remain deliberately unsupported.

## Hypothesis / probe / result log

- Round 1: if missing import proof is causal, exact-base positives fail while
  unsafe Python-anchor negatives pass. Alternative: caller/method extraction
  failure. `base-red-initial.log` has 1 pass/3 failures with extracted call sites;
  it additionally discriminates a wrong Other target on the type/value collision.
  `green-initial.log`: 4/4 after separate type and relative-package proof.
- Round 2: cache pins should fail at 61/30, not an unrelated fixture parse.
  `pins-red.log`: exact numeric mismatches (0/2); bumped CPG62/sidecar31.
  Boundary matrix enumerates all cases before retry (`boundaries-green.log`,
  `namespace-probe.log`). Wrapper hypothesis predicts expression_statement;
  alternative missing name-field predicts no named internal_module. S-expression
  output confirms wrapper plus name and rules out that alternative.
  `focused-green.log`: 83/83; `cache-green.log`: real CPG save/load/incremental
  and resolved navigation sidecar save/load, 2/2.
- Round 3: preservation and final controls, no new production changes.
  `final-controls.log`: 6/6, including arrow/captured parameters, value namespace
  preservation, constructor/shadow/duplicate exclusions, Python anchor additions,
  deletions, unsafe parent changes and competing target creation/removal. Cache
  tests also cover changed-importer duplicate-type transitions in both directions.
  The same final tests on exact-base production: `base-lib-red.log` 836 pass/4
  fail; `base-integration-red.log` 1 pass/5 fail. Failures are the two cache pins,
  missing positive/parity routes, and retained type-name collision scenarios.
  Production ast/call_graph/js_exports/resolution compare byte-identical to base
  git objects; only test hunks added to base cache files. This is the same machine,
  Rust dependency set and target directory, not a different-environment control.

## Verification and verdict

Default suite: 3,732 passed/0 failed/1 ignored (28 summaries). MCP suite: 3,922
passed/0 failed/1 ignored (30 summaries). Existing ignored test:
`resolution_test::slice_elem_variant_reserved`. Format/diff checks, all-target MCP
check and Clippy exit 0 (nonfatal warnings). Immediately rebuilt Tier-A matrix:
104/104 ok, no matrix regression or flip candidates. Immediately rebuilt quick
completed (exit 2): OER/SUT 0/0, oracle quiescent, matrix 104/104. Baseline invalid
reason: `corpus_sha_drift: 862166dba27b != pinned 20c8490591a3`; four stale
adjudications. Pinned observations: `target-c-method` = `flip_candidate`;
`module-deps-feature-gated` and `load-repo-feature-gated` = `missing`;
`ambiguous-symbol-contract` = `ok`. These are unadjudicated observations, not
attributed regressions or gains; no same-environment comparative oracle control
was run. Retained reports/snapshot in evidence `tier-a/`; no baseline rewrite.
An optional process-list status check was sandbox-refused: inadmissible for any
claim about oracle progress. Completion and output artifacts supply the evidence.
Review converged within three rounds; bounded fixes retained on the same artifact.
No rebaseline, multi-corpus run, or hosted-check success claimed.
