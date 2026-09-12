# Bounded callable-contained rvalue captures

Eighth local increment, base `0b345b15`, same future MR bundle; no publication.

## Defect and bounded repair

**WRONG:** QueryCursor byte ranges admit overlapping ancestor captures. When a
compact callback occurs inside an assignment, call, or return, recursive collection
of that ancestor can attribute its signature and outside sibling identifiers to the
callback. A false parameter-token Use can then precede the real argument Use in the
line/path index, causing the correct argument-byte guard to refuse boundary flow.

One JS/TS/TSX-only predicate now requires the entire captured node to be inside the
requested callable (or root). It gates assignment and call captures in string,
structured-path and span rvalue extractors, plus return captures in the span
extractor: seven query gates. String extraction feeds full-flow fallback; spans
feed DFG construction and reaching-definition occurrence lookup. Current source
confirmed the consumers; Prism navigation reported43stale paths, so its snapshot
was not used as freshness authority.

Contained body assignments, augmented-assignment reads, calls and returns remain.
Whole-file roots and all non-JS language behavior are unchanged by the predicate.
It does not change recursive walking of accepted descendants, nearest-callable
execution ownership, first-per-line indexing, argument-byte containment or exact
callee parameter-token binding. It does not special-case namespace spelling.

CPG cache92invalidates altered serialized DFG/graph semantics. Navigation call-edge
cache stays52: call-target extraction/resolution and stored call-edge records are
not changed. A current-cache Hit followed by a forced-v91 Miss is tested.

## RED-first evidence and measured value

Initial raw probe enumerated72style/language/extractor rows:60contained false
signature/outside values,12return name/path controls were already clean. Required
body reads survived every row. The repair removes all60contaminations.

Three initial probe assumptions were corrected before accepting final evidence:

- The inherited observer enumerated only assignment captures, not calls/returns.
- The returned-function fixture has both outer and inner call observations. Its
  proof explicitly selects the inner named `run` and caller-owned boundary edges.
- A parameter-refusal epoch also changed origin bytes, violating an app-only change
  assertion. It remains a standalone refusal; the final seven-state epoch sequence
  changes exactly app. No source mutation is omitted from a changed set.

Final identical tests on same-machine detached production base versus candidate:
**5passed/7failed →12passed**, including direct cache-version fencing. The base
copy contains only test changes/registration; the receipt verifies production
custody and identical test bytes. Earlier partial/inadmissible probes are retained,
not represented as complete populations or newly introduced production failures.

| Population | Base → candidate |
|---|---|
|18forms ×3languages, full builds |54rows:24→42Supported;18→0ParameterTokenUse;3genuine same-line,3self-refusal,6parameter-refusal unchanged |
|7source epochs ×3languages |21full +18incremental rows; exact app-only changed sets |
|Enabled desired-flow regression |15rows,6missing →15exact input edges |
|All matched flow observations |108:51restored,57unchanged, no other disposition changes |
|Raw extractor observations |72:60contaminated →0 |
|Own-callback DFG guards |15rows: no parameter declaration tokens as Uses after repair |

These are repeated synthetic fixture/language/build observations, not51unique
real receiver sites or a corpus recall estimate. Fixtures are parsed source, not
compiler-checked Programs. The former ignored desired-flow test is enabled.
Exact argument/parameter node endpoints and confidence are asserted. Test-only
reindexing and out-of-argument substitutions run on Supported cases too, retaining
the negative after repair. The clone never changes production admission.

Whole-file root controls and six non-JS language controls pass. The nested-owner
audit still asserts its existing six observations: initializer nested returns and
assignment signature/body contamination can still be attributed to outer functions
when the captures are contained in those outer functions. That separate defect is
not fixed. Genuine same-line collisions still refuse; same-name self and unsupported
dynamic-default/destructured parameter refusals remain barriers.

## Verification and custody

Sources/tests frozen; independent round2 source review APPROVE W0/S0. Full Rust
verification completed: default4501, MCP4694, detached-owner4717passed, zero failures
and one `resolution_test::slice_elem_variant_reserved` ignore each. Examples32passed
with zero failures/ignores. Evidence root:
`/private/tmp/prism-contained-rvalue-FSGIPx`; adjacent baseline receipt pins source
hashes and matched rows. The first partial default run is preserved separately:
verification restarted after ensuring repaired cases still execute the substitution
negative. No production source changed during that test-only refinement.

Fresh-release Tier-A matrix159passes; quick timed out after300066ms without a
verdict/report, not passed. Formatting/Clippy complete:257warning lines, none in
the new test files. Authority40and Python940pass; one intentional live-adoption skip.
Three historical Node source-custody tests remain excluded because required
`/private/tmp/prism-imported-alias-O4d6E1/real-sites.jsonl` is absent.

Initial runnable Node suite:785passed/1failed with `budget_exceeded` in the package
link defining-identity test. The unchanged JS observer maps both timeout and
buffer-overflow errors to that reason; the precise budget trigger is not established.
Same-environment isolated base and candidate controls both pass (about3.5seconds).
No Rust runtime import is used on this observer path, and its source is unchanged.
One full-suite retry with file concurrency2 passed all786 runnable tests in340seconds,
retaining the same tests, limits and pinned binaries. The original failure and
controls remain recorded rather than silently re-baselined; the successful retry
does not establish the precise trigger of the initial budget refusal.

Local delivery: eighth bundle commit, parent `0b345b15`, message
`fix(js-ts): contain rvalue captures within callable spans`. No publication.
The baseline receipt validates all focused/full totals, matched source/test bytes,
base production custody and pinned binaries. The verifier's stale comparison hash
for `tests/ast/cpg_test.rs` was corrected by directly comparing live bytes with
committed base0b345b15: both are SHA388200578196506763562eb0896b842cd500c1f6e1818470e09c5272f324e543;
the file is unchanged. No full multi-corpus, live adoption or real-site census ran.

## Next

Recommend **bounded nested-callable execution-owner proof design before repair**.
Distinguish signature/default evaluation, nested body reads, captures and each
string/path/span consumer. Containment alone does not grant that ownership. Keep
the genuine same-line occurrence-index design separate. Static skipped CJS export
names remain independent. No unknown-CJS surface, require-time snapshot forwarding,
closure admission, unresolved react-scripts or React.FC decision changes.
