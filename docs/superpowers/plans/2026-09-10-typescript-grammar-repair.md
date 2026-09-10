# TypeScript grammar repair — design and RED checkpoint

Base: PR304 merge `9270ab43` (freshly fetched). This is an incomplete repair
checkpoint, not an implementation completion or a merge-ready artifact.
Primary owns architecture; two bounded read-only audits investigated upstream
grammar and call consumers. Repair review cap: two rounds; none dispatched yet.

## Authority decision — approved

The supported Rust crate consumes generated C, not grammar.js. Its two C files
total 17,515,764 bytes. A narrow local grammar edit therefore needs a distributable
patched dependency plus a generator. No tree-sitter CLI is on PATH and no CLI or
generate crate is in the inspected local Cargo registry. No ready upstream fix
was found: [upstream issue367](https://github.com/tree-sitter/tree-sitter-typescript/issues/367)
is open and matches the first-type-argument defect. Current upstream grammar
retains the same relevant precedence pairs. Search absence is not proof that no
unindexed fix exists; no verified candidate has been acquired or tested.

Owner approved pinned in-repo vendoring (~17 MiB generated C), isolated pinned
generator bootstrap, and repair of both existing call-recording defects on resume.
The original tests/docs checkpoint remains preserved at b69a0401. Implementation
is now in progress; generation provenance and full verification remain pending.

## Corrected defect population

All probes use production source unchanged from the merged base. Initial consumer
audit used an existing pre-change rlib; primary then rebuilt `cargo build --offline
--lib` in this checkout and reproduced the findings. No regression is attributed
to this tests/docs-only checkpoint.

1. **WRONG: inline import-type generic call parse failure.** Existing PR304 cases
   await/plain/parenthesized get<typeof import("./module")>() have native ERROR
   nodes in both TS and TSX, although compiler-valid. Original bytes must survive.
2. **WRONG: await/generic call structure and callee.** The previously zero-error
   number/typeof-identifier/alias controls parse a call whose function child is
   await_expression(get). Public APIs and CallGraph record callee `await get`.
   Compiler AST has get as callee with AwaitExpression outside CallExpression;
   emitted JavaScript is await get(). Plain await get() is correctly represented.
   Zero parse errors in PR304 were never proof of correct tree/call structure.
3. **WRONG: type-only calls leak into runtime observations.** Already-parseable
   `function owner() { type T = typeof import("m"); }` and parameter type annotation
   produce owner->import sites across all six public call APIs and CallGraph.
   A mixed same-line type/runtime import emits two sites; line-based argument lookup
   returns the type argument "types" instead of runtime "runtime". Real dynamic
   import is the positive control. Do not suppress based on import/typeof spelling.
4. **SMELL / conditional design hazard:** current build.rs watches/hashes src,
   build.rs, Cargo.toml and Cargo.lock only. Same-version edits in a future vendor
   directory would evade root rebuild/input identity unless explicitly included.
   No current vendored source exists, so this is not a demonstrated current cache bug.

## Tests and probe record

`tests/lang/typescript/import_type_grammar_test.rs` promotes the nine PR304 cases
to separate TS/TSX tests. Desired structure and byte ranges are checked, not just
error counts. Fresh result: **6 passed /12 failed**, with 50 pre-existing tests
filtered out. Six failures are original error nodes; six are newly measured
await/callee structure failures. Keep these failing expectations; do not mark
ignored, weaken them, or merge the WIP test-only branch.

Existing TypeScript suite control excluding the new module: **50 passed**, with
18 new tests explicitly filtered out. Full Rust/MCP/Python/observer/Tier-A gates
have not run this turn because no implementation candidate exists. PR304 totals
are historical, not current verification. Format and whitespace checks are separate.

Evidence directory: `/private/tmp/prism-grammar-repair-BR8ivP`:
structural-red.log, typescript-control.log, await-structure-probe.log,
consumer-base.log, consumer-fresh.log, consumer_probe.rs. Initial grammar-red.log
had nine grouped tests that stopped at the first dialect failure; superseded by
18 individually executed tests in structural-red.log, not an extra defect count.

Hypotheses/probes/results:

- Invalid source versus grammar ambiguity: PR304 compiler-valid fixtures reproduce
  native failures against unchanged base; malformed control still refuses.
- Missing call node versus await nested in callee: full tree shows call_expression
  with await_expression function child. Fresh call APIs/graph output `await get`;
  compiler AST and ordinary-await control distinguish this from test-only shape taste.
- Type import token rejected by name extractor versus leaking runtime call: all six
  APIs, exact spans and CallGraph prove leakage; genuine import is retained.
- Upstream ready fix versus local generation requirement: exact open issue found;
  its reported unsuccessful experiments are not our local experiments. A two-way
  conflict replacing static precedence is only an untested candidate.

## Proposed bounded sequence after approval

1. **Dependency custody/reproduction:** pin published upstream
   f975a621f4e7f532fe322e13c4f79495e0a7b2e7, exact generator/runtime and JavaScript
   grammar inputs, retain licenses/notices; isolate bootstrap from real repositories
   and global installations. Regenerate unchanged baseline first. If baseline does
   not reproduce, classify generation drift before applying a fix. Review generated
   payload as mechanical provenance, not thousands of independently authored lines.
2. **Cache identity prerequisite:** include actual vendored build inputs in rerun
   watches, tracked/untracked and gitless enumeration, dirty detection and cache
   fingerprints. Test same-version source mutation, source restoration, cold/warm
   misses and metadata. A one-time cache-version bump alone is insufficient.
3. **Grammar candidate:** repair import-type and await/call structure, preserving
   source bytes. Compare direct/member/optional/nested/new/second-argument and TSX
   ambiguity cases. Distinguish await get<T>() from (await get)<T>()/parenthesized
   callee semantics. No blanket flattening of await callee nodes or text rewriting.
4. **Runtime-call boundary:** add a TS/TSX type-ancestry refusal at shared call-name
   extraction, verified across six APIs, spans, argument indexes/manual lookup and
   full/subset graphs. Audit direct is_call_node consumers before any broader claim.
   Preserve dynamic import and runtime typeof import, string/comment lookalikes,
   and same-line mixed types/runtime sites. Syntax queries may retain type nodes.
5. **Verification/value:** all original 628 native files, exact errors/functions/
   call facts; regular suites and fresh Tier-A matrix/quick after rebuild; original
   compiler packets and source custody unchanged. Native's 286 non-parser skip
   barriers, semantic closure and react-scripts remain unresolved. No promised
   eligible-owner or recall gain merely from a successful parse.

Keep authored work in bounded increments if prerequisite/review scope cannot fit.
Never replace the existing test artifact to escape RED or review pressure.
No upstream repository writes, dependency publication, JSON/.mts/.cts admission,
closure expansion, private reads, real source execution or edits are authorized here.

## Approved implementation checkpoint — design reassessment

Owner approval received; resumed at b69a0401, remote main freshly verified9270ab43.
Isolated archive/bootstrap root: `/private/tmp/prism-grammar-implementation-aZxq3g`.
Upstream package-lock pins CLI0.24.4 and JavaScript0.23.1. Downloaded JavaScript
matches locked SRI. Unchanged TS/TSX regeneration matches every src/ byte in both
the source archive and published Cargo crate. No npm lifecycle scripts ran.

Cache identity implementation has7 green tests (six behavioral plus child harness),
after captured mutation/restore/dirty/watch/symlink RED. Runtime guard has11 tests:
all10 nongeneric tests pass before grammar repair; the generic integration passes
with import-only repair. Additional generic boundary tests have captured base
RED4pass/2fail. A direct generic-callee experiment temporarily passed79 TS tests
but failed upstream compatibility and was rejected; those79 passes are **not**
acceptance of the current checkpoint.

Grammar hypothesis/probe ledger (all original expectations retained):

| Candidate | Evidence | Result / disposition |
|---|---|---|
| Unchanged baseline | upstream-baseline-tests.log; baseline generation byte diffs |112 upstream cases pass |
| Import-only two conflicts | upstream-import-only-tests.log |112 pass; inline import syntax repaired; await association still wrong |
| Additional unary_void ordering | candidate2-rust-parse.log | simple precedence relation insufficient |
| Narrowed generic callee | upstream-candidate-diffs.log | six real regressions: heritage, instantiation, comparison, constructor; rejected |
| Preferred primary generic branch | candidate4-boundaries.log; upstream-candidate4-fresh-tests.log | same six regressions; rejected |
| call/expression conflict | candidate5-boundaries.log | generator says unused; same six; rejected |
| Right-associative await | await-assoc-rust-parse.log | still wrong; rejected |
| Preferred completed-call await operand | upstream-await-prefer-call-tests.log; await-expanded-base.log/candidate.log |112 upstream pass, but ordinary await chained calls/member access regress; rejected |

The import-only control was added before attributing six regressions specifically
to callee changes. Generated-state design inspection follows; no more grammar
edits until that mechanism is understood. The checkout retains the independently
passing **import-only** grammar, not any rejected candidate. Await regressions
remain intentionally active. No merge-ready or completion claim.

One CLI probe failed before parsing because it needs a host cache lock even with
temporary library storage; inadmissible. Direct Rust parser probes remained fully
isolated. Upstream corpus runs used explicit escalation for the standard cache
lock and temporary parser libraries, not global installs. A reproduction-script
syntax error was corrected before its first admissible run.

Newly measured separate pre-existing WRONG: import-type options can expose `with`
as a runtime rvalue in three collectors after the call guard. Compiler-valid erased
source, base/current outputs and follow-up are in
`/private/tmp/prism-type-call-repair-vrHCV1/handoff.md`. This is not a runtime call
site and is deferred rather than silently expanding this repair into DFG traversal.

Original628-file corpus baseline is saved under `corpus/` in the bootstrap root:
628 exact hashes,3 errors/2 files,8614 function records,44670 extracted calls,
45740 raw call nodes;591 skips including286 independent nonparser barriers.
Public custody unchanged. Candidate measurement and final gates remain pending.

### Design reassessment resolved; integrated candidate

The generator-state audit identified state58/index1703: expression reduction
competes with a preferred primary-callee call shift at `<`. Generator0.24.4
[handle_conflict](https://github.com/tree-sitter/tree-sitter/blob/v0.24.4/cli/generate/src/build_tables/build_parse_table.rs#L508)
discards lower static-precedence actions before consulting declared conflicts.
Thus the explicit call/expression conflict was correct but ineffectual while the
added branch carried static call precedence at its prefix. The conjectured
supertype-flattening explanation was refuted by the actual state report.

Targeted correction after this design pause: retain the original branch, give the
preferred primary/new-callee generic branch only dynamic precedence at its prefix,
and apply static call precedence at the argument list/completion. This retains
instantiation/comparison/heritage alternatives and preserves constructor priority.
No await-expression override is shipped. Candidate6 and constructor-inclusive
final candidate both pass112 unchanged upstream cases; focused structural/runtime
and new ordinary-await continuation tests pass. The expanded final suite and
independent implementation review remain gates, not inherited green claims.

Interim full default run of the import-only checkpoint:4070 passed/9 expected
await failures/1 known ignored, no other failures. Both unchanged-baseline and
import-only shipped generation reproduced via the committed verifier; final
candidate reproduction is rerun separately. Grammar trials are not review rounds;
formal review cap remains two rounds, not yet dispatched.

### Integrated verification and review correction

The preceding checkpoint descriptions are historical. Implementation f1acaff5
contains the final grammar, not import-only. Full Rust default/MCP/owner-audit
pass4082/4275/4298, one known ignored each; observer726 and authority40 pass.
Receiver helpers15/18 fail at setup because an inherited public archive lacks
App.tsx; same-environment base control and fresh exact-pin archive are pending.
Matched Python controls940 pass/1 deliberate live-model skip. Final generation
reproduction and three bootstrap negatives pass; public628 corpus comparison has
zero errors and zero unclassified changes. See the durable receiver-closure readout.

Formal round1 found one bounded identity WRONG: normalization of literal Unix
backslashes conflated distinct vendor names. New captured regression fails0/1 on
f1acaff5, then all8 identity tests pass after normalizing only MAIN_SEPARATOR.
This preserves actual Unix filename bytes and Windows separator behavior; grammar
bytes and extraction code are unchanged. Round2/final full gates follow on the
committed correction. Ancestor-walk performance is an unmeasured nonblocking SMELL.
The two-round cap is unchanged; no artifact restart or test rebaselining.

### Final source-bound closeout

At3c997c65 all full gates pass: Rust4,083/4,276/4,299 (one known ignored each),
observer726, helpers18, authority40, Python940 (one deliberate live-model skip),
membership example12, bootstrap negatives3. Reviewround2 APPROVE closes the one
identity WRONG. The unchanged helper base reproduces the stale-fixture failure;
both base/current pass under a freshly verified1,229-file exact public archive.
No helper assertions changed. All pending statements in earlier checkpoints above
are historical, superseded by the final receiver-closure readout and machine receipt.

Tier-A159 matrix cases pass; clean stable quick remains invalid due to historical
pin drift and4/30 oracle errors/probe shortfalls. Preserve flip/missing/pending
results without attribution or rebaselining. Full multi-corpus/live-model runs,
other-host regeneration and registry publication are not verified by this slice.
Next recommendation is the separately controlled erased import-options rvalue
repair, retaining real dynamic-import-option/label controls and closure barriers.
